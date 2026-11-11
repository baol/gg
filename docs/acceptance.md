# Acceptance tests

Purpose: list Git Good's user-visible behavior as acceptance cases, each run against a named fixture.

Owners: the behavior is specified in [architecture](architecture.md), [final design](final-design.md), and [CLI](cli.md); when they change, update the matching cases here. Automated evidence is mapped in [regression](regression.md); results are recorded in [verification](verification.md).

Each case has an ID (area prefix and number), a fixture, a layer, Given/When/Then steps, and a **Covered by** line. Window size is 1440×900 unless a case says otherwise; *narrow* means 720×480. A case reading `— see XXX-NN` is a duplicate kept so its ID stays valid.

- **Layer:** *product* (a task completes, consequences are clear, work is preserved and recoverable), *interaction* (selection, focus, shortcuts, stale results, narrow layouts), or *visual* (sizes, colors, placement, exact copy). Product cases come first when time is short.
- **Covered by (kind):** the tests that check the case today and where they run: *backend* (Git-level tests in `crates/`), *interaction* (egui input and geometry tests in `apps/git-good/src`), *end-to-end* (the `git good` binary in `apps/git-good/tests/cli.rs`). *(partial)* means the tests check only part of the case, for example a GUI case backed only by Git-level tests, or a dialog that is rendered but not completed. *Not automated* means no test yet. A test (`every_test_acceptance_cites_exists_where_it_says`, apps/git-good/tests/acceptance_cites.rs) fails when a cited test does not exist in the file named in the parenthesis after it (`(diff_view.rs)`, `(crates/engine)`), or anywhere when none is named, so a renamed or deleted test cannot leave a case looking covered.

Areas: [Frame and toolbar](#frame-and-toolbar) (FRM) · [History tree and refs](#history-tree-and-refs) (HIS) · [Review and reader](#review-and-reader) (REV) · [Local Changes](#local-changes) (CHG) · [Network](#network) (NET) · [Stash](#stash) (STA) · [Merge, cherry-pick, revert and conflicts](#merge-cherry-pick-revert-and-conflicts) (INT) · [Rewrite and recovery](#rewrite-and-recovery) (RWR) · [Preferences](#preferences) (PRF) · [Worktrees, submodules and LFS](#worktrees-submodules-and-lfs) (WTR, SUB, LFS) · [Signing](#signing) (SIG) · [Repository dialogs](#repository-dialogs) (DLG) · [CLI](#cli) (CLI) · [Shared rules](#shared-rules) (RULE) · [Cross-cutting scenarios](#cross-cutting-scenarios) (XCT) · [CLI contract](#cli-contract) (CLI-C) · [Spec decisions](#spec-decisions--2026-11-04)

## Fixtures

Every fixture is a disposable directory built from scratch by a script with fixed author, committer, dates (`GIT_AUTHOR_DATE`/`GIT_COMMITTER_DATE` from 2026-10-01T10:00:00Z, +1 h per commit), `user.name "Ada Lovelace"`, `user.email ada@example.com`, `core.autocrlf false`, `init.defaultBranch main`, and no global or system Git config (`GIT_CONFIG_GLOBAL=/dev/null`, `GIT_CONFIG_NOSYSTEM=1`). Git Good runs with `GG_OFFLINE=1` and an empty `XDG_CONFIG_HOME` (so default preferences) unless a case says otherwise. Hashes in cases are named by commit subject, never by literal ID.

| ID | Contents |
| --- | --- |
| `fx-not-repo` | An empty folder that is not inside a repository. |
| `fx-unborn` | `git init` only: no commits, branch `main` unborn; one untracked file `README.md`. |
| `fx-linear` | 30 commits on `main` (`c01 Initial commit` … `c30 Update docs`), two authors (Ada Lovelace, Grace Hopper `grace@example.com`), multi-paragraph messages on c10 and c20, lightweight tag `v0.1.0` on c10 and annotated tag `v0.2.0` on c20. Clean worktree. |
| `fx-branchy` | `fx-linear` plus: `feature/login` (4 commits off c25, not merged), `topic` merged into `main` with a merge commit `Merge branch 'topic'`, `old` fully merged and deletable, tag `v1.0.0` on HEAD. HEAD on `main`. Variant `fx-branchy-detached`: HEAD detached at c28. |
| `fx-dirty` | `fx-linear` with: `a.txt` partially staged (two hunks, one staged), `b.txt` staged, `c.txt` unstaged edit, `d.txt` deleted unstaged, `e.txt` renamed to `e2.txt` and staged, `run.sh` mode change 644→755, `logo.png` binary edit, `crlf.txt` line-ending-only change, `ws.txt` trailing-whitespace-only change, untracked `new.txt` and untracked folder `notes/` with 2 files, ignored `build/out.o`. |
| `fx-remote` | A bare `origin.git` and a clone of it. Local `main` is 2 ahead and 3 behind `origin/main`; `origin/release` exists with no local branch; local `gone` tracks a deleted remote branch; local `local-only` has no upstream. The clone's `origin` URL is a local path, so no network is needed. Variant `fx-remote-even`: `main` even with `origin/main`. |
| `fx-shallow` | `git clone --depth 5 file://…/fx-linear`. |
| `fx-bare` | `git clone --bare` of `fx-linear`. |
| `fx-stash` | `fx-linear` with three stashes: `stash@{0}` an edit to `a.txt`, `stash@{1}` with an untracked file (`-u`), `stash@{2}` made on branch `feature/login`; worktree clean. Variant `fx-stash-dirty`: same with an unstaged edit that conflicts with `stash@{0}`. |
| `fx-merge-conflict` | `main` and `feature/login` both edit `src/app.rs` lines 10–12; `docs.md` deleted on one side and modified on the other; `logo.png` changed on both sides. `git merge feature/login` stopped with these three conflicts. |
| `fx-cherry-pick-conflict` | A `git cherry-pick` of `feature/login~1` onto `main` stopped with a text conflict. |
| `fx-revert-conflict` | A `git revert c27` stopped with a text conflict. |
| `fx-rebase` | `main` 6 commits ahead of `origin/main` in an `fx-remote-even`-style clone, including a commit touching `folder_picker.rs`, a commit with a branch `wip` on it, and one with tag `v0.1.39`. Used for interactive rebase plans. Clean worktree. |
| `fx-rebase-edit` | `fx-rebase` with `git rebase -i` stopped at an `edit` step (step 3 of 6). |
| `fx-rebase-conflict` | `fx-rebase` with a rebase stopped on a conflict at step 4 of 6. |
| `fx-rewrite-reopened` | `fx-rebase` with `git good --cli rewrite --edit` stopped at a reopened commit (its changes staged), and the private backup ref written by gg. |
| `fx-backups` | `fx-branchy` with three Git Good backup refs (`refs/gg/backup/…`) named 2 days, 40 days and 120 days old, and a reflog containing a reset, an amend, a rebase and a branch switch. |
| `fx-file-history` | `src/lib.rs` with 12 versions across 12 commits, renamed from `src/old.rs` at version 6, a line range (lines 20–30) changed in 4 of them; `img/logo.png` with 3 versions of different sizes; a file over 2 MiB `big.log`; a file with 120,000 lines `long.txt`; a binary `blob.bin`; a symlink `link → src/lib.rs`. |
| `fx-hooks` | `fx-dirty` with a `pre-commit` hook that fails with output `lint: 2 problems`, a `commit-msg` hook that rejects subjects starting with `wip`, and a `prepare-commit-msg` hook that appends a trailer. |
| `fx-no-identity` | `fx-dirty` with `user.name` and `user.email` unset. |
| `fx-worktrees` | `fx-branchy` with linked worktrees `../wt-login` (on `feature/login`), `../wt-locked` (locked, reason `on usb disk`), and `../wt-gone` (its folder deleted, prunable). |
| `fx-submodule` | A superproject with submodule `vendor/lib` (pointing to a local repo) at a commit behind the submodule's `main`, and a second submodule `vendor/uninit` not initialized. |
| `fx-lfs` | Requires `git-lfs`. `*.psd` tracked by LFS: `art.psd` with content available locally, `old.psd` as a pointer whose object is missing; a local file-based LFS remote holding `old.psd`. |
| `fx-signed` | SSH signing key in the fixture (`gpg.format ssh`, `user.signingkey`, `gpg.ssh.allowedSignersFile`): a good signed commit, a commit signed by a key not in allowed signers, an unsigned commit, a signed annotated tag. |
| `fx-nested` | A repository with another repository inside `vendor/nested` (not a submodule) and a linked worktree; used for path and label resolution. |
| `fx-large` | 100,000 linear commits, packed (`git gc`), generated by `scripts` tooling; opt-in performance cases only. |
| `fx-prefs-*` | An `XDG_CONFIG_HOME` holding a `gg/settings.json`: `fx-prefs-custom` (light theme, comfortable density, side-by-side diffs, avatars on, two clone folders), `fx-prefs-malformed` (one key with an invalid value and one unknown key from a newer Git Good). |
| `fx-reset-collision` | `fx-dirty` plus untracked `later.txt`, a path that is tracked in c05 and absent at HEAD (a Hard reset to c05 would overwrite it). |
| `fx-rebase-autostash` | `fx-rebase` with a staged edit and an unstaged edit to `folder_picker.rs` that conflict with the rewritten commits when the stash is restored, plus untracked `scratch.txt`. |
| `fx-recent` | An `XDG_CONFIG_HOME` whose `gg/settings.json` holds 12 recent repositories (10 existing fixture paths, 1 deleted folder, 1 not-a-repo folder). |
| `fx-clone-root` | A folder holding two repositories and one plain folder, for clone-folder repository counts. |
| `fx-wide-commit` | `fx-linear` plus one commit touching 2,500 files (the `2000+ files` stats cap). |
| `fx-many-lanes` | `fx-branchy` plus 6 unmerged branches off c05 with interleaved dates (graph widening, pill overflow on one commit). |
| `fx-review-edge` | `fx-linear` plus an empty commit; a commit adding Unicode, backslash, non-UTF-8 and tab paths; a file without final newline; a whitespace-only edit; a 400-character line; a `chmod +x`; a symlink retarget; a file→symlink type change; and a merge whose second parent's tree equals the merge tree. |
| `fx-file-history-merge` | A file whose content is carried into a merge by both parents through different commits (ambiguous predecessor). |
| `fx-remote-auth` | `fx-remote-even` with `origin` at a local HTTP server that answers 401, URL carrying `ada:s3cret@`; no credential helper. |
| `fx-remote-slow` | `fx-remote-even` whose `origin` is reached through a `GIT_SSH_COMMAND` wrapper that prints progress then sleeps 30 s (cancel and serialization). |
| `fx-merge-conflict-multi` | `main` and `side` both edit `src/many.rs` in three regions; `git merge side` stopped with three conflict blocks in one file. |
| `fx-remote-rewritten` | A bare `origin.git`, a clone `theirs`, and a clone `mine` whose `main` holds `my work` on top of `add a · add b · add c`; `theirs` then replaced `add c` with `add c (rewritten)` and force-pushed, and `mine` fetched (Pro Git 3.6, *The Perils of Rebasing*). |
| `fx-merge-revert` | `main` with `topic` (`topic t`, `topic t2`, adding files `t` and `t2`) merged by the merge commit `Merge branch 'topic'` after `main` gained `main c` (file `c`) (Pro Git 7.8, *Undoing Merges*). |
| `fx-attributes` | `.gitattributes` with `*.dat binary` and `*.min.js -diff`; text files `data.dat`, `app.min.js` and `note.txt`, all three changed by the last commit (Pro Git 8.2, *Identifying Binary Files*). |
| `fx-onto` | Pro Git 3.6's *More Interesting Rebases*: `master` `C1 · C2 · C5 · C6`; `server` off `C2` with `C3 · C4 · C10`; `client` off `C3` with `C8 · C9`; HEAD on `client`. |
| `fx-gpg` | A temporary `GNUPGHOME` with an ed25519 GPG key for `Ada Lovelace <ada@example.com>` and no passphrase, set as `user.signingkey` (Pro Git 7.4). Requires `gpg`. |
| `fx-fork` | A bare project, a bare fork cloned from it, and a clone of the fork with the project added as `upstream`; the project then gains `upstream work`; topic branch `featureA` with one commit and no upstream (Pro Git 5.2, *Forked Public Project*). |
| `fx-split` | `master`: `Initial commit` (`README`), `Update README formatting and add blame` (`README2`, `lib.rb`), `Add cat-file` (`version`) (Pro Git 7.6, *Splitting a Commit*). |
| `fx-rebase-local` | `fx-rebase` on a branch without an upstream (the post-rewrite follow-up falls back to **Recovery…**). |
| `fx-signed-bad` | `fx-signed` plus a commit whose signed payload was altered so verification reports bad. |
| `fx-lfs-image` | `fx-lfs` plus a small PNG and an object over 2 MiB tracked by LFS, and a malformed pointer `bad.psd`. |

## Frame and toolbar

Sources: docs/architecture.md (Launch, Browser state and keyboard, Presentation, empty states), docs/final-design.md (Window frame, Command placement, Search, Density), docs/cli.md (Launch and inspect). Shortcuts written with Ctrl; on macOS read Cmd.

### Launch and window

### FRM-01 Launch from a nested directory opens the enclosing repository
Fixture: `fx-linear`
Layer: product
- Given cwd is `<fx-linear>/src` (create the folder in Given if absent)
- When `git good` is run with no path
- Then the window opens on the History tree of `fx-linear` and loads rows immediately; the title reads `Git Good — <path of fx-linear>`
Covered by (end-to-end, partial): `git_dispatch_discovers_current_and_nested_repositories` (apps/git-good/tests/cli.rs, CLI discovery); GUI part Not automated

### FRM-02 Explicit path overrides cwd
Fixture: `fx-linear`, `fx-branchy`
Layer: product
- Given cwd is inside `fx-linear`
- When `git good <fx-branchy>` is run
- Then `fx-branchy` opens (title names its path; `feature/login` exists in the Branch switcher)
Covered by (end-to-end, partial): `explicit_path_overrides_cwd_and_failure_uses_stderr` (apps/git-good/tests/cli.rs, CLI); GUI part Not automated

### FRM-03 Launch in a folder that is not a repository shows the welcome view
Fixture: `fx-not-repo`, recent list `[fx-linear, fx-branchy, <missing folder>]`
Layer: product
- Given cwd is `fx-not-repo`
- When `git good` is run
- Then the History tree is replaced by a centered 640 px welcome column: the 36 px logo, **Open a repository**, the weak line `Git Good <version> · Git <version>`, one amber line `<~path> is not a Git repository.` (warning glyph, truncating; full path and Git's message on hover) with **Initialize here…**, three equal cards **Open…** (filled, `A folder on this computer · Ctrl+O`), **Clone…** (`From a URL into <~preferred clone folder>`), **New repository…** (`git init in a folder`), and RECENT with count `3`: rows name / `~` path / branch / opened / ×, the missing folder dimmed with an amber `missing` chip
- And the toolbar shows only the search field `Search repositories and actions`: no sidebar toggle, Local Changes/History tree tabs, Fetch/Pull/Push or branch button; no refs sidebar, History tree header or inspector; the footer does not repeat the error and lists the welcome keys
- And every menu stays complete; repository commands are disabled with *Open a repository first*
- And `Ctrl+K` lists the recent repositories under REPOSITORIES and **New repository…** under ACTIONS
- When **Initialize here…**
- Then the New repository dialog opens on `fx-not-repo` (DLG-17)
Covered by (interaction): `the_welcome_frame_shows_only_search_and_the_actions`, `without_a_repository_menus_stay_complete_and_repository_commands_say_why`, `a_launch_folder_that_is_not_a_repository_offers_initialize_here`, `a_missing_recent_folder_is_marked_and_offers_remove_and_locate`, `welcome_search_lists_recent_repositories_and_new_repository` (apps/git-good/src/main.rs)

### FRM-04 Welcome view recent list caps at ten
Fixture: `fx-not-repo` with a settings file listing 12 existing recent repositories
Layer: interaction
- When Git Good launches in `fx-not-repo`
- Then the welcome view lists exactly ten recent entries, most recent first, no duplicates, and **Repository › Open recent** and the Open… dialog list the same ten (one limit, `RECENT_LIMIT`); clicking one opens it
Covered by (interaction): `one_recent_limit_for_the_welcome_list_open_recent_and_open` (apps/git-good/src/main.rs), `recent_repositories_move_to_the_front_without_duplicates` (apps/git-good/src/settings.rs)

### FRM-05 Opened outside a repository, Git Good shows the welcome view
Fixture: `fx-not-repo`, recent list `[<fx-linear>]` (a repository that still exists); a settings file from an older version may still hold `"reopen_last": true`
Layer: product
- Given I just opened Git Good (`git good`, no path)
- When the folder is not a Git repository and not inside one
- Then Git Good opens on the welcome view we designed (FRM-03), with `fx-linear` offered only in RECENT
- And no other repository, such as the last one opened, opens in its place
Covered by (interaction): `a_launch_outside_a_repository_shows_the_welcome_view_not_the_last_repository` (apps/git-good/src/main.rs)

### FRM-06 Opened in a repository's subfolder, Git Good opens that repository
Fixture: `fx-linear` with a folder `src/deep` inside it
Layer: product
- Given I just opened Git Good (`git good`, no path)
- When the folder is inside a repository's working tree (`fx-linear/src/deep`)
- Then `fx-linear` opens in the History tree; no welcome view
Covered by (interaction): `a_launch_in_a_repository_subfolder_opens_that_repository` (apps/git-good/src/main.rs)

### FRM-07 Removed: the Reopen the last repository preference — see FRM-05

### FRM-08 Unborn branch empty state — see HIS-73

### FRM-09 Bare, detached and shallow repositories are represented explicitly
Fixture: `fx-bare`, `fx-branchy-detached`, `fx-shallow`
Layer: product
- When each is opened
- Then `fx-bare` opens the History tree without Local Changes content and no error; `fx-branchy-detached` shows **detached HEAD** on the branch button; `fx-shallow` loads the History tree to its boundary and the right-aligned window identity ends with `shallow`
Covered by (Not automated)

### FRM-10 OS window title shortens the home folder
Fixture: `fx-linear` placed at `$HOME/src/fx-linear`; a second copy at `/tmp/<x>/fx-linear`
Layer: visual
- When each is opened
- Then the OS title is `Git Good — ~/src/fx-linear` and `Git Good — /tmp/<x>/fx-linear` respectively (a sibling folder like `/home/meow` is not shortened for home `/home/me`)
Covered by (interaction): `window_title_names_the_folder_and_a_stopped_rebase` (apps/git-good/src/main.rs)

### FRM-11 Window title names a stopped rebase — see RWR-51

### FRM-12 Menu-bar window identity
Fixture: `fx-linear`, `fx-shallow`
Layer: visual
- When opened at 1440 px
- Then the menu bar's identity (right-aligned with the system title bar, centred in the free gap with Git Good's, FRM-84) equals the OS title text; for `fx-shallow` it adds `· shallow`; the branch name is not repeated there
Covered by (interaction, partial): `the_branch_and_its_tracking_show_only_on_the_branch_button` (apps/git-good/src/main.rs) — branch not repeated; shallow suffix Not automated

### FRM-13 Failed repository switch keeps the displayed repository
Fixture: `fx-linear` open; `fx-not-repo`
Layer: product
- When **Repository › Open…** targets `fx-not-repo`
- Then `fx-linear` rows, selection and title stay; an error names the folder; a retry with a valid folder adopts the new repository and clears the old filter/selection
Covered by (interaction): `failed_switch_keeps_displayed_identity_and_retry_adopts_new_repository` (apps/git-good/src/interaction_tests.rs), `switching_repository_resets_filter_and_selection` (apps/git-good/src/state.rs)

### FRM-14 Repository › Open recent — see DLG-06

### FRM-15 Repository › Open… dialog — see DLG-02

### Menus

### FRM-16 Menu bar titles and order
Fixture: `fx-linear` (Linux/Windows)
Layer: visual
- When the window is shown
- Then a single 26 px row shows **Repository · Branch · Commit · Changes · View · Help**; on macOS a **Git Good** menu comes first
Covered by (interaction): `menus_follow_the_final_design_mockup`, `menu_bar_is_one_row_and_view_shortcuts_switch_views` (apps/git-good/src/main.rs)

### FRM-17 Every menu's items, order and shortcuts
Fixture: `fx-branchy`
Layer: interaction
- When each menu is opened
- Then the items equal the Command placement table: Repository: Open… `Ctrl+O` · Clone… · Open recent ▸ · Fetch `Ctrl+Shift+F` · Pull (rebase) `Ctrl+Shift+L` · Push `Ctrl+Shift+P` · Push with force-with-lease… · Worktrees… · Submodules… · LFS… · Recovery… · Preferences… `Ctrl+,` · Quit `Ctrl+Q`; Branch: New branch… `Ctrl+B` · New tag… · Switch to… `Ctrl+Shift+B` · Merge into current… · Rebase current onto… · Interactive rebase of current branch… · Set upstream… · Track remote branch… · Rename… `F2` · Delete… `Del`; Changes: Stage all `Ctrl+Shift+S` · Unstage all `Ctrl+Shift+U` · Discard all tracked changes… · Commit `Ctrl+Enter` · Amend HEAD `Ctrl+Shift+Enter` · Stash changes… `Ctrl+Shift+Z` · Stashes… · Resolve conflicts…; Commit, View, Help as tabled
- And no command appears in two menus; there is no **Backups…** or **Reflog…** item (Backups and Reflog are sources inside **Recovery…**)
Covered by (interaction, partial): `menus_follow_the_final_design_mockup`, `commands_off_the_toolbar_keep_their_menu_home_and_search_entry` (apps/git-good/src/main.rs) (labels, order and single menu home; shortcuts not checked)

### FRM-18 macOS Git Good menu moves Settings and Quit
Fixture: `fx-linear` on macOS
Layer: visual
- Then **Git Good** holds **Settings…** (`Cmd+,`) and **Quit** (`Cmd+Q`); **Repository** does not end with Preferences…/Quit; the native menu bar shows it as the app menu (FRM-88)
Covered by (interaction): `menus_follow_the_final_design_mockup` (apps/git-good/src/main.rs, macOS branch)

### FRM-19 Menus are in-window egui menus with uniform rows
Fixture: `fx-linear`
Layer: visual
- When **View** is opened and the pointer moves to **Help**
- Then Help opens without a second click; rows are 24 px with a 14 px check column, label, shortcut; the active item (current view, current density) shows a checkmark and no selected fill; on Windows and Linux no native OS menu bar entries appear (macOS: FRM-88)
Covered by (interaction): `checked_rows_show_a_checkmark_and_no_selected_fill` (apps/git-good/src/menus.rs); `moving_onto_another_title_switches_the_open_menu_and_a_click_closes_it` (apps/git-good/src/menu_tests.rs)

### FRM-20 Submodules… and LFS… are real windows
Fixture: `fx-linear`, `fx-submodule`
Layer: interaction
- When the Repository menu is opened and search is given `submodule`, then `lfs`
- Then **Submodules…** and **LFS…** sit in Repository after **Worktrees…**, each with one search Action and the same readiness (*Open a repository first* without one); they open the Submodules list (SUB-07) and the LFS objects list (LFS-08); submodule and LFS details stay reachable from **File actions** in Review and Local Changes (SUB-01, LFS-01)
Covered by (interaction, partial): `commands_off_the_toolbar_keep_their_menu_home_and_search_entry`, `menus_follow_the_final_design_mockup` (apps/git-good/src/main.rs) (menu home and search entry; the opened windows are covered by SUB-07 and LFS-08)

### FRM-21 View menu switches views and toggles panes
Fixture: `fx-linear`, 1440 px
Layer: interaction
- When `Ctrl+1`, then `Ctrl+2`, then `Ctrl+\`, then `Space` (the History tree focused)
- Then Local Changes shows, the History tree shows, the refs sidebar hides (toggle unpressed), the commit inspector hides; each View item's checkmark follows
Covered by (interaction): `menu_bar_is_one_row_and_view_shortcuts_switch_views` (apps/git-good/src/main.rs)

### FRM-22 Help › Keyboard shortcuts
Fixture: `fx-linear`
Layer: interaction
- When **Help › Keyboard shortcuts** (`?`, list focused, or Preferences › Keyboard › **Show keyboard shortcuts…**), then `reword` is typed in **Filter shortcuts**, then **Edit keys…** is clicked; reopened, `Esc`
- Then a non-modal window shows one key | action table generated from the command table, grouped by menu (Repository, Branch, Commit, Changes, View, Help), then the **Menu bar** keys (Alt or F10, each menu's Alt+letter, the open-menu keys), then by list scope (History tree list, Local Changes list, Review, Rebase list, Conflicts, Stashes), including `Ctrl+O`, `Ctrl+B`, `Ctrl+Shift+B/F/L/P/S/U/Z`, `Ctrl+1`, `Ctrl+2`, `Ctrl+\`, `F2`, `Ctrl+Enter` and `Ctrl+R` Reword… and no Refresh row; the filter keeps only rows whose key or action matches and hides empty groups; **Edit keys…** closes it and opens Preferences › Keyboard; `Esc` closes it. About Git Good is DLG-11
Covered by (interaction): `help_shortcuts_window_renders`, `keyboard_shortcuts_come_from_the_command_table`, `keyboard_shortcuts_close_on_escape_and_edit_keys_opens_preferences` (apps/git-good/src/main.rs); `groups_follow_the_menus_then_the_list_scopes`, `the_filter_matches_keys_or_actions_and_hides_empty_groups`, `no_row_offers_refresh` (apps/git-good/src/shortcuts.rs)

### FRM-80 The menu bar from the keyboard
Fixture: `fx-linear` (Linux/Windows)
Layer: interaction
- When Alt is tapped, then Right, then Down; then Esc and `Alt+B`; then Alt is held while a key, a wheel turn, a click or a switch to another window happens; then `F10`, Space
- Then the tap lights **Repository** with no menu open and the History tree shortcuts wait; Right lights **Branch** and Down opens it with its first row lit and the mnemonics underlined; `Alt+B` opens Branch at once; Alt with another key, the wheel, a click or a window switch is no tap (`Alt+A` still opens the commit actions, `Alt+V` still pages up: no menu takes A or V, View is `Alt+I`); `F10` lights Repository and Space leaves the bar opening nothing; a second tap, Esc or a click leaves the bar; a command shortcut such as `Ctrl+B` leaves it and runs; a focused text field types none of the letters the bar or an open menu took
Covered by (interaction): `alt_focuses_the_bar_arrows_move_and_down_opens_the_lit_menu`, `f10_focuses_the_bar_and_enter_opens_the_lit_menu`, `alt_with_a_key_a_wheel_a_click_or_lost_focus_is_no_tap`, `a_held_alt_repeating_is_still_one_tap`, `a_second_alt_esc_space_or_a_click_takes_the_keyboard_off_the_bar`, `alt_with_a_letter_opens_that_menu_but_never_takes_alt_shortcuts`, `a_command_shortcut_leaves_the_bar_and_runs`, `a_focused_text_field_keeps_its_text_while_the_bar_or_a_menu_has_the_keys` (apps/git-good/src/menu_tests.rs); `top_letters_are_unique_conventional_and_avoid_alt_shortcuts` (apps/git-good/src/menu_keys.rs)

### FRM-81 Mnemonics in open menus
Fixture: `fx-linear` (Linux/Windows)
Layer: interaction
- When **Repository** is opened with the mouse, then Alt is held; then it is opened with `Alt+R` and a row's underlined letter is typed
- Then a mouse-opened menu shows no underlines until Alt is held; each row's letter is unique in its menu level (word starts first); typing it runs the row, or opens its submenu with the first row lit; a disabled row's letter does nothing
Covered by (interaction): `mnemonics_are_underlined_only_with_alt_or_a_keyboard_opened_menu`, `a_letter_in_an_open_menu_runs_the_row_with_that_mnemonic_or_opens_its_submenu` (apps/git-good/src/menu_tests.rs); `item_letters_take_word_starts_first_then_any_free_letter`, `underlined_text_marks_the_mnemonic_once` (apps/git-good/src/menu_keys.rs)

### FRM-82 Open menus navigate like native menus
Fixture: `fx-linear` (Linux/Windows)
Layer: interaction
- When a menu is open and Up/Down, Right/Left, Enter/Space and Esc are pressed, and the pointer moves over rows
- Then Up/Down move one highlight, shared with the pointer and drawn like a hovered row, skipping disabled rows and wrapping; Right opens the lit submenu (its first row lit) or the next menu, Left closes the submenu or opens the previous menu; Enter or Space runs the lit item (or opens its submenu); Esc closes the menu; the commit actions popup and choice lists keep Up/Down/Enter/Esc
Covered by (interaction): `arrows_skip_disabled_rows_wrap_and_enter_runs_the_lit_row`, `right_and_left_open_and_close_submenus_and_switch_menus`, `the_pointer_and_the_keys_share_one_highlight`, `the_lit_row_and_the_focused_title_are_filled_like_hovered_ones`, `context_menus_and_choice_lists_keep_egui_keyboard_navigation` (apps/git-good/src/menu_tests.rs)

### FRM-83 Tall menus scroll and submenus wait for the pointer
Fixture: `fx-linear` (Linux/Windows), a window shorter than a menu
Layer: interaction
- When a menu taller than the window is opened; Up is pressed; the wheel turns; the pointer rests on the bottom arrow; then a submenu is opened by hovering its row and the pointer heads for it diagonally across other rows, pausing briefly, then rests on a row
- Then the menu stays below the menu bar and inside the window, scrolls with ▲/▼ arrows at its ends, keeps the lit row in view, scrolls by wheel and while the pointer rests on an arrow; the submenu stays open and its row stays lit while the pointer heads for it, and the rested row takes over after about 0.3 s
Covered by (interaction): `a_menu_taller_than_the_window_scrolls_by_keys_wheel_and_arrows`, `an_open_submenu_stays_open_while_the_pointer_heads_for_it` (apps/git-good/src/menu_tests.rs); `heading_accepts_the_triangle_toward_the_submenu_only`, `level_room_keeps_every_level_below_the_bar` (apps/git-good/src/menu_nav.rs)

### Toolbar

### FRM-23 Toolbar holds its commands in one order everywhere
Fixture: `fx-branchy`
Layer: interaction
- When shown in the History tree, Local Changes and Review, wide and narrow
- Then left to right: refs sidebar toggle, **Local Changes** tab, **History tree** tab, **Fetch**, **Pull**, **Push**, **New branch**, **Stash changes**, **Stashes**, **Interactive rebase from here**, **Recovery**, then right-aligned branch button and search; New tag, Merge into current and Interactive rebase of the current branch have no buttons (FRM-72); icons never overlap, move between views, or extend past the window edge
Covered by (interaction, partial): `window_toolbar_keeps_the_owners_buttons_in_place_across_views_and_widths`, `local_changes_and_history_share_one_navigation_strip_at_every_width`, `commands_off_the_toolbar_keep_their_menu_home_and_search_entry` (apps/git-good/src/main.rs) (History tree and Local Changes at 720 and 1120 px; Review not checked)

### FRM-24 Toolbar tooltips name the command and its menu shortcut
Fixture: `fx-remote`
Layer: visual
- When hovering each icon button
- Then the tooltip names the command and shortcut, e.g. `Fetch  Ctrl+Shift+F`, `Pull from origin/main (rebase)  Ctrl+Shift+L` (`Pull 3 from origin/main (rebase)  Ctrl+Shift+L` when behind), `Push  Ctrl+Shift+P`
Covered by (Not automated)

### FRM-25 Toolbar buttons share menu enablement and reason
Fixture: `fx-linear` (no remote, no upstream)
Layer: interaction
- When hovering disabled Pull on the toolbar and **Repository › Pull (rebase)**
- Then both are disabled with the same reason *The current branch has no upstream to pull from*; search's **Actions** entry shows the same reason
Covered by (interaction): `search_offers_frame_actions_with_their_readiness_and_refs`, `pull_is_ready_whenever_the_current_branch_has_an_upstream` (apps/git-good/src/main.rs)

### FRM-26 Pull enablement and badge — see NET-16

### FRM-27 Pull at zero behind fetches, then reports up to date
Fixture: `fx-remote-even`
Layer: product
- Given **Pull** is enabled (valid upstream, no badge)
- When **Pull** is clicked
- Then Git Good fetches `origin` first; with nothing to integrate (even, or only ahead) the footer reads `Already up to date with origin/main.` and HEAD, index and worktree are unchanged; when the fetch brings commits, Pull integrates them by the **Pull strategy** (NET-17, NET-18)
Covered by (interaction, partial): `pull_reports_already_up_to_date_after_fetching`, `pull_is_ready_whenever_the_current_branch_has_an_upstream` (apps/git-good/src/main.rs) (fetch outcome simulated; no Git run)

### FRM-28 Local Changes tab badge counts changed paths
Fixture: `fx-dirty`, then `fx-linear`
Layer: interaction
- Then in `fx-dirty` the **Local Changes** tab badge equals the number of distinct changed paths (staged, unstaged, untracked or conflicted, each path once; `a.txt` partially staged counts once; ignored `build/out.o` not counted); in `fx-linear` the tab is present with no badge
Covered by (Not automated)

### FRM-29 Stash changes disabled on a clean worktree — see STA-01

### FRM-30 Stashes count and disabled state — see STA-07

### FRM-31 Interactive rebase of current branch covers the branch, not the selection — see RWR-12

### FRM-32 Interactive rebase of current branch disabled with no own commits — see RWR-13

### FRM-33 Refs sidebar toggle shows pressed state
Fixture: `fx-branchy`, 1440 px
Layer: interaction
- When the toggle is clicked twice
- Then the sidebar hides (toggle unpressed) and returns (pressed); at widths without room the sidebar is not shown
Covered by (interaction): `refs_sidebar_appears_only_when_there_is_room` (apps/git-good/src/main.rs)

### FRM-34 Background frame summary never blocks a frame
Fixture: `fx-dirty`, `fx-stash`
Layer: interaction
- When Git Good opens, Local Changes refreshes, or the window regains focus
- Then the Changes badge, the stash count (**Changes › Stashes…**, refs sidebar) and branch-rewrite readiness update from a background read; frames keep rendering (no freeze) while it runs
Covered by (Not automated)

### Branch button and switcher

### FRM-35 Branch button names branch and tracking
Fixture: `fx-remote`, `fx-branchy-detached`
Layer: visual
- Then the button reads `main ↑2 ↓3`; detached reads **detached HEAD**; the branch and tracking appear nowhere else in the frame (no idle operation row)
Covered by (interaction): `the_branch_and_its_tracking_show_only_on_the_branch_button` (apps/git-good/src/main.rs)

### FRM-36 Switcher lists local branches with current first, checked, disabled
Fixture: `fx-branchy`
Layer: interaction
- When the branch button is clicked (or `Ctrl+Shift+B`)
- Then a popup shows a filter field, `main` first with a check and disabled, then the other local branches, then **New branch from HEAD…**; last line `Up/Down choose · Enter switch · Esc close`
Covered by (interaction): `switcher_lists_local_branches_current_first_and_filters`, `ctrl_shift_b_opens_the_branch_switcher_before_other_keys` (apps/git-good/src/main.rs)

### FRM-37 Switcher filter requires every word
Fixture: `fx-branchy`
Layer: interaction
- When typing `log feat` then `zzz`
- Then only `feature/login` remains; with `zzz` no branch rows remain (New branch from HEAD… stays)
Covered by (interaction): `switcher_lists_local_branches_current_first_and_filters` (apps/git-good/src/main.rs)

### FRM-38 Switcher REMOTE BRANCHES lists only remotes without a local branch
Fixture: `fx-remote`
Layer: interaction
- Then under **REMOTE BRANCHES** `origin/release` is listed and `origin/main` is not
Covered by (interaction): `the_switcher_lists_remote_branches_without_a_local_one` (apps/git-good/src/main.rs)

### FRM-39 Choosing a remote branch tracks and switches in one step
Fixture: `fx-remote` with clean worktree
Layer: product
- When `origin/release` is chosen
- Then no dialog opens; the footer shows `Creating tracking branch…`, then local `release` tracks `origin/release` and is checked out (HEAD is never detached); button reads `release`
- And with a dirty tracked worktree the same choice is blocked by the usual clean-worktree reason and nothing is created
Covered by (interaction): `the_switcher_lists_remote_branches_without_a_local_one` (apps/git-good/src/main.rs)

### FRM-40 Switcher keyboard cursor skips rows that cannot run
Fixture: `fx-branchy`
Layer: interaction
- When the switcher opens and Down, Down, Enter are pressed
- Then the cursor starts on the first runnable row (not the disabled current branch), moves over runnable rows only, and Enter switches to the row under the cursor
Covered by (interaction): `the_switcher_runs_the_row_under_the_keyboard_cursor` (apps/git-good/src/main.rs)

### FRM-41 Switch from switcher is validated and keeps blocked intent — see HIS-56

### FRM-42 Branch button at narrow width
Fixture: `fx-branchy`, 720 px
Layer: interaction
- Then below 1040 px the branch button is an icon inside the window; its tooltip and the switcher's current row name the branch with its ↑↓ counts, and it opens the same switcher; every toolbar button (sidebar, Local Changes | History tree, Fetch Pull Push, New branch, Stash and Stashes, Interactive rebase from here, Recovery, branch, search) fits at 720 px
Covered by (interaction): `the_branch_button_opens_the_switcher_at_both_widths`, `window_toolbar_keeps_the_owners_buttons_in_place_across_views_and_widths` (apps/git-good/src/main.rs)

### Search

### FRM-43 Search field placeholder and chip
Fixture: `fx-linear`, 1440 px then 900 px then 720 px
Layer: visual
- Then the empty field reads *Search commits, branches, files, actions* with a `Ctrl+K` chip; where space is short it reads *Search*; when too narrow it becomes a search icon
Covered by (Not automated)

### FRM-44 Ctrl+K focuses, typing opens results, Ctrl+K/Esc/click outside closes and clears
Fixture: `fx-file-history`
Layer: interaction
- When `Ctrl+K`, type `p`, then `Esc`; repeat and close with `Ctrl+K`; repeat and click outside
- Then the field gains focus, results open beneath it, and each close hides results and empties the text
Covered by (interaction): `search_field_lists_files_in_a_popup_of_one_height`, `search_filters_history_or_runs_the_chosen_action` (apps/git-good/src/main.rs)

### FRM-45 Results popup keeps one fixed height
Fixture: `fx-file-history`
Layer: visual
- When the query changes from one matching many results to one matching one
- Then the popup's size does not change
Covered by (interaction): `search_field_lists_files_in_a_popup_of_one_height` (apps/git-good/src/main.rs)

### FRM-46 Groups, order and preselected Filter
Fixture: `fx-branchy`
Layer: interaction
- When typing `v1`
- Then groups appear in order **Filter** (first, preselected), **Actions**, **Branches**, **Remotes**, **Tags**, **Files**, at most eight per group; `v1.0.0` is under Tags
Covered by (interaction): `filter_comes_first_then_groups_in_order_with_prefix_matches_first`, `each_group_is_capped` (apps/git-good/src/search.rs)

### FRM-47 Matching: every word, case-insensitive, prefix first
Fixture: `fx-branchy`
Layer: interaction
- When typing `LOGIN feat`, then `log`
- Then results contain both words in label or context ignoring case; for `log` labels starting with `log` rank above labels merely containing it
Covered by (interaction): `every_word_must_match_ignoring_case`, `files_prefer_names_that_start_with_the_query` (apps/git-good/src/search.rs)

### FRM-48 Empty query lists actions without a Filter item
Fixture: `fx-linear`
Layer: interaction
- When `Ctrl+K` with no text
- Then no **Filter** row; Actions are listed
Covered by (interaction): `empty_query_lists_actions_without_a_filter_item` (apps/git-good/src/search.rs)

### FRM-49 Enter on Filter filters the History tree
Fixture: `fx-linear`
Layer: interaction
- When `Ctrl+K`, type `docs`, `Enter`
- Then the popup closes and the History tree is filtered by `docs`; the column header reads `Commit · <count>` (`+` while counting) with a `docs` chip and clear button
Covered by (interaction): `search_filters_history_or_runs_the_chosen_action` (apps/git-good/src/main.rs)

### FRM-50 Actions run through the same dispatch and show menu home and reason
Fixture: `fx-linear`
Layer: interaction
- When typing `refs sidebar`, Down, Enter; then typing `pull`
- Then the refs sidebar toggles and the popup closes; **Pull (rebase)** is listed with its menu home and disabled with *The current branch has no upstream to pull from*
Covered by (interaction): `search_filters_history_or_runs_the_chosen_action`, `search_offers_frame_actions_with_their_readiness_and_refs` (apps/git-good/src/main.rs)

### FRM-51 Ref results inspect without moving HEAD
Fixture: `fx-branchy`
Layer: product
- When choosing `feature/login` (Branches) and `v0.2.0` (Tags)
- Then the History tree selects that ref's commit; HEAD, index and worktree are unchanged; branch button still `main`
Covered by (interaction, partial): `search_offers_frame_actions_with_their_readiness_and_refs` (apps/git-good/src/main.rs) — dispatch to Inspect; HEAD unchanged Not automated

### FRM-52 File result opens Review on the latest change
Fixture: `fx-file-history`
Layer: interaction
- When typing `lib.rs` and choosing `src/lib.rs` under **FILES**
- Then Review opens the latest commit that changed `src/lib.rs` with that file selected
Covered by (Not automated)

### FRM-53 Ctrl+F opens search from the current History tree filter
Fixture: `fx-linear` with History tree filter `docs`
Layer: interaction
- When `Ctrl+F`
- Then search opens with `docs` and Filter first; Enter keeps/applies it; there is no separate filter row
Covered by (interaction): `ctrl_f_opens_search_with_the_history_filter` (apps/git-good/src/main.rs)

### FRM-54 Narrow search icon opens results with their own field
Fixture: `fx-linear`, 720 px
Layer: interaction
- When the search icon is clicked
- Then the same results popup opens with its own text field
Covered by (Not automated)

### Footer (status line)

### FRM-55 Idle hints shorten at narrow widths instead of clipping
Fixture: `fx-linear`
Layer: interaction
- Then at 1440 px the footer reads `j k: select · Enter: review · Alt+A: actions · R: rebase from here · Ctrl+K: search · Ctrl+F: filter · Ctrl+C: copy ID`; below 1040 px `j/k: select · Enter: review · Space: details · Esc: History tree · Ctrl+F: filter` (no `copy ID`)
Covered by (interaction): `status_hints_shorten_at_narrow_widths` (apps/git-good/src/main.rs)

### FRM-56 Receipts fade after 8 seconds
Fixture: `fx-remote-even`
Layer: interaction
- When Fetch completes, or a rewrite leaves `Rewrote 4 commits · backup <ref>` with **Recovery…** and **Details…**
- Then the footer shows a check and the receipt in plain words (no `Network ·` or `History ·` prefix, the part after the first ` · ` in grey) with ×, and fades back to the key hints after 8 s; a failure stays
Covered by (interaction): `receipts_fade_after_eight_seconds`, `receipts_and_failures_read_without_prefixes` (apps/git-good/src/main.rs); `receipts_split_into_text_and_tail` (apps/git-good/src/footer.rs)

### FRM-57 Failures stay red with one recovery action, Details… and ×
Fixture: `fx-remote-auth`
Layer: product
- When Push runs and fails authentication
- Then the footer is tinted red with the failure, one recovery action, **Details…** (a non-modal, resizable window titled by the operation, such as `Push rejected · details`, with the summary line, then Git's output in selectable monospace, **Copy details**, **Close** `Esc`, and the footer's same recovery action as the filled primary), and ×; it outranks a running job and stays until dismissed or the next job starts; authentication offers **Retry push**, a non-fast-forward rejection offers **Fetch** (NET-20)
Covered by (interaction): `operation_outcome_shows_details_and_recovery`, `operation_details_window_lists_full_text`, `receipts_and_failures_read_without_prefixes` (apps/git-good/src/main.rs); `the_title_names_the_operation` (apps/git-good/src/outcome_details.rs)

### FRM-58 Network progress offers Cancel without clipping
Fixture: `fx-remote`
Layer: interaction
- When Fetch is running at 1440 px and 720 px
- Then the footer reads `Fetching origin · Receiving objects 42% · 812/1930` with a thin progress bar and **Cancel**, visible and unclipped (`Starting fetch…` until Git reports); Cancel turns the message into `Cancelling fetch…` with `waits for Git to stop` at the right (NET-32); the footer height does not change
Covered by (interaction): `the_footer_shows_network_progress_then_cancelling`, `network_cancel_button_stays_inside_the_operation_area` (apps/git-good/src/main.rs)

### FRM-59 The footer leaves operation controls to the band
Fixture: `fx-rebase-conflict`; `fx-rebase` (a running rewrite)
Layer: visual
- Then **Resolve conflicts**, **Continue**, **Skip commit** and **Abort and restore** appear only in the two-line operation band under the toolbar; a running Continue is named in the band, not the footer; during a rewrite the footer reads `Backup: refs/gg/backup/…`; panes below do not move as the stop changes
Covered by (interaction): `panes_hold_still_while_a_rebase_stop_changes`, `rewrite_progress_lives_once_in_the_operation_area`, `footer_labels_name_the_job_and_its_target` (apps/git-good/src/main.rs)

### FRM-74 Footer job labels name the target
Fixture: `fx-branchy`
Layer: interaction
- When a switch, a create, a stash-and-switch, a remote-branch checkout, a merge, or a rebase onto runs
- Then the footer reads, with a spinner, `Switching to feature/login…`, `Creating feature/x…`, `Stashing 3 files and switching to feature/login…`, `Creating other from origin/other…`, `Merging origin/main into main…`, `Rebasing main onto origin/main…`; a merge or rebase that never stops shows no band and finishes with a receipt (`Merged origin/main into main · 3 commits`, `Rebased main onto origin/main`); the receipt shows as soon as Git returns and the commit count, counted afterwards, joins it without restarting its fade (RULE-16)
Covered by (interaction): `footer_labels_name_the_job_and_its_target`, `the_switcher_runs_the_row_under_the_keyboard_cursor`, `rebase_worker_keeps_frames_rendering_while_a_hook_waits` (apps/git-good/src/main.rs)

### FRM-75 A blocked switch is amber with Stash and switch
Fixture: `fx-dirty`
Layer: interaction
- When a switch to `feature/rebase-ui` is blocked by 3 changed files
- Then the footer is tinted amber: `Can’t switch to feature/rebase-ui: 3 changed files would be overwritten` with the primary **Stash and switch** (**Retry switch** when stashing is not the fix) and ×, which clears the retained intent
Covered by (interaction): `a_blocked_switch_offers_stash_and_switch_in_amber`, `created_branch_is_reported_when_the_switch_is_blocked` (apps/git-good/src/main.rs)

### FRM-76 Refs drawer below the docked sidebar width
Fixture: `fx-branchy`, narrow
Layer: interaction
- Given the History tree at 720 px
- When the user clicks the toolbar's sidebar toggle (or presses `Ctrl+\`)
- Then the refs sidebar's content opens as a 248 px drawer at the left over the dimmed list, with focus in **Filter refs**; Up/Down move an outline over the branches (any matching ref while filtering); `Enter` selects that ref's commit in the History tree and closes the drawer; `Esc`, `Ctrl+\`, or a click outside closes it; right-click rows keep the branch actions
Covered by (interaction): `narrow_refs_drawer_opens_over_the_list_jumps_and_closes` (apps/git-good/src/main.rs)

### Automatic refresh

### FRM-60 No Refresh command; Ctrl+R is Reword
Fixture: `fx-linear`
Layer: interaction
- Then no menu or search Action is named Refresh; `Ctrl+R` with a commit selected opens **Reword…** in the interactive rebase
Covered by (interaction, partial): `menus_follow_the_final_design_mockup` (apps/git-good/src/main.rs) — menu contents; Ctrl+R Not automated

### FRM-61 External file edit refreshes status after settling
Fixture: `fx-linear`
Layer: interaction
- When `a.txt` is edited by another process, with the window idle and no input
- Then about 300 ms after the last event the Local Changes badge shows `1` and Local Changes lists `a.txt`, without waiting for the next input; the History tree is not reloaded; the previous content stays until the new content arrives
Covered by (interaction): `a_real_repository_reports_a_changed_file`, `a_batch_reports_after_it_settles_or_when_flushed` (apps/git-good/src/watcher.rs); `changes_made_outside_show_in_an_idle_window` (apps/git-good/src/idle_tests.rs)

### FRM-61a A repository that cannot be watched says so
Fixture: `fx-linear`
Layer: interaction
- When the watcher cannot start for the open repository (inotify out of instances)
- Then a warning reads *Outside changes are not shown automatically: Git Good cannot watch this repository. It re-reads when the window gets focus.* with the system's reason in Details…, once per repository; each time the window gets focus back it reloads the History tree and status
Covered by (interaction): `a_repository_that_cannot_be_watched_is_told_once_and_reread_on_focus` (apps/git-good/src/main.rs)

### FRM-62 External ref change reloads the History tree; objects, logs, locks ignored
Fixture: `fx-linear`
Layer: interaction
- When `git commit --allow-empty -m ext` runs outside Git Good; separately `git gc` runs
- Then the commit appears in the History tree after settling with selection preserved by ID, also in an idle window with no input (as does a branch made with `git branch`); `git gc` alone (objects/packs, logs, locks) triggers no History tree reload
Covered by (interaction): `paths_are_sorted_into_worktree_index_and_refs` (apps/git-good/src/watcher.rs); `changes_made_outside_show_in_an_idle_window` (apps/git-good/src/idle_tests.rs) — `git branch`

### FRM-63 Commands flush unsettled changes first
Fixture: `fx-dirty`
Layer: product
- When `git stash -u` runs externally and **Changes › Stash changes…** is chosen within 300 ms
- Then Git Good re-reads state before running: the worktree is clean, so the command is refused with *No local changes to stash* rather than acting on stale state
Covered by (interaction, partial): `a_batch_reports_after_it_settles_or_when_flushed` (apps/git-good/src/watcher.rs) — flush only; Not automated end to end

### Dialogs, keyboard, density, layout

### FRM-64 Decision dialogs are modal
Fixture: `fx-branchy`
Layer: interaction
- When **Branch › New branch…** is open and a History tree row is clicked and Down pressed
- Then a dimmed backdrop absorbs the click; selection does not change; after closing the dialog, clicks and keys work again
Covered by (interaction): `decision_dialogs_block_the_window_behind_them` (apps/git-good/src/main.rs), `a_modal_window_blocks_clicks_and_navigation_behind_it` (apps/git-good/src/navigation.rs)

### FRM-65 Tool windows are non-modal
Fixture: `fx-stash`
Layer: interaction
- When the Stashes picker, Recovery, Worktrees, Details… and Keyboard shortcuts windows are open in turn
- Then the History tree rows remain clickable and Up/Down still move selection beside them
Covered by (Not automated)

### FRM-77 Full-text windows are titled by their field
Fixture: `fx-linear` (Given: a commit whose subject is 212 characters)
Layer: interaction
- When the user clicks the "…" button at the end of the truncated subject in the commit inspector (a real button named *Show full commit subject*), then presses `Ctrl+C`, then `Esc`
- Then a non-modal window titled **Commit subject** shows the exact text, selectable, then `212 characters · text is selectable`, **Close** `Esc` and the primary **Copy** `Ctrl+C`; `Ctrl+C` copies the text and `Esc` closes it; a path, the parents, a commit message, File details and an error open the same window titled **Path**, **Parents**, **Commit message**, **File details**, **Error**, with paths and IDs in mono
Covered by (interaction, partial): `the_full_text_window_is_titled_by_its_field_with_copy_primary`, `overflow_opens_exact_text_without_changing_the_slot` (bounded_text.rs)

### FRM-66 History tree keys respect text focus and open menus
Fixture: `fx-linear`
Layer: interaction
- When focus is in the search field, `j`/`k` and Ctrl+V are typed; then `Esc`
- Then text is edited/pasted, the History tree selection is unchanged; Esc leaves the field; a focused toolbar button still allows Up/Down list navigation
Covered by (interaction): `focused_buttons_allow_navigation_and_text_editors_keep_keys` (apps/git-good/src/navigation.rs), `navigation_bindings_preserve_text_focus_and_empty_selection` (apps/git-good/src/interaction_tests.rs)

### FRM-67 Copy full ID with Ctrl+C — see HIS-34

### FRM-68 Compact density is the default
Fixture: `fx-linear`, default preferences; then `fx-prefs-custom`
Layer: visual
- Then the History tree rows are 22 px and diff rows 19 px, **View › Compact density** is checked; with `fx-prefs-custom` (comfortable) rows are taller and the item is unchecked
Covered by (interaction): `densities_name_their_row_heights` (apps/git-good/src/density.rs)

### FRM-69 Narrow width: details replace the History tree reversibly — see HIS-42

### FRM-70 Panes never show loading text; a pending read is visible in the footer
Fixture: `fx-linear` then `fx-branchy` (slow read simulated; `fx-large` opt-in)
Layer: interaction
- When the repository opens, and while switching to another repository or file
- Then no pane shows "Loading…"/"Reading…" text; the footer reads `Reading <path>` while the read runs; panes stay empty on first open, or keep the previous content together with its previous identity (title, header, path) until the new content arrives, never an old list under a new label (FRM-73)
Covered by (interaction, partial): `empty_history_explains_itself_only_after_the_scan` (apps/git-good/src/main.rs); the identity hold while switching is Not automated

### FRM-71 Frame rows hold still (no layout shift)
Fixture: `fx-remote`
Layer: visual
- When Fetch runs and finishes, the Changes badge appears/disappears, and an error arrives in the footer
- Then menu bar (26 px), toolbar (34 px) and status line keep fixed heights; panes do not move; toolbar icons keep x positions
Covered by (interaction, partial): `window_toolbar_keeps_the_owners_buttons_in_place_across_views_and_widths` (apps/git-good/src/main.rs) — positions only; Not automated for heights

### FRM-72 Every command is reachable at narrow width through menus and search
Fixture: `fx-branchy`, `fx-stash`; narrow
Layer: interaction
- When each command — the toolbar's **New branch…**, **Stash changes…**, **Stashes…** and **Recovery…**, and **New tag…**, **Merge into current…** and **Interactive rebase of current branch…**, which have no button — is looked up by its menu home and by typing its name in search
- Then each is found in exactly one menu (every menu fits the window, no clipped item) and under search **Actions** with its menu home; both routes open the same dialog or window; a disabled command shows the same reason in both
Covered by (interaction, partial): `commands_off_the_toolbar_keep_their_menu_home_and_search_entry`, `changes_stashes_opens_the_picker_inside_the_narrow_window`, `repository_recovery_opens_one_window_on_the_last_source` (apps/git-good/src/main.rs) (menu home, search entry and shared reason; menu fit at narrow width and the search route not driven)

### FRM-73 Pane identity and content always match
Fixture: `fx-file-history`, `fx-linear`, `fx-branchy` (slow reads simulated)
Layer: interaction
- When the user selects file B while file A's diff is shown, switches repository while the History tree shows `fx-linear`, and moves the History tree selection quickly
- Then in every frame each pane's label (file path and endpoints, window title and identity, commit header) names the content shown: the old label with the old content until the new content arrives, then both change in the same frame; never a new label over old rows
Covered by (interaction, partial): `selection_keeps_displayed_diff_until_replacement_arrives`, `pending_endpoint_labels_wait_for_matching_content` (apps/git-good/src/diff_view.rs), `failed_switch_keeps_displayed_identity_and_retry_adopts_new_repository` (apps/git-good/src/interaction_tests.rs); the hold during a successful repository switch is Not automated

### FRM-84 The menu bar is the title bar on Windows and Linux
Fixture: `fx-linear`, 720, 1120 and 1440 px, Windows or Linux, Use the system title bar off
Layer: interaction
- When the window is shown
- Then the window has no OS decorations; the 26 px menu bar holds the menus at the left, the window identity centred on the window in the free gap (slid toward the menus, then shortened, when it does not fit), and minimize, maximize/restore and close buttons at the end the desktop's layout names; no menu title or identity overlaps a caption button
- And Close runs **Repository › Quit**; Maximize reads Restore while maximized
Covered by (interaction): `gg_draws_the_title_bar_with_menus_identity_and_caption_buttons` (apps/git-good/src/main.rs), `caption_buttons_take_the_right_end_and_nothing_overlaps_them`, `the_identity_is_centred_in_the_free_gap`, `close_runs_quit_and_the_others_follow_the_window_state` (apps/git-good/src/titlebar/tests.rs)

### FRM-85 Only the free gap moves the window; the edges resize it
Fixture: `fx-linear`, 1440 px, Git Good's title bar
Layer: interaction
- When the primary button drags the gap between the menus and the caption buttons (the identity included), double-clicks it, drags a menu title, and presses within 8 px of a window edge (12 px at a corner)
- Then the gap starts an OS move and a double-click maximizes or restores; a menu title keeps its press; an edge starts an OS resize in its direction with a one-sided resize cursor; a maximized window has no resize zones; after an OS move or resize whose release the compositor kept, the next press works
Covered by (interaction): `empty_bar_space_drags_and_double_click_maximizes`, `edge_zones_resize_unless_maximized`, `after_an_os_resize_the_edge_hovers_and_shows_its_cursor_again`, `after_an_os_move_the_bar_and_the_edges_take_the_next_press`, `a_release_the_os_sends_is_not_doubled` (apps/git-good/src/titlebar/tests.rs)

### FRM-86 Caption buttons follow the desktop
Fixture: `fx-linear` on Windows, GNOME, KDE and another Linux desktop
Layer: interaction
- When the window is shown
- Then Windows shows 46 px full-height buttons at the right, Close red on hover; GNOME shows libadwaita's round buttons in its `button-layout` order (default: Close only, at the right); KDE shows Breeze's in KWin's order; other desktops show plain round buttons in `gtk-decoration-layout` order (Adwaita's or Breeze's under those GTK themes); round buttons also drag the window; `GG_CAPTION_STYLE` and `GG_CAPTION_LAYOUT` override what is found
Covered by (interaction, partial): `gtk_layouts_pick_the_buttons_and_their_sides`, `linux_buttons_sit_where_the_layout_says_and_drag_the_window`, `only_sides_with_buttons_take_room` (apps/git-good/src/titlebar/tests.rs), `the_desktop_comes_from_the_first_variable_set`, `the_family_follows_the_desktop_then_the_gtk_theme`, `kwin_button_letters_become_a_gtk_layout`, `layouts_and_overrides_resolve_to_a_style`, `settings_files_are_read_from_the_config_dir` (apps/git-good/src/caption_style.rs); reading GNOME's setting from a live desktop is Not automated

### FRM-86a Regions and the context menu key
Fixture: `fx-linear`, the History tree at 1280 px with the refs sidebar docked
Layer: interaction
- When the user presses F6 with no field focused
- Then the toolbar's search field has the keyboard; F6 again moves it to **Filter refs**, then to the list (no field focused, so `j`/`k` move the selection), then back to search; Shift+F6 walks the other way; below 1120 px the sidebar is skipped
- And Shift+F10 (or the Menu key) opens the selected commit's actions, as Alt+A does, without putting the keyboard on the menu bar
- And Help › Keyboard shortcuts lists F6 under **Window** and Shift+F10 beside Alt+A
Covered by (interaction): `f6_moves_between_search_sidebar_and_content_and_shift_f10_opens_actions` (apps/git-good/src/main.rs), `f6_walks_the_shown_regions_and_wraps` (apps/git-good/src/a11y.rs), `groups_follow_the_menus_then_the_list_scopes` (apps/git-good/src/shortcuts.rs)

### FRM-87 Use the system title bar
Fixture: `fx-linear`, Windows or Linux
Layer: interaction
- Given **Preferences › Appearance › Use the system title bar** has not been changed
- Then it follows the desktop: on for Windows (Windows' own title bar, with snap layouts and Mica) and KDE Plasma (KWin's frame, with the user's decoration theme and window rules), off for GNOME and other Linux desktops, which get Git Good's title bar
- When it is turned on or off and Git Good restarts
- Then with it on the window has the OS title bar and frame; the menu bar shows the menus and the right-aligned identity, no caption buttons, no drag gap and no resize zones; with it off Git Good's menu bar is the title bar; the choice is saved in `settings.json` (`system_title_bar`; absent follows the desktop) and wins over the desktop's default; macOS has no such preference
Covered by (interaction): `each_desktop_frames_the_window_its_own_way`, `preferences_search_keeps_sections_with_a_match` (apps/git-good/src/main.rs), `the_system_title_bar_keeps_the_identity_at_the_right_and_no_buttons` (apps/git-good/src/titlebar/tests.rs), `round_trips_and_keeps_unknown_keys` (apps/git-good/src/settings.rs)

### FRM-88 macOS menu bar
Fixture: `fx-linear` on macOS
Layer: interaction
- When the window is shown
- Then the window's title bar is merged into the toolbar row: the title is hidden, the close, minimize and zoom buttons sit at the row's left with the toolbar's first button 78 pt in, dragging the row's empty space moves the window and a double-click zooms it
- And the screen's menu bar holds **Git Good** (About Git Good · Settings… `⌘,` · Services · Hide Git Good `⌘H` · Hide Others · Show All · Quit Git Good `⌘Q`), **Repository · Branch · Commit · Changes · View** with the in-window items, labels, shortcuts, enablement and checks, **Window** (Minimize · Zoom · Bring All to Front) and **Help** with the system search; the window has no in-window menu bar and the toolbar is its top row
- And a key equivalent reaches Git Good as the key press it was (text fields keep ⌘C, ⌘X, ⌘V, ⌘A and ⌘Z, which are not key equivalents); a click runs the command as the in-window menu does; a new Open recent entry rebuilds the menu, other changes update it in place; a system key Git Good binds moves Hide to ⌃⌘H and Minimize to ⌃⌘M
Covered by (interaction, partial): `each_desktop_frames_the_window_its_own_way`, `a_merged_title_bar_leaves_room_for_the_window_buttons_and_moves_the_window`, `the_mac_menu_bar_comes_from_the_command_table`, `selection_changes_update_the_mac_menu_in_place`, `the_native_menu_bar_replaces_the_in_window_menus` (apps/git-good/src/main.rs), `key_equivalents_reach_egui_as_key_presses_and_clicks_run_commands`, `clashes_with_system_keys_are_resolved_in_ggs_favour`, `gg_shortcuts_become_the_key_presses_egui_sees` (apps/git-good/src/native_menu/tests.rs); AppKit itself (muda) is Not automated

### FRM-89 Dialogs take the keyboard
Fixture: `fx-branchy`
Layer: interaction
- When **Branch › New branch…** opens and the user types `topic` without clicking, then presses Enter
- Then the name field had the keyboard from the first frame and Enter creates the branch, as **Create** does
- And in **Rename branch…**, Esc closes it as **Cancel** does; in a dialog whose primary is disabled Enter does nothing
- And a destructive dialog (Delete branch, Delete tag, Delete remote branch, Discard all tracked changes, Reset here with Hard, Force push with lease, Remove worktree, Delete backup) opens with **Cancel** focused: Enter cancels and nothing changes; Tab or Shift+Tab reaches the red primary and Enter there runs it
- And a dialog closed and opened again on the next frame takes the keyboard again
Covered by (interaction): `a_dialog_takes_typing_at_once_and_enter_or_esc_answer_it` (apps/git-good/src/navigation.rs), `dialogs_take_typing_at_once_and_answer_enter_and_esc` (apps/git-good/src/main.rs), `delete_backup_confirms_then_removes_only_that_ref` (apps/git-good/src/recovery_view.rs)

## History tree and refs

### FRM-90 Help › Star Git Good on GitHub
Fixture: `fx-linear`
Layer: interaction
- When **Help › Star Git Good on GitHub**
- Then the browser opens `https://github.com/baol/gg`, the repository page where GitHub's Star button is; the Help menu reads Keyboard shortcuts · Documentation · Star Git Good on GitHub · About Git Good
Covered by (interaction): `star_on_github_opens_the_repository_page`, `menus_follow_the_final_design_mockup` (apps/git-good/src/main.rs)

### FRM-91 Key labels name the platform's keys
Fixture: `fx-linear` on macOS
Layer: interaction
- When the window shows a key in a button, hint, tooltip, menu, Keyboard shortcuts or Preferences › Keyboard, such as **Start rebase** in Interactive rebase
- Then on macOS the Command modifier reads ⌘ and the others ⌥ and ⇧, in Apple's order with no plus signs, and Enter after a modifier reads ↩ (**Start rebase** `⌘↩`, Stage all `⇧⌘S`, Actions `⌥A`); no label reads `Ctrl+`; on Linux and Windows the same labels read `Ctrl+Enter`, `Ctrl+Shift+S`, `Alt+A`
- And the keys themselves are unchanged: egui's Command modifier is ⌘ on macOS and Ctrl elsewhere, and saved key settings keep the `Ctrl+Shift+S` form on every platform
Covered by (interaction): `on_macos_the_rewrite_dialog_names_command_keys` (apps/git-good/src/main.rs), `mac_labels_use_the_command_symbols`, `labels_follow_the_platform_this_thread_is_set_to` (apps/git-good/src/keymap.rs)

### HIS-01 First page streams before traversal finishes
Fixture: `fx-large` (opt-in performance) (size 1440×900 unless stated)
Layer: interaction
- Given Git Good is launched in the repository
- When the window first paints the History tree
- Then the first visible rows show subject, author, short hash and date before the count completes; the column header reads `Commit · N+` while counting, and the UI stays responsive (scrolling, menus) during traversal
Covered by (interaction): `packed_history_first_frame_and_deep_navigation` (apps/git-good/src/main.rs), `real_repository_history_first_frame_and_deep_navigation` (main.rs)

### HIS-02 Count header reaches an exact total
Fixture: `fx-linear`
Layer: visual
- Given the History tree is open
- When traversal and counting complete
- Then the History tree column header reads `Commit · 30` with no `+`
Covered by (Not automated)

### HIS-03 History tree includes commits reachable only from other refs
Fixture: `fx-branchy`
Layer: product
- Given HEAD is on `main`
- When the History tree loads
- Then the 4 `feature/login` commits appear in the graph (union of HEAD, every local branch, remote branch and tag), each exactly once, with parents after all children
Covered by (backend, partial): `history_includes_commits_from_all_local_remote_and_tag_refs` (`crates/git/tests/history.rs`), `pinned_head_and_skewed_merge_order_are_stable` (`crates/git/tests/history.rs`); GUI path not automated

### HIS-04 Every commit beyond the page budget is reachable
Fixture: `fx-large` (opt-in)
Layer: interaction
- Given the History tree is open
- When the user presses End-equivalent paging (Page Down repeatedly) or drags the scrollbar to the bottom
- Then fixed-height empty slots show while uncached rows stream, then fill in place; the oldest commit is reachable; no loading text appears in the pane
Covered by (interaction, end-to-end): `json_includes_every_commit_beyond_the_former_cap` (apps/git-good/tests/cli.rs), `pages_evict_and_reload_without_losing_identity_or_selection` (apps/git-good/src/state.rs)

### HIS-05 Growing count does not move the viewport
Fixture: `fx-large` (opt-in)
Layer: interaction
- Given the user scrolls by wheel mid-history while counting continues
- When the discovered extent grows
- Then the row under the pointer and its pixel offset stay put while the extent grows; the scrollbar follows the new extent
Covered by (interaction): `a_growing_count_shows_at_once_and_keeps_the_hovered_row_and_offset` (interaction_tests.rs)

### HIS-06 Scrollbar drag keeps the grab offset
Fixture: `fx-large` (opt-in)
Layer: interaction
- Given the user drags the scrollbar thumb while rows stream
- When the user releases (even with pointer motion in the same frame)
- Then the extent stayed frozen during the drag while rows still filled in place, the release position is preserved, and the grown extent applies right after release
Covered by (interaction): `scrollbar_drag_freezes_extent_and_release_preserves_offset`, `scrollbar_release_followed_by_pointer_motion_in_one_frame_keeps_release_position` (apps/git-good/src/interaction_tests.rs), `unfilled_slots_fill_in_place_while_the_extent_is_frozen` (state.rs)

### HIS-07 A row arriving under the pointer fades in and refuses an early click
Fixture: `fx-large` (opt-in)
Layer: interaction
- Given the pointer rests on an unfilled slot
- When the slot's commit arrives, and the user presses on it
- Then the commit's row fades in over 0.25 s and a press within 0.3 s of its arrival does nothing; a press after that selects it; a press and release act only on the same row, so a row that swaps while the button is down is not clicked
Covered by (interaction): `a_row_arriving_under_the_pointer_fades_in_and_refuses_an_early_click`, `moving_onto_a_row_or_scrolling_never_starts_the_grace`, `a_replacement_under_a_press_shows_at_once_and_the_release_clicks_nothing` (interaction_tests.rs)

### HIS-08 External ref change refreshes the History tree and keeps selection
Fixture: `fx-branchy`
Layer: interaction
- Given c27 is selected and scrolled to a fractional offset
- When `git branch extra c05` runs outside Git Good and 300 ms pass
- Then the History tree reloads (no Refresh command exists); c27 stays selected at the same pixel offset; the new ref appears
Covered by (interaction): `refresh_preserves_commit_anchor_and_fractional_row_offset` (interaction_tests.rs)

### HIS-09 A refresh replacement under a press shows at once and clicks nothing
Fixture: `fx-branchy`
Layer: interaction
- Given the user is pressing a row
- When a replacement History tree session becomes ready
- Then it shows at once; the release does not click the row that replaced the pressed one (RULE-17); a retired worker never overwrites the adopted rows
Covered by (interaction): `a_replacement_under_a_press_shows_at_once_and_the_release_clicks_nothing` (interaction_tests.rs)

### HIS-10 Refresh selects the first row when the selection vanished
Fixture: `fx-branchy`
Layer: interaction
- Given the `feature/login` tip is selected
- When `git branch -D feature/login` runs outside Git Good (commits now unreachable) and the History tree refreshes
- Then the first visible row is selected and details follow it
Covered by (Not automated)

### HIS-11 Filter by text through search — see FRM-49

### HIS-12 Filter matches the full message, not only the subject
Fixture: `fx-linear`
Layer: product
- Given a word that appears only in the second paragraph of c10
- When the History tree is filtered by that word
- Then c10 is the single match
Covered by (backend, partial): `search_covers_identity_and_full_message` (crates/engine); GUI path not automated

### HIS-13 Filter keeps a matching selection, else selects first match
Fixture: `fx-linear`
Layer: interaction
- Given c20 is selected
- When filtering by a word in c20's message, then by a word not in c20
- Then c20 stays selected the first time; the second time the first published match is selected
Covered by (interaction): `filtering_preserves_visible_selection_and_clears_empty_details` (state.rs)

### HIS-14 Filter with no matches
Fixture: `fx-linear`
Layer: interaction
- Given the History tree is open
- When filtering by `zzzz-nomatch`
- Then the pane reads *No commits match “zzzz-nomatch”* with **Clear filter**; selection and details are empty; Up/Down select nothing; **Clear filter** restores all 30 rows
Covered by (interaction): `filtering_preserves_visible_selection_and_clears_empty_details` (state.rs), `renders_empty_error_and_filtered_snapshots` (main.rs)

### HIS-15 Rapid filter edits apply only the latest revision
Fixture: `fx-large` (opt-in)
Layer: interaction
- Given the user types, deletes and retypes the same text quickly
- When older searches finish late
- Then only the latest revision's results are shown, even when the text repeats
Covered by (interaction): `rapid_filters_reject_superseded_results_even_when_query_text_repeats` (interaction_tests.rs), `late_query_and_selection_results_do_not_replace_new_intent` (state.rs)

### HIS-16 Filter edit survives a refresh in flight
Fixture: `fx-branchy`
Layer: interaction
- Given a refresh is loading
- When the user edits the filter
- Then the edit is kept and applied to the replacement session
Covered by (interaction): `filter_edit_during_refresh_survives_late_replacement` (interaction_tests.rs)

### HIS-17 Repository switch clears filter and selection
Fixture: `fx-linear` then `fx-branchy`
Layer: interaction
- Given a filter and selection in `fx-linear`
- When the user opens `fx-branchy` via **Repository › Open…**
- Then the History tree shows unfiltered with the first row selected
Covered by (interaction): `switching_repository_resets_filter_and_selection` (state.rs)

### HIS-18 Graph: a linear History tree stays in one lane; column 56 px
Fixture: `fx-linear`
Layer: visual
- Given the History tree is open
- Then every dot is in lane 0 and the graph column is 56 px wide
Covered by (interaction): `linear_history_stays_in_lane_zero`, `the_column_fits_four_lanes_at_56_px_and_grows_to_92` (apps/git-good/src/graph.rs)

### HIS-19 Graph: merges, colors, and ring
Fixture: `fx-branchy`
Layer: visual
- Given the History tree is open
- Then `Merge branch 'topic'` has a ring dot and its second parent occupies a distinct lane in the parent lane's color; `feature/login` keeps its own lane color until it joins; only the row's own lane draws a downward segment from the dot
Covered by (interaction): `merge_parents_get_distinct_lanes`, `merge_parents_keep_the_color_of_the_lane_they_start`, `a_single_parent_branch_keeps_its_color_until_it_joins_an_existing_lane`, `only_the_rows_own_lane_starts_below_the_commit` (graph.rs)

### HIS-20 Graph widens but never narrows
Fixture: `fx-branchy` + 4 extra branches off c05 (set up in Given)
Layer: visual
- Given more than four concurrent lanes appear further down
- When the user scrolls to them and back to the top
- Then the column widened one lane at a time (max 92 px) and stays at that width at the top; rows do not shift back
Covered by (interaction, partial): `the_column_fits_four_lanes_at_56_px_and_grows_to_92` (graph.rs) (widening only; never-narrow Not automated)

### HIS-21 Graph lanes stay coherent under a filter
Fixture: `fx-branchy`
Layer: visual
- Given the History tree is filtered by `login`
- Then remaining rows keep the lane and color they have unfiltered
Covered by (Not automated)

### HIS-22 Ref pills order and colors
Fixture: `fx-branchy`
Layer: visual
- Given HEAD (`main`) carries tag `v1.0.0`
- Then the HEAD row shows pills before the subject: `v1.0.0` (tag, gold in dark theme) first, then `main` (current branch, blue) before other locals, then remotes; pills are plain text with no kind icon
Covered by (interaction): `ref_labels_prefix_the_subject`, `tags_sort_first_then_the_current_branch` (apps/git-good/src/history_view.rs)

### HIS-23 Pill overflow count
Fixture: `fx-branchy` + branches `a`,`b`,`c`,`d` at HEAD (set up in Given)
Layer: visual
- Given 6 refs on HEAD
- Then at most four pills show (two at narrow subject width) followed by an explicit `+N` overflow count; long labels stay single-line
Covered by (interaction): `extra_ref_labels_show_an_overflow_count`, `long_fields_stay_single_line_at_supported_scales` (history_view.rs)

### HIS-24 Clicking a pill inspects without moving HEAD
Fixture: `fx-branchy`
Layer: product
- When the user single-clicks the `feature/login` pill, then the `v0.1.0` pill, then the `main` pill
- Then each selects its target commit; HEAD, index, worktree and refs are unchanged; no switch is requested
- And a double-click switches only on a branch or remote pill that is not the current branch (HIS-54, HIS-65); tag pills and the current branch's pill only select
Covered by (interaction): `clicking_a_branch_pill_selects_and_double_clicking_switches`, `clicking_a_tag_pill_selects_its_commit`, `clicking_the_current_branch_pill_only_selects_it` (history_view.rs)

### HIS-25 Show tags in rows off
Fixture: `fx-branchy`
Layer: visual
- Given Preferences › Appearance › History tree › **Show tags in rows** is turned off
- Then tag pills disappear from rows; branch pills remain
Covered by (Not automated)

### HIS-26 Avatars off by default
Fixture: `fx-linear` (default prefs)
Layer: visual
- Then the History tree rows show no avatar image; the commit inspector keeps its avatar slot showing initials (`AL`, `GH`); no network request is made; **Show avatars** is unchecked in Preferences
Covered by (Not automated)

### HIS-27 Avatars on: cached, bounded, deferred while interacting
Fixture: `fx-linear` with `fx-prefs-custom` (avatars on), local Gravatar stub or cache
Layer: interaction
- Given avatars are enabled
- Then rows show 18 px avatars and details 64 px with reserved initials slots; an arriving image is published only after hover/scroll ends; a 404 is cached across restarts
Covered by (interaction): `list_and_details_use_display_sized_textures_with_reserved_slots`, `arriving_avatar_waits_until_interaction_ends`, `missing_avatar_is_negatively_cached_across_restarts`, `lookups_follow_the_enabled_sources_in_order` (apps/git-good/src/avatars.rs)

### HIS-28 GG_OFFLINE disables network avatars — see PRF-13

### HIS-29 Relative and absolute dates
Fixture: `fx-linear`
Layer: visual
- Given **Relative dates** on, clock at c30 + 3 h
- Then c30 reads `3 h ago`; hovering the date shows the full time; turning **Relative dates** off shows absolute dates
Covered by (interaction, partial): `relative_dates_read_git_times_with_offsets` (history_view.rs), `dates_are_utc_and_handle_the_epoch` (engine) (toggle Not automated)

### HIS-30 Keyboard: next/previous commit in three styles
Fixture: `fx-linear`
Layer: interaction
- Given c30 selected, the History tree focused, no editor focused
- When the user presses Down, then `j`, then Ctrl+N (physical Control, also on macOS), then Up, `k`, Ctrl+P
- Then selection moves to c29, c28, c27, then back to c30; at the top Up does nothing (no wrap); viewport moves only by a minimal reveal
Covered by (interaction): `keyboard_moves_through_all_matches_and_stops_at_edges` (state.rs), `keyboard_on_visible_rows_preserves_viewport` (interaction_tests.rs)

### HIS-31 Keyboard: paging
Fixture: `fx-large` (opt-in) or `fx-linear` at 1440×500
Layer: interaction
- When Page Down, then Page Up, then Alt+V
- Then each pages by the rendered viewport with one row of overlap, clamps at boundaries, and streams uncached destinations; the last page has stable geometry
Covered by (interaction): `navigation_bindings_share_page_size_bounds_and_stable_reveal`, `keyboard_last_page_has_stable_geometry_between_frames` (interaction_tests.rs)

### HIS-81 Keyboard: Home and End
Fixture: `fx-linear`
Layer: interaction
- When End, then Home, then Ctrl+End (Command+End on macOS), then Ctrl+Home in the History tree; and End then Home in a focused Local Changes list
- Then the History tree selects c01, then c30, then c01, then c30 and reveals each; the Local Changes list selects its last file, then its first, and the other list keeps its selection; while counting continues End selects the last row found so far
Covered by (interaction): `home_and_end_go_to_the_first_and_last_commit` (main.rs), `home_and_end_go_to_the_first_and_last_file_of_the_focused_list` (status_view.rs), `home_and_end_step_past_either_end_and_page_keys_by_a_page` (navigation.rs)

### HIS-32 Ctrl+V is paste, keys stay with focused editors — see FRM-66

### HIS-33 Keyboard selection loads an evicted row and late details are rejected
Fixture: `fx-large` (opt-in)
Layer: interaction
- When the user holds Down past cached rows
- Then the summary of the newly selected commit shows immediately, the message slot stays empty until its message arrives, and a late message for a previous commit is never shown
Covered by (interaction): `keyboard_navigation_rejects_late_details_and_loads_an_evicted_row`, `keyboard_details_show_current_summary_until_message_arrives` (interaction_tests.rs)

### HIS-34 Copy full ID
Fixture: `fx-linear`
Layer: interaction
- Given c15 selected (possibly scrolled off-screen by keyboard)
- When Ctrl+C (Cmd+C on macOS), or **Commit › Copy ID**
- Then the clipboard holds the exact 40-hex ID of c15
Covered by (interaction): `keyboard_selects_offscreen_commit_and_copies_exact_id` (main.rs)

### HIS-35 Commit inspector content
Fixture: `fx-linear`
Layer: visual
- Given 1440 px wide, c20 selected
- Then a 58 px inspector under the list shows avatar/initials, the subject (hover shows the full message), an 8-char ID pill, **Review** (`Enter`), **Actions** (`Alt+A`), and line 2: author, date, parent ID
- When the user clicks the ID pill
- Then the full ID is copied
Covered by (Not automated)

### HIS-36 Inspector stats line appears without moving content
Fixture: `fx-linear` (any commit changing several files)
Layer: visual
- When the background count against the first parent finishes
- Then `· 5 files +86 −41`-style text appears at the end of line 2 (added in added color, deleted in deleted color); no other text moves
Covered by (interaction): `labels_read_like_the_mockup`, `counts_a_commit_against_its_parent` (apps/git-good/src/commit_stats.rs)

### HIS-37 Inspector stats cap
Fixture: proposed `fx-wide-commit`
Layer: visual
- Given a commit touching 2,500 files
- Then the stats read `2000+ files`
Covered by (Not automated)

### HIS-38 Toggle inspector with Space
Fixture: `fx-linear`
Layer: interaction
- When Space (or **View › Commit details**)
- Then the inspector hides and the list grows; the refs sidebar keeps full height; Space shows it again; the choice persists across launch (`details_visible`)
Covered by (Not automated)

### HIS-39 Actions menu, context menu, Alt+A share entries
Fixture: `fx-branchy`
Layer: interaction
- When the user opens **Actions**, right-clicks a row, and presses Alt+A
- Then each lists the **Commit** menu entries as 24 px rows with the empty check column and shortcuts at the right, no group headings or hashes in labels: Review changes `Enter`, Compare with HEAD, Copy ID `Ctrl+C` with **Copy subject** and **Signature details…** under it, Cherry-pick onto HEAD, Revert, Interactive rebase from here… `R`, Reword… `Ctrl+R`, Reopen for editing… `E`, Fixup into… `F`, Drop… `D` (red), **Reset current branch to here ▸** (red; Soft · keep index and files, Mixed · keep files ✓ `default`, Hard · discard changes… in red); no Recovery… (it is in the Repository menu); during a rebase the mutations stay, greyed, under one band reading `Rebase in progress. Continue or abort it to change commits.`
Covered by (interaction): `scoped_commit_actions_menu_lists_grouped_actions`, `actions_shortcut_opens_the_scoped_menu`, `commit_action_menu_returns_the_clicked_command`, `commit_action_menu_matches_the_inspector_with_one_reason_band` (main.rs)

### HIS-40 Right-click selects the row first
Fixture: `fx-linear`
Layer: interaction
- Given c30 selected
- When the user right-clicks c25 and picks **Copy ID**
- Then c25 becomes selected and its ID is copied
Covered by (Not automated)

### HIS-41 Merge commit actions disabled with reasons
Fixture: `fx-branchy`
Layer: interaction
- Given `Merge branch 'topic'` selected
- When **Actions** opens
- Then rewrite entries (Reword…, Reopen for editing…, Fixup into…, Drop…) are disabled with a reason naming merge commits on hover
Covered by (Not automated)

### HIS-42 Narrow commit details
Fixture: `fx-linear`, narrow
Layer: interaction
- Given c20 selected; the inspector under the list shows **Details** `Space`, **Review** `Enter`, and **Actions** `Alt+A`
- When Space (or **Details**)
- Then the commit details replace the list (not a side pane): **‹ History tree** `Esc`, the subject, and newer/older buttons; Commit (copy button), Author with email, Date, Parent, Refs, Signature **Verify…**; the message; `FILES n` with `+x −y` and per-file counts; **Actions** `Alt+A` and the filled primary **Review** `Enter` at the bottom. `j`/`k` show the older/newer commit with the details still open; Up/Down choose a file without moving the History tree selection; `Enter`, **Review**, or a double click opens Review on the chosen file and Esc there returns to the details; Alt+A opens the details first then the popup; Escape/Left returns to the History tree with c20 selected at the same pixel offset
Covered by (interaction): `narrow_details_navigation_preserves_hidden_history_and_back_anchor`, `narrow_details_step_commits_with_j_and_k`, `narrow_details_returns_to_the_same_selected_history_row`, `history_strip_leaves_the_narrow_details_overlay` (interaction_tests.rs, main.rs); `narrow_details_review_opens_on_the_chosen_file`, `scoped_commit_actions_menu_lists_grouped_actions` (main.rs)

### HIS-43 Enter opens Review and Back restores place — see REV-02

### HIS-44 Compare with HEAD — see REV-07

### HIS-45 Refs sidebar groups and counts
Fixture: `fx-remote`
Layer: visual
- Given 1440 px wide History tree
- Then the sidebar shows `BRANCHES  n` expanded, `REMOTES`, `TAGS` collapsed (`▸ n`), `STASHES`; the current branch is bold with a HEAD checkmark; branch rows start with a dot in their graph lane color; `main` ends with dimmed `↑2 ↓3`; `local-only` has no counts; no tracking line or Pull button
Covered by (interaction): `shows_grouped_sections_and_current_branch` (apps/git-good/src/refs_view.rs)

### HIS-46 Group header toggles
Fixture: `fx-branchy`
Layer: interaction
- When the user clicks the `TAGS` header twice
- Then it expands to list `v0.1.0`, `v0.2.0`, `v1.0.0`, then collapses to `▸ 3`
Covered by (Not automated)

### HIS-47 Folder nodes and refs filter
Fixture: `fx-branchy`
Layer: interaction
- Then `feature/login` shows under a `feature` folder node
- When the refs filter has `log`
- Then every group opens, only matching rows remain (folders flattened); with `nomatch` the sidebar reads `No matching refs`
Covered by (interaction): `filter_flattens_and_clears_unmatched_groups` (refs_view.rs)

### HIS-48 Worktrees group
Fixture: `fx-worktrees`
Layer: interaction
- Then `WORKTREES n` lists each folder with its branch, the open one checked; clicking another opens it
Covered by (interaction): `worktrees_are_listed_when_linked_ones_exist_and_open_on_click` (refs_view.rs)

### HIS-48a Refs sidebar at scale
Fixture: a repository with 10,000 branches in 100 folders (`team00/topic00000` …)
Layer: interaction
- When the History tree opens with the sidebar docked
- Then only the rows in view are laid out (fewer than 80 texts at 600 px), and frames that change nothing reuse the built list; a new refs read builds it again
- And clicking the folder `team00` closes it (`team01` moves up) and clicking again opens it
- And the drawer's cursor on `team90/topic09042` brings that row into view
Covered by (interaction): `ten_thousand_branches_draw_only_the_rows_in_view_from_one_list`, `a_folder_closes_and_opens_on_click`, `the_keyboard_cursor_scrolls_a_far_branch_into_view` (apps/git-good/src/refs_view.rs)

### HIS-49 Clicking a sidebar row inspects; never moves HEAD
Fixture: `fx-remote`
Layer: product
- When the user clicks `local-only`, `origin/release`, a tag, and the current branch
- Then each selects its target commit; HEAD, index, worktree and refs are byte-identical before/after; no tracking branch is created
Covered by (interaction): `clicking_a_branch_row_inspects_its_commit`, `clicking_a_remote_row_inspects_its_commit`, `clicking_the_current_branch_row_inspects_it_without_switching` (refs_view.rs), `ref_sidebar_click_preserves_head_index_worktree_and_refs` (main.rs)

### HIS-50 Ref selection clears a hiding filter
Fixture: `fx-branchy`
Layer: interaction
- Given the History tree filtered by `docs` (hiding `feature/login`'s tip)
- When the user clicks `feature/login` in the sidebar
- Then the filter clears and the tip is selected and revealed
Covered by (interaction): `ref_jump_clears_filters_and_retargets_selection` (main.rs)

### HIS-51 Sidebar visibility and width
Fixture: `fx-branchy`
Layer: interaction
- Then the sidebar is open by default in the History tree; `Ctrl+\` hides/shows it for the History tree only (session-long); below the sidebar breakpoint the toolbar toggle is disabled with a reason; a dragged width stays fixed on idle frames
Covered by (interaction): `refs_sidebar_appears_only_when_there_is_room`, `resized_refs_and_details_keep_their_width_on_idle_frames` (main.rs)

### HIS-52 Local branch context menu entries
Fixture: `fx-branchy`
Layer: interaction
- When the user right-clicks `old`
- Then 24 px rows list Switch to, Merge into current…, Rebase current onto this…, Compare with HEAD, New branch here…, Push (disabled, not current), Set upstream…, Rename… (`F2`), Copy name, Copy ID, Delete branch… (`Del`, red); disabled items show their reason on hover
Covered by (interaction, partial): `refs_menu_offers_the_explicit_switch_command_at_both_widths` (main.rs)

### HIS-53 Remote and tag context menus
Fixture: `fx-remote` / `fx-branchy`
Layer: interaction
- When right-clicking `origin/release`, then `v0.2.0`
- Then the remote offers Switch to release (its local branch, or a new one tracking it), Merge, Rebase, Compare, New branch here…, Copy name, Copy ID, **Delete on origin…** (red); the tag offers Merge, Rebase, Compare, New branch here…, Signature details…, the copies, Delete tag… (red)
Covered by (Not automated)

### HIS-54 Switch branch (clean)
Fixture: `fx-branchy`
Layer: product
- When **Switch to feature/login** from the ref menu (or double-click the row or its History tree pill)
- Then HEAD moves to `feature/login`, filter clears, the new HEAD is selected, refs and the History tree reload, the current branch row is bold/checked
Covered by (backend, interaction, end-to-end): `double_clicking_a_branch_row_switches` (refs_view.rs), `clicking_a_branch_pill_selects_and_double_clicking_switches` (history_view.rs), `switches_branches_only_when_clean` (crates/git/tests/branch.rs), `switch_cli_changes_branches_and_reports_dirty_worktrees` (cli.rs)

### HIS-55 Switch to the current branch is disabled/no-op
Fixture: `fx-branchy`
Layer: interaction
- Then **Switch to main** is disabled on `main`; double-clicking `main` does nothing
Covered by (interaction): `clicking_the_current_branch_row_inspects_it_without_switching` (refs_view.rs)

### HIS-56 Dirty switch is blocked and keeps intent
Fixture: `fx-dirty` + branch `other` (set up in Given)
Layer: product
- When switching to `other`
- Then nothing changes; a warning outcome offers **Retry switch** and **Stash and switch…** (with Include untracked); retry revalidates only when clicked
Covered by (backend, interaction): `blocked_switch_keeps_intent_and_retries_explicitly` (main.rs), `rejects_dirty_missing_and_unborn_branches_and_switches_from_detached_head` (branch.rs)

### HIS-57 Untracked files do not block; overwriting ones are reported
Fixture: `fx-branchy` with untracked `new.txt`; and an untracked file colliding with a tracked file on `feature/login`
Layer: product
- Then the first switch succeeds keeping `new.txt`; the colliding one fails with Git's message and the file is untouched
Covered by (backend, partial): `untracked_files_do_not_block_switching_or_creation`, `git_rejects_untracked_files_that_would_be_overwritten` (branch.rs); GUI path not automated

### HIS-81 A refused switch names Git's reason
Fixture: `fx-branchy` + `other` checked out in a second worktree (set up in Given)
Layer: product
- When switching to `other`
- Then HEAD stays; the warning reads `Can’t switch to other: 'other' is already used by worktree at '<path>'` (Git's last `fatal:` or `error:` line, without the exit status) with **Retry switch**, and **Details…** shows Git's full output
Covered by (interaction): `a_refused_switch_shows_gits_reason`, `git_reason_reads_the_last_fatal_or_error_line` (main.rs)

### HIS-58 Post-checkout hook failure is a warning
Fixture: `fx-branchy` + failing `post-checkout` hook
Layer: product
- When switching to `old`
- Then HEAD is on `old` and the status line shows a warning naming the hook output
Covered by (backend, partial): `post_checkout_failure_is_a_warning_after_the_switch` (branch.rs); GUI path not automated

### HIS-59 Create branch here
Fixture: `fx-branchy`
Layer: product
- Given c25 selected
- When **New branch here…** (or **Branch › New branch…** `Ctrl+B`) with name `fix`, **Switch after creation** on, **Create**
- Then the dialog reads `Create a branch at <selector>`; `fix` is created at c25 and HEAD switches to it; the refs sidebar lists `fix` as the current branch, also when the pointer rests on the History tree while the switch runs
Covered by (backend, interaction, GUI): `creates_and_switches_to_a_new_branch`, `creates_a_branch_at_a_revision_without_switching` (branch.rs), `branch_creation_window_renders_at_supported_widths`, `new_branch_switches_after_creation` (main.rs)

### HIS-60 Create rejects invalid and colliding names
Fixture: `fx-branchy`
Layer: product
- When creating `old`, then `-x`, then `a..b`
- Then each is refused with a reason, nothing is created, the dialog keeps its fields
Covered by (Not automated)

### HIS-61 Created branch kept when switch is blocked
Fixture: `fx-dirty`
Layer: product
- When creating `fix` with **Switch after creation**
- Then `fix` exists, HEAD stays, and a warning offers **Retry switch**
Covered by (interaction): `created_branch_is_reported_when_the_switch_is_blocked` (main.rs)

### HIS-62 Rename branch
Fixture: `fx-remote`
Layer: product
- When `F2` on `main` (or **Branch › Rename…**), new name `trunk`, **Rename**
- Then the dialog **Rename branch** reads `Rename main to:` and notes *The upstream setting moves with the branch.*; `trunk` tracks `origin/main`
- Negative: renaming to `local-only` or `-bad` is refused and nothing changes
Covered by (Not automated)

### HIS-63 Set and unset upstream
Fixture: `fx-remote`
Layer: product
- When **Branch › Set upstream…** on `local-only`, choose `origin/release`, **Set upstream**
- Then `local-only` shows ahead/behind counts in the sidebar; choosing **None** later removes the upstream and the counts
Covered by (backend, partial): `reads_remotes_and_upstream_ahead_behind` (crates/git/tests/refs.rs) (read side only)

### HIS-64 Upstream gone shows no counts
Fixture: `fx-remote`
Layer: product
- Then `gone` shows no ahead/behind counts (unavailable), not zeros
Covered by (backend, partial): `upstream_without_tracking_ref_has_no_counts` (refs.rs); GUI path not automated

### HIS-65 Track remote branch
Fixture: `fx-remote`
Layer: product
- When **Branch › Track remote branch…** and **Track branch**, or **Switch to release** on `origin/release` (one step, no dialog), or a double-click on its row or pill
- Then local `release` is created tracking `origin/release` and HEAD switches to it; a dirty tree is refused, and the dialog refuses an existing local name
- And with a local `release` already present, the menu item, double-click and pill switch to it instead of creating one
Covered by (backend, interaction): `creates_a_tracking_branch_only_when_clean_and_absent` (branch.rs), `double_clicking_a_remote_row_switches_to_it` (refs_view.rs), `double_clicking_a_remote_pill_switches_to_it` (history_view.rs), `switching_to_a_remote_branch_uses_its_local_branch_or_tracks_it`, `clone_and_tracking_dialogs_render_at_supported_widths` (main.rs)

### HIS-66 Delete merged branch
Fixture: `fx-branchy`
Layer: product
- When `Del` on `old` (or **Branch › Delete…**)
- Then **Delete branch** asks `Delete local branch old?`, the list slot shows no commits, the primary reads `Delete old`; confirming removes `old`
Covered by (backend, interaction, end-to-end): `deletes_other_branches_with_safe_or_forced_delete` (branch.rs), `branch_and_tag_delete_dialogs_render_at_supported_widths` (main.rs), `branch_and_tag_delete_cli` (cli.rs)

### HIS-67 Force delete unmerged branch
Fixture: `fx-branchy`
Layer: product
- When deleting `feature/login`
- Then the dialog lists `NOT IN THE CURRENT BRANCH` with the 4 commits (loaded in a fixed slot; primary disabled until loaded), primary red `Delete feature/login and 4 unmerged commits`; confirming force-deletes it; commits remain reachable via Recovery reflog
Covered by (backend, partial): `deletes_other_branches_with_safe_or_forced_delete` (branch.rs); GUI path not automated

### HIS-68 Current branch cannot be deleted
Fixture: `fx-branchy`
Layer: product
- Then **Delete branch…** on `main` is disabled with a reason (*The current branch cannot be deleted.*)
Covered by (Not automated)

### HIS-69 Delete on origin with lease — see NET-27

### HIS-70 New tag (annotated, unsigned)
Fixture: `fx-branchy`
Layer: product
- Given c25 selected
- When **Branch › New tag…** (or its search Action), Name `v2.0.0`, Target prefilled with the selected commit or `HEAD`, Message `release`, **Create tag**
- Then an annotated tag at c25 appears as a gold pill; HEAD and index are unchanged; empty name/message keeps **Create tag** disabled; an existing name fails, leaving fields
Covered by (end-to-end, partial): `tag_create_cli_creates_annotated_tags_and_blocks_broken_signing` (cli.rs); GUI path not automated

### HIS-71 New signed tag
Fixture: `fx-signed`
Layer: product
- Then **Sign tag** starts from `tag.gpgsign`; checked, the button reads **Create signed tag** and the tag verifies under **Signature details…**; with no key configured (`fx-linear`) **Sign tag** is disabled with *Signing is unavailable.*; a signing failure leaves no tag
Covered by (end-to-end, partial): `tag_create_cli_creates_annotated_tags_and_blocks_broken_signing` (cli.rs); GUI path not automated

### HIS-72 Delete tag
Fixture: `fx-branchy`
Layer: product
- When **Delete tag…** on `v0.1.0`, **Delete**
- Then **Delete tag** asks `Delete local tag v0.1.0?`; confirming removes it and the pill disappears; **Cancel** keeps it
Covered by (backend, end-to-end, partial): `deletes_a_tag_and_rejects_invalid_names` (crates/git/tests/tag.rs), `branch_and_tag_delete_cli` (cli.rs); GUI path not automated

### HIS-73 Unborn branch
Fixture: `fx-unborn`
Layer: product
- Then the History tree reads *No commits yet on main* with **Go to Local Changes**; the sidebar lists `main` with *no commits*; New branch explains the unborn HEAD; Copy ID/Actions are disabled
Covered by (backend, interaction): `empty_history_explains_itself_only_after_the_scan`, `new_branch_without_a_commit_explains_the_unborn_head` (main.rs), `unborn_and_detached_heads_are_explicit` (refs.rs)

### HIS-74 Detached HEAD
Fixture: `fx-branchy-detached`
Layer: product
- Then no branch row is checked or bold; c28 is HEAD in the graph; switching to `main` from the ref menu works
Covered by (backend, partial): `unborn_and_detached_heads_are_explicit` (refs.rs), `rejects_dirty_missing_and_unborn_branches_and_switches_from_detached_head` (branch.rs); GUI path not automated

### HIS-75 Shallow History tree boundary
Fixture: `fx-shallow`
Layer: product
- Then the History tree shows 5 commits, `Commit · 5`, the oldest has no parent row beyond the boundary, and the window identity ends with `shallow`
Covered by (backend, partial): `shallow_history_stops_at_the_available_boundary` (`history.rs`); GUI path not automated

### HIS-76 Bare repository
Fixture: `fx-bare`
Layer: product
- Then the History tree and refs load (30 commits, tags); switch/create-and-switch and Local Changes report the working-tree requirement instead of acting
Covered by (backend, partial): `merge_bare_and_linked_worktree` (`history.rs`); GUI path not automated

### HIS-77 Refs CLI is read-only
Fixture: `fx-remote`
Layer: product
- When `git good --cli refs` runs
- Then JSON lists HEAD, local branches, remotes, tags with full/short names, peeled IDs and tracking; `refs/gg/` is excluded; the repository is byte-identical afterwards
Covered by (backend, end-to-end): `refs_cli_streams_branches_and_tags_without_mutating` (cli.rs), `reads_head_branches_remotes_and_annotated_tags` (refs.rs)

### HIS-79 Delete branch revalidates on confirm
Fixture: `fx-branchy`
Layer: product
- Given **Delete branch** for `old` is open listing no commits
- When a terminal moves `old` to a new commit not in `main` (`git update-ref refs/heads/old <commit>`) and the user confirms
- Then nothing is deleted; the list refreshes to `NOT IN THE CURRENT BRANCH` with that commit, the primary becomes `Delete old and 1 unmerged commit`, and the notice reads `The repository changed; review the updated list and confirm again.`; a second confirm deletes it
Covered by (interaction, partial): `guard_confirms_only_what_the_dialog_showed` (apps/git-good/src/consequence.rs) (the shared re-read guard; the Delete branch dialog is not driven)

### HIS-80 Narrow columns and one pill for an in-sync branch
Fixture: `fx-remote-even`, narrow then 1120 px
Layer: visual
- Then at 720 px the History tree header reads `Commit · n`, `Hash`, `Date` (no Author), and below a 600 px list Date drops too, so subjects never truncate before them; `main` at the tip shows one pill `main ⇄ origin`; at 1120 px Author is back and the pills read `main` and `origin/main`
Covered by (interaction): `narrow_history_drops_author_and_merges_in_sync_pills` (main.rs); `author_drops_first_then_date_below_600`, `a_branch_in_sync_with_its_upstream_shows_one_pill` (history_view.rs)

### HIS-78 Status hint at narrow width — see FRM-55

## Review and reader

Narrow (720×480) is below the 1040 px details/History tree break. "Reader" is the shared `DiffView` on the right of the 300 px file list.

### REV-01 Enter opens Review for the selected commit
Fixture: `fx-linear`
Layer: interaction
- Given the History tree with `c29` selected
- When the user presses `Enter` (or **Commit → Review changes**)
- Then Review opens: header row holds the back pill `‹ History tree` with dimmed `Esc`, the `c29` subject (truncated first, full text on hover), its 8-character ID in a pill, and `author · YYYY-MM-DD`; the file list header reads `FILES n`; the first file is selected and shown in the reader
Covered by (interaction): `enter_opens_selected_commit_at_both_widths` (main.rs), `commit_identity_shares_one_line_with_back` (review_view.rs)

### REV-02 Back restores the exact History tree place
Fixture: `fx-linear`
Layer: interaction
- Given the History tree filtered by `Update`, scrolled so the selected row `c20` sits mid-viewport, then Review opened on it
- When the user presses `Esc` from the file list (or clicks `‹ History tree`)
- Then the History tree shows the same filter, `c20` selected, and the same pixel offset; the History tree results that arrived while Review was open are applied only after the place is restored
Covered by (interaction): `review_close_restores_the_recorded_history_location` (main.rs), `review_navigation_and_late_history_preserve_history_until_back` (interaction_tests.rs)

### REV-03 Back pill names its caller
Fixture: `fx-backups`
Layer: interaction
- Given Review opened from **Recovery…** via **Open commit** (and separately from the rebase plan via **Open in Review**)
- Then the back pill reads `‹ Recovery` (resp. `‹ Rebase`); `Esc` returns to the Backups source with the same entry selected (resp. the unchanged plan)
Covered by (interaction): `recovery_caller_names_its_back_target` (review_view.rs), `rebase_dialog_opens_review_and_returns_to_the_same_plan` (main.rs)

### REV-04 Root commit compares with the empty tree
Fixture: `fx-linear`
Layer: product
- Given the History tree with `c01 Initial commit` selected
- When Review opens
- Then every file is listed as added (`+`); the Before endpoint reads `Empty tree`; **Compare with** lists no parent, only **HEAD**
Covered by (backend, partial): `changed_files_match_git_for_root_nested_and_type_changes` (`crates/git/tests/history.rs`); GUI path not automated

### REV-05 Merge defaults to first parent
Fixture: `fx-branchy`
Layer: product
- Given Review on `Merge branch 'topic'`
- Then the comparison is against parent 1; **Compare with** lists `parent 1 <short>` and `parent 2 <short>` in stored order with one-based numbers and full IDs in tooltips, plus **HEAD**
Covered by (backend, partial): `review_merge_defaults_to_first_parent_and_can_select_second` (`crates/git/tests/history.rs`); GUI path not automated

### REV-06 Switching parent keeps the displayed comparison until the replacement arrives
Fixture: `fx-branchy`
Layer: interaction
- Given Review on the merge with the pointer over the reader
- When the user chooses `parent 2` in **Compare with**
- Then the old file list and diff stay visible and labeled with their identity while loading; the new list and diff are adopted together as soon as both arrive, even under the pointer; the new comparison starts at its first file; no loading text appears in the panes
Covered by (interaction): `parent_replacement_keeps_labelled_content_until_it_arrives_and_resets_selection` (review_view.rs)

### REV-07 Compare with HEAD
Fixture: `fx-linear`
Layer: product
- Given Review on `c25`
- When the user chooses **HEAD** in **Compare with** (or **Commit → Compare with HEAD** from the History tree)
- Then the comparison reloads in place; the Before/After header shows `HEAD` with the resolved commit; parents stay listed; choosing a parent switches back
Covered by (interaction): `compare_with_switches_between_head_and_a_parent` (review_view.rs), `against_comparison_names_before_and_after_sources` (review_view.rs)

### REV-08 Endpoint label shows the ref alone until its commit is known
Fixture: `fx-linear`
Layer: interaction
- Given Review compared with **HEAD** before the resolved commit has arrived
- Then the endpoint reads `HEAD` only, then `HEAD (<short>)` once known; reader endpoint labels appear only when displayed content matches the requested identity
Covered by (interaction): `pending_endpoint_labels_wait_for_matching_content` (diff_view.rs)

### REV-09 Missing parent object gives an incomplete-comparison state
Fixture: `fx-shallow`
Layer: product
- Given Review on the oldest available commit (its parent object is absent)
- Then the reader/list show an identified incomplete-comparison state with metadata retained and a retry, not an empty or root comparison
Covered by (Not automated)

### REV-10 Empty comparison explains itself
Fixture: `fx-linear` (Given: `git commit --allow-empty -m "c31 Empty"`)
Layer: product
- Given Review on `c31 Empty`
- Then the file list is blank (no selection) and the reader says *This commit changes no files*
Covered by (Not automated)

### REV-11 Empty comparison against HEAD or a parent
Fixture: `fx-branchy`
Layer: product
- Given Review on HEAD compared with **HEAD**; then Review on the merge where parent 2 equals the merge tree
- Then the reader says *No differences from HEAD*; for the parent case *No changes against parent n* with button **Compare with parent m**, which reloads against parent m
Covered by (Not automated)

### REV-12 File list rows: marker, elided path, tooltip
Fixture: `fx-file-history` (Review on the rename commit of `src/old.rs` → `src/lib.rs`)
Layer: visual
- Then rows are 22 px with one-character markers (`+ ~ − T R`), the path elided at its start (`…/folder/file.rs`) so the file name stays visible, full path and change kind on hover; the rename is one row, `R` `src/lib.rs`, its hover `R Renamed from src/old.rs`, and `src/old.rs` is not listed; rows are in Git tree order
Covered by (backend, interaction): `compact_rows_and_placeholder_identity_survive_arrival` (review_view.rs), `comparison_is_pinned_and_reads_trees_without_blobs` (`crates/git/tests/history.rs`)

### REV-13 Type change marker
Fixture: `fx-file-history` (Given: a commit replacing `link` symlink with a regular file)
Layer: visual
- Then the row shows `T` with "type changed" on hover; its line count includes every line of both sides
Covered by (backend, partial): `changed_files_match_git_for_root_nested_and_type_changes` (`crates/git/tests/history.rs`); GUI path not automated

### REV-14 Per-file line counts in a fixed slot
Fixture: `fx-file-history`
Layer: visual
- Given Review on a commit editing `src/lib.rs`, adding `blob.bin`, and adding `long.txt`
- Then counts read like `+18 −7` (zero sides omitted), `bin` for `blob.bin`, `12k`-style past 9,999; the slot is as wide as `+9999 −9999` so rows never move when counts arrive; uncounted rows leave the slot empty
Covered by (interaction): `line_stat_labels_follow_the_mockup` (review_view.rs), `file_list_header_and_line_counts_keep_their_slots_at_both_widths` (review_view.rs)

### REV-15 Counts match git numstat and skip over-budget sides
Fixture: `fx-file-history`
Layer: product
- Given a commit changing `big.log` (> 2 MiB) and `src/lib.rs`
- Then `src/lib.rs` counts equal `git diff --numstat`; `big.log` stays uncounted; gitlinks and unreadable objects stay uncounted
Covered by (backend, partial): `line_stats_match_git_numstat` (crates/git/tests/review_special.rs), `line_stats_count_changes_and_skip_binary_or_oversized_sides` (crates/engine/src/review.rs); GUI path not automated

### REV-16 Header count and totals show progress until complete
Fixture: `fx-large` (or any comparison with thousands of files)
Layer: visual
- While listing, the header reads `FILES n+`; the green/red `+x −y` totals have a trailing `+` on each number until every listed file is counted; then exact values without `+`
Covered by (interaction): `file_list_header_and_line_counts_keep_their_slots_at_both_widths` (review_view.rs)

### REV-17 Unread rows keep fixed empty slots
Fixture: `fx-large`
Layer: interaction
- Given a long file list being streamed
- Then unread rows are empty fixed-height slots, loaded rows never shift, and no pane shows loading text; the reader's one-line state slot is reserved and shows only completeness notes and errors
Covered by (interaction): `unread_rows_stay_empty_and_loaded_rows_stay_put` (review_view.rs), `header_controls_keep_their_place_across_states` (review_view.rs)

### REV-18 Unicode and undecodable paths keep distinct labels
Fixture: `fx-linear` (Given: commit adding `café.txt`, `a\b.txt`, and a file whose name contains byte `0xFF` and a tab)
Layer: product
- Then labels preserve Unicode, escape backslash/controls/invalid bytes as `\xNN`, and are pairwise distinct; the copy action for an escaped path is labeled **Copy escaped path**; a Unicode path copies verbatim
Covered by (backend, partial): `unusual_paths_keep_distinct_one_line_labels_and_original_bytes` (crates/git/tests/review_special.rs), `path_display_preserves_unicode_and_escapes_ambiguous_bytes` (crates/engine/src/review.rs); GUI path not automated

### REV-19 Selecting a file keeps the displayed diff until replacement
Fixture: `fx-linear`
Layer: interaction
- Given Review on a multi-file commit, reader showing file A scrolled down, pointer on the reader
- When the user selects file B with the keyboard in the list
- Then A's rows and offset stay until B's identity, metadata, and rows arrive together; an explicit selection publishes even with the pointer resting on the reader; B starts at its first row
Covered by (interaction): `selection_keeps_displayed_diff_until_replacement_arrives` (diff_view.rs), `reading_keys_scroll_without_changing_file_and_a_replacement_shows_under_the_pointer` (diff_view.rs)

### REV-20 Rapid selection never shows a stale file
Fixture: `fx-large`
Layer: interaction
- When the user holds `Down` through many files
- Then superseded diff requests are cancelled; the reader finally shows exactly the last selected file with matching header path and endpoints
Covered by (interaction): `packed_diff_stress_and_rapid_selection` (diff_view.rs), `selection_cancels_old_work_and_pages_reuse_the_document` (diff_worker.rs)

### REV-21 A page arriving under the pointer fades in and refuses an early click
Fixture: `fx-file-history` (`long.txt` change)
Layer: interaction
- Given the pointer resting over the reader while a later page of rows arrives
- Then rows show as they arrive; the row under the pointer fades in and ignores a click within 0.3 s of changing; the scroll position holds
Covered by (interaction): `a_page_arriving_under_the_pointer_fades_in_and_refuses_an_early_click`, `a_hunk_that_changes_under_the_pointer_refuses_its_stage_button_briefly`, `stale_pages_are_ignored_and_a_page_fills_its_slots_at_once` (diff_view.rs)

### REV-22 Retry keeps selection, anchor, and diff
Fixture: `fx-linear`
Layer: product
- Given a comparison whose listing failed mid-way (e.g. worker error injected)
- When the user chooses retry
- Then the selected path, displayed diff, pixel anchor, and known count are retained while enumeration catches up
Covered by (interaction): `retry_same_comparison_keeps_selection_anchor_and_diff` (review_view.rs)

### REV-23 Reader header: one row with path, endpoints, and modes
Fixture: `fx-linear`
Layer: visual
- Given Review wide
- Then one reader header row holds the path and exact endpoints (e.g. `main (a1b2c3d)` / parent), then **Diff | Full file**, **Inline | Side by side**, **Blame**, **File history**, ‹ ›, ⋯; no Stage button; the path and endpoints are not repeated above the reader; modes, blob IDs, and sizes are in the opt-in **File details** row (⋯ › File details ✓, off by default); ⋯ lists 24 px rows: Open in editor, Reveal in folder, Copy path, Export before…, Export after…, File details, LFS / Submodule details… when they apply, then **Discard file…** last in red (Local Changes, unstaged)
Covered by (interaction): `the_reader_header_names_both_exact_endpoints` (diff_view.rs), `header_controls_keep_their_place_across_states` (review_view.rs), `file_actions_are_menu_rows_with_discard_last_in_red`, `workspace_uses_one_reader_header_with_on_demand_details` (status_view.rs)

### REV-24 Byte sizes per side
Fixture: `fx-linear`
Layer: visual
- Then **File details** labels the Before and After byte sizes and exact modes/object IDs
Covered by (interaction): `byte_sizes_identify_each_side_in_the_rendered_header` (diff_view.rs)

### REV-25 Unified diff content
Fixture: `fx-linear` (Given: commit editing a file without a final newline)
Layer: product
- Then rows are fixed-height with old/new line numbers, `+`/`−`/context symbols and colors, hunk headers, and a missing-final-newline marker; bytes match `git diff`
Covered by (backend, partial): `byte_exact_lines_context_and_missing_newline` (crates/engine/src/diff.rs), `selected_file_diff_matches_git_and_preserves_original_bytes` (`crates/git/tests/history.rs`); GUI path not automated

### REV-26 Context lines 3 · 5 · 10
Fixture: `fx-file-history`
Layer: product
- Given Preferences › Diff › **Context lines** set to 10
- Then the open comparison is read again as a new generation, centered on the current line, and hunks match `git diff -U10` (default 3)
Covered by (backend, interaction): `context_lines_match_git_unified` (crates/git/tests/diff_options.rs), `changed_diff_options_read_the_comparison_again` (diff_view.rs)

### REV-27 Whitespace option
Fixture: `fx-linear` (Given: commit with a trailing-whitespace-only edit in `ws.txt` and an indentation change in `a.txt`)
Layer: product
- When **Whitespace** is **Ignore at line end**, then **Ignore all**
- Then hunks match `--ignore-space-at-eol` / `-w`; rows keep exact bytes; `ws.txt` reads *Only whitespace changed; whitespace changes are hidden*
Covered by (backend, partial): `whitespace_options_match_git` (crates/git/tests/diff_options.rs), `whitespace_only_changes_leave_no_hunks_and_say_so` (crates/git/tests/diff_options.rs); GUI path not automated

### REV-28 Invisible characters on changed lines
Fixture: `fx-linear` (Given: commit adding a line with a tab and trailing spaces)
Layer: visual
- With **Show invisible characters on changed lines** on (default), added/removed lines draw tab as `→` and each trailing space as `·` in the weak color without shifting columns; context lines and Full file stay plain; off draws plain text
Covered by (backend, interaction): `changed_lines_show_dimmed_tabs_and_trailing_spaces` (diff_view.rs), `invisible_markers_keep_display_columns` (crates/engine/src/diff.rs)

### REV-29 Long lines scroll sideways by default
Fixture: `fx-file-history` (Given: a 400-character line in `src/lib.rs`)
Layer: interaction
- With **Wrap long lines** off, the line is clipped and Shift+wheel/trackpad scrolls text horizontally; in Side by side both columns share one offset; no horizontal scroll bar; only visible text is shaped
Covered by (interaction): `horizontal_window_shapes_only_visible_unicode` (diff_view.rs)

### REV-30 Wrap long lines
Fixture: as REV-29
Layer: interaction
- When **Wrap long lines** is turned on
- Then Diff (both layouts) and Full file continue the line on extra fixed-height rows broken at the last whole monospace column; gutter only on the first row; no sideways scrolling; the top row stays in view; resizing re-measures; turning it off restores scrolling
Covered by (interaction): `wrapped_long_lines_fit_the_reader_in_every_layout` (diff_view.rs), `wrapping_keeps_the_top_row_and_turning_it_off_restores_scrolling` (diff_view.rs), `rows_map_both_ways` (wrap_rows.rs)

### REV-31 Side by side layout
Fixture: `fx-linear`
Layer: visual
- When the user clicks **Side by side** (or **View → Side-by-side diff**)
- Then Before is left and After right, each with own line numbers/signs; k-th removed pairs with k-th added and the shorter side is padded; context in both; hunk headers and no-newline notes span both; the top row stays in view; the View menu item is checked; the choice applies to every reader in the window and persists (`diff_layout`)
Covered by (interaction): `side_by_side_shows_before_and_after_columns` (diff_view.rs), `deletions_pair_with_additions_and_the_shorter_side_is_padded` (split_rows.rs)

### REV-32 Change navigation and line selection work side by side
Fixture: `fx-linear`
Layer: interaction
- In Side by side, `[`/`]`, keyboard scrolling, and the cursor behave as inline
Covered by (interaction): `side_by_side_keeps_change_navigation_and_line_selection` (diff_view.rs)

### REV-33 Inline | Side by side disabled in Full file
Fixture: `fx-linear`
Layer: interaction
- When the user switches to **Full file**
- Then the layout control keeps its slot, disabled with *Full file shows one side; choose Diff to compare* (see gaps: final-design puts Before | After in that slot instead)
Covered by (Not automated)

### REV-34 Change navigation with ‹ › and `[` `]`
Fixture: `fx-file-history` (commit with 3 hunks in `src/lib.rs`, first offscreen hunk far below)
Layer: interaction
- `]` jumps to the next hunk (requested from the retained document, even off-page), `[` to the previous; **Prev change**/**Next change** name the current change and total; they disable at the ends without wrapping and are disabled when the file has no changes
Covered by (interaction): `diff_change_navigation_reaches_the_first_hunk_and_stops_at_the_ends` (diff_view.rs), `change_key_scrolls_to_an_offscreen_hunk` (diff_view.rs), `a_single_change_is_reachable_and_then_terminal_in_both_modes` (diff_view.rs)

### REV-35 Full file presentation keeps place
Fixture: `fx-file-history`
Layer: interaction
- Given Diff with the cursor on a hunk
- When the user clicks **Full file**
- Then the After side opens on the first changed line of that hunk with one-based line numbers; added lines (After) / deleted lines (Before) carry markers and diff colors; switching Before/After and back to Diff keeps path bytes and line
Covered by (interaction): `mode_toggle_from_a_hunk_anchors_the_first_changed_line` (diff_view.rs), `full_file_side_switch_and_mode_toggle_keep_the_line` (diff_view.rs), `full_file_marks_changes_and_navigates_by_change` (diff_view.rs)

### REV-36 Full file cursor and range keys
Fixture: `fx-file-history`
Layer: interaction
- In Full file: click sets the cursor, Shift-click extends; `Up`/`Down` and `j`/`k` move, `Shift+Up/Down` extend, `PgUp`/`PgDn` move a page, and the cursor stays visible; `[`/`]` move between runs of marked lines and stop at both ends
Covered by (interaction): `full_file_change_navigation_handles_both_boundaries` (diff_view.rs)

### REV-37 Full file explicit summaries
Fixture: `fx-file-history`
Layer: product
- Full file on an added file's Before side, on `big.log`, on `blob.bin`, on `link`, and on a gitlink side
- Then each shows an explicit summary (absent side, over 2 MiB / over 100,000 lines, binary, symlink target, gitlink commit), never empty text
Covered by (backend, partial): `full_file_absent_and_gitlink_sides_are_explicit` (crates/git/tests/review_special.rs), `empty_binary_and_over_budget_are_explicit` (crates/engine/src/file.rs); GUI path not automated

### REV-38 Preferred starting presentation
Fixture: `fx-file-history`, Preferences › Diff › **Open files in** = Full file
Layer: interaction
- Then a newly opened reader starts in Full file; `D`/`F` still switch; a restored reading place wins
Covered by (interaction): `a_new_reader_starts_in_the_preferred_presentation` (diff_view.rs)

### REV-39 Large text diff budget
Fixture: `fx-file-history`
Layer: product
- Given a commit changing `big.log` (> 2 MiB) and `long.txt` (> 100,000 combined lines)
- Then each reader shows an explicit incomplete summary naming the budget with a completion state and per-side export; the UI stays responsive
Covered by (backend, partial): `selected_file_binary_and_large_object_return_explicit_summaries` (`crates/git/tests/history.rs`), `empty_binary_limits_and_cancellation_are_explicit` (crates/engine/src/diff.rs); GUI path not automated

### REV-40 Mode-only, symlink, gitlink presentations
Fixture: `fx-submodule` (submodule bump), `fx-linear` (Given: commit `chmod +x run.sh`; commit retargeting a symlink)
Layer: product
- Then mode-only shows before/after modes and unchanged content; symlink shows labeled target text; gitlink shows recorded before/after commit IDs even when submodule objects are unavailable
Covered by (interaction): `gitlink_change_exposes_recorded_before_and_after_ids` (diff_view.rs)

### REV-41 Image compare metadata and layouts
Fixture: `fx-file-history` (commit changing `img/logo.png`)
Layer: visual
- Then the reader keeps its one header row with **Side by side | Swipe | Onion skin | Hex** in the layout slot and a **Fit | 1:1** segment; Diff | Full file, Blame and ‹ › are hidden; each side names its short blob ID, exact dimensions, encoded bytes; a theme-aware checkerboard shows transparency; the starting layout follows Preferences › **Compare images** and then the last choice
Covered by (interaction): `presentation_controls_metadata_and_swipe_are_rendered`, `the_layout_starts_from_and_remembers_the_last_choice` (binary_preview.rs); `image_readers_keep_the_one_row_header_without_text_controls` (diff_view.rs)

### REV-42 Fit and 1:1
Fixture: as REV-41
Layer: visual
- **Fit** preserves aspect and never upscales a small image; **1:1** shows actual size with scrolling
Covered by (interaction): `fit_preserves_aspect_and_never_upscales` (binary_preview.rs)

### REV-43 Swipe boundary
Fixture: as REV-41
Layer: interaction
- In **Swipe**, After is clipped at the **Boundary** slider in the strip under the canvas; arrow keys move it when focused; dragging on the canvas moves it; each image aligns by its own dimensions without stretching
Covered by (interaction): `presentation_controls_metadata_and_swipe_are_rendered` (binary_preview.rs)

### REV-44 Onion skin requires both images
Fixture: `fx-file-history` (commit adding `img/logo.png` — Before absent)
Layer: product
- In **Onion skin** with both images, After overlays at 50% with the keyboard-adjustable **After opacity** slider in the strip under the canvas; with Before absent Swipe and Onion skin are disabled (*Needs both Before and After images*) and the sides stack with their absence state
Covered by (interaction): `onion_skin_overlays_after_and_requires_both_images` (binary_preview.rs)

### REV-45 Narrow image previews stack
Fixture: as REV-41, narrow
Layer: interaction
- Then the layout tabs stay; Side by side stacks named Before/After previews
Covered by (interaction, partial): `presentation_controls_metadata_and_swipe_are_rendered` (binary_preview.rs) checks the tabs at 720 px

### REV-46 Corrupt or oversized image falls back to hex
Fixture: `fx-file-history` (`blob.bin`; Given: a `.png` with corrupt bytes, and a PNG of 5000×100 px)
Layer: product
- Then a labeled hexadecimal preview of the first 256 bytes with fixed columns is shown; dimensions are checked before decoding pixels; blobs beyond 16 MiB show only a summary plus export
Covered by (interaction): `png_and_jpeg_decode_with_dimensions_and_hex_fallback_is_bounded` (binary_preview.rs), `image_dimensions_are_checked_before_decoding_pixels` (binary_preview.rs), `hexadecimal_rows_keep_fixed_columns_in_narrow_views` (binary_preview.rs)

### REV-47 Export a side exactly
Fixture: `fx-file-history` (commit changing `big.log`)
Layer: product
- When the user exports After from ⋯ › **Export after…** (dialog titled `Export after version`, File / Version / Save to with **Browse…**, Cancel `Esc`, primary **Export** `Enter`, **Exporting…** while busy) to a new path
- Then the dialog closes, the footer reads `Exported <n> bytes to <path>`, and the written file is byte-identical to the blob (also beyond the diff budget); a symlink exports its target bytes; an absent side is not offered
Covered by (interaction, end-to-end): `export_is_exact_beyond_diff_budget_and_preserves_existing_destinations` (tests/cli.rs), `export_capabilities_follow_the_displayed_sides` (diff_view.rs), `a_finished_export_closes_and_reports_in_the_footer`, `the_dialog_names_the_side_version_and_a_suggested_file` (export.rs)

### REV-48 Export never overwrites
Fixture: as REV-47
Layer: product
- When the chosen destination already exists
- Then export refuses with the inline error `That file already exists. Choose a new name.` and the dialog stays; the existing file is untouched; no temporary file remains
Covered by (interaction, end-to-end): `export_is_exact_beyond_diff_budget_and_preserves_existing_destinations` (tests/cli.rs), `a_finished_export_closes_and_reports_in_the_footer` (export.rs)

### REV-49 Escape cancels export first
Fixture: as REV-47
Layer: interaction
- Given an export in progress
- When the user presses `Esc`
- Then the export window closes, work is cancelled, the partial temp file is removed, and Review stays open (a second `Esc` navigates back)
Covered by (interaction, partial): `escape_closes_export_and_cancels_its_work` (export.rs), `cancelled_export_removes_partial_file` (export.rs)

### REV-50 Prev/Next file while reading
Fixture: `fx-linear`, narrow
Layer: interaction
- Given the narrow reading view on file 2 at line 40
- When the user presses `Alt+Down` (or **Next file**)
- Then file 3 opens in the same presentation and side, at line 40 or clamped; `Alt+Up` goes back
Covered by (interaction): `reading_mode_exposes_file_and_change_navigation_at_both_widths` (review_view.rs)

### REV-51 Narrow layout: list then reader
Fixture: `fx-linear`, narrow
Layer: interaction
- Given Review narrow showing the file list over MESSAGE (the commit body and `author · date`); the identity row reads **‹ History tree** `Esc`, the subject, the ID pill, and `vs parent <id>` (choosing HEAD there compares with HEAD)
- When the user presses `Enter` on a file
- Then a full-width reading view opens whose one header row reads **‹ Files** `1/3`, the path (start elided), Diff | Full file, Inline | Side by side, ‹ ›, ⋯ (Blame and File history inside; `B` and `H` still work); there are no Prev/Next file buttons (`Alt+Up/Down` change files) and no endpoint text; `F`, **‹ Files**, or Left return to the list with the same file selected and the list offset preserved; `Esc` returns to the History tree
Covered by (interaction): `enter_and_left_open_and_return_to_files_at_both_widths`, `narrow_files_key_returns_to_the_list_and_escape_to_the_caller`, `reading_mode_exposes_file_and_change_navigation_at_both_widths` (review_view.rs)

### REV-52 Narrow reader folds tools
Fixture: `fx-linear`, reader width < 760 px
Layer: interaction
- Then **Blame** and **File history** move into the ⋯ menu; when the path does not fit, controls wrap to a second line
Covered by (interaction): `a_narrow_reader_folds_blame_into_the_file_actions_menu` (diff_view.rs)

### REV-53 Blame in Full file
Fixture: `fx-file-history`
Layer: product
- Given Diff on `src/lib.rs`
- When the user presses `B` (or **Blame**)
- Then Full file opens with **Blame** pressed and a gutter where each run of lines from one commit shows one label (author, short ID, compact age such as `5d`) and every line an age bar, brighter for newer commits; After uses the selected commit, Before the parent; selecting a line fills the 26 px bar at the reader bottom with `LINE n`, author · short ID · `5 days ago` · subject, **Open commit** and **Line history**; `B` again hides it
Covered by (backend, interaction): `attribution_column_and_selected_line_metadata_render` (diff_view.rs), `runs_collapse_to_one_label_per_commit`, `newer_commits_draw_brighter_age_bars`, `ages_read_compactly_in_the_gutter_and_fully_in_the_bar`, `the_bar_keeps_its_height_with_and_without_a_line` (blame_view.rs), `attributes_a_bounded_range_and_preserves_origin_lines` (crates/git/src/blame.rs)

### REV-54 Blame refuses non-text
Fixture: `fx-file-history`
Layer: product
- On `blob.bin`, `link`, `big.log`, and in Local Changes
- Then Blame is disabled with the reason (binary, symlink, over budget, working tree)
Covered by (backend, partial): `binary_content_is_refused` (crates/git/src/blame.rs); GUI path not automated

### REV-55 Blame Open commit keeps return path
Fixture: `fx-file-history`
Layer: interaction
- Given a blamed line selected
- When the user chooses **Open commit**
- Then Review opens on that commit; Back returns to the original reader place
Covered by (Not automated)

### REV-56 File history opens in the reader
Fixture: `fx-file-history`
Layer: interaction
- Given Review on the newest commit with `src/lib.rs` selected
- When the user presses `H` (or **File history**)
- Then the file-list pane shows the version list: header `FILE HISTORY 12+` with ×, 22 px rows of kind · short ID · subject (a rename adds `← old path`) · date, the selected row being the comparison; **File history** stays pressed; scrolling to the end loads the next page with its slot blank (no loading text); `H` again does not stack another source; × or `Esc` closes it
Covered by (interaction): `history_opens_from_the_wide_reader_and_never_stacks` (review_view.rs), `the_version_list_names_each_version_and_its_old_path` (file_history_view.rs)

### REV-57 Versions skip unchanged commits
Fixture: `fx-file-history`
Layer: product
- Then the counter totals 12 content versions, not the number of commits; **Diff** shows previous → current, **Full file** the current version; Before/After never navigates versions
Covered by (backend, interaction): `linear_history_skips_commits_that_carry_the_same_content` (crates/git/tests/file_history.rs), `the_edge_uses_before_and_after_sides_and_an_unknown_edge_has_no_diff` (file_history_view.rs)

### REV-58 File history follows the rename
Fixture: `fx-file-history`
Layer: product
- Given a file was renamed in a commit (`Pane e pizza/Focaccia.md` → `Pane e pizza/Focaccia genovese.md`)
- When the user reviews that commit
- Then the file list shows one row, `R` at the new path, not a deleted and an added file
- When the user opens File history on it (`H`)
- Then the version list runs past the rename: the rename version reads `← Pane e pizza/Focaccia.md` and the older versions follow the old path down to the commit that added it
- And in `fx-file-history`, `Alt+Left` past version 6 labels the version a rename naming `src/old.rs`; older versions read `src/old.rs`
Covered by (backend, interaction): `renames_are_followed_and_name_the_old_path` (crates/git/tests/file_history.rs), `renames_are_one_change_with_the_old_path_as_git_pairs_them` (crates/git/tests/review_special.rs), `a_renamed_file_reviews_as_one_rename_and_its_history_crosses_it` (apps/git-good/src/main.rs)


### REV-78 File history follows a renamed and rewritten file
Fixture: as in `baol/recipes` 935e221: `Pane e pizza/Focaccia.md` renamed to `Pane e pizza/Focaccia con il Bimby.md` in a commit that also rewrote most of it (27 % alike there, 30 % in the test) and deleted `notes.md` (10 % alike)
Layer: product
- Given a file was added in a commit that deleted one or more files
- When the user opens File history on it (`H`)
- Then File history continues with the most similar deleted file, if at least 25 % of the content matches (`git diff -M25%`): the rename version reads `← Pane e pizza/Focaccia.md` and the older versions follow `Focaccia.md` down to the commit that added it
- And Review of that commit shows one `R` row for the rename, and `notes.md` as deleted
- And a new file less than 25 % alike to every deleted file starts its own File history (`Added`)
Covered by (backend, interaction): `a_rewritten_rename_follows_the_most_similar_deleted_file_down_to_25_percent`, `a_new_file_less_than_25_percent_like_a_deleted_one_starts_its_own_versions` (crates/git/tests/file_history.rs), `a_rewritten_rename_reviews_as_a_rename_of_the_most_similar_deleted_file` (crates/git/tests/review_special.rs), `a_renamed_and_rewritten_file_keeps_its_file_history` (apps/git-good/src/main.rs)


### REV-79 The absent version names the commit just before the file
Fixture: as in `baol/recipes`: `Focaccia.md` added in d7fb0bb, whose parent is `d46c131 Update pancakes.`; the repository starts at `Initial commit`
Layer: product
- Given a file that did not exist before the commit that added it
- When the user opens File history on it and reaches its oldest version
- Then the last row is the absent version at the commit just before the file was added (`Update pancakes.`), not the repository's first commit
- And for a file that was deleted and later re-added, the absent version between them is the commit that deleted it
Covered by (backend, interaction): `the_absent_version_before_a_file_is_the_commit_just_before_it_was_added`, `the_absent_version_after_a_deletion_is_the_commit_that_deleted_it` (crates/git/tests/file_history.rs), `a_renamed_and_rewritten_file_keeps_its_file_history` (apps/git-good/src/main.rs)


### REV-80 File history dates each version and names the selected author
Fixture: `Focaccia.md` added by Ada Lovelace on 2026-04-16 09:30 +02:00 and changed by Grace Hopper on 2026-11-08 14:02 +01:00; reported on `baol/recipes`, where rows read `11-08` with no year and nothing named the selected version's author or date
Layer: product
- Given a file with versions by different authors on different days
- When the user opens File history on it
- Then each row ends with its full date in the author's time zone, as the History tree's Date column shows it (`2026-11-08`, `2026-04-16`; relative with Preferences › Appearance › Relative dates)
- And the reader header's After endpoint names the selected version's author and time: `After … · Grace Hopper · 2026-11-08 14:02`
- When the user steps to the older version (`Alt+Left`)
- Then the header reads `Ada Lovelace · 2026-04-16 09:30`
Covered by (interaction): `file_history_shows_each_version_date_and_the_selected_author` (apps/git-good/src/main.rs)

### REV-81 Files `.gitattributes` marks binary review as binary
Fixture: `fx-attributes`
Layer: product
- Given `.gitattributes` holds `*.dat binary` and `*.min.js -diff` (Pro Git 8.2, *Identifying Binary Files*)
- When the commit changing `data.dat`, `app.min.js` and `note.txt` is reviewed
- Then `data.dat` and `app.min.js` show *Binary content (.gitattributes)* with no text rows and count no lines, as `git diff` (`Binary files … differ`) and `git show --numstat` (`-  -`) report them; `note.txt` keeps its text diff; attributes are read in process from the worktree's attribute files (the index's in a bare repository), `info/attributes` and `core.attributesFile`, with no Git process
Covered by (backend): `gitattributes_binary_and_no_diff_files_review_as_binary` (crates/git/tests/review_special.rs)

### REV-59 Version boundaries never wrap
Fixture: `fx-file-history`
Layer: interaction
- At the oldest version `Alt+Left` does nothing and the list header's tooltip names `Oldest version`; at newest `Alt+Right` stops (`Newest version`); a single-version file says `Only version`; while a page loads the reason names the pending page; budget hit names the limit
Covered by (interaction): `boundaries_name_the_reason_and_never_wrap` (file_history_view.rs), `a_loading_predecessor_disables_older_and_names_the_reason` (file_history_view.rs), `rapid_version_steps_stop_at_both_boundaries` (file_history_view.rs)

### REV-60 Deleted and re-added versions are explicit
Fixture: `fx-linear` (Given: delete `a.txt`, then re-add it)
Layer: product
- Then the deletion is an absent version (not empty text) and the re-add starts an added edge
Covered by (backend, interaction): `delete_and_readd_produce_an_explicit_absent_version` (crates/git/tests/file_history.rs), `a_deleted_version_compares_the_absent_after_side` (file_history_view.rs)

### REV-61 Ambiguous predecessor asks
Fixture: `fx-file-history-merge` (proposed; Given: two branches each independently commit the same content of `x.txt`, then merge)
Layer: product
- Then File history stops and nests the candidates under the merge row (`CHOOSE EARLIER VERSION 2`, then 22 px rows of kind · short ID · subject and `← old path` · author and date); Up/Down and Enter pick one; it never takes the first parent; Full file shows the current content and Diff is disabled until chosen
Covered by (interaction): `an_ambiguous_predecessor_requires_a_choice`, `the_strip_offers_every_predecessor_choice` (file_history_view.rs), `an_ambiguous_history_edge_keeps_full_content_and_disables_diff` (review_view.rs)

### REV-62 Closing File history restores entry context once
Fixture: `fx-file-history`
Layer: interaction
- Given File history opened at line 25, stepped 3 versions back
- When the user presses `Esc` (or Close File history)
- Then the original comparison, path, presentation, side, cursor, and pixel place are restored exactly once; version steps added no Back entries; the next `Esc` leaves the reader
Covered by (interaction): `history_opens_in_the_reader_and_escape_restores_the_entry_context_once` (review_view.rs), `saved_reader_context_restores_place_and_endpoints` (diff_view.rs)

### REV-63 Selecting another file resets File history
Fixture: `fx-file-history`
Layer: interaction
- Given File history active on `src/lib.rs`
- When the user selects another file
- Then File history stays open at that file's newest version; Close returns to `src/lib.rs` entry context
Covered by (interaction): `selecting_another_file_resets_history_and_keeps_the_return_context` (review_view.rs)

### REV-64 Switching parent leaves File history
Fixture: `fx-branchy`
Layer: interaction
- Given File history open in Review of the merge
- When the user picks `parent 2`
- Then File history closes because the comparison changed
Covered by (Not automated)

### REV-65 Narrow File history
Fixture: `fx-file-history`, narrow
Layer: interaction
- Then the version list sits in a seven-row pane above the full-width reader; `Esc` closes File history first, then leaves Review (`F` returns to the file list)
Covered by (interaction): `narrow_history_uses_the_reading_layout_and_escape_closes_it_first` (review_view.rs)

### REV-66 Failed File history page keeps loaded versions
Fixture: `fx-file-history`
Layer: product
- Given a failure while paging older versions
- Then loaded versions stay, the last list row is one red row `File history page failed · <error>` with **Retry**, the boundary stays pending, and Retry resumes
Covered by (backend, interaction): `a_failed_page_retry_keeps_the_loaded_timeline`, `a_failed_page_is_one_red_row_with_retry` (file_history_view.rs), `cancellation_and_a_failed_step_keep_the_loaded_versions` (crates/git/tests/file_history.rs)

### REV-67 Line history of a range
Fixture: `fx-file-history`
Layer: product
- Given Full file with lines 20–30 selected (Shift-click)
- When the user chooses **Line history** from blame
- Then the version list is titled `LINE HISTORY 20–30`, anchored to the range, and lists the 4 versions that changed it; where the range was introduced the boundary reads **Line introduced in this version**; a deleted range reads **Line absent in this version**; ranges are reported whole, never shrunk
Covered by (backend, interaction): `line_history_names_the_anchor_and_boundary` (file_history_view.rs), `line_mapping_keeps_ranges_whole_and_names_introduced_or_absent_content` (crates/engine/src/file_history.rs)

### REV-68 Line history from the Before side
Fixture: `fx-file-history`
Layer: interaction
- Given Full file **Before** with a deleted line selected
- When **Line history** opens
- Then the cursor starts at the older version and its Newer edge reports the deletion
Covered by (interaction): `initial_line_history_index_selects_the_before_version` (file_history_view.rs)

### REV-69 File history and Blame are read-only
Fixture: `fx-file-history`
Layer: product
- After browsing File history and Blame, `HEAD`, refs, index, and working files are unchanged
Covered by (interaction): `reading_versions_preserves_the_repository` (file_history_worker.rs)

### REV-70 Open in editor
Fixture: `fx-linear`
Layer: product
- When the user chooses ⋯ → **Open in editor** (or **Reveal in folder**) for a file deleted in the worktree
- Then the footer reports the missing file; for a present file the configured editor opens (Reveal in folder opens the folder that holds it)
Covered by (Not automated)

### REV-71 CLI show streams summaries and validates parent
Fixture: `fx-branchy`
Layer: product
- `git good --cli show <merge> --parent 2` → JSON summaries with repository/comparison identity and exact path bytes; `--parent 0` or `3` → error on stderr, nonzero exit; cwd discovery without `--repo`
Covered by (end-to-end): `show_discovers_repo_streams_exact_files_and_rejects_invalid_parent` (tests/cli.rs)

### REV-72 CLI show --file and --against
Fixture: `fx-linear`
Layer: product
- `git good --cli show c20 --file a.txt` → selected-file diff with exact source-line bytes; missing path → stderr error; `--against c10` compares with that base; non-UTF-8 path bytes match
Covered by (end-to-end): `show_file_returns_diff_and_missing_path_uses_stderr`, `show_file_uses_exact_non_utf8_path_bytes`, `show_against_compares_with_an_arbitrary_base` (tests/cli.rs)

### REV-73 CLI export and blame
Fixture: `fx-file-history`
Layer: product
- `git good --cli show <c> --file big.log --export after --output out.log` → JSON receipt, exact bytes, existing destination refused; `git good --cli blame src/lib.rs --lines 20,30` → bounded attributed JSON; bad range → error
Covered by (end-to-end): `export_is_exact_beyond_diff_budget_and_preserves_existing_destinations`, `blame_cli_reports_attributed_ranges_and_errors` (tests/cli.rs)

### REV-75 Find in file
Fixture: a file reading `alpha`, `beta alpha`, `gamma`, `ALPHA`, open in Full file
Layer: interaction
- When the user presses `Ctrl/Cmd+F` in the reader and types `alpha`
- Then the bar reads `1 of 3`, the three matches are highlighted (the current one more strongly), and line 1 is the place
- And Enter goes to `beta alpha`, then `ALPHA`, then back to line 1; Shift+Enter or ↑ goes back; **Aa** shows `…` until the case-sensitive answer arrives and draws no old highlights meanwhile; Esc closes the bar
- And the worker searches the whole document, every page and not just those shown, once per query; `total_x` is not the whole word `total`; character ranges hold past wide characters
- And `Ctrl/Cmd+F` in the History tree still opens search with the History tree filter
Covered by (interaction): `ctrl_f_finds_steps_highlights_and_closes` (apps/git-good/src/diff_view.rs); `find_matches_case_whole_words_and_wide_characters`, `the_whole_document_is_searched_once_per_query` (apps/git-good/src/diff_worker.rs)

### REV-76 Changed words inside changed lines
Fixture: a file whose line `let total = price * count;` becomes `let total = price * amount;`, and a line rewritten throughout
Layer: interaction
- When the user opens the file in Diff, inline or side by side
- Then `count` on the deleted line and `amount` on the added line are painted in a stronger tint than the rest of their lines
- And the rewritten line keeps only its line color; lines past 1,000 characters are not compared
- And when the block starts on one page and ends on the next, each line pairs with the same partner on either page; another file shows none of the previous file's emphasis
Covered by (interaction): `changed_words_are_found_between_paired_lines`, `emphasis_pairs_deleted_and_added_lines_in_order_on_any_page` (crates/engine/src/diff.rs); `changed_words_are_kept_per_row_until_another_document_arrives` (apps/git-good/src/diff_view.rs)

### REV-77 CJK text draws and wraps by its width
Fixture: a file whose added line repeats `漢字NNかな한글`, on a machine with a CJK font installed
Layer: interaction
- When the user opens it in Diff or Full file
- Then the ideographs, kana and Hangul draw from the system font, not as boxes; Latin text keeps its fonts
- And with **Wrap long lines** on, each wide character takes two columns: no wrapped row is wider than the reading area, and the rows rebuild the line in order
- And a file that is not a font, or a font without ideographs, is never installed
Covered by (interaction): `the_first_ideograph_loads_a_system_font`, `a_file_that_is_not_a_cjk_font_is_never_installed`, `known_cjk_files_rank_monospace_and_regular_first` (apps/git-good/src/wide_font.rs); `wide_characters_wrap_as_two_columns` (apps/git-good/src/diff_view.rs); `wide_characters_take_two_columns_and_start_their_row` (apps/git-good/src/wrap_rows.rs)

### REV-74 Inline by default below 1040 px
Fixture: `fx-linear` with Preferences › Diff › Default layout **Side by side**
Layer: interaction
- When a reader opens at 720 px
- Then it shows **Inline**; choosing **Side by side** there switches it for the session without changing the preference; at 1120 px the reader follows the preference
Covered by (interaction): `narrow_windows_start_inline_whatever_the_preference`, `side_by_side_shows_before_and_after_columns` (diff_view.rs)

## Local Changes

Sources: docs/architecture.md "Working status · M2" (Working diffs, File staging, Hunk/line/drop actions, Dropping working changes, Committing, Whole-file unstaging), "Amend", "Signing", rebase-stop commit labels, Preferences › Commit, Automatic refresh, empty states, destructive confirmations; docs/final-design.md Changes menu; docs/user-flows.md F03–F04; docs/cli.md "Stage and commit". The wide layout starts at 1040 px of workspace width.

Expected `fx-dirty` rows (renames appear as deletion + addition, untracked folders as one row per file):
- **Unstaged**: `~ a.txt`, `~ c.txt`, `− d.txt`, `~ run.sh`, `~ logo.png`, `~ crlf.txt`, `~ ws.txt`, `? new.txt`, `? notes/<file1>`, `? notes/<file2>` → header `UNSTAGED 10`.
- **Staged**: `~ a.txt`, `~ b.txt`, `− e.txt`, `+ e2.txt` → header `STAGED 4`.

### CHG-01 Opening Local Changes hides the refs sidebar
Fixture: `fx-dirty`
Layer: interaction
- Given Git Good opened on the History tree with the refs sidebar visible
- When the user chooses **View › Local Changes** (`Ctrl+1`) or the **Local Changes** tab
- Then the toolbar stays visible, the refs sidebar is hidden, and the Local Changes workspace fills the main area; **View › Refs sidebar** (`Ctrl+\`) shows it again on demand
Covered by (interaction): `menu_bar_is_one_row_and_view_shortcuts_switch_views` (apps/git-good/src/main.rs)

### CHG-02 Local Changes tab count badge — see FRM-28

### CHG-03 Stacked Unstaged/Staged lists in one 300 px column (wide)
Fixture: `fx-dirty` (1440×900)
Layer: visual
- When Local Changes opens
- Then the left column is 300 px wide with **Unstaged** above **Staged**; the reader is to the right of both lists; the commit form sits below the reader at the lower right; headers read `UNSTAGED 10` and `STAGED 4`; the status line shows both counts
Covered by (interaction): `wide_and_narrow_geometry_place_both_groups_above_or_beside_the_reader`, `wide_file_list_is_fixed_and_long_paths_keep_their_file_name` (apps/git-good/src/status_view.rs)

### CHG-04 Unstaged takes the rows it has, at most 60% of the column
Fixture: `fx-dirty` (1440×900); variant: `fx-dirty` plus 60 extra untracked files `many/f01..f60`
Layer: visual
- Then in `fx-dirty` the Unstaged list is only as tall as its 10 rows and the `STAGED` header follows directly below it
- And in the variant the Unstaged list stops at 60% of the column height and scrolls; Staged keeps the remaining 40% and stays visible
Covered by (interaction, partial): `empty_groups_say_so_after_the_scan_and_stack` (status_view.rs) (stacking only; 60% cap Not automated)

### CHG-05 Change markers per kind
Fixture: `fx-dirty`; `fx-merge-conflict` for `!`
Layer: visual
- Then each 22 px row shows one marker then the path: `~` modified (`a.txt`, `run.sh`), `−` deleted (`d.txt`, staged `e.txt`), `+` added (staged `e2.txt`), `?` untracked (`new.txt`, `notes/…`), `!` conflicted (`src/app.rs` in `fx-merge-conflict`); `T` for a type change (file → symlink, set up in Given)
- And hovering a row names the full kind and area (e.g. Untracked, Conflict)
Covered by (interaction): `change_markers_are_single_distinct_characters` (status_view.rs)

### CHG-06 Long paths shorten at their start
Fixture: `fx-dirty` plus untracked `very/deeply/nested/folder/structure/that/is/long/file.rs`
Layer: visual
- Then the row reads `…/folder/…/file.rs`-style with the file name fully visible; hover shows the full path
Covered by (interaction): `wide_file_list_is_fixed_and_long_paths_keep_their_file_name` (status_view.rs)

### CHG-07 Untracked folder lists each file; ignored files absent
Fixture: `fx-dirty`
Layer: product
- Then `notes/` appears as two `?` rows (one per file), not one folder row; ignored `build/out.o` does not appear anywhere
Covered by (backend, partial): `status_matches_git_and_preserves_repository` (crates/git/tests/status.rs); GUI path not automated

### CHG-08 Partially staged path appears once in each list
Fixture: `fx-dirty`
Layer: product
- When the user selects `a.txt` in Unstaged, then `a.txt` in Staged
- Then the reader shows index → working file (unstaged hunk only), then HEAD → index (staged hunk only)
Covered by (interaction, partial): `partially_staged_path_is_in_both_groups_and_selects_distinct_comparisons` (status_view.rs)

### CHG-09 Selecting a row opens its comparison immediately
Fixture: `fx-dirty`
Layer: interaction
- When the user single-clicks `c.txt`
- Then the reader shows its diff with endpoints named (index → working file); no **View diff** or **Review & stage** step exists
Covered by (interaction): `workspace_uses_one_reader_header_with_on_demand_details` (status_view.rs)

### CHG-10 Unstaged | Staged toggle and disabled-side reasons
Fixture: `fx-dirty`; `fx-merge-conflict`
Layer: interaction
- When `a.txt` is selected and the user clicks **Staged** in the reader header
- Then the reader switches to HEAD → index and the Staged `a.txt` row is selected
- And for `b.txt` (fully staged) **Unstaged** is disabled with *All changes in this file are staged.*; for `c.txt` **Staged** is disabled with *Nothing from this file is staged.*; for a conflict *Resolve the conflict before staging.*; before the scan reaches the counterpart *Waiting for the status scan*
Covered by (interaction): `reader_toggle_switches_the_same_path_between_unstaged_and_staged`, `toggle_reasons_name_the_missing_side` (status_view.rs)

### CHG-11 Tab moves focus between the groups
Fixture: `fx-dirty`
Layer: interaction
- Given a row in Unstaged is focused
- When the user presses `Tab`
- Then focus moves to Staged and selects its remembered (or first) row; `Tab` again returns to the remembered Unstaged row; inside the commit editor `Tab`/text keys stay with the editor
Covered by (interaction): `tab_switches_keyboard_focus_between_the_file_groups` (status_view.rs)

### CHG-12 Independent selection and scroll per list
Fixture: `fx-dirty` plus 300 untracked files
Layer: interaction
- When the user scrolls Unstaged to the bottom, selects a row there, then selects a Staged row
- Then Unstaged keeps its offset and remembered row; after a refresh both offsets are preserved
Covered by (interaction): `workspace_preserves_independent_list_selection_and_scroll` (status_view.rs)

### CHG-13 Stage one file (row **+**, double-click, `S`)
Fixture: `fx-dirty`
Layer: product
- When the user stages `c.txt` with (a) the row's **+** button (tooltip *Stage this file*), (b) double-click, or (c) `S` with the Unstaged list focused; the reader header has no Stage button
- Then `c.txt` moves to Staged, the index blob equals `git add c.txt`, the working file is unchanged, and selection advances to the next Unstaged row in display order
Covered by (backend, interaction): `row_buttons_stage_and_unstage_one_file`, `double_click_stages_and_unstages_with_the_row_button_and_advances` (status_view.rs); `stage_matches_git_add_and_preserves_working_files` (crates/git/tests/status.rs)

### CHG-14 Staging the last Unstaged row follows it into Staged
Fixture: `fx-linear` plus one unstaged edit to `a.txt`
Layer: interaction
- When the user stages `a.txt`
- Then *No unstaged changes.* shows in Unstaged and the selection follows `a.txt` into Staged
Covered by (interaction): `staging_the_only_file_follows_it_into_staged` (status_view.rs)

### CHG-15 Unstage one file (row **−**, double-click, `U`)
Fixture: `fx-dirty`
Layer: product
- When the user unstages `b.txt` by any of the three gestures
- Then `b.txt` returns to Unstaged, the index entry equals HEAD, the working file is unchanged, other index entries are untouched
- And staging, unstaging, or discarding a file named `*.txt` leaves `a.txt` as it was
Covered by (backend, interaction): `row_buttons_stage_and_unstage_one_file` (status_view.rs); `unstage_matches_head_and_preserves_working_edits_and_other_entries`, `stage_unstage_and_discard_treat_a_glob_name_literally` (crates/git/tests/status.rs)

### CHG-16 Whole-file staging of deletion, untracked, mode change, binary
Fixture: `fx-dirty`
Layer: product
- When the user stages `d.txt`, `new.txt`, `run.sh`, `logo.png` one by one
- Then each moves to Staged: `d.txt` as `−`, `new.txt` as `+`, `run.sh` staged as 100755, `logo.png` with its new bytes; unrelated entries unchanged
Covered by (backend, interaction): `whole_file_actions_preserve_unrelated_index_and_working_data` (status_view.rs); `stage_matches_git_add_and_preserves_working_files` (crates/git/tests/status.rs)

### CHG-75 An image in Local Changes shows its picture
Fixture: `fx-dirty` (`logo.png` modified, staged later), plus an untracked `muffin-farro.jpeg`
Layer: interaction
- Given an image file is new (untracked), modified, or staged
- When the user selects it in Local Changes
- Then the reader shows the image comparison Review uses (the Side by side | Swipe | Onion skin | Hex layouts with Fit | 1:1), not *Binary content* or *No inline preview*
- And each side names where it comes from with its dimensions and size: untracked `Before absent` · `After working · 5 × 5`; unstaged `Before index` · `After working`; staged `Before HEAD` · `After index`
- And the working side is the file as it is on disk, within the 16 MiB preview budget
Covered by (interaction): `an_untracked_and_a_modified_image_show_their_pictures` (apps/git-good/src/status_view.rs)

### CHG-76 Files `.gitattributes` marks binary compare as binary in Local Changes
Fixture: `fx-attributes` with an unstaged edit to `data.dat`
Layer: product
- When `data.dat` is selected in Unstaged
- Then the reader shows *Binary content (.gitattributes)* after the working-snapshot note, with no hunks or lines to stage, as `git diff` reports it (REV-81)
Covered by (backend): `gitattributes_binary_files_compare_as_binary_in_local_changes` (crates/git/tests/status.rs)

### CHG-77 Ignore an untracked file, its extension, or its folder
Fixture: `fx-linear` with untracked `logs/a.log`
Layer: product
- When ⋯ File actions opens on `logs/a.log`
- Then, above **Discard file…**, it offers **Ignore this file** (`/logs/a.log`), **Ignore all \*.log files** (`*.log`) and **Ignore folder logs/** (`/logs/`), each naming its pattern on hover; a file at the root has no folder row and a file without an extension (or a dotfile) has no extension row; a tracked file has none of them
- When one is chosen
- Then Git Good adds the pattern on its own line to `.gitignore` at the repository root (created when missing, a missing final newline added first, other lines kept), the footer reads *Ignored \*.log · added to .gitignore* (or *\*.log is already in .gitignore*, writing nothing), and Local Changes reads again without the ignored files; patterns are anchored to the root and escape `\`, `*`, `?`, `[` and a trailing space, so they match exactly what was chosen (`git check-ignore`); a non-UTF-8 path is refused
- And `git good --cli ignore FILE [--extension|--folder]` adds the same pattern and prints `{"operation":"ignore","pattern","file","added","complete":true}`
Covered by (backend, interaction, end-to-end): `ignore_patterns_match_what_git_ignores`, `ignore_writes_the_root_gitignore_from_a_subfolder` (crates/git/tests/ignore.rs), `patterns_anchor_files_and_folders_and_escape_specials` (crates/git/src/ignore.rs), `an_untracked_file_can_be_ignored_from_file_actions` (apps/git-good/src/status_view.rs), `ignore_adds_the_pattern_and_names_it_in_the_footer` (apps/git-good/src/main.rs), `ignore_cli_adds_the_pattern_for_a_file_its_extension_or_folder` (apps/git-good/tests/cli.rs)

### CHG-17 Unstaging a staged rename restores both halves
Fixture: `fx-dirty`
Layer: product
- When the user unstages `e2.txt` then `e.txt`
- Then `e2.txt` becomes `? e2.txt` in Unstaged and `e.txt` returns to HEAD in the index (shown as `− e.txt` Unstaged while the file is absent on disk)
Covered by (Not automated)

### CHG-18 Conflict and intent-to-add rows are inert
Fixture: `fx-merge-conflict`; `fx-dirty` plus `git add -N new2.txt`
Layer: product
- Then conflicted rows (`src/app.rs`, `docs.md`, `logo.png`) show `!`, no row **+**/**−** button, double-click does nothing, the header **Stage** is disabled with its reason; the reader shows an explicit *Unmerged path* summary with a route to **Resolve conflicts…**
- And the intent-to-add row shows `+`, no row button, double-click is inert
Covered by (backend, interaction): `conflict_row_reads_an_unmerged_summary_and_route` (status_view.rs); `conflicts_are_explicit_and_scan_cancels` (crates/git/tests/status.rs)

### CHG-19 **Stage hunk** stages exactly one hunk
Fixture: `fx-dirty`
Layer: product
- Given `a.txt` selected in Unstaged
- When the user clicks **Stage hunk** on its hunk header
- Then the index matches `git add -p` choosing that hunk; `a.txt` leaves Unstaged if nothing remains; lists and reader refresh in place, scroll offsets kept
Covered by (backend, interaction): `inline_hunk_button_moves_the_hunk_and_keeps_the_workspace` (status_view.rs); `stage_hunks_match_git_add_patch_and_keep_remaining_edits` (crates/git/tests/status.rs)

### CHG-20 **Unstage hunk** restores the HEAD side
Fixture: `fx-dirty`
Layer: product
- Given `a.txt` selected in Staged
- When the user clicks **Unstage hunk**
- Then the index equals HEAD for that hunk, working file unchanged; `a.txt` leaves Staged
Covered by (interaction): `inline_unstage_restores_head_and_keeps_the_workspace` (status_view.rs)

### CHG-21 Line picking and the selection bar
Fixture: `fx-linear` plus a 3-hunk edit to `a.txt`
Layer: interaction
- Given nothing picked, the bar below the reader reads *Click changed lines to pick them · click a hunk header to pick the hunk*
- When the user clicks one added row
- Then the bar offers **Stage 1 line**, a discard button naming the change block it reverts (e.g. **Discard 1 change block (1 line)**, CHG-35), and **Clear**; rows do not move; **Clear** empties the picks
- When the user clicks a hunk header, every changed line of that hunk is picked; clicking it again clears them
Covered by (interaction): `selection_bar_counts_picked_lines_and_clears_them` (apps/git-good/src/status_view.rs)

### CHG-22 **Stage N lines** stages only the picked lines
Fixture: as CHG-21
Layer: product
- When the user picks one line and clicks **Stage 1 line**
- Then only that line is in the index; the file is in both lists (`MM`); picks are forgotten
Covered by (backend, interaction, partial): `inline_selected_line_stages_only_that_line_and_keeps_the_workspace` (status_view.rs); `stage_selected_lines_keeps_unselected_sides_and_marks_partial_stat` (crates/git/tests/status.rs)

### CHG-23 Picks across hunks apply as one mutation; **Unstage N lines**
Fixture: as CHG-21
Layer: product
- When the user picks lines in two hunks and clicks **Stage 2 lines**, then in Staged picks one line and clicks **Unstage 1 line**
- Then both lines are staged together, then the one is restored to HEAD; working file never changes
Covered by (backend, interaction): `picked_lines_across_hunks_stage_and_unstage_as_one_change` (status_view.rs); `unstage_selected_lines_restores_old_sides_and_preserves_working_tree` (crates/git/tests/status.rs)

### CHG-24 Partial actions refused for unsupported kinds
Fixture: `fx-dirty`
Layer: product
- When the user selects `new.txt`, `d.txt`, `run.sh`, `logo.png`
- Then no **Stage hunk**/**Discard hunk** or line picking is offered (binary and mode-only show explicit summaries); the header **Stage** is still available
- And CLI `git good --cli stage --file run.sh --hunk '<any>'` fails with an error and no index change
Covered by (backend, partial): `stage_hunks_reject_unsupported_changes_and_selections` (crates/git/tests/status.rs); GUI path not automated

### CHG-25 Hidden whitespace hides hunk/line actions
Fixture: `fx-dirty`, Preferences › Diff › **Whitespace** set to ignore all
Layer: product
- When the user selects `ws.txt`
- Then the reader says *Only whitespace changed; whitespace changes are hidden*; for `a.txt` hunk/line actions are hidden and the bar reads *Whitespace changes are hidden · choose Show all changes in Preferences › Diff to stage hunks or lines*; whole-file **Stage** and **Discard unstaged changes** stay available
Covered by (backend, interaction): `hidden_whitespace_disables_hunk_and_line_actions` (apps/git-good/src/diff_view.rs); `staging_follows_the_reader_context_and_refuses_hidden_whitespace` (crates/git/tests/status.rs)

### CHG-26 Whitespace-only and CRLF-only changes visible by default
Fixture: `fx-dirty` (default prefs)
Layer: product
- When the user selects `ws.txt`, then `crlf.txt`
- Then `ws.txt` shows changed lines with trailing spaces drawn as `·`; `crlf.txt` shows the line-ending change as changed lines (not an empty reader); staging `crlf.txt` hunks writes the exact line endings
Covered by (backend, interaction): `changed_lines_show_dimmed_tabs_and_trailing_spaces` (diff_view.rs); `stage_hunks_respect_attributes_crlf_and_final_newlines` (crates/git/tests/status.rs)

### CHG-27 Hunk headers follow the reader's context lines
Fixture: `fx-linear` plus two edits 8 lines apart in `a.txt`; Preferences › Diff › **Context lines** 10
Layer: product
- Then the reader shows one joined hunk and **Stage hunk** stages both edits
Covered by (backend, partial): `staging_follows_the_reader_context_and_refuses_hidden_whitespace` (crates/git/tests/status.rs); GUI path not automated

### CHG-28 Stale hunk/line action publishes nothing
Fixture: `fx-dirty`
Layer: product
- Given `a.txt` selected
- When `a.txt` is rewritten externally before **Stage hunk** is clicked
- Then the action fails with an error above the reader, the index is byte-identical, both lists and the reader are kept, then refresh shows the new content
Covered by (backend, interaction): `inline_action_failure_keeps_the_workspace_and_index` (status_view.rs); `stage_hunks_reject_stale_inputs_and_locks_without_publishing` (crates/git/tests/status.rs)

### CHG-29 Locked index is reported
Fixture: `fx-dirty` plus `.git/index.lock` created externally
Layer: product
- When the user stages `c.txt`
- Then an actionable lock error is shown; nothing changes
Covered by (backend, partial): `stage_unborn_lock_and_stale_observations` (crates/git/tests/status.rs); GUI path not automated

### CHG-30 Pending mutation disables duplicates
Fixture: `fx-dirty`
Layer: interaction
- While a stage is in flight, **Stage**/**Unstage**, row buttons, hunk actions, **Stage all**/**Unstage all** and Discard are disabled; browsing rows stays possible
Covered by (interaction): `external_operation_keeps_browsing_but_blocks_file_actions`, `an_external_operation_disables_incompatible_mutations` (status_view.rs)

### CHG-31 Fully staged path keeps an explicit empty comparison
Fixture: `fx-dirty`
Layer: interaction
- When the user stages the last hunk of `a.txt` from Unstaged
- Then the reader keeps `a.txt` with an empty comparison reading *All changes in this file are staged.* beside the toggle; **Staged** is one click away
Covered by (interaction): `staging_the_last_hunk_leaves_the_staged_side_one_click_away`, `staged_away_untracked_entry_keeps_an_empty_comparison` (status_view.rs)

### CHG-32 **Stage all** / **Unstage all**
Fixture: `fx-dirty`
Layer: product
- When the user clicks **Stage all** in the `UNSTAGED` header (or `Ctrl+Shift+S`)
- Then everything incl. `d.txt` deletion and untracked `new.txt`, `notes/*` is staged (`git add -A`); `build/out.o` stays ignored
- When the user clicks **Unstage all** (`Ctrl+Shift+U`)
- Then the index equals HEAD; working files unchanged
Covered by (backend, interaction): `group_headers_offer_stage_all_and_unstage_all` (status_view.rs); `stage_all_includes_modified_untracked_and_deleted_files`, `unstage_all_keeps_working_files_on_born_and_unborn_branches` (crates/git/tests/worktree_changes.rs)

### CHG-33 Unstage all on an unborn branch
Fixture: `fx-unborn` with `README.md` staged
Layer: product
- When the user clicks **Unstage all**
- Then `README.md` returns to `?` Unstaged; the file is kept
Covered by (backend, partial): `unstage_all_keeps_working_files_on_born_and_unborn_branches` (crates/git/tests/worktree_changes.rs); GUI path not automated

### CHG-34 **Discard hunk** requires confirmation; index unchanged
Fixture: `fx-dirty`
Layer: product
- When the user clicks **Discard hunk** on `a.txt`'s unstaged hunk
- Then an inline confirmation reads *Discard this hunk from a.txt? The index is unchanged.* with **Discard** / **Cancel** and *Working-tree changes will be lost. The index is unchanged.*
- When `Esc` or **Cancel**: nothing changes. When **Discard**: the working hunk reverts to the index; the staged hunk and index bytes are unchanged
Covered by (interaction): `drop_requires_confirmation_and_leaves_the_index_alone` (status_view.rs)

### CHG-35 Line discard names and reverts the whole change block
Fixture: `fx-linear` plus, in `a.txt`, a replacement block (line 5 replaced by two lines) and an unrelated edit at line 40
Layer: product
- When the user picks only one of the added lines
- Then the bar offers **Discard 1 change block (3 lines)**, not a one-line discard
- When it is clicked
- Then the confirmation names the block and its working-file range, `Discard 1 change block (3 lines) at lines 5–6 from a.txt? The index is unchanged.` (a deletion-only block reads `deleted lines 9–10`; at most five places are named, then `…`), with *Working-tree changes will be lost. The index is unchanged.* below; confirming reverts the whole block (no half-discard); the line-40 edit and the index remain
Covered by (backend, interaction): `selected_line_drop_reverts_the_whole_change_block`, `line_discards_name_their_blocks_and_line_ranges` (apps/git-good/src/status_view.rs); `drop_selected_working_lines_restores_index_sides_and_keeps_other_hunks` (crates/git/tests/status.rs)

### CHG-36 **Discard unstaged changes** for one file keeps the index
Fixture: `fx-dirty`
Layer: product
- When the user chooses ⋯ › **Discard file…** (last, in red) on `c.txt`, `d.txt`, `run.sh`, `new.txt` and partially staged `a.txt`, confirming each (*Discard unstaged changes from <path>? The index is unchanged.*)
- Then `c.txt`/`d.txt`/`run.sh` are restored from the index (mode 644 again), `new.txt` is removed, `a.txt` keeps its staged hunk and loses only the unstaged one; index bytes unchanged; the lists reload in place
Covered by (backend, interaction, partial): `drop_requires_confirmation_and_leaves_the_index_alone` (apps/git-good/src/status_view.rs); `drop_whole_working_changes_restores_index_and_removes_untracked` (crates/git/tests/status.rs) (the partially staged `a.txt` case not checked)

### CHG-37 Discard refused for conflicts, staged rows, partial mode changes
Fixture: `fx-merge-conflict`; `fx-dirty`
Layer: product
- Then **Discard unstaged changes** is absent/disabled for a conflict row and for a Staged-side comparison; partial discard is not offered for `run.sh` (mode)
Covered by (backend, interaction, partial): `conflicts_and_staged_discards_are_refused_before_spawning` (apps/git-good/src/stage_view.rs); `drop_rejects_partial_mode_changes_and_staged_inputs` (crates/git/tests/status.rs)

### CHG-38 **Discard all tracked changes…** names what it loses
Fixture: `fx-dirty`
Layer: product
- When the user chooses **Changes › Discard all tracked changes…**
- Then a modal **Discard all tracked changes** reads *Restore every tracked file to HEAD? Staged and unstaged changes are both lost.*, lists `WILL BE RESTORED TO HEAD` (empty slot until read), states *This cannot be undone. Untracked files are kept; files only added to the index become untracked.*, and offers **Cancel** (`Esc`) and a filled red **Discard 9 files, staged and unstaged** (`Discard tracked changes`, disabled, until the list arrives)
- When confirmed: every tracked file and the index equal HEAD (the staged `b.txt` change is lost too); `new.txt`, `notes/*` remain untouched; `e2.txt` becomes untracked
- And files only added to the index whose names are `*.txt` or not UTF-8 also become untracked with their bytes kept, also in a repository whose folder name ends with a space
Covered by (backend, interaction): `discard_all_names_staged_changes_and_keeps_untracked_files`, `discard_all_rereads_and_asks_again_after_an_outside_change` (apps/git-good/src/main.rs); `discard_all_restores_tracked_files_and_keeps_untracked`, `discard_all_keeps_a_newly_added_file_whose_name_is_not_utf8`, `discard_all_keeps_newly_added_files_by_their_exact_names` (crates/git/tests/worktree_changes.rs)

### CHG-39 Discard all refused during an operation
Fixture: `fx-rebase-conflict` (also `fx-merge-conflict`)
Layer: product
- When the user chooses **Discard all tracked changes…** and confirms
- Then it is refused with a reason; nothing changes
Covered by (backend, partial): `discard_all_refuses_during_a_rebase` (crates/git/tests/worktree_changes.rs); GUI path not automated

### CHG-40 Commit message box: placeholder, subject counter 72
Fixture: `fx-dirty`
Layer: visual
- Then the empty field shows *Commit message (first line is the subject)*
- When the user types a 54-char subject, the corner shows `54/72` in weak text; at 80 chars `80/72` in the warning color; body lines do not count
Covered by (Not automated)

### CHG-41 Subject guide 50 and Off
Fixture: `fx-dirty`; Preferences › Commit › **Subject length guide** `50 characters`, then `Off`
Layer: visual
- Then a 54-char subject shows `54/50` in the warning color; with **Off** no counter is painted
Covered by (interaction, partial): `preferences_show_diff_and_commit_sections` (main.rs) (preference only; counter Not automated)

### CHG-42 Commit disabled reasons
Fixture: `fx-dirty`; `fx-linear` plus an unstaged edit
Layer: product
- Then with an empty message the Commit button is disabled with *Enter a commit message*; with a message and nothing staged *Stage at least one file*; during a scan *Waiting for the status scan*; during another operation *Wait for the current operation*; whitespace-only message counts as empty
Covered by (backend, interaction): `commit_readiness_explains_disabled_states` (status_view.rs); `rejects_empty_messages_no_staged_changes_and_stale_identities` (crates/git/tests/commit.rs)

### CHG-43 Commit with Ctrl+Enter writes the index only
Fixture: `fx-dirty`
Layer: product
- When the user types `Add b` and presses `Ctrl+Enter` (button tooltip `Ctrl+Enter`; also **Changes › Commit**)
- Then a commit with the staged tree (a.txt staged hunk, b.txt, e→e2) is created; unstaged edits remain; status line reads **Committed <id7> · 4 files** followed by remaining counts; draft cleared; status hints read `S / U: stage / unstage · [ ]: hunks · Ctrl+Enter: commit`
Covered by (interaction, end-to-end): `commit_panel_commits_the_index_and_reports_hook_failures` (status_view.rs); `commit_cli_commits_the_index_and_preserves_working_changes` (apps/git-good/tests/cli.rs)

### CHG-44 After commit, **History tree** selects the new commit
Fixture: `fx-dirty`
Layer: interaction
- When the user commits, then clicks **History tree** (`Ctrl+2`)
- Then the History tree refreshes and the new commit is selected at the top
Covered by (Not automated)

### CHG-45 Commit on an unborn branch creates the root commit
Fixture: `fx-unborn`
Layer: product
- When the user stages `README.md` and commits `Initial`
- Then a root commit exists on `main`; Local Changes shows the clean view
Covered by (backend, partial): `creates_root_and_normal_commits_without_touching_worktree` (crates/git/tests/commit.rs); GUI path not automated

### CHG-46 Commit refused with conflicts or intent-to-add
Fixture: `fx-merge-conflict`; `fx-dirty` plus `git add -N x.txt`
Layer: product
- When the user attempts to commit
- Then it is refused with a reason naming the unmerged / intent-to-add entries; nothing is written
Covered by (backend, partial): `rejects_conflicts_and_intent_to_add` (crates/git/tests/commit.rs); GUI path not automated

### CHG-47 pre-commit hook failure keeps the draft and shows output
Fixture: `fx-hooks`
Layer: product
- When the user commits `Fix lint`
- Then no commit is created; the error reads *The pre-commit hook failed (exit status: 1). Commit aborted; the index and working tree are preserved. Fix the hook and retry.* followed by `lint: 2 problems`; the draft is kept; view stays on Local Changes; index byte-identical
Covered by (interaction, end-to-end): `commit_panel_commits_the_index_and_reports_hook_failures` (status_view.rs); `commit_cli_runs_hooks_and_reports_failures_with_recovery` (apps/git-good/tests/cli.rs)

### CHG-48 commit-msg hook rejects the message
Fixture: `fx-hooks` with pre-commit made to pass
Layer: product
- When the user commits `wip stuff`
- Then the error names the `commit-msg` hook and its output; draft `wip stuff` retained; HEAD unchanged
Covered by (backend, partial): `hooks_run_in_order_and_linked_worktrees_commit_independently` (crates/git/tests/commit.rs); GUI path not automated

### CHG-49 prepare-commit-msg edits the message
Fixture: `fx-hooks` with pre-commit passing
Layer: product
- When the user commits `Add feature`
- Then the commit message contains the hook's trailer; the hook received `COMMIT_EDITMSG` and source `message`
Covered by (backend, partial): `prepare_and_commit_message_hooks_run_and_may_edit_the_message` (crates/git/tests/commit.rs); GUI path not automated

### CHG-50 pre-commit hook that stages content is reflected
Fixture: `fx-dirty` plus a pre-commit hook running `git add c.txt`
Layer: product
- When the user commits
- Then the commit includes `c.txt`; status refreshes to match
Covered by (backend, partial): `pre_commit_hook_can_stage_content_and_failures_preserve_the_index` (crates/git/tests/commit.rs); GUI path not automated

### CHG-51 post-commit failure is a warning
Fixture: `fx-dirty` plus failing `post-commit`
Layer: product
- Then the commit exists; the status line shows **Committed <id>** plus a warning *The commit was created, but its post-commit hook failed…*
Covered by (backend, partial): `post_commit_failure_is_a_warning_after_the_commit_exists` (crates/git/tests/commit.rs); GUI path not automated

### CHG-52 Run commit hooks off
Fixture: `fx-hooks`; Preferences › Commit › **Run commit hooks** unchecked
Layer: product
- When the user commits `wip x`
- Then pre-commit and commit-msg are skipped (commit succeeds), prepare-commit-msg still appends its trailer
Covered by (backend, partial): `without_verify_the_pre_commit_and_commit_msg_hooks_are_skipped` (crates/git/tests/commit.rs); GUI path not automated

### CHG-53 core.hooksPath honored
Fixture: `fx-dirty` plus `core.hooksPath=.githooks-test` with a failing pre-commit
Layer: product
- Then committing fails naming `pre-commit`
Covered by (backend, partial): `custom_hooks_path_is_honoured` (crates/git/tests/commit.rs); GUI path not automated

### CHG-54 No identity configured
Fixture: `fx-no-identity`
Layer: product
- Then Preferences › Commit reads *No user.name or user.email in Git config*
- When the user commits with a message and staged files
- Then it fails with *Author identity is not configured. Set user.name/user.email or GIT_AUTHOR_NAME/EMAIL.*; draft and index kept; HEAD unchanged
Covered by (Not automated)

### CHG-55 Drafts survive navigation and do not cross worktrees
Fixture: `fx-dirty`; `fx-worktrees`
Layer: product
- When the user types a message, selects `c.txt`, goes to History tree and back
- Then the exact text and the `c.txt` selection are restored; opening a linked worktree shows an empty draft
Covered by (interaction): `commit_draft_survives_history_navigation`, `commit_draft_does_not_cross_worktrees` (interaction_tests.rs); `draft_restores_the_selected_comparison` (status_view.rs)

### HIS-82 Several commits compare, copy and cherry-pick together
Fixture: `main` with three commits; `feature` with two commits not on `main`
Layer: interaction
- When the user clicks a commit and Ctrl/Cmd-clicks another
- Then both rows are filled, the footer starts `2 commits selected`, and the Commit menu reads **Compare these two**, **Copy 2 commits** and **Cherry-pick 2 commits onto HEAD**
- And Compare these two opens Review on the newer commit against the older; Copy copies `id subject author date` per commit, newest first
- And with the two `feature` commits picked, Cherry-pick runs the same Git commands as `git good --cli cherry-pick feature~1 feature` (oldest first, one `git cherry-pick`)
- And Shift-click and Shift+Up/Down pick from the anchor; a range over rows not loaded picks nothing and the footer names how many are missing; a reload or filter ends the picks
Covered by (interaction): `two_picked_commits_compare_with_each_other_and_copy_as_lines` (apps/git-good/src/main.rs); `commits_are_picked_by_ctrl_and_shift_and_listed_oldest_first` (apps/git-good/src/state.rs); `cherry_picking_two_picked_commits_runs_the_cli_commands` (apps/git-good/src/parity_tests.rs); `several_commits_are_picked_oldest_first_in_one_cherry_pick` (crates/git/tests/mutation.rs)

### CHG-74 Several files stage, unstage, discard and copy together
Fixture: five modified files `a`…`e`
Layer: interaction
- When the user clicks `a`, Ctrl/Cmd-clicks `b`, then Shift-clicks `d`
- Then `b`, `c` and `d` are selected (the range runs from the anchor `b`), the reader shows `d`, the status line starts `3 files selected`, and Ctrl/Cmd+C copies `b`, `c`, `d` on three lines
- And `S` stages all three as one mutation and ends the selection; clicking `a` and pressing `Shift+Down` selects `a` and `e`, and Discard file… asks *Discard unstaged changes from 2 selected files?* and restores both, leaving the staged files' work alone
- And a Shift range past the rows any page holds (1000 files, rows 10–699) is read whole by the status worker, each row once and in row order
Covered by (interaction): `several_files_are_picked_and_staged_or_discarded_together` (apps/git-good/src/status_view.rs); `large_status_pages_are_complete_bounded_and_revisitable` (apps/git-good/src/status_worker.rs)

### CHG-73 Drafts survive quit and crash
Fixture: `fx-dirty`
Layer: product
- When the user types a message in Local Changes, pauses for a second, and Git Good quits or crashes, then reopens the repository
- Then Local Changes shows the same message; committing it removes the saved draft; a draft saved for another repository is not restored
Covered by (interaction): `a_commit_draft_survives_reopening_the_window` (main.rs); `a_draft_is_written_after_a_pause_and_read_back`, `a_commit_rewrites_at_once_and_an_empty_draft_has_no_file`, `closing_writes_what_is_still_waiting`, `a_file_for_another_repository_is_not_restored` (drafts.rs)

### CHG-56 Commit editor expands and collapses
Fixture: `fx-dirty`
Layer: interaction
- When the user clicks the expand control (*Expand the commit message editor*) at 1440 px
- Then the editor grows; collapsing restores the file lists' slots
- At 720 px there is no Expand: dragging the commit form's top edge up makes the message box taller (up to 60% of the height)
Covered by (interaction): `commit_editor_expands_and_collapses`, `narrow_commit_form_leaves_sign_to_preferences_and_resizes_its_box` (status_view.rs)

### CHG-57 Amend HEAD: banner, base, message, Staged shows HEAD parent → index
Fixture: `fx-dirty`
Layer: product
- When the user checks **Amend HEAD**
- Then the message loads `c30 Update docs`; a banner reads *Amending <c30> · Update docs · Base <c29> · Staged is <c30>'s parent → index; …*; Staged lists c30's files plus staged changes; button **Amend HEAD**, tooltip `Ctrl+Shift+Enter`; **Unstage all** disabled with *Unstage all resets the index to HEAD; leave Amend HEAD first, or unstage files one by one.*
- When the user unstages a file that `c30` changed
- Then it moves to Unstaged, the index holds the parent's version, and the working file is unchanged
Covered by (backend, interaction): `amend_panel_loads_head_names_the_base_and_rewrites_the_commit`, `amend_view_shows_commit_contents_and_restages_through_unstage_and_stage` (status_view.rs); `amend_base_shows_the_commit_and_unstage_restores_the_parent` (crates/git/tests/status.rs)

### CHG-58 Amend succeeds, preserves author, writes a backup
Fixture: `fx-dirty`
Layer: product
- When the user edits the message and presses `Ctrl+Shift+Enter`
- Then HEAD is rewritten with the same author and parent (`c29`), committer Ada Lovelace; status reads **Amended HEAD → <id> · N files**; `git reflog -1` reads `commit (amend): …`; a `refs/gg/backup/<label>/<unix-nanos>` ref points at the old tip (not shown in the sidebar or graph); the toggle clears and the normal draft is retained
Covered by (backend, end-to-end, partial): `amend_rewrites_head_preserves_author_and_saves_a_backup` (crates/git/tests/commit.rs); `commit_amend_and_reflog_cli` (apps/git-good/tests/cli.rs)

### CHG-59 Message-only amend with nothing staged
Fixture: `fx-linear` (clean except an unstaged edit to `a.txt`)
Layer: product
- When the user enables **Amend HEAD**, changes the message, and amends
- Then the button is enabled with no staged changes and the tree equals the old tree
Covered by (interaction, end-to-end, partial): `commit_readiness_explains_disabled_states` (status_view.rs); `commit_amend_and_reflog_cli` (apps/git-good/tests/cli.rs); the GUI amend itself Not automated

### CHG-60 Normal and amend drafts stay separate
Fixture: `fx-dirty`
Layer: product
- When the user types `normal draft`, enables Amend, edits the amend text, disables Amend
- Then `normal draft` is restored; re-enabling shows the edited amend draft
Covered by (interaction): `normal_and_amend_drafts_stay_separate` (status_view.rs)

### CHG-61 Amend on an unborn branch is unavailable
Fixture: `fx-unborn`
Layer: product
- Then **Amend HEAD** is disabled (no HEAD); CLI `git good --cli commit --amend -m x` fails
Covered by (backend, partial): `amend_rejects_unborn_head_and_empty_messages` (crates/git/tests/commit.rs); GUI path not automated

### CHG-62 Sign checkbox follows commit.gpgsign and readiness
Fixture: `fx-signed` with `commit.gpgsign=true`; `fx-dirty` (no key)
Layer: product
- Then **Sign** next to Amend starts checked in `fx-signed`; in `fx-dirty` it is disabled with *Signing key or tool is unavailable.*
- When committing in `fx-signed` with Sign checked, status reads **Committed <id> (signed) · N files**
- At 720 px the commit form has no Sign checkbox; commits sign as Preferences › Commit › Sign commits says
Covered by (interaction): `the_sign_checkbox_starts_from_the_commit_preference`, `narrow_commit_form_leaves_sign_to_preferences_and_resizes_its_box` (status_view.rs)

### CHG-63 Signing failure has no unsigned fallback — see SIG-09

### CHG-64 Clean worktree view
Fixture: `fx-linear`; `fx-remote` (behind 3); `fx-remote` after a pull leaving ahead only
Layer: product
- Then Local Changes replaces lists, reader and form with *Working tree clean*, *Last commit <id8> · Update docs*, one primary step: **Open History tree** (`Ctrl+2`) in `fx-linear`, **Pull 3** when behind, **Push n** when only ahead, and *Changed files appear here as soon as they are saved.*; not shown while amending
Covered by (interaction): `empty_groups_say_so_after_the_scan_and_stack` (status_view.rs)

### CHG-65 Empty group messages and blank during scan
Fixture: `fx-linear` plus one staged edit
Layer: visual
- Then during the scan both groups are blank; after it Unstaged reads *No unstaged changes.*; with one unstaged edit only, Staged reads *No staged changes.*
Covered by (interaction): `empty_groups_say_so_after_the_scan_and_stack` (status_view.rs)

### CHG-66 Automatic refresh after external edits
Fixture: `fx-dirty`
Layer: interaction
- Given Local Changes open, `c.txt` selected
- When an external editor appends a line to `c.txt` and creates `z.txt`, and a terminal runs `git add b.txt`/`git reset b.txt`
- Then within ~300 ms of quiet the lists and the reader update without user action, even with the pointer over the lists; no Refresh button exists; previous content stays until replaced; selection and offsets retained
- And a list row that changes under a still pointer fades in and drops a click made within 0.3 s; a click acts only on the row that was pressed (RULE-17)
Covered by (interaction): `a_real_repository_reports_a_changed_file` (apps/git-good/src/watcher.rs); `refresh_rereads_working_bytes_when_head_and_index_are_unchanged`, `rows_and_refresh_results_show_at_once_under_the_pointer`, `a_row_filling_under_the_pointer_fades_in_and_refuses_an_early_click`, `moving_onto_a_row_or_pressing_one_that_swaps_follows_the_aimed_row` (status_view.rs)

### CHG-67 Failed scan keeps results and offers Retry
Fixture: `fx-dirty`, then make the worktree unreadable (e.g. `chmod 000 notes`)
Layer: product
- Then the status line shows the error and **Retry**; previous rows retained
Covered by (Not automated)

### CHG-68 Narrow layout
Fixture: `fx-dirty` (720×480)
Layer: interaction
- Then `UNSTAGED`/`STAGED` groups sit above the reader, each folding with ▾/▸ in its header (*Collapse this group* / *Show this group*); an empty group starts folded once the scan finishes; the open groups show at most six rows together (fewer in a short window), so the reader keeps the rest (about 430 px at 720×740); the reader header stays one row; the commit form stays at the bottom; the footer carries both counts; status hints shorten; the reader folds Blame/File history into ⋯
Covered by (interaction): `wide_and_narrow_geometry_place_both_groups_above_or_beside_the_reader`, `commit_panel_is_visible_at_both_widths`, `narrow_groups_fold_with_carets_and_empty_ones_start_folded`, `narrow_groups_share_six_rows_and_folded_groups_take_none`, `empty_groups_say_so_after_the_scan_and_stack` (status_view.rs); `status_hints_shorten_at_narrow_widths` (main.rs)

### CHG-69 Reopened-stop commit button labels
Fixture: `fx-rewrite-reopened`
Layer: product
- Then Local Changes opens once with the original message; with everything staged the button reads **Amend reopened commit** (*Commits everything staged.*); after unstaging a file **Commit staged part** (*Unstaged files stay for the next commit.*); after one commit at the stop, with the rest staged, **Commit last part**; commits made are listed under `COMMITS MADE AT THIS STOP`
Covered by (interaction, partial): `commits_made_at_a_rebase_stop_sit_under_the_lists` (status_view.rs); `reopened_commit_opens_local_changes_with_its_message_and_waits_to_continue` (main.rs) (labels Not automated)

### CHG-70 CLI stage/unstage/commit receipts
Fixture: `fx-dirty`; `fx-hooks`
Layer: product
- When running `git good --cli stage --file c.txt`, `git good --cli stage --file a.txt --hunk '<header>'`, `git good --cli unstage --file b.txt`, `git good --cli commit -m "CLI"` (and `--sign`/`--no-sign`, `--amend`)
- Then each prints a JSON receipt (stage adds `"hunks"`; commit has commit, tree, parents, signed, warnings; `--amend` adds `amended`, `previous`, `backup`); errors go to stderr with non-zero exit and no index change
- And in `fx-hooks` `commit` fails with `lint: 2 problems` on stderr and the index kept; `--sign` without a signer fails with HEAD and index unchanged
Covered by (end-to-end): `stage_cli_receipt_and_errors_preserve_working_files`, `stage_cli_selected_hunks_match_git_and_keep_remaining_edits`, `unstage_cli_receipt_and_errors_preserve_working_files`, `commit_cli_commits_the_index_and_preserves_working_changes`, `commit_cli_runs_hooks_and_reports_failures_with_recovery`, `commit_cli_sign_failure_preserves_head_and_index`, `commit_amend_and_reflog_cli` (apps/git-good/tests/cli.rs)

### CHG-71 `git good --cli status` on bare and unborn repositories
Fixture: `fx-bare`; `fx-unborn`
Layer: product
- Then bare returns an identified working-tree requirement error; unborn lists `README.md` as untracked
Covered by (backend, end-to-end): `unborn_clean_bare_and_linked_worktree` (crates/git/tests/status.rs); `status_cli_streams_whole_worktree_and_reports_errors` (apps/git-good/tests/cli.rs)
### CHG-72 Discard confirmations revalidate on confirm
Fixture: `fx-dirty`
Layer: product
- Given **Discard all tracked changes…** is open listing 9 files
- When another process edits tracked `f.txt` and the user confirms
- Then nothing is discarded; the list refreshes (`f.txt` added, primary `Discard 10 files, staged and unstaged`), and the notice reads `The repository changed; review the updated list and confirm again.`; a second confirm discards exactly the refreshed list
- Gap: the per-file **Discard unstaged changes…** confirmation does not revalidate yet (decided: [spec decisions](#spec-decisions--2026-11-04))
Covered by (interaction, partial): `discard_all_rereads_and_asks_again_after_an_outside_change` (apps/git-good/src/main.rs), `guard_confirms_only_what_the_dialog_showed` (apps/git-good/src/consequence.rs) (Discard all only)

Sources: architecture.md §Remote tracking and network mutations · M4, §History tree mutations and recovery · M5, operation area / outcome paragraphs (~79–81), Stashes picker (~91), refs context menu (~180), Pull (~184), Preferences (~358–364); final-design.md §Stashes and command table; cli.md §Sync and stash, §Merge, rebase, rewrite, and conflicts; user-flows.md F06–F09. Rebase/interactive rebase and the Recovery/backups/reflog windows are out of scope here.

## Network

### NET-01 Clone dialog seeds its URL from a repository URL on the clipboard
Fixture: `fx-not-repo` (size 1440×900)
Layer: interaction
- Given the clipboard holds `git@github.com:owner/repo.git`
- When the user chooses **Repository → Clone…**
- Then the modal **Clone repository** opens with **Repository URL** filled with `git@github.com:owner/repo.git` and **Name** filled with `repo`
Covered by (backend, partial): `detects_clone_urls_without_treating_paths_as_urls` (crates/engine/src/sync.rs); GUI path not automated

### NET-02 Clone ignores web pages, issue/PR links and URLs with query or fragment
Fixture: `fx-not-repo`
Layer: interaction
- Given the clipboard holds, in turn, `https://github.com/owner/repo/issues/4`, `https://example.com/page`, and `https://github.com/owner/repo?tab=readme`
- When **Clone…** is opened each time
- Then **Repository URL** stays empty (placeholder `https://host/owner/repo.git or git@host:owner/repo.git`) every time
Covered by (backend, partial): `detects_clone_urls_without_treating_paths_as_urls` (crates/engine/src/sync.rs); GUI path not automated

### NET-03 Clone **Into folder** list order and default checkmark
Fixture: `fx-not-repo` with `XDG_CONFIG_HOME` = `fx-prefs-custom` (two clone folders, second marked preferred)
Layer: interaction
- Given Git Good was launched from `fx-not-repo`
- When **Clone…** is opened and the **INTO FOLDER** choice list is opened
- Then it lists the two saved folders in saved order, then the launch folder, then **Choose another folder…** and **Manage clone folders…**; the preferred folder carries the checkmark and is preselected
Covered by (interaction): `the_preferred_clone_folder_is_marked_and_kept_across_launches` (src/main.rs)

### NET-04 Clone target line composes parent folder and URL slug
Fixture: `fx-not-repo`
Layer: interaction
- Given **Name the clone after the repository** is on (default)
- When the URL `file:///…/fx-remote/origin.git` is typed
- Then **Name** becomes `origin` and the dialog shows `Clones into <launch folder>/origin` once
Covered by (interaction): `clone_destination_uses_the_repository_slug` (src/main.rs)

### NET-05 Manual name edits survive URL changes
Fixture: `fx-not-repo`
Layer: interaction
- Given a URL was typed and **Name** was edited by hand to `my-copy`
- When the URL is changed to another repository
- Then **Name** stays `my-copy` and `Clones into …/my-copy` updates only its parent part if the folder changed
Covered by (Not automated)

### NET-06 Name preference off leaves Name empty
Fixture: `fx-not-repo`
Layer: interaction
- Given Preferences › General › **Name the clone after the repository** is off
- When a repository URL is typed in Clone
- Then **Name** stays empty and **Clone** is disabled with `Enter a repository URL, an absolute parent folder, and a single project name`
Covered by (Not automated)

### NET-07 Clone rejects a name that is not a single path component
Fixture: `fx-not-repo`
Layer: product
- Given a valid URL
- When **Name** is `../escape` (or `a/b`)
- Then **Clone** is disabled and the inline error reads `Enter an absolute parent folder and one project name.`; nothing is created on disk
Covered by (interaction): `clone_rejects_a_project_name_outside_the_root` (src/main.rs)

### NET-08 Choose another folder… sets the clone start folder for the session
Fixture: `fx-not-repo`
Layer: interaction
- Given the Clone dialog is open
- When **Choose another folder…** is chosen and a folder is picked in the native dialog
- Then that folder is the parent in `Clones into …`; reopening Clone in the same session preselects it; after relaunch the preferred folder is preselected again
Covered by (Not automated)

### NET-09 Manage clone folders… opens Preferences on General
Fixture: `fx-not-repo`
Layer: interaction
- When **Manage clone folders…** is chosen in **INTO FOLDER**
- Then Preferences opens on **General › Clone folders**; adding a folder there does not move the Clone dialog
Covered by (interaction, partial): `adding_a_clone_folder_does_not_move_the_dialog` (src/main.rs)

### NET-10 Enter and the Clone button validate identically
Fixture: `fx-not-repo`
Layer: interaction
- Given the URL is empty
- When Enter is pressed in a field, and when **Clone** is clicked
- Then neither starts a clone; with valid fields, Enter in a focused field starts the same clone as the button; Enter with no field focused does nothing
Covered by (interaction): `clone_enter_requires_a_focused_field` (src/main.rs)

### NET-11 Successful clone opens the new repository
Fixture: `fx-remote` (clone source `file://…/origin.git`), launched from `fx-not-repo`
Layer: product
- When Clone runs with valid fields
- Then the dialog closes, the status line shows progress with **Cancel**, and on success the new repository opens in the History tree and the footer reports `Cloned <path>`
Covered by (interaction, end-to-end): `finish_network_clone_opens_the_new_repository` (src/main.rs); `sync_cli_clones_fetches_pushes_and_reports_failures` (tests/cli.rs)

### NET-12 Clone into an existing nonempty folder fails without touching it
Fixture: `fx-remote`, plus a folder `<parent>/origin` containing `keep.txt`
Layer: product
- When Clone targets `<parent>/origin`
- Then Git's error is shown in the status line (with **Details…**), `keep.txt` still exists, no repository is opened, and the previous view is unchanged
Covered by (end-to-end, partial): `sync_cli_clones_fetches_pushes_and_reports_failures` (tests/cli.rs); GUI path not automated

### NET-13 Fetch updates remote-tracking refs only
Fixture: `fx-remote`, with a new commit pushed to `origin.git` `main` from another clone after setup
Layer: product
- Given a History tree row is selected
- When toolbar **Fetch** (tooltip `Fetch  Ctrl+Shift+F`) is clicked
- Then `origin/main` moves to the new commit, local `main` and the worktree are unchanged, the branch button's behind count rises by 1, the selection is kept, and the footer reports `Fetched origin`
Covered by (backend, partial): `clone_fetch_push_upstream_and_non_fast_forward` (crates/git/tests/sync.rs); GUI path not automated

### NET-14 Fetch prune follows the Preference
Fixture: `fx-remote` (remote branch of `gone` deleted on origin; a stale `origin/gone` ref present)
Layer: product
- Given Preferences › General › **Fetch** prune is on
- When **Fetch** runs
- Then `origin/gone` disappears from **REMOTES**; with the option off it remains. CLI: `git good --cli fetch origin --prune` removes it
Covered by (Not automated)

### NET-15 Fetch with no or ambiguous default remote is an error
Fixture: `fx-linear` (no remote); variant with two remotes `a` and `b`, neither `origin`
Layer: product
- When **Fetch** runs (or `git good --cli fetch`)
- Then it fails before Git runs with an error naming the missing/ambiguous remote; nothing changes
Covered by (backend, partial): `fetch_reports_missing_and_ambiguous_default_remotes` (crates/git/tests/sync.rs); GUI path not automated

### NET-16 Pull is enabled whenever a valid upstream exists
Fixture: `fx-remote` (main 2 ahead / 3 behind); `fx-remote-even`; branches `local-only` and `gone`
Layer: interaction
- Then on `main` toolbar **Pull** is enabled with badge `3` (the cached behind count), tooltip `Pull 3 from origin/main (rebase)  Ctrl+Shift+L`, menu **Pull 3 (rebase)**; on `fx-remote-even` it is enabled with no badge, tooltip `Pull from origin/main (rebase)  Ctrl+Shift+L`, menu **Pull (rebase)** (FRM-27); on `local-only` and with HEAD detached it is disabled with `The current branch has no upstream to pull from`; on `gone` (upstream configured, ref deleted) it is enabled, fetches, and then reports `origin/gone was not found on origin. Check the branch's upstream.`
Covered by (interaction, partial): `pull_is_ready_whenever_the_current_branch_has_an_upstream` (apps/git-good/src/main.rs) (readiness and badge count; tooltips and `gone` not checked)

### NET-17 Pull (rebase) default strategy fetches then rebases
Fixture: `fx-remote`
Layer: product
- When **Repository → Pull (rebase)** runs
- Then Git fetches `origin`, then Git Good reads `main` against `origin/main` from local refs and, because it is behind, rebases `main` onto `origin/main` with `--fork-point` (as `git pull --rebase`; NET-39) on the operation worker: 2 local commits now sit above the 3 remote ones; the History tree refreshes once
Covered by (backend, partial): `upstream_relation_reads_local_refs_only` (crates/git/tests/sync.rs), `upstream_relation_follows_ahead_and_behind_counts` (crates/engine/src/sync.rs) (relation only; the GUI fetch-then-rebase is Not automated)

### NET-18 Pull strategy Merge and Fast-forward only
Fixture: `fx-remote`
Layer: product
- Given Preferences › Rebase & recovery › **Pull strategy** = Merge
- When Pull runs
- Then the menu/toolbar read `Pull (merge)` and a merge commit joins `main` and `origin/main`
- And with **Fast-forward only** (`Pull (fast-forward)`) on the diverged `main`, Pull refuses with nothing changed; on a branch only behind it fast-forwards
Covered by (backend, partial): `fast_forward_only_moves_or_refuses_without_merging` (crates/git/tests/merge.rs); GUI path not automated

### NET-39 Pull (rebase) after the upstream was rewritten
Fixture: `fx-remote-rewritten`
Layer: product
- Given `main` holds `my work` on top of `add c`, and the upstream has since replaced `add c` with `add c (rewritten)` and force-pushed (Pro Git 3.6, *The Perils of Rebasing* and *Rebase When You Rebase*)
- When **Pull (rebase)** runs
- Then, as `git pull --rebase` does, Git Good rebases with `--fork-point`: `main` is `add a · add b · add c (rewritten) · my work`, the old `add c` is not replayed, and no conflict stops the rebase
Covered by (backend, interaction): `pull_leaves_out_commits_the_upstream_rewrote` (crates/git/tests/rebase.rs), `pull_after_the_upstream_was_rewritten_replays_only_your_commits` (apps/git-good/src/main.rs)

### NET-40 A fork: fetch upstream, rebase onto it, push to origin
Fixture: `fx-fork`
Layer: product
- Given a clone of the user's fork as `origin`, the project as `upstream`, and topic branch `featureA` with no upstream (Pro Git 5.2, *Forked Public Project*)
- When **Fetch** runs, then Fetch from the `upstream` remote's menu, then **Rebase current onto…** `upstream/main`, then **Push**
- Then Fetch without a remote name takes `origin`; `upstream/main` holds the project's new commit and `origin/main` does not; `featureA` sits on `upstream/main`; Push creates `origin/featureA` and sets it as the upstream; the project gains no branch
Covered by (backend): `a_fork_fetches_upstream_rebases_onto_it_and_pushes_to_origin` (crates/git/tests/sync.rs)

### NET-19 Push of a branch without upstream sets the upstream after success
Fixture: `fx-remote`, HEAD switched to `local-only`
Layer: product
- When toolbar **Push** (`Push  Ctrl+Shift+P`) runs
- Then `origin/local-only` is created, `local-only` now tracks it, the branch button shows tracking, and the footer reads `Pushed local-only to origin · upstream origin/local-only`
Covered by (backend, end-to-end, partial): `clone_fetch_push_upstream_and_non_fast_forward` (crates/git/tests/sync.rs); `sync_cli_clones_fetches_pushes_and_reports_failures` (tests/cli.rs)

### NET-20 A non-fast-forward rejection offers Fetch; Retry push only for retryable failures
Fixture: `fx-remote` (main diverged from origin/main); `fx-remote-auth`; `fx-remote-slow` with a short timeout
Layer: product
- When **Push** runs on diverged `main`
- Then Git rejects it, `origin.git` is unchanged, and the footer reads `Push rejected: origin/main has commits you don't have. Fetch to see them.` with **Details…** (Git's output) and **Fetch**: no **Retry push**, no force option, no automatic fetch, merge or rebase; **Fetch** only fetches and leaves integration to the user
- And when Push fails authentication (`fx-remote-auth`) or times out (`fx-remote-slow`), or the host cannot be reached, the follow-up is **Retry push**; any other failure (e.g. a declined pre-receive hook) offers no follow-up
Covered by (backend, interaction): `push_failures_offer_the_follow_up_that_can_help` (apps/git-good/src/main.rs); `clone_fetch_push_upstream_and_non_fast_forward`, `rejected_push_names_the_upstream_and_suggests_fetch` (crates/git/tests/sync.rs), `classifies_push_failures_by_the_follow_up_that_helps` (crates/git/src/sync.rs)

### NET-21 Push with force-with-lease… is disabled without a remote-tracking upstream
Fixture: `fx-remote`, HEAD on `local-only`
Layer: product
- When **Repository** is opened
- Then **Push with force-with-lease…** is disabled with `Force with lease requires an upstream remote-tracking ref; track and fetch first`
Covered by (backend, partial): `force_with_lease_requires_a_matching_remote_tracking_ref` (crates/git/tests/sync.rs); GUI path not automated

### NET-22 Force-with-lease confirmation names branch, destination, lease and replaced commits
Fixture: `fx-remote` (main 2 ahead, 3 behind)
Layer: product
- When **Push with force-with-lease…** is chosen
- Then the modal **Force push with lease** reads `Rewrite origin/main with local main?`, `Lease: <8-char id> at …`, lists **REPLACED ON THE REMOTE** (3 commits) and **PUSHED IN THEIR PLACE** (2 commits), states `Git rejects this push if the remote branch no longer points at that lease, leaving the remote unchanged. This never uses bare --force.`, and its filled red primary reads `Replace 3 remote commits`; no control anywhere reads bare `Force push`
Covered by (interaction): `force_push_dialog_names_the_lease_and_never_offers_bare_force` (src/main.rs)

### NET-23 Force-with-lease succeeds and reports the lease
Fixture: `fx-remote`
Layer: product
- When the user confirms **Replace 3 remote commits**
- Then `origin/main` equals local `main`, tracking refreshes to 0/0, and the footer reads `Force-pushed main to origin · lease <id>`
Covered by (end-to-end, partial): `sync_cli_force_with_lease_is_explicit_and_rejects_stale_leases` (tests/cli.rs); GUI path not automated

### NET-24 Stale lease fails without changing the remote and without a non-force retry
Fixture: `fx-remote`
Layer: product
- Given the dialog is open and another clone then pushes a new commit to `origin.git` `main`
- When the user confirms (even after a later Fetch)
- Then the push is rejected, `origin.git` keeps the other clone's commit, the footer reads `Force push with lease failed: …`, and **Retry push** is not offered
Covered by (backend, end-to-end, partial): `sync_cli_force_with_lease_is_explicit_and_rejects_stale_leases` (tests/cli.rs); `force_with_lease_requires_a_matching_remote_tracking_ref` (crates/git/tests/sync.rs)

### NET-25 Force-with-lease follow-up after a rewrite — see RWR-42

### NET-26 CLI refuses bare --force
Fixture: `fx-remote`
Layer: product
- When `git good --cli push origin main --force` runs
- Then it exits nonzero with a usage error on stderr, prints no receipt, and `origin.git` is unchanged
Covered by (end-to-end): `sync_cli_force_with_lease_is_explicit_and_rejects_stale_leases` (tests/cli.rs)

### NET-27 Delete on origin uses the last fetched commit as a lease
Fixture: `fx-remote`
Layer: product
- When the user right-clicks `origin/release` in **REMOTES** and chooses **Delete on origin…**
- Then the modal **Delete remote branch** reads `Delete release on origin?` and `Last fetched at <id>. Git refuses if someone pushed to it since. Local branches and other clones keep their copies.`; the red **Delete on origin** removes `release` from `origin.git` and the row disappears
Covered by (backend, partial): `delete_remote_branch_uses_a_lease` (crates/git/tests/sync.rs); GUI path not automated

### NET-28 Delete on origin refuses when the remote moved
Fixture: `fx-remote`
Layer: product
- Given another clone pushed a new commit to `release` after the last fetch
- When **Delete on origin** is confirmed
- Then Git refuses, `release` still exists on `origin.git`, and the error is reported in the status line
Covered by (backend, partial): `delete_remote_branch_uses_a_lease` (crates/git/tests/sync.rs); GUI path not automated

### NET-29 Missing credentials fail non-interactively with guidance
Fixture: proposed `fx-remote-auth`
Layer: product
- When **Fetch** runs (no credential helper, no terminal)
- Then no prompt appears, the app does not hang, and the error names the remote and ends with `Configure a Git credential helper or SSH agent, then retry.`; **Retry fetch** is offered
Covered by (backend, partial): `recognizes_actionable_credential_failures` (crates/git/src/sync.rs); GUI path not automated

### NET-30 URL user information is redacted everywhere
Fixture: proposed `fx-remote-auth` (remote URL `https://ada:s3cret@127.0.0.1:1/repo.git`)
Layer: product
- When Fetch fails and **Details…** is opened; also `git good --cli fetch` stderr and receipt
- Then the URL appears with its host and path but never `s3cret`; no helper output or environment values appear
Covered by (backend, partial): `redacts_passwords_but_keeps_url_identity` (crates/git/src/sync.rs); GUI path not automated

### NET-31 Footer shows bounded progress phases with Cancel
Fixture: proposed `fx-remote-slow`
Layer: visual
- When **Fetch** (or Clone/Push) runs
- Then the footer names the job and target (`Fetching origin`, `Pushing <branch> to origin`, `Cloning <name>`), the Git phase, percentage and counts (`· Receiving objects 42% · 812/1930`) with a progress bar and **Cancel**, at a fixed height; long text is truncated with `…`
Covered by (backend, interaction): `parses_bounded_progress_phases_and_percentages` (crates/git/src/sync.rs); `network_progress_reads_phase_percent_and_counts` (apps/git-good/src/footer.rs); `the_footer_shows_network_progress_then_cancelling` (apps/git-good/src/main.rs)

### NET-32 Cancel reports what happened and re-reads the repository
Fixture: `fx-remote-slow`
Layer: product
- When **Cancel** is clicked (a) while the Fetch is queued, before Git starts; (b) during transfer; (c) after Git has updated refs, as the job finishes
- Then the label changes to `Cancelling fetch…` and the Git child is killed and reaped; in every case (Git Good does not tell them apart) Git Good re-reads refs, the History tree and status (refs that moved are shown as moved; no rollback is attempted or implied) and the footer reads `Fetch from origin cancelled. Git Good re-read the repository; check what changed before retrying.`; **Retry fetch** is offered once the re-read finishes
Covered by (backend, interaction, partial): `cancelled_fetch_rereads_the_repository_before_offering_retry` (apps/git-good/src/main.rs); `cancellation_reports_cancel_without_a_receipt` (crates/git/tests/sync.rs) (the three cancel timings are not driven)

### NET-33 Network operations are serialized
Fixture: proposed `fx-remote-slow`
Layer: interaction
- Given a Fetch is running
- Then **Fetch**, **Push**, **Clone…** and remote-branch checkout are disabled (toolbar, menu and search **Actions**) until it finishes; pressing `Ctrl+Shift+F` again starts nothing
Covered by (Not automated)

### NET-34 No network on launch, refresh, or ref navigation
Fixture: `fx-remote` with `GG_OFFLINE=1`; repeat without it
Layer: product
- When Git Good launches, the window regains focus, files change, and refs are clicked
- Then no `git fetch`/`push` process is started (origin.git's access time / a wrapper `git` on PATH logs no network command); avatars come only from the cache/initials, and Preferences › **Load avatars from the network** is disabled for the launch without changing its saved value
Covered by (Not automated)

### NET-35 Missing Git executable gives installation guidance
Fixture: `fx-remote` with `PATH` lacking `git`
Layer: product
- When **Fetch** runs
- Then it fails with guidance to install Git; nothing changes
Covered by (Not automated)

### NET-36 Create tracking branch from a remote row — see FRM-39

### NET-37 CLI fetch/push/clone receipts and failure shape
Fixture: `fx-remote`
Layer: product
- When `git good --cli fetch origin`, `git good --cli push origin local-only --set-upstream`, `git good --cli clone <url> <path>` run with stderr not a terminal
- Then stdout carries exactly one JSON receipt each (remote and fetched state; branch, remote, upstream, force-with-lease state, lease, warning; cloned path); progress goes only to stderr; a non-fast-forward push and a stale `--force-with-lease` fail nonzero before the remote changes; a failing command exits nonzero with stderr only and no partial receipt
Covered by (end-to-end): `sync_cli_clones_fetches_pushes_and_reports_failures`, `sync_cli_force_with_lease_is_explicit_and_rejects_stale_leases` (tests/cli.rs)

### NET-38 Delete on origin revalidates before Git runs
Fixture: `fx-remote`
Layer: product
- Given **Delete remote branch** for `origin/release` is open naming the last fetched commit
- When `git fetch` in a terminal moves `origin/release` to a newer commit, and the user confirms
- Then Git Good does not push; the dialog re-reads and names the new fetched commit, with the notice `The repository changed; review the updated list and confirm again.`; a second confirm deletes with the new lease (NET-28 covers a remote that moved without a local fetch: Git refuses)
- Gap: **Delete remote branch** does not revalidate yet; confirming pushes with the lease the dialog showed, which Git refuses as in NET-28 (decided: [spec decisions](#spec-decisions--2026-11-04))
Covered by (Not automated)

## Stash

### STA-01 Stash changes is disabled on a clean worktree
Fixture: `fx-linear`
Layer: interaction
- Then **Changes → Stash changes…** and its search Action are disabled with `No local changes to stash` (no toolbar button exists); `git good --cli stash push -m x` exits nonzero with an actionable error, no receipt
Covered by (backend, interaction, partial): `stash_commands_follow_local_changes_and_entries` (src/main.rs); `rejects_clean_trees_and_invalid_references` (crates/git/tests/stash.rs)

### STA-02 Stash dialog counts what it will stash and follows Include untracked
Fixture: `fx-dirty`
Layer: product
- When **Stash changes…** (`Ctrl+Shift+Z`) opens
- Then **Include untracked files** is checked by default, **Keep staged changes in the index** unchecked, a count line `<n> files · <s> staged, <u> unstaged` (untracked included) appears, and the primary reads `Stash <n> files`; unchecking Include untracked lowers the count (drops `new.txt`, `notes/`) and the primary updates
Covered by (interaction, partial): `stash_dialog_and_window_render_at_supported_widths` (src/main.rs)

### STA-03 Stash with untracked files removes them from the worktree
Fixture: `fx-dirty`
Layer: product
- Given message `wip one`, Include untracked checked
- When **Stash <n> files** is pressed
- Then the worktree has no tracked changes, `new.txt` and `notes/` are gone, ignored `build/out.o` stays, the Changes badge disappears, the refs sidebar lists `STASHES 1` and **Changes → Stashes…** is enabled, and the footer reports `Stashed …`
Covered by (backend, partial): `push_supports_message_untracked_and_keep_index` (crates/git/tests/stash.rs); GUI path not automated

### STA-04 Keep staged changes in the index
Fixture: `fx-dirty`
Layer: product
- Given **Keep staged changes in the index** checked, Include untracked unchecked
- When stashing
- Then staged content (`b.txt`, `e2.txt`, the staged hunk of `a.txt`) remains staged, unstaged edits are gone, untracked `new.txt` remains
Covered by (backend, partial): `push_supports_message_untracked_and_keep_index` (crates/git/tests/stash.rs); GUI path not automated

### STA-05 Stash options are remembered from the last submission
Fixture: `fx-dirty`
Layer: interaction
- Given the user unchecks Include untracked and submits
- When the dialog is opened again (also after relaunch)
- Then Include untracked starts unchecked; cancelling the dialog without submitting does not change the remembered value; Preferences › General › **Dialog defaults** shows the same values
Covered by (Not automated)

### STA-06 CLI stash push keeps Git's default for untracked files
Fixture: `fx-dirty`
Layer: product
- When `git good --cli stash push -m wip` runs
- Then `new.txt` stays in the worktree; with `--include-untracked` it is stashed; the receipt names `stash@{0}` and created state
Covered by (end-to-end): `stash_cli_lists_pushes_applies_pops_and_drops` (tests/cli.rs)

### STA-07 Stash entry points show the count; disabled with none
Fixture: `fx-stash`; `fx-linear`
Layer: interaction
- Then on `fx-stash` **Changes → Stashes…** is enabled and the refs sidebar shows `STASHES 3`; there is no toolbar Stashes button; on `fx-linear` **Changes → Stashes…** and its search Action are disabled with `No stashes`
Covered by (interaction): `stash_commands_follow_local_changes_and_entries`, `commands_off_the_toolbar_keep_their_menu_home_and_search_entry`, `window_toolbar_keeps_the_owners_buttons_in_place_across_views_and_widths` (apps/git-good/src/main.rs)

### STA-08 Picker rows list ref, message, branch, age and file count
Fixture: `fx-stash`
Layer: visual
- When **Changes → Stashes…** is chosen
- Then a 440 px picker lists `stash@{0}`…`stash@{2}` newest first in two-line 40 px rows: message without the `WIP on <branch>:` prefix over `on main · <age> · <n files>` (`stash@{2}` reads `on feature/login`; `stash@{1}` counts its untracked file); the header reads `Up/Down choose · Enter apply · Del drop`
Covered by (backend, interaction): `stash_picker_lists_entries_with_restore_show_and_drop` (src/main.rs); `entries_name_their_branch_age_and_file_count` (crates/git/tests/stash.rs)

### STA-09 Stash made on a detached HEAD shows **detached**
Fixture: `fx-branchy-detached` with an edit stashed
Layer: visual
- Then its picker row reads `on detached · …`; `git good --cli stash list` has `"branch": null`
Covered by (backend, partial): `subject_names_the_branch` (crates/engine/src/stash.rs); GUI path not automated

### STA-10 Picker keeps a fixed size and scrolls past six rows
Fixture: `fx-stash` plus 5 more stashes (8 total)
Layer: visual
- Then the list scrolls inside six rows; selecting entries, asking to drop, and showing an error never resize the picker
Covered by (Not automated)

### STA-11 Apply restores and keeps the entry
Fixture: `fx-stash`
Layer: product
- When `stash@{0}` is selected and **Apply** (tooltip `Restore the changes and keep the stash entry`) is clicked, or Enter is pressed
- Then `a.txt` is modified in the worktree, the picker still lists 3 entries, the footer reads `Restored stash@{0}`, and the status and History tree refresh
Covered by (interaction): `stash_picker_keys_select_ask_to_drop_and_apply` (src/main.rs); `stash_restore_reports_the_restore_action` (src/main.rs)

### STA-12 Pop restores then removes the entry
Fixture: `fx-stash`
Layer: product
- When **Pop** is clicked on `stash@{0}`
- Then `a.txt` is modified, the count becomes 2, and the former `stash@{1}` is now `stash@{0}`
Covered by (backend, partial): `push_list_apply_pop_and_drop_match_git` (crates/git/tests/stash.rs); GUI path not automated

### STA-13 Restore staged state also restores the index
Fixture: `fx-linear` with a stash made from one staged and one unstaged edit
Layer: product
- Given **Restore staged state** checked
- When **Apply** runs
- Then the staged edit is staged again and the other unstaged; unchecked, both come back unstaged; the checkbox value is remembered
Covered by (backend, partial): `push_list_apply_pop_and_drop_match_git` (crates/git/tests/stash.rs); GUI path not automated

### STA-14 Show changes opens the stash commit in Review and returns
Fixture: `fx-stash`
Layer: interaction
- When **Show changes** is clicked on `stash@{0}`
- Then Review shows the stash commit with `a.txt`; Back returns to the view the picker was opened from; no refs or files change
Covered by (interaction): `stash_picker_lists_entries_with_restore_show_and_drop` (src/main.rs)

### STA-15 Drop… asks in place and drops only the entry
Fixture: `fx-stash-dirty`
Layer: product
- When **Drop…** (right-aligned, error color) or Del is pressed on `stash@{1}`
- Then an inline confirmation reads `Drop stash@{1}? This removes the stash entry, not working files.` with a filled red `Drop stash@{1}`; confirming (button or Enter) leaves 2 entries, the unstaged edit untouched, and the footer reads `Dropped stash@{1}`
Covered by (interaction): `stash_picker_keys_select_ask_to_drop_and_apply` (src/main.rs)

### STA-16 Esc cancels a pending drop
Fixture: `fx-stash`
Layer: interaction
- Given the drop confirmation is shown
- When Esc is pressed
- Then the confirmation disappears, 3 entries remain, and the picker stays open; a second Esc closes it
Covered by (interaction): `stash_picker_keys_select_ask_to_drop_and_apply` (src/main.rs)

### STA-17 Conflicting apply keeps the entry and reports paths
Fixture: `fx-stash-dirty` with the unstaged `a.txt` edit committed first (so Git applies and conflicts rather than refusing)
Layer: product
- When **Apply** runs on `stash@{0}`
- Then `a.txt` contains conflict markers and is listed as conflicted, the footer reads `Restored stash@{0} · 1 conflict` with **Stashes…**, the entry is kept, and no operation band with Continue/Skip/Abort appears
Covered by (backend, partial): `apply_conflicts_keep_the_stash_and_report_paths` (crates/git/tests/stash.rs); GUI path not automated

### STA-18 Conflicting pop does not drop the entry
Fixture: same as STA-17
Layer: product
- When **Pop** runs
- Then the result matches STA-17 and the picker still lists `stash@{0}`
Covered by (backend, partial): `pop_conflicts_do_not_drop_the_stash` (crates/git/tests/stash.rs); GUI path not automated

### STA-19 Apply blocked by local changes reports Git's error
Fixture: `fx-stash-dirty` (unstaged edit to `a.txt` conflicting with `stash@{0}`)
Layer: product
- When **Apply** runs on `stash@{0}`
- Then Git refuses, the error is shown in the picker's reserved error line and the status line, the edit and the entry are unchanged
Covered by (Not automated)

### STA-20 Sidebar STASHES group opens the picker and follows the filter
Fixture: `fx-stash`
Layer: interaction
- When the refs filter is `login` and the `stash@{2}` row in **STASHES** is clicked
- Then only matching stash rows are listed, and the picker opens with `stash@{2}` selected
Covered by (interaction): `stash_entries_open_the_picker_and_follow_the_filter` (src/refs_view.rs)

### STA-21 Changes → Stashes…, the sidebar and the footer open the same picker
Fixture: `fx-stash`
Layer: interaction
- When **Changes → Stashes…** is chosen, a `stash@{2}` row in the sidebar **STASHES** group is clicked, and a stash outcome's **Stashes…** recovery button is clicked
- Then the same picker opens each time (on `stash@{2}` from the sidebar), centered horizontally just below the toolbar and inside the window at 720×480; no separate Stashes window or Restore dialog exists
Covered by (interaction, partial): `changes_stashes_opens_the_picker_inside_the_narrow_window` (apps/git-good/src/main.rs), `stash_entries_open_the_picker_and_follow_the_filter` (apps/git-good/src/refs_view.rs) (menu and sidebar routes; the footer route not checked)

### STA-22 CLI rejects option-shaped or non-numeric references
Fixture: `fx-stash`
Layer: product
- When `git good --cli stash drop --all`, `git good --cli stash apply foo`, `git good --cli stash pop stash@{x}` run
- Then each exits nonzero before Git runs; `git good --cli stash apply 1` and `stash@{1}` are accepted
Covered by (backend): `rejects_clean_trees_and_invalid_references` (crates/git/tests/stash.rs)

### STA-23 CLI stash list is read-only and explicit when empty
Fixture: `fx-stash`; `fx-linear`
Layer: product
- When `git good --cli stash list` runs
- Then entries carry `index`, `reference`, `commit`, `message`, `branch`, `age`, and `files: null`, newest first; on `fx-linear` `entries` is `[]` with exit 0; refs, index and files are unchanged
Covered by (backend, end-to-end): `lists_newest_first_and_preserves_unrelated_files` (crates/git/tests/stash.rs); `stash_cli_lists_pushes_applies_pops_and_drops` (tests/cli.rs)

### STA-24 Stash and switch from a blocked switch
Fixture: `fx-dirty` on `fx-branchy` layout (dirty `main`, target `feature/login`)
Layer: product
- When the switch is blocked and **Stash and switch…** is submitted
- Then the changes are stashed, HEAD is on `feature/login`, the entry is kept and reported; nothing is stashed unless the dialog is submitted
Covered by (backend, interaction): `blocked_switch_keeps_intent_and_retries_explicitly` (src/main.rs); `stashes_tracked_changes_before_switching_and_keeps_the_entry` (crates/git/tests/branch.rs)

### STA-25 Drop revalidates on confirm
Fixture: `fx-stash`
Layer: product
- Given the drop confirmation for `stash@{1}` is shown
- When `git stash drop stash@{0}` runs in a terminal (the entry is now `stash@{0}`) and the user confirms
- Then nothing is dropped; the picker re-reads, keeps the same entry selected (matched by commit) under its new name `stash@{0}`, and shows `The repository changed; review the updated list and confirm again.`; a second confirm drops that entry, never whichever entry now holds the old index
- Gap: the drop confirmation does not revalidate yet (decided: [spec decisions](#spec-decisions--2026-11-04))
Covered by (Not automated)

### STA-26 A stash shows its result at once, at CLI speed
Fixture: `fx-dirty`
Layer: interaction
- Given the pointer resting over the History tree, where the Stash dialog sat
- When the user stashes
- Then the stash runs one Git process, as `git stash push` does, and the History tree, refs and sidebar show the result as soon as Git returns, without waiting for the pointer to leave; the top row and scroll position hold (RULE-16)
Covered by (interaction): `a_stash_shows_its_history_at_once_under_the_pointer` (main.rs), `a_reload_under_the_pointer_shows_once_its_old_top_row_is_found`, `a_branch_switch_shows_its_first_page_at_once_under_the_pointer` (interaction_tests.rs), `push_receipt_names_the_newest_entry_as_git_lists_it` (stash.rs)

## Merge, cherry-pick, revert and conflicts

### INT-01 Merge into current creates a merge commit for diverged branches
Fixture: `fx-branchy`
Layer: product
- When **Branch → Merge into current…** opens the modal **Merge branch**, `feature/login` is chosen, and **Merge** is pressed
- Then the dialog closes immediately, `main` gains a two-parent merge commit, the History tree refreshes once, and the footer reads `Merged feature/login as a merge commit`
Covered by (backend, interaction, partial): `diverged_merge_creates_a_merge_commit` (crates/git/tests/merge.rs); `merge_dialog_and_banner_render_at_supported_widths` (src/main.rs)

### INT-02 Merge fast-forwards by default and reports already up to date
Fixture: `fx-branchy`, plus `git branch ahead && git switch ahead && commit c31 && git switch main`
Layer: product
- When `ahead` is merged with **Always create a merge commit** unchecked
- Then `main` moves to c31 with no merge commit; merging `old` reports `Merge old · already up to date` and nothing changes
Covered by (backend, partial): `fast_forward_and_up_to_date` (crates/git/tests/merge.rs); GUI path not automated

### INT-03 Always create a merge commit (no-ff) and its remembered default
Fixture: same as INT-02
Layer: product
- Given **Always create a merge commit** checked
- When `ahead` is merged
- Then a merge commit is created; the dialog opens checked next time and Preferences › General › Dialog defaults shows it. CLI `git good --cli merge ahead --no-ff` does the same
Covered by (backend, end-to-end, partial): `no_ff_forces_a_merge_commit` (crates/git/tests/merge.rs); `merge_cli_no_ff_creates_a_merge_commit` (tests/cli.rs)

### INT-04 Merge requires a clean tracked worktree
Fixture: `fx-dirty` with branch `feature/login` from `fx-branchy`
Layer: product
- When a merge is submitted
- Then it is refused with a clean-worktree reason before Git changes anything; untracked files alone do not block
Covered by (Not automated)

### INT-05 Merge accepts remote-tracking refs and tags; rejects option-shaped refs
Fixture: `fx-remote`
Layer: product
- When `origin/main` is merged from its REMOTES row (**Merge**), and `git good --cli merge -- --abort-me` / `git good --cli merge nosuch` run
- Then the remote-tracking merge works; the invalid references fail before Git runs with nonzero status
Covered by (backend, partial): `rejects_option_shaped_and_missing_references` (crates/git/tests/merge.rs); GUI path not automated

### INT-06 Merge conflict shows the operation band
Fixture: `fx-merge-conflict`
Layer: product
- When Git Good opens the repository
- Then a fixed two-line amber band under the toolbar reads `Merge conflict · merging <ref> <8-char id> into main` with the status `3 conflicts left in 3 files` right after the title, then **Resolve conflicts** (filled primary), **Continue** (`Ctrl+Enter`, disabled, tooltip `Resolve and stage every conflicted file first`), and red **Abort and restore** (`Abort the merge and restore main to its previous tip`); there is no **Skip commit**; line 2 reads `Ours is main, Theirs is <ref>. Resolve and stage each file, then continue.`; the footer does not repeat these controls
Covered by (interaction): `conflict_bands_name_the_operation_and_what_is_left`, `continue_is_the_primary_only_once_nothing_is_conflicted` (apps/git-good/src/operation_band.rs); `merge_operation_area_has_no_skip_at_either_width`, `conflict_window_leaves_continuation_to_the_operation_area` (apps/git-good/src/main.rs)

### INT-07 Merge abort restores the pre-merge state
Fixture: `fx-merge-conflict`
Layer: product
- When **Abort** is clicked
- Then HEAD, index and worktree match `main` before the merge, the band disappears, and the footer reads `Aborted merge`
Covered by (backend, end-to-end, partial): `conflict_can_abort_and_continue` (crates/git/tests/merge.rs); `merge_cli_fast_forwards_conflicts_and_recovers` (tests/cli.rs)

### INT-08 Merge continue after all paths are resolved
Fixture: `fx-merge-conflict`
Layer: product
- Given all three paths are resolved and staged
- Then the band is purple: `Merge ready · merging <ref> <id> into main`, `All files staged`, `Continue writes the merge commit of <ref> into main.`, **Continue** the filled primary with `Ctrl+Enter`
- When **Continue** (or `Ctrl+Enter`) is pressed
- Then a merge commit is created, the band disappears, the History tree/refs/status refresh once, and the footer reads `Continued merge`
Covered by (backend, interaction, partial): `conflict_can_abort_and_continue` (crates/git/tests/merge.rs); `continue_is_the_primary_only_once_nothing_is_conflicted` (apps/git-good/src/operation_band.rs); GUI continue not driven

### INT-09 Cherry-pick onto HEAD applies the selected commit
Fixture: `fx-branchy`
Layer: product
- When the second-newest `feature/login` commit is selected in the History tree and **Commit → Cherry-pick onto HEAD** runs
- Then `main` gains one new commit with the same message and change, and the footer reads `Cherry-picked <id>`
Covered by (backend, partial): `cherry_pick_applies_a_commit` (crates/git/tests/mutation.rs); GUI path not automated

### INT-10 Cherry-pick of a merge commit is refused
Fixture: `fx-branchy`
Layer: product
- When `Merge branch 'topic'` is selected and **Cherry-pick onto HEAD** runs
- Then it is refused with `Use the Merge action for merge operations.` before Git runs, also when the merge is one of several picked commits; nothing changes
Covered by (backend): `cherry_pick_of_a_merge_is_refused` (crates/git/tests/mutation.rs)

### INT-11 Cherry-pick conflict: band, Skip commit, Abort, Continue
Fixture: `fx-cherry-pick-conflict`
Layer: product
- Then the amber band reads `Cherry-pick conflict · applying <8-char id> onto main` · `1 conflict left in 1 file` with **Resolve conflicts**, **Continue** (disabled), **Skip commit** and **Abort and restore**, and line 2 `Theirs is the picked commit. Skip commit leaves it out and ends the cherry-pick.`; **Abort and restore** restores `main`; after resolving, **Continue** creates the commit; **Skip commit** drops it and ends the operation
Covered by (backend, end-to-end, interaction, partial): `cherry_pick_conflict_can_abort_and_continue` (crates/git/tests/mutation.rs); `cherry_pick_cli_applies_conflicts_and_recovers` (tests/cli.rs); `mutation_banner_renders_at_supported_widths` (apps/git-good/src/main.rs)

### INT-12 Revert creates the inverse commit
Fixture: `fx-linear`
Layer: product
- When c27 is selected and **Commit → Revert** runs
- Then a new commit on `main` undoes c27's change and the footer reads `Reverted <id>`
Covered by (backend, end-to-end, partial): `revert_inverts_a_commit_and_can_skip_a_conflict` (crates/git/tests/mutation.rs); `revert_cli_inverts_a_commit` (tests/cli.rs)

### INT-43 Revert of a merge commit undoes what it brought in
Fixture: `fx-merge-revert`
Layer: product
- Given `Merge branch 'topic'` selected (Pro Git 7.8, *Undoing Merges*, *Reverse the commit*)
- When **Commit → Revert** runs
- Then, as `git revert -m 1`, a new commit `Revert "Merge branch 'topic'"` removes `topic`'s files `t` and `t2` and keeps `main`'s own `c`; the footer reads `Reverted <id> · Merging this branch again will not bring these changes back; revert this revert first.`
Covered by (backend, interaction): `reverting_a_merge_undoes_what_it_brought_in` (crates/git/tests/mutation.rs), `a_reverted_merge_says_how_to_bring_it_back` (apps/git-good/src/main.rs)

### INT-13 Revert conflict recovery
Fixture: `fx-revert-conflict`
Layer: product
- Then the amber band reads `Revert conflict · reverting <8-char id> on main` with **Skip commit**; once nothing is conflicted it reads `Revert ready · …` with `Continue commits “Revert …” with Git’s message.`; **Skip commit** ends with no new commit; **Abort and restore** restores HEAD; side labels follow INT-19
Covered by (backend, interaction, partial): `revert_inverts_a_commit_and_can_skip_a_conflict` (crates/git/tests/mutation.rs); `conflict_bands_name_the_operation_and_what_is_left` (apps/git-good/src/operation_band.rs); GUI path not driven

### INT-14 Cherry-pick/revert need a selected commit and a clean worktree
Fixture: `fx-dirty`
Layer: product
- When **Cherry-pick onto HEAD** or **Revert** is invoked with no commit selected, then with a selection on a dirty tracked worktree
- Then the first reports `Select a commit before cherry-picking or reverting.`; the second is refused with a clean-worktree reason; nothing changes
Covered by (Not automated)

### INT-15 Operation detected on launch from external Git
Fixture: `fx-cherry-pick-conflict` (state created by command-line Git)
Layer: interaction
- When Git Good opens it
- Then the same operation band is present immediately; after `git cherry-pick --abort` in a terminal it disappears within ~300 ms without user action
Covered by (Not automated)

### INT-16 The conflict workspace is the Changes view; Esc returns to the plain list
Fixture: `fx-merge-conflict`, Local Changes open
Layer: interaction
- When the band's **Resolve conflicts** (or **Changes → Resolve conflicts…**, or the Local Changes tab while conflicts exist) is clicked
- Then the conflict workspace replaces Local Changes under the band with the Local Changes tab selected, the band hides **Resolve conflicts**, there is no Close and no heading, and the refs sidebar hides (its toggle still works)
- When Esc is pressed
- Then the plain changes list shows; the band's **Resolve conflicts** brings the workspace back; sizes do not change when switching files
Covered by (interaction): `conflicts_open_in_the_main_workspace_and_hold_still`, `conflict_window_leaves_continuation_to_the_operation_area`, `conflict_view_renders_at_supported_widths` (apps/git-good/src/main.rs); `resolve_conflicts_hides_while_the_workspace_is_open` (apps/git-good/src/operation_band.rs)

### INT-17 Resolve conflicts… is disabled without conflicts
Fixture: `fx-linear`
Layer: interaction
- Then **Changes → Resolve conflicts…** is disabled with `No conflicts to resolve`
Covered by (Not automated)

### INT-18 Files panel lists CONFLICTED, RESOLVED, STAGED and ALSO CHANGED
Fixture: `fx-merge-conflict`
Layer: interaction
- Then the 300 px file list shows 22 px rows: `CONFLICTED` with its count at the right and `!` rows in amber (`src/app.rs`, `docs.md`, `logo.png`), each with what is left (`2 left`, `ready`); `RESOLVED, STAGED 0`; `ALSO CHANGED BY THIS MERGE` with `~` rows for staged paths that are not conflicted; at the bottom the legend `In a merge, Ours is main, the branch you are on, and Theirs is <ref>, the branch coming in.`
- And after staging one path it moves under `RESOLVED, STAGED 1` marked ✓ with **−**, and the next unresolved path is selected; with none left the list reads `All conflicts resolved. Continue with Ctrl+Enter.`
Covered by (interaction): `conflict_selection_prefers_the_next_path`, `hand_edits_in_the_result_are_what_stage_resolution_stages_and_minus_reopens` (apps/git-good/src/main.rs); `markers_and_legend_name_the_sides` (apps/git-good/src/conflict_workspace.rs); `a_staged_resolution_reopens_as_a_conflict` (crates/git/tests/conflict.rs)

### INT-19 Ours and Theirs labels name their source (merge, cherry-pick, revert)
Fixture: `fx-merge-conflict`; `fx-cherry-pick-conflict`; `fx-revert-conflict`
Layer: product
- Then block columns and the Whole file columns name each side: merge `Ours · main (current branch)` / `Theirs · origin/main (being merged)` (the ref at `MERGE_HEAD`, else its short ID); cherry-pick `Theirs · <8-char id> (picked commit)`; revert `Theirs · undo of <8-char id>`; a detached HEAD reads `Ours · HEAD <8-char id> (current branch)`
- And the stage number and full identity are in the label's tooltip (`Stage 3 · 9a4f9a95 Add login`), not the text; the block choice tooltips name both sides (`Theirs · origin/main (being merged), then Ours · main (current branch)`); Ours is blue and Theirs amber, but the text alone identifies each side
Covered by (backend, interaction, partial): `side_labels_name_the_real_refs_with_the_stage_in_the_tooltip` (apps/git-good/src/conflict_workspace.rs), `conflict_workspace_shows_side_names` (apps/git-good/src/main.rs), `choice_tooltips_name_both_sides` (apps/git-good/src/conflict_blocks.rs); `side_names_follow_the_operation` (crates/git/tests/conflict.rs)

### INT-20 Text file opens Block by block with Base/Ours/Theirs per block
Fixture: `fx-merge-conflict`, `src/app.rs` selected
Layer: visual
- Then the file header reads the path, `1 conflict left`, **Block by block** (selected) | **Whole file**, **Open in editor**, **Refresh external edits**; each block is a bordered frame (amber until chosen, green after) whose header reads `Conflict 1/1` and `line 10` with **Ours**, **Theirs**, **Ours + theirs**, **Theirs + ours** at the right, then Base, Ours and Theirs in three equal columns
- And a long line (`eframe = { version = "0.37", default-features = false, features = [...] }`) wraps inside its own column at 1120 and 720 px; no column paints over its neighbour
Covered by (backend, interaction): `blocks_know_their_start_lines_and_how_many_are_left`, `long_lines_wrap_inside_their_column` (apps/git-good/src/conflict_blocks.rs); `blocks_split_the_conflict_and_compose_a_staged_resolution` (crates/git/tests/conflict.rs)

### INT-21 Keys 1–4 choose for the current block and advance
Fixture: proposed `fx-merge-conflict-multi` (one file, three blocks)
Layer: interaction
- When `1` is pressed on block 1
- Then block 1's **Ours** chip is filled and block 2 becomes current (outlined); pressing `3` on block 2 composes ours then theirs; keys do nothing while a text field has focus
Covered by (interaction): `number_keys_choose_for_the_current_block` (src/conflict_blocks.rs); `choosing_moves_to_the_next_open_block_and_completes` (src/conflict_blocks.rs)

### INT-22 [ and ] move between blocks and wrap; click makes a block current
Fixture: proposed `fx-merge-conflict-multi`
Layer: interaction
- When `]` is pressed on the last block, `[` on the first, and a block is clicked
- Then current wraps to the first, then to the last, and the clicked block becomes current
Covered by (interaction): `brackets_move_between_blocks_and_wrap` (src/conflict_blocks.rs)

### INT-23 Result keeps named markers for open blocks
Fixture: proposed `fx-merge-conflict-multi`
Layer: product
- Given only block 1 has a choice
- Then the line-numbered **Result** shows block 1 resolved and blocks 2–3 with `<<<<<<< main` / `=======` / `>>>>>>> origin/main` in the warning tint with orange markers; **Stage resolution** is disabled
Covered by (backend, interaction): `unresolved_blocks_carry_named_markers` (apps/git-good/src/conflict_result.rs); `markers_of_unresolved_blocks_take_the_warning_color` (apps/git-good/src/conflict_workspace.rs)

### INT-24 Stage resolution publishes the Result and stages the path
Fixture: `fx-merge-conflict`, `src/app.rs`
Layer: product
- Given the Result has no markers left
- When **Stage resolution** (`Ctrl+S`, also while typing in the Result) is pressed
- Then the working file equals the Result, hand edits included, the path is staged (not unmerged), the footer reads `Resolved src/app.rs`, and the next unresolved path is selected
Covered by (backend, interaction): `blocks_split_the_conflict_and_compose_a_staged_resolution` (crates/git/tests/conflict.rs); `hand_edits_in_the_result_are_what_stage_resolution_stages_and_minus_reopens`, `conflict_staging_uses_the_operation_worker` (apps/git-good/src/main.rs)

### INT-25 Blocks come from the index stages; a hand-edited working file is detected
Fixture: `fx-merge-conflict` with `git config merge.conflictStyle zdiff3`; variant: working `src/app.rs` also edited by hand outside the conflict markers
Layer: product
- Then the blocks and Base column are the same with or without the config (computed from stages)
- And in the variant Block by block shows `This file was edited outside the conflict blocks.` with **Use working file** (stages the working file as it is); the blocks' **Stage resolution** becomes **Replace with blocks…** (`Ctrl+S` too), which opens the modal **Replace edited file** (`Staging the blocks overwrites src/app.rs. The edits made outside the conflict blocks are lost.`, **Cancel** `Esc`, red **Replace and stage** `Enter`); nothing is written without one of those choices
- And conflict markers labelled differently, CRLF endings, or the merge, diff3 or zdiff3 style do not count as edits
Covered by (backend, interaction): `an_edited_working_file_offers_use_working_file_and_confirms_replace`, `use_working_file_stages_a_hand_edited_file_as_is` (apps/git-good/src/main.rs); `a_hand_edited_working_file_is_told_apart_from_what_git_wrote` (crates/git/tests/conflict.rs), `working_files_count_as_edited_only_when_a_side_changed` (crates/engine/src/conflict.rs)

### INT-26 Whole file sources only change the preview
Fixture: `fx-merge-conflict`, `src/app.rs`
Layer: product
- When **Whole file** is chosen and **Take ours**, **Take theirs**, **Keep working file** are picked in turn
- Then only the read-only **Result** changes (Keep working file is the default; each column shows the side's size, `12 lines · text`), and the working file and index are unchanged until **Stage resolution**
Covered by (backend, interaction): `conflict_preview_is_read_only_until_stage_resolution` (apps/git-good/src/main.rs); `previewed_sides_publish_only_after_validation` (crates/git/tests/conflict.rs)

### INT-27 External edit makes the preview stale
Fixture: `fx-merge-conflict`, `src/app.rs` previewed in Whole file
Layer: product
- Given the file is changed on disk after the preview was read
- When **Stage resolution** is pressed
- Then nothing is written; the workspace and footer show `The conflict changed. Refresh external edits and retry.`; after **Refresh external edits** the new bytes preview and staging succeeds
Covered by (backend, partial): `publish_rejects_changed_stages_or_working_bytes` (crates/git/tests/conflict.rs); `composed_resolution_refuses_a_changed_working_file` (crates/git/tests/conflict.rs)

### INT-28 Binary conflict has no blocks; whole-file sides apply
Fixture: `fx-merge-conflict`, `logo.png`
Layer: product
- Then **Block by block** is disabled with its reason, **Whole file** is active with Ours/Theirs/Base sources; choosing **Theirs** and **Stage resolution** stages theirs' bytes exactly
Covered by (backend, partial): `blocks_name_why_a_file_needs_a_whole_side` (crates/git/tests/conflict.rs); GUI path not automated

### INT-29 Delete/modify conflict offers Keep deletion
Fixture: `fx-merge-conflict`, `docs.md` (deleted on one side, modified on the other)
Layer: product
- Then Block by block is disabled with `One side deleted this file. Choose Keep deletion or a whole side.`; the deleted side's pane reads `(deleted on this side)`; the **Whole file** sources add **Keep deletion** (tooltip *Delete the file and stage its removal*)
- When **Keep deletion** is chosen
- Then the preview reads `docs.md will be deleted; the other side's changes to it are discarded.`; **Stage resolution** removes the file, stages the removal, and moves the path to `RESOLVED, STAGED`
- And choosing the deleted side as the source disables staging with `This side deleted the file. Choose Keep deletion to stage the removal.`; choosing the modified side and staging keeps the file; a modify/modify conflict has no **Keep deletion**
Covered by (backend, interaction, partial): `delete_modify_conflict_offers_keep_deletion` (apps/git-good/src/main.rs); `keeping_a_deletion_stages_the_removal`, `keeping_a_deletion_is_refused_without_a_deleted_side_or_after_an_edit`, `use_side_points_the_deleted_side_to_keep_deletion`, `non_regular_working_paths_keep_the_side_preview_explicit` (crates/git/tests/conflict.rs), `a_deleted_side_is_one_absent_side` (crates/engine/src/conflict.rs) (staging from the GUI not driven)

### INT-30 Add/add conflict has no Base
Fixture: `fx-merge-conflict` plus a file `new.rs` added differently on both sides
Layer: product
- Then Base is shown as absent while Ours and Theirs preview; resolving works
Covered by (backend, partial): `add_add_conflict_has_no_base` (crates/git/tests/conflict.rs); GUI path not automated

### INT-31 Oversized working file validates without a text preview
Fixture: `fx-merge-conflict` plus a conflicted file over the preview limit
Layer: product
- Then no text preview is shown, Block by block is disabled, and Stage resolution of a side still validates the working bytes by hash
Covered by (backend, partial): `oversized_working_files_validate_without_a_preview` (crates/git/tests/conflict.rs); `oversized_sides_publish_and_changed_bytes_are_detected` (crates/git/tests/conflict.rs)

### INT-32 Write succeeded but staging failed is explicit
Fixture: `fx-merge-conflict` with `.git/index.lock` created after the preview
Layer: product
- When **Stage resolution** is pressed
- Then the error states the working file was already replaced; the path stays conflicted and inspectable; the operation state is kept
Covered by (backend, partial): `publishing_reports_a_partial_worktree_write` (crates/git/tests/conflict.rs)

### INT-33 Open in editor uses the Preferences command
Fixture: `fx-merge-conflict`, Preferences › General › **Editor** = `myedit --line 1 {file}` (a stub recording argv)
Layer: product
- When **Open in editor** is clicked on `src/app.rs`
- Then the stub receives `--line 1 <abs path>/src/app.rs`; with `myedit` (no `{file}`) the path is appended; an empty command uses `xdg-open`/`open`/`start`; Git Good does not wait, and the saved file is picked up by the watcher
Covered by (interaction, partial): `the_file_goes_where_the_command_says_or_at_the_end` (src/editor.rs)

### INT-34 All resolved: the band turns purple and Continue is primary
Fixture: `fx-merge-conflict`
Layer: product
- Given all three paths are staged in the workspace
- Then the file list reads `All conflicts resolved. Continue with Ctrl+Enter.`, the band is purple with `Merge ready · …` and `All files staged`, and **Continue** is the filled primary with `Ctrl+Enter`
Covered by (interaction, partial): `continue_is_the_primary_only_once_nothing_is_conflicted` (apps/git-good/src/operation_band.rs); the workspace state not driven

### INT-35 A running Continue keeps the band in place
Fixture: `fx-merge-conflict`
Layer: interaction
- When **Continue** is clicked
- Then while it runs the band keeps its layout: the icon becomes a spinner, the title reads `Continuing merge · merging …` (`Skipping cherry-pick`, `Aborting revert`), line 2 reads `Writing the merge commit…`, and every button stays in place disabled; Merge/Pull/Stash are disabled; a failure stays in the footer with **Details…** and keeps Git's state
Covered by (interaction): `a_running_job_keeps_the_buttons_and_names_itself`, `the_busy_band_keeps_every_button_in_place` (apps/git-good/src/operation_band.rs)

### INT-36 Stage a conflicted path from Local Changes is refused
Fixture: `fx-merge-conflict`
Layer: product
- When the user tries to stage/discard `src/app.rs` from Local Changes
- Then the action is refused with a reason pointing to **Resolve conflicts…**; the conflict row shows an unmerged summary
Covered by (interaction): `conflicts_and_staged_discards_are_refused_before_spawning` (src/stage_view.rs); `conflict_row_reads_an_unmerged_summary_and_route` (src/status_view.rs)

### INT-37 CLI conflicts and conflict --use/--stage — see CLI-16i, CLI-16j

### INT-38 Rebase conflict labels name the base and the replayed commit
Fixture: `fx-rebase-conflict` (step 4 of 6, replaying `<id> <subject>` onto `origin/main`)
Layer: product
- When **Resolve conflicts** opens the conflicted file
- Then block columns read `Ours · rewritten branch (onto origin/main)` and `Theirs · <8-char id> (replayed commit)` (the subject in the tooltip), the legend reads `In a rebase, Ours is the rewritten branch so far and Theirs is the commit being replayed.`, the footer's right reads `Abort restores <backup ref>`; keys `1`/`2` choose the sides as labeled
Covered by (backend, interaction, partial): `side_labels_name_the_real_refs_with_the_stage_in_the_tooltip`, `markers_and_legend_name_the_sides` (apps/git-good/src/conflict_workspace.rs); `side_names_follow_the_operation` (crates/git/tests/conflict.rs) (the rebase workspace and keys not driven)

### INT-39 The Result is editable and keeps hand edits
Fixture: `fx-merge-conflict`, `src/app.rs`
Layer: interaction
- When the user types in the Result, then chooses an option for another block
- Then only that block's region is rewritten and the typed text stays; choosing an option for a block already edited by hand asks first (`Replace edited block`, **Cancel** `Esc`, danger **Replace block**); `Ctrl+Z` undoes typing and choices together, `Ctrl+Y` redoes
Covered by (interaction): `typing_in_the_result_edits_it_and_ctrl_z_undoes` (apps/git-good/src/main.rs); `an_option_replaces_only_its_block`, `undo_restores_text_and_origins_and_typing_coalesces`, `an_emptied_block_comes_back_in_place` (apps/git-good/src/conflict_result.rs); `a_hand_edited_block_asks_before_an_option_replaces_it` (apps/git-good/src/conflict_blocks.rs)

### INT-40 Result lines are tinted by origin
Fixture: `fx-merge-conflict`, `src/app.rs`
Layer: visual
- Given block 1 is **Ours + theirs** and block 2 is open
- Then block 1's Ours lines carry the Ours tint (blue), its Theirs lines the Theirs tint (amber), block 2 the warning tint with orange markers, common lines none; a typed line turns neutral and the others keep their tint while typing
Covered by (interaction): `origins_tint_each_line_and_stay_while_typing` (apps/git-good/src/conflict_result.rs); `markers_of_unresolved_blocks_take_the_warning_color` (apps/git-good/src/conflict_workspace.rs)

### INT-41 − unstages a resolution back to conflicted
Fixture: `fx-merge-conflict`, `src/app.rs` resolved and staged
Layer: product
- When **−** is clicked on its `RESOLVED, STAGED` row (tooltip `Unstage resolution · back to conflicted`)
- Then Git recreates the conflict (markers back in the working file) and the path returns under `CONFLICTED`; a path with nothing to recreate is unstaged to a plain change
Covered by (backend, interaction): `a_staged_resolution_reopens_as_a_conflict` (crates/git/tests/conflict.rs); `hand_edits_in_the_result_are_what_stage_resolution_stages_and_minus_reopens` (apps/git-good/src/main.rs)

### INT-42 Workspace layout matches Final · Rebase conflict
Fixture: `fx-merge-conflict` at 1440×900 and 1120×740
Layer: visual
- Then the left pane is 300 px with 22 px rows, the file header 28 px, the blocks take up to 60% of the height and scroll, and the Result fills the rest; the footer reads `1 / 2 / 3 / 4: ours · theirs · ours+theirs · theirs+ours`, `[ ]: previous / next conflict`, `Ctrl+S: stage file`, `Esc: changes list`, and at the right `Abort restores main to its tip before the merge`
Covered by (Not automated; native screenshots against the mockup)

## Rewrite and recovery

### RWR-01 Amend HEAD loads the HEAD message and names the amend base — see CHG-57

### RWR-02 Amend mode shows the commit contents as Staged — see CHG-57

### RWR-03 Amend drafts stay separate from the normal draft — see CHG-60

### RWR-04 Amend success rewrites HEAD, keeps author, writes a backup — see CHG-58

### RWR-05 Message-only amend with an empty index — see CHG-59

### RWR-06 Amend is refused on an unborn branch — see CHG-61

### RWR-07 Amend hooks get the old commit; post-rewrite failure is a warning
Fixture: `fx-hooks` plus a `post-rewrite` hook that exits 1
Layer: product
- When the user amends HEAD with a subject not starting with `wip` and the pre-commit hook passing (disable it)
- Then `prepare-commit-msg` receives `commit` and the old ID; the amend is published; the post-rewrite failure is reported as a warning, not an error
Covered by (backend, partial): `amend_hooks_receive_the_old_commit_and_post_rewrite_warnings_are_kept` (crates/git/tests/commit.rs); GUI path not automated

### RWR-08 Commit menu lists the rewrite commands
Fixture: `fx-linear`, the History tree focused, `c25` selected
Layer: interaction
- When the user opens **Commit**
- Then it lists **Interactive rebase from here…** (`R`), **Reword…** (`Ctrl+R`), **Reopen for editing…** (`E`), **Fixup into…** (`F`), **Drop…** (`D`), **Reset current branch to here…**
Covered by (interaction): `menus_follow_the_final_design_mockup` (apps/git-good/src/main.rs)

### RWR-09 Reword/Reopen/Fixup/Drop open the plan with the commit preset
Fixture: `fx-rebase`, the 2nd-newest commit selected in the History tree
Layer: interaction
- When the user presses `Ctrl+R` (then repeats with `E`, `F`, and `D` with **Confirm before Drop and Reset** on)
- Then the interactive rebase dialog opens with that row selected and its action chip set to **Reword** / **Reopen** / **Fixup** / **Drop**; a reword focuses the message field seeded with the original
Covered by (interaction): `commit_menu_presets_select_the_commit_in_the_rebase_plan` (apps/git-good/src/main.rs)

### RWR-10 Fixup preset on a commit with nothing kept below is refused
Fixture: `fx-rebase`, the oldest of the 6 own commits selected
Layer: product
- When the user chooses **Commit › Fixup into…**
- Then the dialog shows the row as Pick and the reason `No kept commit below to combine into.`; **Start rebase** stays disabled with that reason
Covered by (interaction): `commit_menu_presets_select_the_commit_in_the_rebase_plan` (apps/git-good/src/main.rs)

### RWR-11 Interactive rebase opens from History tree with R
Fixture: `fx-rebase`, the 4th-newest commit selected, the History tree focused
Layer: visual
- When the user presses `R`
- Then a large modal opens 12 px below the toolbar, centered, over the dimmed History tree; menu bar and toolbar stay visible; header reads **Interactive rebase**, the branch `main`, the range `<from>…HEAD onto <base>` with 7-char IDs, **Cancel** `Esc`, and **Start rebase** `Ctrl+Enter`
Covered by (interaction, partial): `rewrite_dialog_renders_at_supported_widths` (apps/git-good/src/main.rs)

### RWR-12 Interactive rebase of current branch covers the branch's own commits
Fixture: `fx-rebase`, `c20` selected in the History tree
Layer: product
- When the user runs **Branch › Interactive rebase of current branch…** or its search Action
- Then its hint reads *Rewrite the commits the upstream, or else the default branch, lacks*; the plan opens centered 12 px below the toolbar (RWR-11) and lists exactly the 6 commits not on `origin/main`, regardless of the History tree selection, which stays on `c20`
Covered by (backend, interaction, partial): `branch_menu_rewrite_covers_the_branch_and_keeps_the_selection`, `commands_off_the_toolbar_keep_their_menu_home_and_search_entry` (apps/git-good/src/main.rs); `branch_start_prefers_unpushed_commits_then_the_default_branch` (crates/git/tests/rewrite.rs) (menu route; the hint and the search route not driven)

### RWR-13 Interactive rebase of current branch is disabled when the branch has no own commits
Fixture: `fx-remote-even`
Layer: interaction
- Then the **Branch › Interactive rebase of current branch…** menu item and its search Action are disabled with *The current branch has no commits beyond its upstream or the default branch; use Commit › Interactive rebase from here… on a selected commit*
Covered by (interaction): `stash_commands_follow_local_changes_and_entries` (reason present), `commands_off_the_toolbar_keep_their_menu_home_and_search_entry` (menu and search share it) (apps/git-good/src/main.rs)

### RWR-14 Plan lists newest first and starts unchanged
Fixture: `fx-rebase`, dialog from **Interactive rebase of current branch…**
Layer: visual
- Then the plan header reads `PLAN · NEWEST FIRST, LIKE THE HISTORY TREE` with `j k select · Alt+↑↓ move · P R E S F D`; rows are in History tree order; every chip reads **Pick**; the last row names the base (`<id> base · unchanged`); under the list: `Keys on the selected row: P pick · R reword · E reopen · S squash · F fixup · D drop`; **Start rebase** is disabled with `The plan matches the History tree.`
Covered by (interaction): `lists_newest_first_and_starts_unchanged` (apps/git-good/src/rebase_plan.rs)

### RWR-15 Plan rows have two lines, pills, and Pick has no badge
Fixture: `fx-rebase`
Layer: visual
- Then each row is 40 px with a drag handle, move up/down buttons, short ID (hover: full ID and refs), `wip` / `v0.1.39` pills when they fit, the subject; line 2 is empty for Pick rows
Covered by (interaction): `rebase_dialog_layout_holds_still_when_actions_change` (apps/git-good/src/main.rs)

### RWR-16 Action chips use the action colors
Fixture: `fx-rebase`
Layer: visual
- When the user sets six rows to P, R, E, S, F, D (in a valid arrangement)
- Then chips are 24 px rounded: Pick neutral with border, Reword blue, Reopen purple, Squash and Fixup amber, Drop red; each badge is in its action's color
Covered by (Not automated)

### RWR-17 Chip menu lists all six actions; unavailable ones disabled with reason
Fixture: `fx-rebase`, oldest row selected
Layer: interaction
- When the user clicks the oldest row's chip
- Then a menu lists Pick, Reword, Reopen, Squash, Fixup, Drop, each with its hint (e.g. `Keep the commit as it is`, `Combine into the commit below and keep both messages`) and key; Squash and Fixup are disabled with `No kept commit below to combine into.`
Covered by (interaction, partial): `every_action_has_a_distinct_key` (apps/git-good/src/rebase_plan.rs) (keys only); menu rendering Not automated

### RWR-18 j/k and Up/Down select rows
Fixture: `fx-rebase`
Layer: interaction
- When the user presses `j` twice then `k` (and Down/Up)
- Then the selection moves down two rows then up one; the right pane follows the selected commit
Covered by (interaction): `rebase_dialog_keys_choose_actions_and_start_only_when_valid` (apps/git-good/src/main.rs)

### RWR-19 Letter keys set the selected row's action
Fixture: `fx-rebase`
Layer: interaction
- When the user presses `P`, `R`, `E`, `S`, `F`, `D` in turn on the 2nd-newest row
- Then the chip follows each key (Pick, Reword, Reopen, Squash, Fixup, Drop)
Covered by (interaction): `rebase_dialog_keys_choose_actions_and_start_only_when_valid` (apps/git-good/src/main.rs)

### RWR-20 Keys keep working after a button took focus
Fixture: `fx-rebase`
Layer: interaction
- Given the user clicked a row's move-up button
- When the user presses `D`
- Then the selected row becomes Drop
Covered by (interaction): `rebase_dialog_keys_still_work_after_a_button_took_focus` (apps/git-good/src/main.rs)

### RWR-21 Letters wait while the reword message has focus
Fixture: `fx-rebase`, a row set to Reword with the message field focused
Layer: interaction
- When the user types `Drop squash`
- Then the text goes into the message; no row action changes
Covered by (Not automated)

### RWR-22 Alt+↑/↓ and the move buttons reorder rows
Fixture: `fx-rebase`, 3rd-newest row selected
Layer: interaction
- When the user presses `Alt+Up`, then clicks the row's move-down button twice
- Then the row moves up one, then down two, keeping its selection; result count updates; **Start rebase** becomes enabled
Covered by (interaction): `moving_rows_reorders_the_steps` (apps/git-good/src/rebase_plan.rs)

### RWR-23 Drag handle reorders rows
Fixture: `fx-rebase`
Layer: interaction
- When the user drags the newest row's handle (hover `Drag to move`) to below the 3rd row
- Then the row drops at that position; the plan steps follow
Covered by (Not automated)

### RWR-24 Squash combines into the nearest kept commit below
Fixture: `fx-rebase`, rows newest first A…F
Layer: product
- When the user sets B to **S**
- Then B is indented 12 px under C, its subject is dimmed, its badge reads `↓ squash into <C>`; C's badge reads `absorbs <B>`
Covered by (interaction): `squash_and_fixup_combine_into_the_nearest_kept_commit_below` (apps/git-good/src/rebase_plan.rs); `rebase_dialog_keys_choose_actions_and_start_only_when_valid` (apps/git-good/src/main.rs)

### RWR-25 Squash/fixup skip dropped commits
Fixture: `fx-rebase`
Layer: product
- When the user sets C to **D** and B to **F**
- Then B's badge reads `↓ fixup into <D>` (skipping dropped C); D's badge reads `absorbs <B>`; after Start, the result has D's message only
Covered by (backend, interaction): `squash_and_fixup_combine_into_the_nearest_kept_commit_below` (apps/git-good/src/rebase_plan.rs); `rewrite_squash_and_fixup_skip_dropped_commits` (crates/git/tests/rewrite.rs)

### RWR-26 Chained squashes all absorb into one commit
Fixture: `fx-rebase`
Layer: product
- When the user sets A and B to **S**
- Then C's badge reads `absorbs <B>, <A>`
Covered by (interaction): `squash_and_fixup_combine_into_the_nearest_kept_commit_below` (apps/git-good/src/rebase_plan.rs)

### RWR-27 The oldest kept commit cannot be squashed or fixed up
Fixture: `fx-rebase`
Layer: product
- When the user presses `S` on the oldest row (or on any row whose rows below are all dropped)
- Then the action does not change and the reason `No kept commit below to combine into.` is shown
Covered by (interaction): `the_oldest_kept_commit_cannot_be_combined` (apps/git-good/src/rebase_plan.rs)

### RWR-28 The last kept commit cannot be dropped
Fixture: `fx-rebase`
Layer: product
- When the user drops five rows, then presses `D` on the sixth
- Then it stays kept and the reason reads `At least one commit must stay. Use Reset to remove them all.`
Covered by (interaction): `reopen_stops_and_the_last_kept_commit_cannot_be_dropped` (apps/git-good/src/rebase_plan.rs)

### RWR-29 Drop row tint, strike-through, and dimmed reader
Fixture: `fx-rebase`
Layer: visual
- When the user sets a row to **D**
- Then the row has the dropped tint, its subject is struck through, badge `dropped · recoverable from the backup`; the right pane dims the reader and reads `Dropped · these changes leave the branch`
Covered by (Not automated)

### RWR-30 Reopen badge
Fixture: `fx-rebase`
Layer: visual
- When the user sets a row to **E**
- Then its chip reads **Reopen** and its badge `⏸ stops here · reopens staged in Local Changes`; the result count includes `1 stop`
Covered by (interaction): `reopen_stops_and_the_last_kept_commit_cannot_be_dropped` (apps/git-good/src/rebase_plan.rs)

### RWR-31 Reword edits the message on the right
Fixture: `fx-rebase`
Layer: product
- When the user presses `R` on a row
- Then the right pane shows `New message (reword)` with an editable message seeded with the original and `Was: <original subject>` below it; the badge reads `reworded`
Covered by (interaction): `reword_starts_from_the_original_message_and_needs_text` (apps/git-good/src/rebase_plan.rs)

### RWR-32 Empty reword message blocks Start
Fixture: `fx-rebase`, a row set to Reword
Layer: product
- When the user clears the message
- Then **Start rebase** is disabled with `Reword <id> needs a message.`
Covered by (interaction): `reword_starts_from_the_original_message_and_needs_text` (apps/git-good/src/rebase_plan.rs)

### RWR-33 Result count summarizes the plan
Fixture: `fx-rebase`
Layer: visual
- When the user drops one row, squashes one, and reopens one
- Then the header reads `6 commits → 4 · 1 dropped · 1 stop`
Covered by (Not automated)

### RWR-34 Right pane shows the selected commit in an embedded Review
Fixture: `fx-rebase`, row touching `folder_picker.rs` selected
Layer: interaction
- Then the right pane shows short ID, action, author, relative date, full message, then files above the shared reader with `folder_picker.rs`'s diff; no Back button, identity, or status line
Covered by (interaction, partial): `rewrite_dialog_renders_at_supported_widths` (apps/git-good/src/main.rs)

### RWR-35 Open in Review holds the plan and returns to it
Fixture: `fx-rebase`
Layer: interaction
- Given rows changed (one Drop, one Reword with edited text)
- When the user presses `Enter` (**Open in Review**), then `Esc` in Review
- Then ordinary Review opens with **‹ Rebase**; Esc returns to the dialog with the same actions, message, and selection
Covered by (interaction): `rebase_dialog_opens_review_and_returns_to_the_same_plan` (apps/git-good/src/main.rs)

### RWR-36 Footer: stash first, refs that stay, backup notice
Fixture: `fx-rebase`
Layer: product
- Then the footer has **Stash working changes first** (unchecked with default prefs), `Stay on the original commits: wip, v0.1.39`, and `Rewrites commits. A private backup ref is created before Git starts; Recovery can restore it. Merge commits are not supported.`; `origin/main` and the current branch are not listed as staying
Covered by (interaction): `rebase_dialog_names_the_branch_and_the_refs_left_behind` (apps/git-good/src/main.rs)

### RWR-37 Stash working changes first starts from the preference
Fixture: `fx-rebase` with an unstaged edit to a tracked file; `rebase_autostash: true` in settings
Layer: product
- When the dialog opens
- Then **Stash working changes first** is checked; with it unchecked, Start fails with the clean-worktree error and nothing is rewritten; checked, the rewrite runs and the edit is restored after
Covered by (backend, partial): `rewrite_requires_a_clean_worktree_unless_autostash_is_enabled` (crates/git/tests/rewrite.rs) (backend only)

### RWR-38 Merge commits are rejected with the reason
Fixture: `fx-branchy`, `c25` selected (range includes `Merge branch 'topic'`)
Layer: product
- When the user presses `R`
- Then the plan is not offered; the reason names merge commits as unsupported; no rewrite runs
Covered by (backend, partial): `rewrite_rejects_merge_commits` (crates/git/tests/rewrite.rs); GUI path not automated

### RWR-39 Cancel / Esc leaves the commits untouched
Fixture: `fx-rebase`, plan edited
Layer: product
- When the user presses `Esc`
- Then the dialog closes, no rewrite job starts, no backup ref is created, HEAD is unchanged
Covered by (interaction): `rebase_dialog_keys_choose_actions_and_start_only_when_valid` (apps/git-good/src/main.rs)

### RWR-40 Start rebase runs in the background with one operation band
Fixture: `fx-rebase`, a reorder plan
Layer: interaction
- When the user presses `Ctrl+Enter`
- Then the dialog closes at once; the purple band alone reads `Rewriting <branch> · step <n> of <m> · picking <8-char id>` with a spinner, `Browsing stays available; the old History tree is labelled until the new one is accepted.`, the step strip, and no buttons; the footer reads `Backup: refs/gg/backup/…`; then the band reads `Refreshing the History tree…` (every step ✓) and the footer `Rewrote <n> commits · backup <ref>`; the History tree stays browsable; duplicate rewrites are disabled; the band clears when the new History tree is adopted without the viewport jumping
Covered by (interaction): `rewrite_progress_lives_once_in_the_operation_area`, `rewrite_refresh_clears_the_operation_area` (apps/git-good/src/main.rs); `the_rewrite_band_names_the_step_git_is_applying` (apps/git-good/src/operation_band.rs); `rewrite_reorders_commits_and_records_the_previous_tip` (crates/git/tests/rewrite.rs)

### RWR-41 A rewrite creates a private backup before Git runs
Fixture: `fx-rebase`
Layer: product
- When a rewrite completes
- Then `refs/gg/backup/rewrite/<unix-nanos>` points at the old HEAD before Git rewrites anything (the footer names it while Git runs); it is not shown in the sidebar or graph; the receipt reads `Rewrote <n> commits · backup refs/gg/backup/rewrite/…`
Covered by (backend, partial): `rewrite_reorders_commits_and_records_the_previous_tip` (crates/git/tests/rewrite.rs); `backup_refs_point_at_the_previous_tip_and_stay_out_of_refs` (crates/git/tests/backup.rs)

### RWR-42 The post-rewrite follow-up matches the upstream relationship
Fixture: `fx-rebase` (only unpushed commits rewritten; `origin/main` stays an ancestor); `fx-remote-even` (a pushed commit reworded; diverged); `fx-rebase-local` (no upstream)
Layer: product
- When a rewrite of unpushed commits succeeds in `fx-rebase`
- Then the footer follow-up is plain **Push**
- When a pushed commit is reworded in `fx-remote-even` with **Offer force-with-lease after a rewrite** on
- Then the follow-up is **Push with force-with-lease…**, opening the confirmation naming the lease commit (NET-22); with the preference off it is **Recovery…**
- When a rewrite succeeds in `fx-rebase-local`, or leaves the branch even with its upstream
- Then the follow-up is **Recovery…**, opening the Backups source; a rewrite that stops on conflicts shows the band, whose **Resolve conflicts** takes over. Git Good reads the relation from local refs only (no fetch)
Covered by (backend, interaction, partial): `rewrite_follow_up_offers_the_push_the_result_needs`, `outcome_recovery_button_opens_backups` (apps/git-good/src/main.rs); `upstream_relation_reads_local_refs_only` (crates/git/tests/sync.rs), `upstream_relation_follows_ahead_and_behind_counts` (crates/engine/src/sync.rs) (the choice of follow-up; the lease confirmation it opens not driven)

### RWR-43 Rewrite failure keeps recovery reachable
Fixture: `fx-rebase` with a `commit-msg` hook rejecting the reworded message
Layer: product
- When a plan with that reword starts
- Then the rewrite stops; the outcome reports Git's hook output as a warning and keeps a **Recovery…** action that opens the backup
Covered by (interaction): `rewrite_failure_keeps_backup_recovery_reachable`, `refresh_failure_keeps_backup_recovery_reachable` (apps/git-good/src/main.rs)

### RWR-44 Reopen stop band
Fixture: `fx-rewrite-reopened` (stopped by Git Good's reopen at step 3 of 6)
Layer: visual
- Then the operation band is purple with a pause glyph; line 1 reads `Rebase stopped · step 3 of 6 · reopened <8-char id>` with **Continue** (`Ctrl+Enter`), **Skip commit**, **Abort and restore** (red); line 2 reads `Its changes are staged. Edit, unstage parts to split, then continue.` followed by the step chips (`✓ <id> pick` for done, the current filled chip reading `reopen`, then next steps; subject on hover)
- And with a 12-step plan at 1120 and 720 px the strip shows `✓ n done`, the current chip, the next chips that fit, and `+N next` (the plan on hover), with no scroll area, never overlapping the explanation or leaving the band
Covered by (interaction): `reopened_commit_opens_local_changes_with_its_message_and_waits_to_continue` (apps/git-good/src/main.rs); `titles_name_the_stop` (apps/git-good/src/rebase_stop.rs); `the_band_holds_its_lines_chips_and_status_without_clipping`, `the_strip_keeps_the_current_step_and_collapses_the_rest` (apps/git-good/src/operation_band.rs)

### RWR-45 Reopened commit opens Local Changes with its message
Fixture: `fx-rewrite-reopened`
Layer: product
- When Git Good loads the stop
- Then Local Changes opens once with the commit's changes staged and its original message in an empty commit box; reloading/focus does not reopen it again
Covered by (interaction): `reopened_commit_opens_local_changes_with_its_message_and_waits_to_continue`, `commits_made_at_a_stop_are_listed_and_continue_is_ready` (apps/git-good/src/main.rs)

### RWR-46 Reopened-stop commit button labels — see CHG-69

### RWR-47 Commits made at this stop are listed; Continue becomes ready
Fixture: `fx-rewrite-reopened`
Layer: product
- When the user commits in two parts
- Then the band lists `Commits made at this stop: <id> <subject>, …` and Local Changes shows `COMMITS MADE AT THIS STOP` under STAGED (wide) with 8-char IDs; once no tracked changes remain, **Continue** is enabled as the filled blue primary
Covered by (interaction): `commits_made_at_a_stop_are_listed_and_continue_is_ready` (apps/git-good/src/main.rs); `commits_made_at_a_rebase_stop_sit_under_the_lists` (apps/git-good/src/status_view.rs)

### RWR-48 Continue waits while tracked changes remain at a reopened stop
Fixture: `fx-rewrite-reopened`
Layer: product
- Then **Continue** is disabled and the band's status reads `Commit or stash the 2 remaining changed files first.` at normal size; `Ctrl+Enter` with no text field focused does nothing
Covered by (interaction): `continue_waits_for_conflicts_and_remaining_changes` (apps/git-good/src/rebase_stop.rs)

### RWR-49 Reopening a root commit stays at HEAD in amend mode
Fixture: `fx-linear`, plan from `c01` with `c01` set to **E**
Layer: product
- When the rewrite stops
- Then HEAD stays at `c01`; Local Changes opens in amend mode; band line 2 reads `A root commit stays at HEAD. Amend it in Local Changes, then continue.`
Covered by (backend, partial): `rewrite_reopen_of_a_root_commit_stays_at_head` (crates/git/tests/rewrite.rs); GUI path not automated

### RWR-50 Conflict stop band
Fixture: `fx-rebase-conflict`
Layer: visual
- Then the band is amber with a warning glyph; line 1 reads `Conflict · step 4 of 6 · applying <8-char id> <subject>`, then the status `Resolve and stage 1 conflicted file first.` at normal size (the title truncates first; at 1120 px the status reads in full), then **Resolve conflicts** (filled primary, hidden while the workspace is open), **Continue** (disabled), **Skip commit**, **Abort and restore**; line 2 `Resolve each conflicted file and stage the result, then continue.` and the step chips, unclipped inside the band; **Recovery…** is not among the continuation buttons
Covered by (interaction): `continue_waits_for_conflicts_and_remaining_changes`, `titles_name_the_stop`, `a_conflict_stop_offers_resolve_conflicts_until_the_workspace_opens` (apps/git-good/src/rebase_stop.rs); `the_band_holds_its_lines_chips_and_status_without_clipping` (apps/git-good/src/operation_band.rs); `rebase_operation_area_keeps_recovery_out_of_continuation` (apps/git-good/src/main.rs)

### RWR-51 Window title and branch button during a stop
Fixture: `fx-rebase-edit`, then `fx-rebase-conflict`
Layer: visual
- Then the OS title and menu-bar identity read `Git Good — ~/… — rebasing main · step 3 of 6` / `Git Good — ~/… — rebase conflict · step 4 of 6`; the branch button reads `main` followed by `rebasing 3/6` / `conflict 4/6` in the band's color, without ahead/behind counts
Covered by (interaction): `window_title_names_the_folder_and_a_stopped_rebase` (apps/git-good/src/main.rs)

### RWR-52 Skip commit leaves the stopped commit out
Fixture: `fx-rebase-conflict`
Layer: product
- When the user clicks **Skip commit**
- Then Git skips step 4 and continues; on completion the commit is absent from the History tree, the band clears, History tree/refs/status/backups refresh
Covered by (backend, partial): `rewrite_conflict_can_abort_or_skip` (crates/git/tests/rewrite.rs); GUI path not automated

### RWR-53 Abort and restore returns the branch to its pre-rewrite tip
Fixture: `fx-rebase-conflict` (or `fx-rewrite-reopened`)
Layer: product
- When the user clicks **Abort and restore** (tooltip `Stop the rebase and restore the branch as it was.` plus `Backup: <ref>` when known)
- Then HEAD equals the pre-rewrite tip, the band clears, the title drops the rebase suffix, and the pre-rewrite History row is selected again
Covered by (backend, partial): `rewrite_conflict_can_abort_or_skip` (crates/git/tests/rewrite.rs); GUI path not automated

### RWR-54 Band holds still while the stop changes
Fixture: `fx-rewrite-reopened`
Layer: visual
- When a commit is made at the stop and the band updates
- Then the band's height and the panes below do not shift
Covered by (interaction): `panes_hold_still_while_a_rebase_stop_changes` (apps/git-good/src/main.rs)

### RWR-55 Rebase current onto… dialog
Fixture: `fx-remote`
Layer: product
- When the user chooses **Branch › Rebase current onto…** (or a branch/remote/tag context-menu Rebase)
- Then a modal **Rebase branch** dialog shows a reference field, an optional **Leave out commits from** field (RWR-80), **Automatically stash working changes** (starting from the preference), and `Rebase rewrites commits. A private backup ref is created before Git starts.`; ref actions are disabled while a mutation runs
Covered by (interaction, partial): `rebase_dialog_and_banner_render_at_supported_widths` (apps/git-good/src/main.rs)

### RWR-56 Rebase onto creates a backup and rewrites the branch
Fixture: `fx-remote`
Layer: product
- When the user rebases `main` onto `origin/main`
- Then `refs/gg/backup/rebase/<unix-nanos>` points at the old `main`; `main` contains `origin/main`; the receipt/outcome lists the backup
Covered by (backend, end-to-end, partial): `rebase_rewrites_the_branch_and_records_the_previous_tip` (crates/git/tests/rebase.rs); `rebase_cli_rewrites_records_backup_and_recovers` (apps/git-good/tests/cli.rs)

### RWR-57 Rebase onto refuses a dirty tree without autostash
Fixture: `fx-dirty` with a branch `other` diverged from `main`
Layer: product
- When the user rebases onto `other` with **Automatically stash working changes** off
- Then it fails before any change with the clean-worktree error; no backup ref is written; untracked `new.txt` alone would not block
Covered by (backend, partial): `rebase_requires_a_clean_worktree_unless_autostash_is_enabled` (crates/git/tests/rebase.rs); GUI path not automated

### RWR-58 Rebase onto rejects missing/option-shaped refs
Fixture: `fx-linear`
Layer: product
- When `git good --cli rebase --exec=x` or `git good --cli rebase nosuch` runs
- Then it fails with an actionable error; no backup, no state change
Covered by (backend, partial): `rebase_rejects_option_shaped_and_missing_references` (crates/git/tests/rebase.rs); GUI path not automated

### RWR-80 Rebase onto, leaving out another branch's commits
Fixture: `fx-onto`
Layer: product
- Given `client` checked out (Pro Git 3.6, *More Interesting Rebases*)
- When **Rebase current onto…** runs with `master` and **Leave out commits from** `server` (or `git good --cli rebase master --leave-out server`)
- Then, as `git rebase --onto master server`, `client` is `C8 · C9` on top of `master`'s `C6`; `server`'s `C3` is not replayed; the backup ref holds the old `client`; the worker reads `Rebasing client onto master, leaving out server…`; an option-shaped or missing leave-out ref fails before any change
Covered by (backend, end-to-end, interaction): `rebase_onto_leaves_out_the_commits_of_another_branch` (crates/git/tests/rebase.rs), `rebase_cli_leaves_out_the_commits_of_another_branch` (apps/git-good/tests/cli.rs), `rebase_dialog_leaves_out_the_commits_of_another_branch`, `rebase_dialog_and_banner_render_at_supported_widths` (apps/git-good/src/main.rs)

### RWR-81 Split a commit with Reopen
Fixture: `fx-split`
Layer: product
- Given `Update README formatting and add blame` (adding `README2` and `lib.rb`) between `Initial commit` and `Add cat-file` (Pro Git 7.6, *Splitting a Commit*)
- When it is reopened for editing, `lib.rb` is unstaged, `Update README formatting` is committed, `lib.rb` is staged and `Add blame` committed, and the rewrite continues
- Then the branch reads `Initial commit · Update README formatting (README2) · Add blame (lib.rb) · Add cat-file`
Covered by (end-to-end): `reopening_a_commit_splits_it_in_two` (apps/git-good/tests/cli.rs)

### RWR-59 CLI rewrite: reword, fixup, drop, edit — see CLI-16e

### RWR-60 CLI rewrite: reorder, squash, continue/abort — see CLI-16f

### RWR-61 CLI rewrite validates the step set
Fixture: `fx-rebase`
Layer: product
- When steps omit a range commit, repeat one, name a commit outside the range, drop every commit, or give an empty reword message
- Then each is refused before any mutation (all-dropped points to Reset)
Covered by (backend): `rewrite_validates_the_full_step_set`, `rewrite_validates_new_actions` (crates/git/tests/rewrite.rs)

### RWR-62 CLI rewrite rejects a shallow boundary and a non-ancestor start
Fixture: `fx-shallow`; `fx-branchy` with `--from` on `feature/login`
Layer: product
- Then both are refused (shallow is not rewritten as a root)
Covered by (backend): `rewrite_rejects_a_shallow_boundary`, `rewrite_rejects_a_start_outside_head_history` (crates/git/tests/rewrite.rs)

### RWR-63 Reset current branch to here… asks with modes and consequence list
Fixture: `fx-dirty`, `c27` selected
Layer: product
- When the user chooses **Commit › Reset current branch to here…**
- Then a modal **Reset here** opens on Mixed (`Reset the current branch using mixed reset to` + `c27`'s summary); radios **Soft** / **Mixed** / **Hard** with hints; `LEAVE THE BRANCH (RECOVERABLE FROM THE REFLOG)` lists `c30`, `c29`, `c28` once read in the background; primary **Reset** `Enter`, **Cancel** `Esc`
Covered by (interaction, partial): `reset_confirmation_renders_at_supported_widths` (apps/git-good/src/main.rs)

### RWR-64 Hard reset lists what it loses and refuses untracked collisions
Fixture: `fx-dirty`; `fx-reset-collision`
Layer: product
- When the user selects **Hard** in **Reset here** targeting `c27` in `fx-dirty`
- Then `CHANGES LOST` lists the tracked staged/unstaged paths (`a.txt`, `b.txt`, `c.txt`, …; not untracked `new.txt`); the primary becomes red **Hard reset**, enabled once the lists arrive
- When **Hard** targets `c05` in `fx-reset-collision`
- Then `UNTRACKED FILES OVERWRITTEN` lists `later.txt` (ignored files it would overwrite too); **Hard reset** is disabled and the notice line and its tooltip read `Move or delete the untracked files listed above, then reset.`; nothing is reset; Soft and Mixed stay available
Covered by (interaction, partial): `reset_confirmation_renders_at_supported_widths`, `hard_reset_lists_untracked_files_it_would_overwrite_and_refuses` (apps/git-good/src/main.rs) (`CHANGES LOST` contents for `fx-dirty` not checked)

### RWR-65 Reset modes act like Git; Hard never overwrites untracked files
Fixture: `fx-dirty`; `fx-reset-collision`
Layer: product
- When the user resets to `c27` Soft / Mixed / Hard (separate runs)
- Then Soft keeps index and working files; Mixed clears the index, keeps working files; Hard discards staged and working tracked changes; untracked files stay in all
- And `git good --cli reset --hard c05` in `fx-reset-collision` refuses nonzero with empty stdout, stderr saying it would overwrite untracked files and naming `later.txt`; HEAD, index and `later.txt` are unchanged
- And the same refusal holds when the untracked file's name is not UTF-8 (`b'raw\xff.txt'`, listed lossily) and when the repository's folder name ends with a space
- And every mode first saves the tip it leaves as a `reset` backup (`refs/gg/backup/reset/…`), named in the receipt's `backup` and in the GUI's status line (*Reset to <id> <subject> · backup <ref>*); when the backup cannot be written the reset fails and nothing changes
Covered by (backend, end-to-end): `soft_reset_keeps_index_and_worktree`, `mixed_reset_clears_the_index_but_keeps_worktree`, `hard_reset_discards_index_and_worktree`, `hard_reset_refuses_to_overwrite_untracked_and_ignored_files`, `hard_reset_refuses_to_overwrite_an_untracked_file_whose_name_is_not_utf8`, `hard_reset_refuses_to_overwrite_untracked_files_in_a_folder_ending_with_a_space`, `every_reset_mode_saves_the_previous_tip_as_a_backup`, `reset_fails_without_moving_when_the_backup_cannot_be_written` (crates/git/tests/reset.rs); `reset_cli_applies_soft_mixed_and_hard`, `reset_cli_hard_refuses_to_overwrite_untracked_files` (apps/git-good/tests/cli.rs)

### RWR-66 Confirm before Drop and Reset off: Soft and Mixed run at once, Hard always confirms
Fixture: `fx-dirty`, **Preferences › Rebase & recovery › Confirm before Drop and Reset** off
Layer: product
- When the user chooses **Reset current branch to here…** on `c27` (Mixed), and Recovery's **Reset branch here › Soft**
- Then no dialog appears; the branch is reset with that mode; working files are intact; a backup is still saved
- When the user chooses Recovery's **Reset branch here › Hard**
- Then the **Reset here** confirmation with `CHANGES LOST` appears as with the preference on; nothing runs until **Hard reset** is confirmed
Covered by (interaction, partial): `reset_runs_as_chosen_and_hard_always_asks` (apps/git-good/src/main.rs) (Commit menu route, with its `reset` backups and the status line naming them; Recovery's route not checked)

### RWR-67 Confirm before Drop and Reset off: Drop runs without the plan
Fixture: `fx-rebase`, preference off, a middle commit selected
Layer: product
- When the user presses `D`
- Then no dialog opens; the rewrite starts at once (**Rewriting commits…**) and drops the commit; a `refs/gg/backup/rewrite/…` ref is written
Covered by (Not automated)

### RWR-68 Recovery window: Backups | Reflog switch, fits 720×480
Fixture: `fx-backups`, size 720×480
Layer: interaction
- When the user opens **Repository › Recovery…** (or its search Action)
- Then a non-modal window titled *Recovery* with a **Backups <n> | Reflog** switch in its header opens fully inside the viewport with no clipped controls, on Backups the first time and on the last chosen source after that; the History tree stays usable beside it; there are no separate **Backups…** or **Reflog…** commands and no Refresh button
Covered by (interaction, partial): `recovery_window_switches_backups_and_reflog_in_its_header`, `recovery_window_stays_inside_the_narrow_viewport`, `repository_recovery_opens_one_window_on_the_last_source`, `commands_off_the_toolbar_keep_their_menu_home_and_search_entry` (apps/git-good/src/main.rs)

### RWR-69 Backups list and actions
Fixture: `fx-backups`
Layer: product
- Then the backups are listed newest first in 22 px rows under *Saved by Git Good before each rebase, amend, rewrite or reset. Hidden from the refs sidebar.*: SAVED (relative time), BRANCH (pill), BEFORE (rebase, amend, rewrite, reset, restore), TIP, SUBJECT; the selected one is a label grid of TIP (copy button), SAVED (*<date>, <time> · before rebase of <branch>*), BRANCH NOW (tip and *<n> commits differ from the backup*) and REF (copy button); the footer holds **Delete backup…**, **⋯** (Copy commit ID `Ctrl+C`, Copy ref `Ctrl+Shift+C`, Reset branch here ▸ Soft *keep index and files* / Mixed *keep files* / Hard… *discard changes*, Delete backup… `Del`), **Open commit**, **Create branch here…** and the filled **Restore <branch>…** `Enter`; two backups at one commit stay separately selectable
Covered by (backend, interaction, partial): `backups_show_time_branch_and_a_label_grid_without_refresh` (apps/git-good/src/recovery_view.rs), `backups_record_the_rewritten_branch_and_their_save_time` (crates/git/tests/backup.rs) (the ⋯ menu not opened)

### RWR-70 Restore from a backup after a rewrite
Fixture: `fx-rebase` after a rewrite that dropped a commit
Layer: product
- When the user opens **Repository › Recovery…** and on the newest backup presses **Restore main…** (or `Enter`)
- Then the confirmation asks *Move main from <tip> back to <id> <subject>?* with *✓ Files are checked out to match the backup.* and *✓ The current tip <tip> is saved as a new backup, so this can be undone.*, Cancel `Esc` and **Restore branch** `Enter`
- When confirmed
- Then a `restore` backup of the rewritten tip is created first, then `main` is hard reset to the pre-rewrite tip with the dropped commit; the old backup still exists; the status strip reads *Restored main to <id> · the previous tip <tip> is saved as a backup*; refs, the History tree, status and the Backups list refresh
Covered by (backend, interaction): `restore_is_one_confirmed_hard_reset_that_saves_the_tip_first` (apps/git-good/src/recovery_view.rs), `restore_saves_the_current_tip_then_hard_resets_a_clean_branch` (crates/git/tests/backup.rs)

### RWR-71 Create branch here from a backup does not switch
Fixture: `fx-backups`
Layer: product
- When the user chooses **Create branch here…**, types `rescue`, submits
- Then `rescue` points at the backup commit; **Switch after creation** was off; HEAD and worktree are unchanged; an existing name (`main`) is rejected; this does not change the remembered switch default
Covered by (interaction, partial): `reflog_branch_creation_dialog_renders_at_supported_widths` (apps/git-good/src/main.rs) (form only)

### RWR-72 Open commit returns to the same Recovery source and entry
Fixture: `fx-backups`
Layer: interaction
- When the user selects the 40-day backup, clicks **Open commit**, then Back
- Then the commit review opens; Back returns to Recovery › Backups with that entry selected (same for Reflog with a non-HEAD reference filter)
Covered by (interaction): `recovery_return_location_reopens_the_same_source`, `recovery_return_location_restores_the_reflog_reference_and_entry` (apps/git-good/src/main.rs)

### RWR-73 Reflog source lists entries in plain language
Fixture: `fx-backups`
Layer: product
- When the user switches to **Reflog**
- Then the header adds **Reference** `HEAD` (24 px menu rows: ✓ HEAD, then local branches with `current` at the right); rows show WHEN, ENTRY `@{n}`, WHAT HAPPENED in plain language (*Reset to …*, *Amended “…”*, *Finished rebase of …*, *Switched from main to …*) and COMMIT; the selected entry shows COMMIT (8-character ID, subject, copy), WHEN (date and relative), ENTRY `HEAD@{n}` (copy) and GIT MESSAGE (the raw `rebase (pick): …`); the footer holds **⋯**, **Open commit**, **Create branch here…** and **Restore <branch> here…** `Enter`, with no **Delete backup…**; the status strip reads *<n> entries · HEAD*
Covered by (backend, interaction): `reflog_reads_in_plain_language_with_the_raw_message_below`, `reflog_events_read_in_plain_language` (apps/git-good/src/recovery_view.rs); `reads_head_and_branch_reflogs_in_order` (crates/git/tests/reflog.rs)

### RWR-74 Reflog restore of a pre-amend commit
Fixture: `fx-backups`
Layer: product
- When the user selects the entry before `commit (amend): …` and chooses **⋯ › Reset branch here › Mixed**, confirms **Reset**
- Then the branch points at the pre-amend commit; working files unchanged; the Reflog list refreshes with a new *Reset to …* entry
Covered by (Not automated)

### RWR-75 Missing reflog reference shows an error, not stale entries
Fixture: `fx-backups`, branch `old` deleted while Recovery shows its reflog
Layer: product
- Then the Reflog source shows no stale entries and the status strip names Git's error; an existing reference with no entries reads *No reflog entries for <ref>.*; `git good --cli reflog nosuch` fails likewise
Covered by (backend, interaction): `reflog_reads_in_plain_language_with_the_raw_message_below` (apps/git-good/src/recovery_view.rs); `rejects_missing_and_option_shaped_reflogs` (crates/git/tests/reflog.rs)

### RWR-76 Keep private backups prunes old backups on open
Fixture: `fx-backups`, **Keep private backups** = 30 days
Layer: product
- When the repository opens
- Then in the background the 40- and 120-day backups are deleted, the 2-day one stays; no other ref is touched; with 90 days only the 120-day one goes; with **Forever** (default) none go
Covered by (backend, partial): `prune_deletes_only_gg_backups_older_than_the_age` (crates/git/tests/backup.rs); GUI path not automated

### RWR-77 Only Delete backup… deletes a backup; Restore needs a clean tree
Fixture: `fx-backups`
Layer: product
- When the user runs every Recovery action except Hard reset and Delete backup…
- Then all `refs/gg/backup/*` refs remain and no untracked file is removed
- Given a tracked file is edited, when **Restore <branch>…** is chosen
- Then the confirmation's footer reads *1 file has uncommitted changes. Commit or stash it first.* with **Restore branch** disabled, and `Enter` does nothing; `Esc` cancels
- When **Delete backup…** (or `Del`) is confirmed
- Then only that backup ref is removed, HEAD is unchanged, and an empty list reads *No rewrite backups yet. Git Good saves one before every rebase, amend, rewrite or reset.*
Covered by (backend, interaction, partial): `a_dirty_tree_keeps_restore_disabled_and_says_why`, `delete_backup_confirms_then_removes_only_that_ref` (apps/git-good/src/recovery_view.rs), `delete_removes_one_backup_and_refuses_other_references` (crates/git/tests/backup.rs) (the full action sweep not automated)

### RWR-79 Recovery refreshes itself
Fixture: `fx-backups`
Layer: interaction
- Given Recovery is open
- When a terminal creates a backup or commits, then Git Good gets focus back; or an amend, rebase, restore or reset finishes in Git Good
- Then the open source re-reads in the background without a Refresh button, keeping its row heights and the selected entry; a return location selects its entry once the list arrives
Covered by (interaction, partial): `recovery_return_location_restores_the_reflog_reference_and_entry` (apps/git-good/src/main.rs), `restore_is_one_confirmed_hard_reset_that_saves_the_tip_first` (apps/git-good/src/recovery_view.rs) (after a restore; focus not simulated)

### RWR-78 Reset confirmation revalidates on confirm
Fixture: `fx-dirty`, `c27` selected
Layer: product
- Given **Reset here** is open on Hard listing `CHANGES LOST`
- When a terminal commits the staged changes (HEAD moves) and the user confirms **Hard reset**
- Then nothing is reset; both lists refresh (`LEAVE THE BRANCH` gains the new commit, `CHANGES LOST` loses the committed paths) and the notice reads `The repository changed; review the updated list and confirm again.`; a second confirm resets
Covered by (interaction, partial): `reset_confirmation_rereads_and_asks_again_after_an_outside_change` (apps/git-good/src/main.rs), `guard_confirms_only_what_the_dialog_showed` (apps/git-good/src/consequence.rs) (checked on Mixed with `LEAVE THE BRANCH`; Hard's `CHANGES LOST` refresh not checked)

Sources: docs/architecture.md (Preferences, Worktrees, Submodules, Git LFS, Signing, Deferred large-repository features, Repository and decision dialogs, Launch, the CLI subsections, Clone in Network operations, empty states), docs/cli.md, docs/final-design.md (Preferences, Command placement). Window 1440×900 unless stated. "Settings file" = `$XDG_CONFIG_HOME/gg/settings.json`.

## Preferences

### PRF-01 Open Preferences from the menu, shortcut and search
Fixture: `fx-linear`
Layer: interaction
- Given Git Good is open on the History tree
- When the user chooses **Repository › Preferences…**, then closes it and presses `Ctrl+,` (`Cmd+,` on macOS), then closes it and types `Preferences` in search (`Ctrl+K`) and runs the action
- Then each route opens the same window: title row with **Search preferences** and ×, sections listed on the left (**General**, **Appearance**, **Diff**, **Commit**, **Rebase & recovery**, **Keyboard**), the selected section on the right, footer naming the settings file path and **Close**
Covered by (interaction, partial): `preferences_list_sections_folders_and_save_changes_at_once` (apps/git-good/src/main.rs) (menu/search routes not automated)

### PRF-02 Window is modal and fixed size; Esc, ×, Close and click-outside close it
Fixture: `fx-linear`
Layer: interaction
- Given Preferences is open
- When the user clicks a History tree row behind it
- Then the click closes Preferences and does not select the row; the window cannot be resized
- When reopened and the user presses `Esc` / clicks × / clicks **Close**
- Then each closes the window
Covered by (Not automated)

### PRF-03 Entry points open on General
Fixture: `fx-linear`
Layer: interaction
- Given Git Good is open
- When the user chooses **Repository › Development base folders…**, or opens **Repository › Clone…** and clicks **Folders…** / **Manage clone folders…**
- Then Preferences opens with **General** selected and the **Clone folders** list visible
Covered by (Not automated)

### PRF-04 Search hides non-matching sections and selects the first match
Fixture: `fx-linear`
Layer: interaction
- Given Preferences is open on General
- When the user types `whitespace` in **Search preferences**
- Then only **Diff** stays listed (other section names are hidden but their slots keep their positions) and **Diff** is selected
- When the field is cleared
- Then all six sections are listed again
Covered by (interaction): `preferences_search_keeps_sections_with_a_match` (apps/git-good/src/main.rs)

### PRF-05 Search with no match
Fixture: `fx-linear`
Layer: interaction
- Given Preferences is open
- When the user types `zzzz` in **Search preferences**
- Then no section is listed, the window keeps its size and nothing is saved
Covered by (Not automated)

### PRF-06 Changes apply at once and save immediately
Fixture: `fx-linear` (empty `XDG_CONFIG_HOME`)
Layer: product
- Given no settings file exists
- When the user sets **Appearance › Theme** to **Light**
- Then the whole window turns light in the same frame, without a Save button, and the settings file now exists with `"theme": "light"`
Covered by (interaction): `preferences_list_sections_folders_and_save_changes_at_once` (apps/git-good/src/main.rs)

### PRF-07 Save failure is shown in the window
Fixture: `fx-linear` with `XDG_CONFIG_HOME/gg` made read-only (`chmod 555`)
Layer: product
- Given Preferences is open
- When the user toggles **Density**
- Then the change still applies to the running window and an error naming the settings file is shown inside Preferences; after restart the old value is back
Covered by (Not automated)

### PRF-08 Theme: System, Dark, Light; default Dark; GG_THEME override does not rewrite
Fixture: `fx-linear`; second run `fx-prefs-custom`
Layer: visual
- Given default preferences
- Then **Theme** shows **Dark** selected and the UI is dark
- When choosing **System**
- Then the UI follows the OS theme
- Given `fx-prefs-custom` (light) and a debug build launched with `GG_THEME=dark`
- Then the UI is dark, and the settings file still contains `"theme": "light"` after quitting
Covered by (Not automated)

### PRF-09 Density: Compact 22 px default, Comfortable 28 px — see FRM-68

### PRF-10 Interface zoom and Code font size
Fixture: `fx-linear`
Layer: visual
- Given a commit open in Review
- When **Interface zoom** is set to **125 %**, then **Code font size** to **14 px**
- Then the whole UI scales by 1.25 (egui zoom factor); then only the reader's code text grows to 14 px. Both persist across restart (`zoom`, `code_font` keys)
Covered by (Not automated)

### PRF-11 Show avatars off by default; initials kept in inspector slot — see HIS-26

### PRF-12 Network avatar sources and their order/defaults
Fixture: `fx-linear`, launched without `GG_OFFLINE`
Layer: product
- Given Show avatars is on
- Then **Load avatars from the network** lists sources **GitHub** (on), **GitLab** (off), **Gravatar** (on), **Libravatar** (off), in that order
- When GitLab is turned on
- Then a cached miss for an author is retried (cache entry names the sources) and the first image found in source order is used
Covered by (interaction, partial): `lookups_follow_the_enabled_sources_in_order` (apps/git-good/src/avatars.rs); UI not automated

### PRF-13 GG_OFFLINE disables network avatars without changing the saved value
Fixture: `fx-prefs-custom` (avatars on, `network_avatars` true), launched with `GG_OFFLINE=1`
Layer: product
- Given Preferences › Appearance › History tree is open
- Then **Load avatars from the network** is checked but disabled, with tooltip `GG_OFFLINE is set for this launch.`; no avatar network request is made
- When Git Good is quit and relaunched without `GG_OFFLINE`
- Then the option is enabled and still on; the settings file still has `"network_avatars": true`
Covered by (Not automated)

### PRF-14 Load avatars from the network off uses cache and initials
Fixture: `fx-linear` with an avatar cache entry for ada@example.com, no `GG_OFFLINE`
Layer: product
- When **Load avatars from the network** is turned off
- Then Ada's cached avatar still shows; Grace (not cached) shows initials; no network request is made
Covered by (Not automated)

### PRF-15 Show tags in rows and Relative dates — see HIS-25, HIS-29

### PRF-16 Diff: Default layout, Context lines, Whitespace apply to every diff
Fixture: `fx-dirty`
Layer: product
- Given `ws.txt` open in Local Changes (Unstaged)
- When **Diff › Whitespace** is set to **Ignore at line end**
- Then `ws.txt` shows no changed lines; with **Show all changes** it shows them again
- When **Context lines** is **10** and **Default layout** is **Side by side**
- Then every open reader (Local Changes and Review) shows 10 context lines and side-by-side columns; `diff_context`, `diff_whitespace`, `diff_layout` are saved
Covered by (interaction, partial): `preferences_show_diff_and_commit_sections` (apps/git-good/src/main.rs)

### PRF-17 Diff › Display: Wrap long lines and Show invisible characters — see REV-30, REV-28

### PRF-18 Open files in and Compare images set reader starting values — see REV-38, REV-41

### PRF-19 Commit: Run commit hooks and Subject length guide — see CHG-52, CHG-41

### PRF-20 Commit: Sign commits starting value
Fixture: `fx-signed` with `commit.gpgsign false`
Layer: product
- When **Sign commits** is **follow `commit.gpgSign`** (default)
- Then Local Changes opens with **Sign** unchecked
- When set to **always**
- Then Local Changes opens with **Sign** checked; when **never**, unchecked even with `commit.gpgsign true`; the checkbox can still be changed per commit
Covered by (interaction): `the_sign_checkbox_starts_from_the_commit_preference` (apps/git-good/src/status_view.rs)

### PRF-21 Rebase & recovery settings exposed and persisted
Fixture: `fx-rebase`
Layer: product
- Then the section shows **Stash working changes first**, **Pull strategy** (Rebase · Merge · Fast-forward only), **Confirm before Drop and Reset** (on), **Keep private backups** (Forever default; 30 days, 90 days), **Offer force-with-lease after a rewrite**, and a note on private backup refs and Recovery: *Every rewrite and reset creates a private backup ref first.*
- When each is changed and Git Good restarted
- Then each value is restored (`rebase_autostash`, `pull_strategy`, `confirm_destructive`, `backup_days`, `offer_force_lease`)
Covered by (interaction, partial): `reset_runs_as_chosen_and_hard_always_asks` (apps/git-good/src/main.rs) (behavior owned by the rewrite section)

### PRF-22 Keep private backups deletes only older backups on open — see RWR-76

### PRF-23 Keyboard section lists every menu shortcut
Fixture: `fx-linear`
Layer: interaction
- When **Keyboard** is selected
- Then it lists every command-table shortcut (e.g. Open… `Ctrl+O`, Preferences… `Ctrl+,`, Stage all `Ctrl+Shift+S`) plus list and reader keys, the shortcut rules, and **Show keyboard shortcuts…**, which opens the shortcuts window
Covered by (Not automated)

### PRF-24 General: Editor command — see INT-33

### PRF-25 General: Name the clone after the repository — see NET-04, NET-06

### PRF-26 Removed: Reopen the last repository when launched outside one — see FRM-05

### PRF-27 General: Fetch prune — see NET-14

### PRF-28 Clone folders list: add, count, missing
Fixture: `fx-linear`; folder `/tmp/fxroot` containing `fx-linear` and `fx-branchy` repositories and a plain folder
Layer: interaction
- When `/tmp/fxroot` is added in **Clone folders**
- Then a row shows the path (full path on hover) and `2` repositories; adding `/tmp/does-not-exist` shows the row with **missing**; the dialog does not move
Covered by (interaction): `counts_repositories_and_reports_missing_folders` (apps/git-good/src/folders.rs), `adding_a_clone_folder_does_not_move_the_dialog` (apps/git-good/src/main.rs)

### PRF-29 Clone folders validation messages
Fixture: `fx-prefs-custom`
Layer: interaction
- When the user submits an empty field / `relative/dir` / an already-listed folder
- Then the field shows `Folder is empty.` / `Folder must be an absolute path.` / `Already in the list.` and the list is unchanged; Enter and the add button behave identically
- When `~/repos` is submitted
- Then it is added as `<home>/repos`
Covered by (interaction, partial): `adds_deduplicates_removes_and_round_trips` (apps/git-good/src/folders.rs); messages not automated

### PRF-30 Preferred folder radio, reorder, remove
Fixture: `fx-prefs-custom` (folders A, B; B preferred)
Layer: product
- When **move up** is used on B
- Then order is B, A and is saved under `clone_folders`
- When B (preferred) is **Remove**d
- Then A becomes preferred; Clone's **Into folder** now checks A
Covered by (interaction): `the_preferred_clone_folder_is_marked_and_kept_across_launches` (apps/git-good/src/main.rs), `defaults_to_the_marked_then_first_saved_folder_and_lists_the_launch_folder_last` (apps/git-good/src/folders.rs)

### PRF-31 Legacy roots.json migration
Fixture: empty config with `gg/roots.json` listing two folders and no `clone_folders` key
Layer: product
- When Git Good launches and Preferences › General is opened
- Then both folders appear; after an edit, `settings.json` has `clone_folders`; other keys are kept
Covered by (interaction): `migrates_roots_json_and_keeps_other_settings` (apps/git-good/src/folders.rs)

### PRF-32 Dialog defaults are remembered on submission only
Fixture: `fx-dirty`
Layer: interaction
- Given **General › Dialog defaults** shows stash include untracked on, keep staged off, merge always create a merge commit off, new branch switch after creation on
- When the Stash dialog is opened, **Include untracked files** unchecked and **Cancel** pressed
- Then the default is unchanged
- When it is unchecked and the stash submitted
- Then the next Stash dialog starts unchecked and Dialog defaults shows it off (`stash_include_untracked: false`); same for merge `merge_no_ff`, new branch `branch_switch`
- A branch created from Recovery starts with switching off and does not change `branch_switch`
Covered by (interaction): `dialogs_open_with_remembered_choices_and_save_them_on_submission` (apps/git-good/src/main.rs)

### PRF-33 Commit details pane visibility remembered — see HIS-38

### PRF-34 Custom settings load on launch
Fixture: `fx-prefs-custom`
Layer: product
- When Git Good launches
- Then the theme is light, rows are 28 px, diffs open side by side, the History tree shows avatar slots, and Clone lists the two folders
Covered by (interaction, partial): `round_trips_and_keeps_unknown_keys` (apps/git-good/src/settings.rs); UI not automated

### PRF-35 Malformed value resets only that key
Fixture: `fx-prefs-malformed` (e.g. `"density": 42`, other keys valid)
Layer: product
- When Git Good launches
- Then density is the default Compact, every other saved preference applies, and no error dialog blocks launch
Covered by (interaction): `a_malformed_value_resets_only_that_preference` (apps/git-good/src/settings.rs)

### PRF-36 Unknown keys from a newer Git Good are preserved on write
Fixture: `fx-prefs-malformed` (contains `"future_option": …`)
Layer: product
- When Theme is changed
- Then the settings file still contains `future_option` with its original value; only `theme` changed
Covered by (interaction): `round_trips_and_keeps_unknown_keys` (apps/git-good/src/settings.rs)

### PRF-37 Malformed JSON file
Fixture: settings file containing `{not json`
Layer: product
- When Git Good launches and Clone folders is opened
- Then defaults apply and Clone folders is an empty list; launch is not blocked
Covered by (interaction): `reports_malformed_configuration_as_an_empty_list` (apps/git-good/src/folders.rs)
- And the first save keeps the unreadable file as `settings.json.unreadable` beside the new one, and leaves no temporary file
Covered by (interaction): `an_unreadable_file_is_set_aside_not_overwritten` (apps/git-good/src/settings.rs)

### PRF-39 Keys can be changed
Fixture: default preferences
Layer: interaction
- When the user opens Preferences › Keyboard, clicks Fetch's `Ctrl+Shift+F` and presses `Ctrl+Shift+G`
- Then Fetch shows `Ctrl+Shift+G` with **Reset**, the Repository menu labels it so, and the settings file keeps `"keys": {"fetch": "Ctrl+Shift+G"}`
- And Esc while it waits cancels without closing Preferences; `Ctrl+Shift+P` is refused with *Ctrl+Shift+P is already Repository › Push.*; `Ctrl+K` with *Ctrl+K is kept for Search.*; `J` alone as a list key
- And with Keyboard shortcuts on `F9`, `F9` opens the window and `?` no longer does
Covered by (interaction): `a_key_changed_in_preferences_labels_the_menu_and_runs_the_command` (apps/git-good/src/main.rs); `chords_read_and_write_as_the_menus_show_them`, `a_key_is_refused_when_fixed_taken_or_a_bare_list_key`, `a_press_runs_only_the_chord_with_exactly_its_modifiers` (apps/git-good/src/keymap.rs)

### PRF-38 Two windows keep each other's preferences
Fixture: `fx-linear` opened in two Git Good windows on one `XDG_CONFIG_HOME`
Layer: product
- When the first window sets Theme to Light and the second, opened before that, sets Density to Comfortable
- Then the settings file holds both, and the next launch shows Light and Comfortable
- And a repository opened in each window is kept in Recent, newest first; a repository removed from Recent in one window stays removed
Covered by (interaction): `two_windows_keep_each_others_preferences`, `two_windows_merge_their_recent_repositories` (apps/git-good/src/settings.rs)

## Worktrees, submodules and LFS

### WTR-01 Worktrees window lists the inventory as a table
Fixture: `fx-worktrees`
Layer: product
- When **Repository › Worktrees…** is chosen
- Then a non-modal window headed **Worktrees** with the count shows a 22 px table **BRANCH**, **PATH**, **STATUS**, **HEAD**: main (`main · main worktree`, blue *Current*), `wt-login` (`feature/login`, green *Clean*), `wt-locked` (grey *Locked*), and last `wt-gone` (red *Missing*); each PATH ends with the worktree's own folder (`…/wt-login`); the label grid under it shows the selected PATH with a copy button, HEAD `id · subject · author · date` and STATE; the footer has **⋯**, **New worktree…**, **Remove…** `Del` and the primary **Open** `Enter`; no Refresh button; empty (no loading text) during the first read
Covered by (backend, interaction): `worktree_window_lists_state_and_fits_the_narrow_viewport`, `worktrees_window_prunes_missing_and_locks_with_a_reason` (apps/git-good/src/main.rs), `lists_main_and_linked_worktrees_with_branch_state` (crates/git/tests/worktree.rs)

### WTR-02 Window fits the narrow viewport
Fixture: `fx-worktrees`, window 720×480
Layer: interaction
- When Worktrees… opens
- Then all columns, the label grid and the footer buttons are visible without clipping
Covered by (interaction): `worktree_window_lists_state_and_fits_the_narrow_viewport` (apps/git-good/src/main.rs)

### WTR-03 Dirty status shown per worktree
Fixture: `fx-worktrees` with an unstaged edit in `wt-login`
Layer: product
- Then the `wt-login` row STATUS shows it dirty; others clean
Covered by (backend, partial): `remove_blocks_main_current_and_dirty_worktrees_then_removes_a_clean_one` (crates/git/tests/worktree.rs); UI not automated

### WTR-04 Re-read on regaining focus
Fixture: `fx-worktrees`
Layer: interaction
- Given Worktrees… is open
- When outside Git Good `git worktree add ../wt-new -b new` runs and the Git Good window regains focus
- Then `wt-new` appears without any click
Covered by (interaction): `the_worktrees_window_rereads_when_gg_regains_focus` (apps/git-good/src/main.rs)

### WTR-05 Open a worktree (button and Enter) restores per-worktree state
Fixture: `fx-worktrees`
Layer: product
- Given in main a commit c25 is selected, filter `docs`, and a Local Changes draft `wip msg`
- When `wt-login` is selected and **Open** (or `Enter`) is used
- Then Git Good shows `wt-login` with an empty selection/filter/draft; reopening main restores c25, `docs`, and `wip msg`
Covered by (interaction): `switching_worktree_restores_its_selection_and_filter`, `worktree_state_round_trips_selection_filters_and_place` (apps/git-good/src/main.rs), `commit_draft_does_not_cross_worktrees`

### WTR-06 Open waits while an operation runs
Fixture: `fx-worktrees`
Layer: interaction
- Given a fetch/rebase is running in main
- When Open on `wt-login` is used
- Then the switch is held/disabled with a reason until the operation finishes
Covered by (interaction): `worktree_switch_waits_for_an_active_operation` (apps/git-good/src/main.rs)

### WTR-07 Create worktree: new branch, default folder follows branch
Fixture: `fx-branchy` at `/tmp/x/fx-branchy`
Layer: product
- When **New worktree…**, new branch `feat/x` from `main`
- Then the folder defaults to `/tmp/x/fx-branchy-feat-x` and follows further branch edits until the folder is edited by hand; **Open after create** is on
- When submitted
- Then progress shows, the row appears only after Git succeeds, and Git Good switches to the new worktree
Covered by (interaction): `worktree_folders_default_to_a_sibling_named_after_the_branch` (apps/git-good/src/main.rs)

### WTR-08 Create worktree with existing branch; Open after create off
Fixture: `fx-branchy`
Layer: product
- When existing branch `old` is chosen, **Open after create** unchecked, submitted
- Then the worktree is listed and Git Good stays in the current worktree
Covered by (backend, partial): `add_existing_branch_and_list_current_worktree` (crates/git/tests/worktree.rs); GUI path not automated

### WTR-09 Create rejects a branch checked out elsewhere
Fixture: `fx-worktrees`
Layer: product
- When New worktree… with existing `feature/login`
- Then the dialog stays open with an error naming `wt-login`'s path; no force option
Covered by (backend, partial): `add_rejects_checked_out_branches_and_nonempty_destinations` (crates/git/tests/worktree.rs); GUI path not automated

### WTR-10 Create rejects a non-empty destination
Fixture: `fx-branchy`; destination folder containing a file
Layer: product
- Then the dialog stays open with the error, fields kept, the folder untouched
Covered by (backend, partial): `add_rejects_checked_out_branches_and_nonempty_destinations` (crates/git/tests/worktree.rs); GUI path not automated

### WTR-11 Remove a clean worktree
Fixture: `fx-worktrees` (clean `wt-login`)
Layer: product
- When `wt-login` selected and `Del` (or **Remove…**)
- Then **Remove worktree** confirmation names the exact path and branch, says the folder is deleted and the branch stays, filled red primary
- When confirmed
- Then `../wt-login` is gone, `feature/login` still exists, the list re-reads
Covered by (backend, partial): `remove_blocks_main_current_and_dirty_worktrees_then_removes_a_clean_one` (crates/git/tests/worktree.rs); GUI path not automated

### WTR-12 Remove blocked for main, current, dirty, locked, missing
Fixture: `fx-worktrees` with `wt-login` dirty
Layer: product
- Then with main, current, dirty `wt-login`, locked `wt-locked`, or missing `wt-gone` selected, **Remove…** is disabled with its reason on hover and `Del` does nothing; no force or delete-files route
- Given `wt-login` becomes dirty after the confirmation opened, confirming re-checks and refuses
Covered by (backend, partial): `remove_blocks_main_current_and_dirty_worktrees_then_removes_a_clean_one`, `nested_current_worktree_cannot_be_removed` (crates/git/tests/worktree.rs); GUI path not automated

### WTR-13 Prune a missing worktree
Fixture: `fx-worktrees`
Layer: product
- When `wt-gone` is selected, whose primary reads **Prune 1 missing** (also **⋯ › Prune 1 missing**)
- Then `git worktree prune` runs, the result is reported in the footer, the row disappears, its branch remains
Covered by (backend, interaction): `worktrees_window_prunes_missing_and_locks_with_a_reason` (apps/git-good/src/main.rs), `lock_unlock_and_prune_keep_branches` (crates/git/tests/worktree.rs)

### WTR-14 Lock and unlock a worktree
Fixture: `fx-worktrees`
Layer: product
- When **⋯ › Lock…** on `wt-login` with reason `on a USB disk`, then **Lock**
- Then the row shows *Locked*, STATE reads *locked: on a USB disk*, the result is reported, and Remove… is disabled
- When **⋯ › Unlock** on `wt-locked`
- Then the lock reason disappears, the result is reported and Remove… becomes available
Covered by (backend, interaction): `worktrees_window_prunes_missing_and_locks_with_a_reason` (apps/git-good/src/main.rs), `lock_unlock_and_prune_keep_branches` (crates/git/tests/worktree.rs)

### WTR-15 Copy path and show in file manager
Fixture: `fx-worktrees`
Layer: product
- When `wt-login` is selected and **⋯ › Copy path**, then **⋯ › Show in file manager**
- Then the clipboard holds the exact path, and the system file manager opens the `wt-login` folder itself
Covered by (Not automated)

### WTR-16 The whole row takes the pointer
Fixture: `fx-worktrees`
Layer: interaction
- When the pointer rests on, then clicks, a row's branch text, its path text (which has a tooltip), or its status pill
- Then the row lights under the pointer and the click selects it, as over the gaps between cells; the same holds for every 22 px inspect table (Recovery, Submodules, LFS)
Covered by (interaction): `a_click_on_a_worktree_rows_text_selects_the_row` (apps/git-good/src/main.rs)

### SUB-01 Submodule details from the reader
Fixture: `fx-submodule`, commit bumping `vendor/lib` open in Review
Layer: product
- When **File actions › Submodule details…** on `vendor/lib`
- Then the header reads *Submodule vendor/lib* with the status pill (amber *Different commit*) and one sentence for the status; a label grid lists BEFORE, AFTER, INDEX and CHECKED OUT as 8-character IDs with subjects, copy buttons and hints (*same as After*, *same as Before · 1 commit behind After*), then URL and BRANCH, and *Local config uses a different URL: …* when it differs; no 40-character hash is shown; nothing is fetched or checked out
Covered by (backend, interaction): `details_show_a_label_grid_with_short_ids_and_same_as_hints` (apps/git-good/src/submodules_view.rs), `submodule_details_window_renders_recorded_state_and_update` (apps/git-good/src/main.rs), `details_name_recorded_checked_out_and_index_state`, `list_reads_every_gitlink_with_subjects_and_how_far_behind_it_is` (crates/git/tests/submodule.rs)

### SUB-02 Uninitialized submodule
Fixture: `fx-submodule`
Layer: product
- When details for `vendor/uninit`
- Then it reads not initialized, keeps the parent index ID, and browsing does not initialize it
Covered by (backend, partial): `uninitialized_submodule_keeps_the_parent_index_identity` (crates/git/tests/submodule.rs); GUI path not automated

### SUB-03 Update (init) confirmation and run
Fixture: `fx-submodule`
Layer: product
- When **Initialize and update…** on `vendor/uninit`
- Then the confirmation *Update submodule* asks *Initialize and check out <id> in vendor/uninit?*, says *Git may fetch from <url>. The parent index is not changed.*, and has Cancel `Esc` and the filled **Update** `Enter` side by side
- When confirmed
- Then `git --literal-pathspecs submodule update --init -- vendor/uninit` runs on the worker, status rescans to initialized, the footer's left reads *Updated vendor/uninit to <id>.*; the parent index is unchanged
- And a submodule at `s*` is read and updated by that path only: `s!` and `sub` keep their index IDs and checkouts
Covered by (backend, interaction, partial): `a_disabled_update_says_why_at_the_footer_left_and_the_confirm_sits_side_by_side` (apps/git-good/src/submodules_view.rs) (confirmation and Enter); `dirty_and_index_mismatch_block_update_and_update_restores_the_index_commit`, `a_glob_named_submodule_path_is_literal` (crates/git/tests/submodule.rs); the GUI run not automated

### SUB-04 Update blocked: dirty nested tree / recorded commit differs from index / object missing
Fixture: `fx-submodule` with an edit inside `vendor/lib`
Layer: product
- Then the primary reads **Update…**, disabled, and the reason replaces the result at the footer's left: *vendor/lib has local changes. Open it to inspect or clean them.*; a historical commit whose recorded ID differs from the index reads *Stage the parent gitlink before updating: the index records <id>.*; a missing recorded object disables it with its reason
Covered by (backend, interaction, partial): `a_disabled_update_says_why_at_the_footer_left_and_the_confirm_sits_side_by_side` (apps/git-good/src/submodules_view.rs) (dirty); `dirty_and_index_mismatch_block_update_and_update_restores_the_index_commit` (crates/git/tests/submodule.rs)

### SUB-05 Open nested repository
Fixture: `fx-submodule` (vendor/lib initialized)
Layer: product
- When **Open nested repository**
- Then Git Good shows `vendor/lib` as its own repository; returning to the parent restores its selection and draft
Covered by (Not automated)

### SUB-06 Submodules… and LFS… menu items — see FRM-20

### SUB-07 Repository › Submodules… lists every gitlink
Fixture: `fx-submodule`
Layer: interaction
- When **Repository › Submodules…**
- Then a non-modal *Submodules* window with a count pill lists PATH, STATUS, RECORDED and CHECKED OUT in 22 px rows: `vendor/lib` with an amber *Different commit* pill and two 8-character IDs, `vendor/uninit` with a grey *Uninitialized* pill and `—`; pills are amber for differs, red for broken, green for Clean; the footer reads *Recorded = commit staged in the parent index.* with **Open nested repository** and **Details** `Enter`
- When Down then `Enter`
- Then Submodule details opens for `vendor/uninit`; the list has no Refresh and re-reads on open, on focus, and after an update; a repository without gitlinks reads *No submodules in this repository.*
Covered by (backend, interaction): `the_list_has_status_pills_short_ids_and_enter_opens_details` (apps/git-good/src/submodules_view.rs), `list_reads_every_gitlink_with_subjects_and_how_far_behind_it_is` (crates/git/tests/submodule.rs)

### LFS-01 LFS details for an available pointer
Fixture: `fx-lfs`, commit adding `art.psd` in Review
Layer: product
- When **File actions › LFS details…**
- Then the header reads *LFS art.psd* with a green *Object available* pill; AFTER shows the 8-character OID, the declared size, *available*, and a copy button that copies the full sha256; pointer text is never shown as content
Covered by (backend, interaction): `details_shorten_oids_and_name_the_file_and_size_in_the_fetch_question` (apps/git-good/src/lfs_view.rs), `lfs_window_renders_pointer_identity_and_missing_object_state` (apps/git-good/src/main.rs), `pointer_details_separate_oid_size_and_local_availability` (crates/git/tests/lfs.rs)

### LFS-02 Missing object state and Fetch object…
Fixture: `fx-lfs`
Layer: product
- When LFS details for `old.psd`
- Then a red *Object missing* pill, AFTER *missing*, GIT LFS *available · remote origin*, *The new version’s content is not in the local LFS store, so the diff shows only the pointer.*, and *Fetching does not change working files.* beside the filled **Fetch object…**
- When **Fetch object…**
- Then the confirmation asks *Download the new version of old.psd?* with *<size> from origin. Working files are not changed.* (no OID) and Cancel `Esc` / **Fetch** `Enter` side by side
- When confirmed
- Then `git lfs fetch … --include=old.psd` runs, availability rescans to present, the footer's left reads *Fetched <id> (<size>).*; `old.psd` in the worktree is still pointer text
Covered by (interaction, partial): `details_shorten_oids_and_name_the_file_and_size_in_the_fetch_question` (apps/git-good/src/lfs_view.rs); fetch not automated

### LFS-03 Fetch disabled without git-lfs or when present
Fixture: `fx-lfs` with `git-lfs` removed from `PATH`
Layer: product
- Then the pill reads amber *LFS unavailable*, GIT LFS *not installed*, and *git-lfs is not installed.* replaces the footer note while **Fetch object…** is disabled; for `art.psd` (present) the footer offers **Export original…** instead
Covered by (interaction, partial): `lfs_window_renders_pointer_identity_and_missing_object_state` (apps/git-good/src/main.rs) (not installed)

### LFS-04 Non-pointer and malformed pointers
Fixture: `fx-lfs` plus `notes.txt` plain text and `bad.psd` with `oid sha256:` of 10 hex digits
Layer: product
- Then `notes.txt` details say it is not a pointer; `bad.psd` reports a malformed pointer state, not LFS content
Covered by (backend, partial): `malformed_pointer_is_reported_without_treating_text_as_lfs`, `before_and_after_pointers_and_plain_text_are_distinguished` (crates/git/tests/lfs.rs), `parses_canonical_pointers_and_rejects_other_text` (crates/engine/src/lfs.rs); GUI path not automated

### LFS-05 Local Changes areas use their own sides; conflicts refused
Fixture: `fx-lfs` with `art.psd` pointer edited and staged, then edited again
Layer: product
- Then Staged details compare HEAD→index, Unstaged index→working file (read ≤4 KiB, no smudge), Untracked absent→working; an unmerged pointer says the conflict must be resolved; a pointer at `a*.bin` is read from its own index entry, never from `a!.bin`
Covered by (backend, interaction): `working_staged_unstaged_and_untracked_pointers_use_their_own_sides`, `plain_working_text_and_conflicts_do_not_masquerade_as_pointers`, `working_pointers_are_read_from_the_literal_path` (crates/git/tests/lfs.rs), `working_pointer_exposes_a_working_lfs_request`

### LFS-06 Bounded content preview of a present object
Fixture: `fx-lfs` with a small PNG tracked as `logo.psd`
Layer: product
- Then details show the image in a 150 px bordered frame labelled *CONTENT PREVIEW · AFTER* with *<w> × <h> PNG* at the right; an object >2 MiB shows an explicit over-budget state; a non-image shows the hex preview in the frame
- When **Export original…** and a new file is chosen
- Then the file holds the complete original bytes and the footer's left reads *Exported <size> to <path>.*
Covered by (backend, partial): `bounded_object_preview_reads_only_an_available_small_object` (crates/git/tests/lfs.rs), `single_content_preview_draws_images_and_hex_within_a_bounded_height` (apps/git-good/src/binary_preview.rs); export not automated

### LFS-07 No smudge/materialize control
Fixture: `fx-lfs`
Layer: product
- Then no control to materialize fetched content into the working file exists (deferred, no placeholder)
Covered by (Not automated)

### LFS-08 Repository › LFS… fetches the missing objects together
Fixture: `fx-lfs`
Layer: product
- When **Repository › LFS…**
- Then *LFS objects* opens on **Missing 1 | All 2** with `old.psd` checked (PATH, 8-character OBJECT, SIZE in 22 px rows) and the footer *1 selected · <size> from origin* with **LFS details** and the filled **Fetch 1 object** `Ctrl+Enter`; **All** adds `art.psd` with its checkbox disabled
- When **Fetch 1 object**
- Then one `git lfs fetch origin --include=old.psd` runs without a further question, the footer reads *Fetched 1 object (<size>).*, and Missing reads *No missing objects.*; unchecking a row lowers the count and size on the button; the list has no Refresh and re-reads on open and on focus
Covered by (backend, interaction, partial): `the_list_shows_missing_objects_checked_and_states_the_batch` (apps/git-good/src/lfs_view.rs), `head_pointers_list_only_lfs_pointers_with_availability` (crates/git/tests/lfs.rs); the fetch itself not automated

## Signing

### SIG-01 Good signature
Fixture: `fx-signed`
Layer: product
- When the good signed commit is selected and **Signature details…** opened from Commit details
- Then the title reads *Signature · commit <8-char id>*; a green banner with an icon reads *Good signature* · *Signed by an allowed signer.*; the label grid lists only reported rows (SIGNER, KEY with a copy button, TRUST, FORMAT `ssh`); no signing configuration is shown; the footer reads *Read-only check; nothing is changed.* with **Signing settings…**, which opens Preferences › Commit
Covered by (interaction): `the_window_lists_only_reported_rows_and_links_the_settings`, `titles_name_the_object_kind`, `banners_take_the_state_colour` (apps/git-good/src/signature_view.rs), `signature_window_shows_a_banner_and_links_the_signing_settings` (apps/git-good/src/main.rs)

### SIG-02 Unknown key is not bad
Fixture: `fx-signed`
Layer: product
- When details for the commit signed by a key not in allowed signers
- Then an amber banner reads *Unknown key* · *Valid signature, but the key is not in allowed signers.* (not bad), with the KEY row
Covered by (interaction, partial): `banners_take_the_state_colour` (apps/git-good/src/signature_view.rs) (banner only)

### SIG-03 Unsigned is explicit
Fixture: `fx-signed`
Layer: product
- When details for the unsigned commit
- Then a grey banner reads *Unsigned* · *This commit has no signature.*, not an empty success
Covered by (backend, interaction, partial): `unsigned_commit_and_tag_report_kind_without_claiming_validity` (crates/git/tests/signature.rs), `banners_take_the_state_colour` (apps/git-good/src/signature_view.rs)

### SIG-04 Bad signature
Fixture: `fx-signed` with a copy of the good commit whose message bytes were altered (rewritten object keeping the gpgsig header)
Layer: product
- Then a red banner reads *Bad signature* · *The signature does not match this commit’s content.*
Covered by (interaction, partial): `banners_take_the_state_colour` (apps/git-good/src/signature_view.rs) (banner only)

### SIG-05 Verification unavailable
Fixture: `fx-signed` with `gpg.program`/`ssh-keygen` absent from `PATH`
Layer: product
- Then a grey banner reads *Verification unavailable* with Git's reason and a **Signing settings…** link; Preferences › Commit shows the configuration and its note
Covered by (backend, partial): `config_separates_signing_key_format_and_tool_readiness` (crates/git/tests/signature.rs), `banners_take_the_state_colour` (apps/git-good/src/signature_view.rs)

### SIG-06 Tag signature from the ref menu
Fixture: `fx-signed`
Layer: product
- When the signed annotated tag's ref menu **Signature details…**
- Then `git verify-tag --raw` result shows good; on lightweight `v0.1.0` (fx-linear) the error names a non-tag/commit target correctly
Covered by (backend, partial): `non_commit_objects_and_invalid_references_are_rejected` (crates/git/tests/signature.rs)

### SIG-07 Verification is read-only
Fixture: `fx-signed`
Layer: product
- After opening every Signature details window
- Then `.git/config`, the allowed signers file, and keyrings are byte-identical
Covered by (Not automated)

### SIG-08 Sign checkbox readiness in Local Changes — see CHG-62

### SIG-09 Signed commit success and failure without fallback
Fixture: `fx-signed` with a staged change
Layer: product
- When committing with Sign on
- Then the status line names a signed commit; Signature details reads good
- Given the key file is removed, committing aborts: HEAD, index, message unchanged; no unsigned commit
Covered by (backend, interaction): `signing_readiness_blocks_the_commit_without_unsigned_fallback` (apps/git-good/src/status_view.rs), `signed_commit_without_a_working_signer_never_falls_back_to_unsigned` (crates/git/tests/commit.rs)

### SIG-10 New tag dialog signing — see HIS-71

### SIG-11 Signing configuration in Preferences › Commit
Fixture: `fx-signed`
Layer: interaction
- When **Repository › Preferences… › Commit** (or **Signing settings…** in Signature details)
- Then under the signing choices read-only rows show Format `ssh · gpg.format`, Signing key `<path> · user.signingkey`, Allowed signers `<path> · gpg.ssh.allowedSignersFile`, then *Read from Git config.* with **Edit in repository config…**, which opens the repository's config in the configured editor; the rows are read in the background and keep their place while Git runs
Covered by (interaction, partial): `preferences_commit_lists_the_signing_configuration_read_only` (apps/git-good/src/main.rs) (rows and stable height; the editor launch not checked)

### SIG-12 GPG signing and verification
Fixture: `fx-gpg`
Layer: product
- Given a GPG key as `user.signingkey` (Pro Git 7.4, *Signing Your Work*, which uses GPG)
- When Git Good signs a commit (**Sign** in Local Changes, `git good --cli commit --sign`) and an annotated tag (New tag › **Sign**, `tag --create --sign`)
- Then **Signature details…** for each reads *Good signature* with FORMAT `openpgp`, SIGNER `Ada Lovelace <ada@example.com>` and KEY the key's fingerprint; the previous commit reads *Unsigned*
Covered by (backend): `gpg_signed_commits_and_tags_verify_as_good` (crates/git/tests/signature.rs; reports a skip where `gpg` is not installed)

## Repository dialogs

### DLG-01 Welcome view without a repository — see FRM-03

### DLG-02 Open dialog layout and filtering
Fixture: `fx-linear`, recent list of 10
Layer: interaction
- When `Ctrl+O`
- Then a modal shows a folder field with **Browse…**, up to ten recent repositories (the limit shared with the welcome view, FRM-04), **Clone…**, **Cancel** (`Esc`), **Open** (`Enter`)
- When `bran` is typed
- Then only `fx-branchy` stays listed (name match first, path dimmed)
Covered by (interaction, partial): `one_recent_limit_for_the_welcome_list_open_recent_and_open` (apps/git-good/src/main.rs) — the limit; filtering Not automated

### DLG-03 Open a valid path and invalid path
Fixture: `fx-linear`; target `fx-branchy` and `fx-not-repo`
Layer: product
- When `fx-branchy`'s path is typed and Enter pressed
- Then it opens, the old filter/selection is cleared, and it moves to the top of Open recent without duplicates
- When `fx-not-repo` is opened next
- Then an error names the folder, and the displayed repository stays `fx-branchy` (the one open since the successful switch), with its title, rows and selection
Covered by (interaction): `recent_repositories_move_to_the_front_without_duplicates` (apps/git-good/src/settings.rs), `failed_switch_keeps_displayed_identity_and_retry_adopts_new_repository`

### DLG-04 Browse… seeds from an existing folder
Fixture: `fx-linear`
Layer: interaction
- When **Browse…** with the field containing a nonexistent path
- Then the native folder dialog opens at the nearest existing folder
Covered by (interaction): `only_existing_folders_seed_the_dialog` (apps/git-good/src/folder_picker.rs), `every_folder_field_offers_the_native_picker` (apps/git-good/src/main.rs)

### DLG-05 Open's Clone… switches dialogs
Fixture: `fx-linear`
Layer: interaction
- When **Clone…** in Open
- Then the Open modal closes and the Clone dialog opens
Covered by (Not automated)

### DLG-06 Open recent menu
Fixture: `fx-linear` open; recent `[fx-linear, fx-branchy, deleted-repo]`
Layer: interaction
- Then **Repository › Open recent** lists up to ten entries, folder name then path; `fx-linear` has a checkmark; `deleted-repo` is disabled with *Not found*; the list ends with **Clear recent**
- When **Clear recent**
- Then the submenu is empty (except Clear recent) and `recent_repositories` is empty after restart
- And `recent_repositories` stores `{"path", "opened"}` entries; a settings file with plain path strings from older versions still loads
Covered by (interaction, partial): `one_recent_limit_for_the_welcome_list_open_recent_and_open` (apps/git-good/src/main.rs), `recent_times_round_trip_and_old_path_lists_still_load` (apps/git-good/src/settings.rs); checkmark and Clear recent Not automated

### DLG-07 Clone dialog Into folder and target line — see NET-03

### DLG-08 Clone URL seeded only from recognized clipboard URLs — see NET-01, NET-02

### DLG-09 Clone name rejects path components — see NET-07

### DLG-10 Clone success opens the repository; failure keeps state — see NET-11, NET-12

### DLG-11 About Git Good
Fixture: `fx-linear`
Layer: visual
- When **Help › About Git Good**
- Then the dialog frame (title **About Git Good** and × in one row, footer below) shows the logo, version, **Commit** (8 characters), **Built** (`YYYY-MM-DD · <target triple>`), authors (people only), license with link, source link, AI-assistance/no-warranty note; **Star on GitHub** opens `https://github.com/baol/gg` in the browser; **Copy version info** copies six plain lines (`Git Good <version>`, `commit <id>`, `built <date>`, `target <triple>`, `git <version>` or `git unknown` before Git's version is read, `os <system> (<release>) <architecture>`); **Close** or `Esc` closes
Covered by (interaction, partial): `about_names_the_license_and_the_human_authors`, `version_info_is_plain_build_git_and_os_lines` (apps/git-good/src/branding.rs); the rendered frame is checked by native review

### DLG-12 Decision dialogs are modal — see FRM-64

### DLG-13 Welcome recent rows: keys, remove, branch and opened time
Fixture: `fx-not-repo` (cwd); recent `[fx-linear (opened 2 h ago), fx-branchy (yesterday), fx-dirty (3 days ago)]`
Layer: interaction
- Then the first row is selected; rows are 22 px with name 132 px, `~` path (mono, truncating, full path on hover), branch 128 px (`main`, blank until read, nothing shifts when it arrives), opened 76 px (`2 h ago`, `yesterday`, `3 days ago`; older dates as `Oct 12`), × on hover or selection
- When ↓ ↓ ↑ then `Del`
- Then `fx-branchy` leaves the list (its folder is untouched) and the count reads `2`
- When `Enter`
- Then the selected row stays selected while the footer reads `Reading <~path>`, the pane shows no loading text, and the repository opens
Covered by (interaction): `recent_rows_take_arrows_enter_and_delete` (apps/git-good/src/main.rs), `opened_times_read_relative_then_as_dates`, `the_branch_is_read_from_head_for_repositories_and_worktrees` (apps/git-good/src/welcome_view.rs)

### DLG-14 A recent row that fails to open shows its fix in place
Fixture: `fx-not-repo` (cwd); recent `[<moved folder>, <plain folder that was a repository>]`
Layer: product
- When the moved folder's row is opened
- Then the row becomes one amber line in the same 22 px slot: `<~path> no longer exists.` with **Remove from recent** and **Locate…**; **Locate…** picks a folder that replaces the entry and opens it; **Remove from recent** removes the entry
- When the plain folder's row is opened
- Then its slot reads `<~path> is no longer a Git repository.` with **Remove from recent**; the launch line is unchanged
Covered by (interaction): `a_missing_recent_folder_is_marked_and_offers_remove_and_locate`, `recent_rows_take_arrows_enter_and_delete` (apps/git-good/src/main.rs)

### DLG-15 First run and filtering many recent repositories
Fixture: `fx-not-repo` (cwd), an empty `XDG_CONFIG_HOME`; then a recent list of 12
Layer: visual
- Given no recent repositories
- Then the title reads **Welcome to Git Good** with *Open a repository to see its History tree and Local Changes.*; RECENT is absent; the lines *Repositories you open are listed here. Drop a folder on this window to open it.* and `Clone folder: <~folder> · Change in Preferences…` show; the link opens Preferences › General
- Given 12 recent repositories
- When `repo-1` is typed while the list has the keyboard
- Then only matching rows show (every word must match the name or path), the count reads `1 of 10`, and the list keeps its height; `Esc` clears; with six or fewer entries typing does not filter
Covered by (interaction): `first_run_explains_where_repositories_will_appear`, `typing_filters_many_recent_repositories_and_esc_clears` (apps/git-good/src/main.rs), `every_word_must_match_the_name_or_path_and_the_list_is_capped` (apps/git-good/src/welcome_view.rs)

### DLG-16 Drop a folder on the welcome view
Fixture: `fx-not-repo` (cwd); `fx-linear`; an empty folder `<plain>`
Layer: interaction
- When `fx-linear` is dragged over the window
- Then an overlay with a dashed `#2f8fc4` border reads `Drop to open <~fx-linear>` (`Drop to open` when the platform gives no path)
- When it is dropped (or a file inside it)
- Then `fx-linear` opens
- When `<plain>` is dropped
- Then the New repository dialog opens on `<plain>`
Covered by (interaction, partial): `a_dropped_file_stands_for_its_folder` (apps/git-good/src/welcome_view.rs); overlay and drop Not automated

### DLG-17 New repository dialog
Fixture: `fx-not-repo` with 14 files, 3 of them ignored by its `.gitignore`; `fx-linear`
Layer: product
- When **Repository › New repository…** (or the welcome card, or **Initialize here…**)
- Then a modal *New repository* shows **Folder** with **Choose…**, **First branch** `main`, checked **Commit the existing files as “Initial commit”**, and the weak line `<~folder> has 14 files (3 ignored by .gitignore).` (read in the background; `will be created.` for a missing folder), **Cancel** (`Esc`) and the filled **Create repository** (`Enter`)
- When **Create repository**
- Then the repository is created on `main`, the 11 files are committed as `Initial commit`, and it opens in the History tree and joins the recent list; unchecked, it opens in Local Changes with nothing committed
- When the folder is `<fx-linear>/docs`
- Then the line reads `<~fx-linear>/docs is inside the repository <~fx-linear>. Open it instead?` with **Open <~fx-linear>**, and **Create repository** stays disabled, also when the repository's folder name ends with a space
Covered by (backend, interaction): `init_commits_the_existing_files_except_ignored_ones`, `init_refuses_inside_an_existing_repository`, `enclosing_repository_keeps_a_folder_name_ending_with_a_space`, `init_creates_the_folder_on_the_named_branch_without_committing` (crates/git/tests/init.rs), `the_new_repository_dialog_creates_and_opens_the_repository` (apps/git-good/src/main.rs)

### DLG-18 Narrow welcome view
Fixture: `fx-not-repo` (cwd), recent list of 6; window 720×480
Layer: visual
- Then the column fills the width; the cards stay three across with short subtitles (`Ctrl+O`, `into <~folder>`, `git init`); the branch column drops while the opened time stays; the logo and version line hide below 760 px; search is its icon
Covered by (interaction): `the_narrow_welcome_keeps_three_cards_and_drops_logo_version_and_branch` (apps/git-good/src/main.rs)

## CLI

All CLI cases: run from the fixture directory unless stated; stdout must parse as JSON on success; failures print to stderr with a nonzero status and an empty stdout (no partial receipt). Output framing, outcomes and path handling are pinned down in the [CLI contract](#cli-contract) (CLI-C).

### CLI-01 Version
Fixture: `fx-not-repo`
Layer: product
- When `git good --version`
- Then stdout `git-good <Cargo version>`, empty stderr, status 0, no repository needed
Covered by (end-to-end): `version_matches_package_without_a_repository` (apps/git-good/tests/cli.rs)

### CLI-02 Help lists every command
Fixture: `fx-not-repo`
Layer: product
- When `git good --help`
- Then status 0, empty stderr, every command line from docs/cli.md is present
Covered by (end-to-end): `help_lists_every_cli_command` (apps/git-good/tests/cli.rs)

### CLI-03 Git dispatch and nested discovery
Fixture: `fx-linear`, cwd `fx-linear/src`
Layer: product
- When `git good --cli history`
- Then JSON for `fx-linear` (30 commits) is streamed
Covered by (end-to-end): `git_dispatch_discovers_current_and_nested_repositories` (apps/git-good/tests/cli.rs)

### CLI-04 history-tree: explicit path wins; non-repo fails
Fixture: cwd `fx-linear`, path `fx-branchy`; then `fx-not-repo`
Layer: product
- When `git good --cli history --repo ../fx-branchy`
- Then JSON is for fx-branchy
- When `git good --cli history --repo <fx-not-repo>`
- Then status nonzero, stdout empty, stderr contains `Could not read`
Covered by (end-to-end): `explicit_path_overrides_cwd_and_failure_uses_stderr` (apps/git-good/tests/cli.rs)

### CLI-05 history-tree includes all commits, no cap; read-only
Fixture: `fx-dirty`; and a 501-commit repo
Layer: product
- Then every commit is listed with `complete: true`; index, refs and dirty files are byte-identical afterwards
Covered by (end-to-end): `json_includes_every_commit_beyond_the_former_cap`, `browsing_preserves_repository_index_and_dirty_worktree` (apps/git-good/tests/cli.rs)

### CLI-06 Extra arguments are rejected
Fixture: `fx-linear`
Layer: product
- When `git good --cli history extra`
- Then stderr `Error: Usage: git good --cli history [--repo PATH]`, nonzero, empty stdout
- When `git good status` (no such folder), `git good --cli stat`, or `git good -x`
- Then no window opens; stderr reads *No folder status. Did you mean git good --cli status?*, *Unknown command stat.*, or *Unknown option -x.*; nonzero
- When `git good --cli blame -- -x.txt` on a committed `-x.txt`
- Then the attribution of `-x.txt` is printed: after `--` every word is a value
Covered by (end-to-end): `a_failure_prints_the_plain_message`, `commands_follow_cli_and_a_bare_word_is_a_folder`, `double_dash_ends_options` (apps/git-good/tests/cli.rs)

### CLI-07 refs — see HIS-77

### CLI-08 show (summaries, file, export, parent validation) — see REV-71, REV-72, REV-73

### CLI-09 status, stage, unstage — see CHG-70, CHG-71

### CLI-10 commit, amend, hooks, sign — see CHG-70

### CLI-11 reflog and blame
Fixture: `fx-backups`; `fx-file-history`
Layer: product
- `git good --cli reflog` prints entries; `reflog --bogus` / missing ref fails; `blame src/lib.rs --lines 20,30` prints attributed ranges; blame of `blob.bin` / `link` fails with a reason
Covered by (end-to-end): `commit_amend_and_reflog_cli`, `blame_cli_reports_attributed_ranges_and_errors` (apps/git-good/tests/cli.rs)

### CLI-12 switch, branch, tag delete, reset
Fixture: `fx-branchy`; `fx-dirty`
Layer: product
- `switch feature/login` succeeds; in `fx-dirty` switch fails naming dirty files; `branch --create x --at c20` does not switch; `branch --delete old` succeeds, unmerged needs `--force`; `tag --delete v1.0.0` succeeds; `reset --soft|--mixed|--hard` each act as Git
Covered by (end-to-end): `switch_cli_changes_branches_and_reports_dirty_worktrees`, `branch_create_at_cli_does_not_switch`, `branch_and_tag_delete_cli`, `reset_cli_applies_soft_mixed_and_hard` (apps/git-good/tests/cli.rs)

### CLI-13 tag --create with and without signing
Fixture: `fx-linear` (no key)
Layer: product
- `tag --create v9 --at HEAD -m release --no-sign` → receipt `operation: "tag-create"`, `signed: false`; `--sign` → nonzero, stderr `Create tag v9 failed…`, no tag created
Covered by (end-to-end): `tag_create_cli_creates_annotated_tags_and_blocks_broken_signing` (apps/git-good/tests/cli.rs)

### CLI-14 fetch, push, clone — see NET-37, NET-26

### CLI-15 stash subcommands — see STA-23, STA-06, STA-22

### CLI-16 merge, rebase, rewrite, cherry-pick, revert, conflicts — see CLI-16a … CLI-16j

### CLI-16a merge fast-forwards
Fixture: `fx-branchy` plus branch `ahead` one commit past `main`
Layer: product
- When `git good --cli merge ahead`
- Then exit 0 and one receipt `operation: "merge"`, `kind: "fast_forward"`, `applied: true`; `main` points at `ahead`'s tip with no merge commit; `git good --cli merge old` reports already up to date and changes nothing
Covered by (end-to-end, partial): `merge_cli_fast_forwards_conflicts_and_recovers` (apps/git-good/tests/cli.rs) (up-to-date not checked)

### CLI-16b merge --no-ff
Fixture: as CLI-16a
Layer: product
- When `git good --cli merge ahead --no-ff`
- Then a two-parent merge commit is created and reported
Covered by (end-to-end): `merge_cli_no_ff_creates_a_merge_commit` (apps/git-good/tests/cli.rs)

### CLI-16c merge conflict, --abort and --continue
Fixture: `fx-branchy` with `main` and `feature/login` editing the same line
Layer: product
- When `git good --cli merge feature/login`
- Then exit 0 with `applied: false`, `in_progress: true`, `conflicts: [<path>]` (the repository changed: the receipt is the record)
- When `git good --cli merge --abort`
- Then HEAD and the file equal the pre-merge state
- When the merge is repeated, the path resolved and staged, and `git good --cli merge --continue` runs
- Then `applied: true`, `in_progress: false`; `git good --cli merge --skip` fails nonzero with `Unknown merge option` and empty stdout
Covered by (end-to-end): `merge_cli_fast_forwards_conflicts_and_recovers` (apps/git-good/tests/cli.rs)

### CLI-16d rebase records a backup and recovers
Fixture: a branch `feature` diverged from `main`, then a conflicting variant
Layer: product
- When `git good --cli rebase main` with a dirty tracked file
- Then it fails nonzero with `not clean` and nothing changes; with `--autostash` it succeeds: receipt `operation: "rebase"`, `reference`, `commit`, `previous`, `backup` under `refs/gg/backup/rebase/`, `applied: true`, and the dirty edit is restored
- When the conflicting variant runs
- Then exit 0 with `in_progress: true`, `conflicts`, and a `backup`; `--abort` restores the old tip; resolving and `--continue` finishes; `git good --cli reset --hard <backup>` restores the pre-rebase tip
Covered by (end-to-end, partial): `rebase_cli_rewrites_records_backup_and_recovers` (apps/git-good/tests/cli.rs) (restore from the backup ref not checked)

### CLI-16e rewrite: reword, fixup, drop, edit
Fixture: `fx-rebase`
Layer: product
- When `git good --cli rewrite --from <oldest> --pick … --reword <id> "New subject" --fixup <id> --drop <id> --edit <id>` runs (every range commit once, oldest first)
- Then the receipt carries `from`, `previous`, `backup`, `applied`, `in_progress: true`, empty `conflicts`, a warning naming the reopened commit, and the step count; no editor opens
Covered by (end-to-end): `rewrite_cli_rewords_fixes_up_drops_and_edits` (apps/git-good/tests/cli.rs)

### CLI-16f rewrite: reorder, squash, continue and abort
Fixture: `fx-rebase`
Layer: product
- When a reorder plus squash rewrite runs, then `git good --cli rewrite --continue` / `--abort` at a stop
- Then the squash keeps both messages; continue and abort print the receipt shape with null `reference`, `commit`, `previous`, `backup`
Covered by (end-to-end): `rewrite_cli_reorders_squashes_and_recovers` (apps/git-good/tests/cli.rs)

### CLI-16g cherry-pick conflict and recovery
Fixture: `fx-branchy` with a commit on another branch that conflicts with `main`
Layer: product
- When `git good --cli cherry-pick other`
- Then exit 0 with `operation: "cherry-pick"`, `applied: false`, `in_progress: true`, `conflicts: [<path>]`; `--abort` restores HEAD and the file; repeating, resolving, staging and `--continue` creates the commit; `cherry-pick --bad` fails with `Unknown cherry-pick option` and empty stdout
Covered by (end-to-end): `cherry_pick_cli_applies_conflicts_and_recovers` (apps/git-good/tests/cli.rs)

### CLI-16h revert
Fixture: `fx-linear`
Layer: product
- When `git good --cli revert HEAD`
- Then receipt `operation: "revert"`, `applied: true`, `in_progress: false`; the file is back to its parent's content; the new subject starts with `Revert`
- And in `fx-revert-conflict` `git good --cli revert --skip` ends the operation with no new commit
Covered by (end-to-end, partial): `revert_cli_inverts_a_commit` (apps/git-good/tests/cli.rs) (conflict and --skip not checked)

### CLI-16i conflicts lists unmerged paths
Fixture: `fx-merge-conflict`
Layer: product
- When `git good --cli conflicts`
- Then receipt `operation: "conflicts"` with `paths` listing `src/app.rs`, `docs.md`, `logo.png` and `path_bytes` their exact bytes in the same order; nothing changes
Covered by (end-to-end): `conflict_cli_lists_reads_and_resolves`, `conflict_cli_round_trips_non_utf8_path_bytes` (apps/git-good/tests/cli.rs)

### CLI-16j conflict --file reads and resolves one path
Fixture: `fx-merge-conflict`
Layer: product
- When `git good --cli conflict --file src/app.rs`, then `--use theirs`, then `git good --cli conflict --file docs.md --stage`
- Then the first prints `base`, `ours`, `theirs`; the second writes theirs, stages it (`operation: "conflict-resolve"`) and the path is no longer unmerged; the third stages the current working content; an unknown path fails with `No unmerged stages` and empty stdout; `--use base` fails nonzero
- And on a fresh `fx-merge-conflict`, `git good --cli conflict --file docs.md --use <the side that deleted it>` fails nonzero naming `--use deletion`; `--use deletion` removes `docs.md` from the working tree and the index (receipt `operation: "conflict-resolve"`, `side: "deletion"`); `--use deletion` on a path both sides kept fails with `Keep deletion applies only when one side deleted <path>.`
- And with conflicts on files named `*.txt` and `a.txt`, `--file '*.txt'` with `--use theirs`, `--use deletion`, `--stage`, a GUI **Stage resolution**, or **−** (reopen) acts on `*.txt` only: `a.txt` stays unmerged with its markers on disk
Covered by (backend, end-to-end, partial): `conflict_cli_lists_reads_and_resolves`, `conflict_cli_keeps_a_deletion`, `conflict_cli_resolves_only_the_literal_glob_named_path` (apps/git-good/tests/cli.rs); `rejects_unknown_paths_and_base_selection`, `keeping_a_deletion_is_refused_without_a_deleted_side_or_after_an_edit`, `use_side_points_the_deleted_side_to_keep_deletion`, `choosing_a_side_for_a_glob_named_file_leaves_other_conflicts`, `staging_and_reopening_a_glob_named_conflict_leaves_other_conflicts`, `publishing_a_glob_named_resolution_leaves_other_conflicts`, `keeping_the_deletion_of_a_glob_named_file_leaves_other_conflicts` (crates/git/tests/conflict.rs) (--stage through the CLI not checked)

### CLI-17 worktree list
Fixture: `fx-worktrees`
Layer: product
- `git good --cli worktree list` → `operation: "worktree list"`, `worktrees` with main flagged `main: true`, lock reason `on usb disk`, prunable `wt-gone`
Covered by (end-to-end, partial): `worktree_cli_lists_adds_and_removes` (apps/git-good/tests/cli.rs); lock/prune fields not automated

### CLI-18 worktree add (create and existing) and errors
Fixture: `fx-worktrees`
Layer: product
- `worktree add ../wt-f --create f [--at c20]` → `operation: "worktree add"`, `branch: "f"`; `worktree add ../wt-x --branch feature/login` → nonzero, stderr contains `already checked out`; missing DEST → usage on stderr
Covered by (end-to-end): `worktree_cli_lists_adds_and_removes` (apps/git-good/tests/cli.rs)

### CLI-19 worktree remove and errors
Fixture: `fx-worktrees`
Layer: product
- `worktree remove ../wt-login` with a local edit → nonzero, stderr contains `local changes`, folder kept; clean → success receipt, branch kept
Covered by (end-to-end): `worktree_cli_lists_adds_and_removes` (apps/git-good/tests/cli.rs)

### CLI-20 submodule status and update
Fixture: `fx-submodule`
Layer: product
- `submodule status vendor/lib` → `operation: "submodule status"`, `submodule.path`, `index`, `initialized`, `dirty`; `submodule update vendor/uninit` → `operation: "submodule update"`, `checked_out` equals the index gitlink; dirty submodule update fails nonzero
Covered by (end-to-end): `submodule_cli_reports_and_updates_a_gitlink` (apps/git-good/tests/cli.rs)

### CLI-21 lfs status (revision and working areas)
Fixture: `fx-lfs`
Layer: product
- `lfs status old.psd` → `operation: "lfs status"`, `lfs.after.oid`, `size`, `available: false`; `--area staged|unstaged|untracked` uses that area's sides; `--area bogus` → usage error
Covered by (end-to-end): `lfs_cli_reports_pointer_identity_and_availability`, `lfs_cli_working_areas_report_index_and_worktree_pointers` (apps/git-good/tests/cli.rs)

### CLI-22 lfs fetch
Fixture: `fx-lfs`
Layer: product
- `lfs fetch old.psd` → receipt with availability now true, working file unchanged; without `git-lfs` on PATH → nonzero with reason
Covered by (Not automated)

### CLI-23 signature config and verify
Fixture: `fx-linear`; `fx-signed`
Layer: product
- `signature config` → `operation: "signature config"`, `config.format` `openpgp`, `commit_signing`/`tag_signing` false; `signature verify HEAD` → `verification.state` `unsigned`, `kind` `commit`; in fx-signed: good/unknown-key states and tag kind; `verify nosuchref` → nonzero
Covered by (backend, end-to-end, partial): `signature_cli_reports_config_and_unsigned_commit` (apps/git-good/tests/cli.rs), `non_commit_objects_and_invalid_references_are_rejected` (crates/git/tests/signature.rs); signed states not automated

### CLI-24 Missing or unknown subcommand of grouped commands
Fixture: `fx-linear`
Layer: product
- `git good --cli signature` → stderr `Use git good --cli signature <config|verify> --help for usage`, nonzero; `git good --cli lfs nope`, `git good --cli worktree nope`, `git good --cli submodule` similarly fail with usage; stdout empty
Covered by (Not automated)

### CLI-25 Unknown option and missing --repo value
Fixture: `fx-linear`
Layer: product
- `git good --cli worktree list --bogus` → usage on stderr; `git good --cli worktree list --repo` → `--repo needs a path`; both nonzero
Covered by (Not automated)

### CLI-26 --repo pointing outside a repository
Fixture: `fx-not-repo`
Layer: product
- `git good --cli worktree list --repo <fx-not-repo>` (and `signature config`, `refs`, `status`, `history` with `--repo <fx-not-repo>`) → nonzero, stderr names the path, empty stdout
Covered by (Not automated)

### CLI-27 Launch with PATH opens the GUI on that repository — see FRM-02

### CLI-28 Init creates a repository and refuses inside one
Fixture: an empty temporary folder with `notes/a.txt`; `fx-linear`
Layer: product
- When `git good --cli init --branch trunk --commit notes`
- Then stdout is one receipt `{"operation":"init","path","branch":"trunk","head","files":1,"complete":true}`, `notes` is a repository on `trunk` whose HEAD is `head` (`Initial commit`)
- When `git good --cli init` runs in `notes/docs`
- Then it exits nonzero with `is inside the repository` on stderr, empty stdout, and nothing created
Covered by (end-to-end): `init_cli_creates_commits_and_refuses_inside_a_repository` (apps/git-good/tests/cli.rs)

Rules every area relies on, cases that span areas, and the CLI contract. Area cases cite a rule when they check one instance of it.

## Shared rules

1. **RULE-01 Inspecting never moves HEAD.** Selecting (a single click) a commit, ref pill, sidebar row or search result, opening Review, file history, blame, signature or submodule/LFS details never switches branches, moves HEAD, or writes the index, worktree or refs (FRM-51, HIS-24, HIS-49, REV-69, SIG-07).
2. **RULE-02 Failed actions preserve work and inputs.** A refused or failed action keeps the commit draft, dialog fields, line picks, rebase plan and conflict choices, and leaves the repository as it was, or names exactly what did change (CHG-47, HIS-60, INT-32, RWR-43).
3. **RULE-03 Stale asynchronous results never replace current content.** Results carry the identity and generation they were requested for; a result for a superseded selection, filter, comparison or repository is dropped (HIS-15, HIS-33, REV-20).
4. **RULE-04 Navigation restores the caller's place.** Back from Review, a reader, file history, Recovery or the rebase plan returns once to the same selection, filter, entry and pixel offset (REV-02, REV-62, RWR-35, RWR-72).
5. **RULE-05 Destructive actions list consequences and revalidate.** Before a red primary the dialog lists what is lost; on confirm Git Good re-reads what the dialog lists, and if any of it changed since it was shown nothing runs, the list refreshes in place and the dialog shows `The repository changed; review the updated list and confirm again.`; the next confirm checks again (CHG-38, CHG-72, RWR-64, RWR-78, HIS-79). Built for **Discard all tracked changes…**, **Reset here** and **Delete branch**; the per-file discard, stash drop and **Delete remote branch** do not revalidate yet (STA-25, NET-38).
6. **RULE-06 Panes never show loading text or mismatched identity.** Pending reads show in the footer (`Reading …`); a pane keeps its old label with its old content until the new content arrives, then both change together (FRM-70, FRM-73, REV-08).
7. **RULE-07 An explicitly chosen operation runs as chosen.** No silent fallback or downgrade: a signed commit never falls back to unsigned, a lease push never retries without the lease, Hard reset is never downgraded and always confirms, Pull uses the chosen strategy (SIG-09, NET-24, RWR-66, NET-18).
8. **RULE-08 Network only on request.** Launch, automatic refresh, focus and ref navigation never fetch or push (NET-34).
9. **RULE-09 One home, one reason.** Every command has one menu home and a search Action; a disabled command shows the same reason in the menu, search and any button (FRM-17, FRM-25, FRM-72).
10. **RULE-10 Commands act on current state.** Before a command runs, unsettled watcher changes are applied and the operation state is re-read (FRM-63).
11. **RULE-11 Frames hold still.** Menu bar, toolbar, operation band, footer and reserved slots keep their size when content, counts, progress or errors arrive (FRM-71, RWR-54, REV-14).
12. **RULE-12 Rewrites are recoverable.** Every rewrite and reset (Soft, Mixed and Hard) writes a private backup before Git starts, and fails with nothing changed when it cannot; Recovery actions never delete backups or untracked files (RWR-41, RWR-77).
13. **RULE-13 Status is not conveyed by color alone.** Every state shown in color also has a marker, glyph or text (XCT-21).
14. **RULE-14 Keyboard completes every workflow.** Each workflow can be finished without a pointer; text fields keep their keys; every dialog opens with its field, or its default button, focused, Enter runs the enabled primary and Esc cancels, and a destructive dialog focuses Cancel (FRM-89); closing a dialog restores focus to where it was opened (XCT-13 … XCT-18).
15. **RULE-15 CLI and GUI share state and consequences.** Both read the same operation state, validate with the same rules and report the same consequences and backups (XCT-24 … XCT-28).
16. **RULE-16 Commands cost what Git costs.** A command run from the GUI takes no more than 1 % longer than the same Git command from the CLI on the same repository. Timing cannot resolve 1 % of a 15 ms Git call, so the rule is enforced structurally: for every mutation the GUI starts exactly the Git processes `git good <cmd>` starts, in the same order, and nothing before or around them; the refreshes it runs afterwards are background reads that never delay the result being shown (STA-26, XCT-30). No GUI mutation runs a Git process on the UI thread: each runs on a worker and publishes its receipt when the worker reports back, and the signing configuration Commit and New tag need is read in the background (XCT-31). Every Git process starts at one recorded chokepoint ([architecture](architecture.md#git-processes)). A nightly job times every mutation to its published receipt against the CLI handler and fails one whose GUI median exceeds 1.01 × the CLI median + 2 ms ([development](development.md)). Watching the repository costs its Git commands nothing measurable: the watcher asks for no open or read events and stays out of objects and logs (FRM-61). An idle window draws no frames (XCT-29).
17. **RULE-17 Targets never swap under a click.** New content shows when it arrives; the scroll position, selection and focus follow identity; a row that changes under a still pointer fades in over 0.25 s and refuses clicks for 0.3 s, and a click acts only on the row that was pressed (HIS-05, HIS-07, REV-06, REV-21, CHG-66).

## Cross-cutting scenarios

### CLI-29 ignore — see CHG-77

### Restart during an operation

### XCT-01 Quit at a reopened rewrite stop and reopen
Fixture: `fx-rewrite-reopened` with one commit already made at the stop and one tracked file still unstaged
Layer: product
- When Git Good is quit (`Ctrl+Q`), and in a second run killed (`kill -9`), then reopened on the repository
- Then the band reads `Rebase stopped · step 3 of 6 · reopened <id>`; `COMMITS MADE AT THIS STOP` lists the commit made before quitting; the unstaged file is intact; the backup ref still exists; **Continue** is disabled with the remaining-changes reason; **Abort and restore** returns the branch to the pre-rewrite tip
Covered by (Not automated)

### XCT-02 Kill Git Good while a rewrite is running
Fixture: `fx-rebase`, a reorder plan that takes long enough to interrupt (slow `GIT_SEQUENCE_EDITOR` hook or many steps)
Layer: product
- When Git Good is killed while the band reads **Rewriting commits…**, then reopened
- Then Git Good shows what Git left, never a stale idle state: either the rewritten commits, or a stop band with **Continue** / **Skip commit** / **Abort and restore**; the private backup written before Git started exists and Recovery lists it; no second backup is written on reopen
Covered by (Not automated)

### XCT-03 Quit during conflict resolution and reopen
Fixture: `fx-merge-conflict`, `src/app.rs` resolved and staged, block choices made but not staged in `docs.md`
Layer: product
- When Git Good is quit and reopened
- Then the amber band reads `Merge conflict · merging <ref> <id> into main` · `2 conflicts left in 2 files`; `src/app.rs` is listed under `ALSO CHANGED BY THIS MERGE` (the session's RESOLVED, STAGED list starts empty); the unstaged choices were never written, so the working files of the open paths still hold their markers; **Continue** stays disabled; **Abort** restores the pre-merge state
Covered by (Not automated)

### XCT-04 Kill Git Good during autostash restoration
Fixture: `fx-rebase-autostash`, rewrite started with **Stash working changes first**
Layer: product
- When Git Good is killed after the rewrite and while the stash is being restored, then reopened
- Then no work is lost: either the changes are back in the worktree (staged part restored as staged when Git restored it), or the autostash entry is listed in **Changes › Stashes…** and the footer names it with **Stashes…**; untracked `scratch.txt` exists or is in the entry; the backup ref exists
Covered by (Not automated)

### Stale destructive confirmations

### XCT-05 A change inside the watcher's settle window is still caught
Fixture: `fx-dirty`
Layer: product
- Given **Discard all tracked changes…** is open
- When another process edits a tracked file less than 300 ms before the user confirms
- Then Git Good flushes the pending watcher batch before running (RULE-10), detects the change and shows the revalidation notice; nothing is discarded
Covered by (Not automated)

### XCT-06 The target disappeared while the dialog was open
Fixture: `fx-branchy` (**Delete branch** `old`); `fx-stash` (drop `stash@{1}`); `fx-remote` (**Delete on origin…** `origin/release`)
Layer: product
- When, while each dialog is open, a terminal deletes `old`, clears every stash, or prunes `origin/release`, and the user confirms
- Then nothing runs; the dialog states that the target no longer exists, the primary is disabled, and **Close** returns to the refreshed view
Covered by (Not automated)

### XCT-07 Revalidation behaves the same in every destructive dialog
Fixture: `fx-dirty`, `fx-branchy`, `fx-stash`, `fx-remote`
Layer: product
- When an external change to what the dialog lists occurs before confirm in **Discard all tracked changes…**, **Reset here** (Hard) and **Delete branch**
- Then each shows the same notice text in its reserved line without resizing; the primary needs a fresh click or `Enter`; the second confirm acts on the refreshed list only (CHG-72, RWR-78, HIS-79); a change to something the dialog does not list does not block
- Gap: **Discard unstaged changes…**, the stash drop confirmation and **Delete remote branch** do not revalidate yet (STA-25, NET-38)
Covered by (interaction, partial): `discard_all_rereads_and_asks_again_after_an_outside_change`, `reset_confirmation_rereads_and_asks_again_after_an_outside_change` (apps/git-good/src/main.rs), `guard_confirms_only_what_the_dialog_showed` (apps/git-good/src/consequence.rs) (Discard all and Reset here; Delete branch and the reserved line not checked)

### Autostash failure paths

### XCT-08 Rewrite succeeds but restoring the stash conflicts
Fixture: `fx-rebase-autostash`, a plan that rewords and drops the commit touching `folder_picker.rs`; **Stash working changes first** on
Layer: product
- When the rewrite finishes and the stash is re-applied
- Then the rewritten commits are in place with their backup; the footer warns that the rewrite finished but restoring the stashed changes conflicted, with **Resolve conflicts…** and **Stashes…**; the stash entry is kept until the user drops it; the staged and unstaged edits are recoverable from it (**Show changes**); untracked `scratch.txt` is present; no operation band with Continue/Skip/Abort appears
Covered by (Not automated)

### XCT-09 Abort and restore when restoring the stash conflicts
Fixture: `fx-rebase-autostash` with the plan stopped on a conflict at step 4
Layer: product
- When **Abort and restore** is clicked
- Then the branch is back at its pre-rewrite tip; the stash re-application conflict is reported as in XCT-08 with the entry kept; staged, unstaged and untracked work is preserved in the worktree or the entry
Covered by (Not automated)

### XCT-10 Rebase onto with autostash, from GUI and CLI
Fixture: `fx-rebase-autostash`; branch `other` whose tip changes `folder_picker.rs`
Layer: product
- When **Branch › Rebase current onto…** `other` with **Automatically stash working changes** runs, and separately `git good --cli rebase other --autostash`
- Then both finish the rebase, keep the entry when restoration conflicts, and report it: the GUI as in XCT-08, the CLI receipt with a warning naming the kept stash entry and the backup
Covered by (Not automated)

### Rebase conflict identities end to end

### XCT-11 Resolve a rebase conflict by its labeled sides
Fixture: `fx-rebase`, a plan reordering two commits so step 4 conflicts
Layer: product
- When the plan starts, stops on the conflict, and the user opens **Resolve conflicts…**, chooses the side labeled `Theirs (<8-char id> <subject>)` for every block, stages and continues
- Then the labels read `Ours (onto <base>)` / `Theirs (<8-char id> <subject>)` (INT-38); `git good --cli conflict --file <path>` returns as `theirs` the bytes shown under that label; after **Continue** the rewritten commit carries the chosen content
Covered by (Not automated)

### XCT-12 Labels follow the step
Fixture: `fx-rebase-conflict` with a second conflict at step 5
Layer: product
- When the user resolves step 4, continues, and the rebase stops at step 5
- Then every label, the band and the window title name step 5 and its commit; no label from step 4 remains (RULE-06)
Covered by (Not automated)

### Accessibility

### XCT-13 Keyboard-only commit
Fixture: `fx-dirty`, no pointer input
Layer: interaction
- When the user presses `Ctrl+1`, moves with Up/Down and `Tab`, stages with `S`, unstages with `U`, picks a hunk with `[`/`]` and stages it, types a message and presses `Ctrl+Enter`
- Then the commit is created with exactly the staged content; every step had a visible focus target
Covered by (Not automated)

### XCT-14 Keyboard-only rebase plan
Fixture: `fx-rebase`, no pointer input
Layer: interaction
- When the user selects a commit with `j`/`k`, presses `R`, sets actions with `P R E S F D`, reorders with `Alt+↑/↓`, types a reword message, leaves the field with `Esc`/`Tab`, and starts with `Ctrl+Enter`
- Then the rewrite runs with the plan as built
Covered by (Not automated)

### XCT-15 Keyboard-only conflict resolution
Fixture: `fx-merge-conflict-multi`, no pointer input
Layer: interaction
- When the user opens **Resolve conflicts…** via `Ctrl+K`, chooses blocks with `1`–`4` and `[`/`]`, stages with `Ctrl+S`, and runs **Continue** from the keyboard
- Then the merge completes; no step required a pointer
Covered by (Not automated)

### XCT-16 Keyboard-only recovery
Fixture: `fx-rebase` after a rewrite that dropped a commit, no pointer input
Layer: interaction
- When the user opens **Recovery…** via `Ctrl+K`, selects the newest backup with Up/Down, presses `Enter` for **Restore main…**, and confirms with `Enter`
- Then the branch is back at the pre-rewrite tip, and the tip it left is a new backup
Covered by (interaction, partial): `restore_is_one_confirmed_hard_reset_that_saves_the_tip_first` (apps/git-good/src/recovery_view.rs) (`Enter` twice; opening via search not automated)

### XCT-17 Focus is always visible
Fixture: `fx-dirty`, `fx-rebase`
Layer: interaction
- When the user tabs through the toolbar, lists, reader header, commit form, dialogs and the rebase plan
- Then every focused control shows a focus indicator that is not clipped, hidden behind another pane, or shown by color alone
Covered by (Not automated)

### XCT-18 Focus returns after a dialog closes
Fixture: `fx-branchy`
Layer: interaction
- When **New branch…** is opened from a History tree row and closed with `Esc`, **Reset here** from the Commit menu is confirmed, and **Discard unstaged changes** is cancelled from a file row
- Then focus returns to the History tree row, the History tree list and the file row respectively, so the next key acts there
Covered by (Not automated)

### XCT-19 Icon-only buttons have accessible names
Fixture: `fx-dirty` with AccessKit enabled (inspect the accessibility tree)
Layer: interaction
- Then the refs sidebar toggle, **Fetch**, **Pull**, **Push**, the branch button, the narrow search icon, row **+**/**−** buttons, move up/down buttons, the drag handle, the commit-editor expand control and window close buttons expose names equal to their tooltip command names; disabled ones expose their disabled state and reason
Covered by (Not automated)

### XCT-20 Accessible names for list rows and state
Fixture: `fx-dirty`, `fx-rebase-conflict`
Layer: interaction
- Then file rows expose path plus kind and area (e.g. `a.txt, modified, unstaged`), History tree rows subject, author and short ID, and the operation band its full first line
Covered by (Not automated)

### XCT-21 Status is never conveyed by color alone
Fixture: `fx-dirty`, `fx-merge-conflict`, `fx-rebase` with a plan using every action
Layer: visual
- When the window is captured in grayscale
- Then change kinds stay distinct by marker (`~ − + ? ! T`), ahead/behind by arrows, stop kinds by glyph and text, dropped rows by strike-through and badge text, Ours/Theirs by their labels, errors and warnings by text, and diff additions/deletions by `+`/`−` signs
Covered by (interaction, partial): `change_markers_are_single_distinct_characters` (apps/git-good/src/status_view.rs)

### XCT-22 Usable at 125 % interface zoom
Fixture: `fx-dirty`, `fx-rebase`; **Interface zoom** 125 % at 1440×900
Layer: interaction
- When the user commits, opens Review, builds a rebase plan and opens Recovery
- Then no control is clipped or overlapping, every dialog fits the window, and the narrow layout takes over where the scaled width requires it
Covered by (Not automated)

### XCT-23 Core workflows complete at 720×480
Fixture: `fx-dirty`, `fx-merge-conflict`, `fx-rebase`, `fx-backups`; narrow
Layer: interaction
- When the user commits, reviews a commit, resolves a conflict, runs an interactive rebase and restores a backup
- Then each completes without resizing the window; every dialog and window fits the viewport with its primary action visible
Covered by (interaction, partial): `recovery_window_stays_inside_the_narrow_viewport`, `worktree_window_lists_state_and_fits_the_narrow_viewport` (apps/git-good/src/main.rs)

### CLI and GUI continuity

### XCT-24 Rebase started in the CLI, finished in the GUI
Fixture: `fx-rebase-conflict` created by `git good --cli rebase origin/main` instead of Git
Layer: product
- When Git Good opens, the user resolves in **Resolve conflicts…** and clicks **Continue**
- Then the band shows the stop the CLI receipt described (step, commit, conflicts); Recovery's newest backup is the receipt's `backup`; afterwards `git good --cli conflicts` lists none and `git good --cli history` shows the rebased commits
Covered by (Not automated)

### XCT-25 Merge in progress shared live by CLI and GUI
Fixture: `fx-merge-conflict` created by `git good --cli merge feature/login`
Layer: product
- When Git Good is open on it and `git good --cli conflict --file src/app.rs --use theirs` runs in a terminal
- Then within ~300 ms Git Good moves `src/app.rs` to `RESOLVED, STAGED` and the count drops to 2; `git good --cli conflicts` and the GUI list the same paths at every point
Covered by (Not automated)

### XCT-26 Cherry-pick started in the GUI, finished in the CLI
Fixture: `fx-branchy`, a conflicting `feature/login` commit
Layer: product
- When **Cherry-pick onto HEAD** stops on a conflict, the path is resolved and staged in the GUI, and `git good --cli cherry-pick --continue` runs in a terminal
- Then the CLI receipt reports `applied: true`, `in_progress: false`, and Git Good's operation band clears without user action
Covered by (Not automated)

### XCT-27 Reopened stop: CLI and GUI agree on Continue
Fixture: `fx-rewrite-reopened` with tracked changes remaining
Layer: product
- When `git good --cli rewrite --continue` runs while the GUI shows **Continue** disabled
- Then the CLI refuses with the same reason (`Commit or stash the 2 remaining changed files first.`) and changes nothing; once committed in the GUI, the CLI continue succeeds and the GUI band clears
Covered by (Not automated)

### XCT-28 Same consequences in both
Fixture: `fx-reset-collision`; `fx-branchy`
Layer: product
- When Hard reset to `c05` is attempted in **Reset here** and with `git good --cli reset --hard c05`; and `feature/login` is deleted in the GUI and with `git good --cli branch --delete feature/login`
- Then both refuse the reset naming `later.txt`; the GUI lists the 4 unmerged commits and the CLI refuses without `--force` naming them; neither changes anything
Covered by (Not automated)

### XCT-29 An idle window draws no frames
Fixture: a five-commit repository with one unstaged edit
Layer: interaction
- Given the History tree, Local Changes, Review and Preferences in turn, each with the pointer resting on it and no text field focused
- When its reads finish and its animations end
- Then no frame asks for a repaint sooner than 1 s; a failure names what asked (RULE-16). Only a pending watcher batch schedules a frame, for the moment it settles (FRM-61)
Covered by (interaction): `every_view_is_idle_once_settled` (apps/git-good/src/idle_tests.rs)

### XCT-30 GUI and CLI run the same Git commands
Fixture: two identical repositories (`main` three commits ahead of base, one unpushed; `feature` and `merged` branches; annotated tag `v1`; a local bare `origin`), fixed commit dates
Layer: interaction
- Given one fixture open in a window and the other used by `git good`
- When each mutation runs through the method its GUI button or menu calls, and through its CLI handler with the same arguments: stash push, apply, pop and drop; commit; branch create, rename (against the backend call, which has no CLI command) and delete; switch; tag create and delete; soft, mixed and hard reset; cherry-pick; revert; merge; fetch; push
- Then the GUI's Git command lines, with paths as `<repo>` and `<remote>`, begin with exactly the CLI's, in the same order, and every later process is a refresh read (RULE-16)
- And none of them, the mutation's own or the refresh, runs on the UI thread
- And no production code starts Git except through `gg_git::process::git()`
- And nightly, on `ubuntu-24.04`, each mutation runs 30 times through each path on fresh fixtures, interleaved; the GUI, timed to its receipt applied to the window's state, has a median within 1.01 × the CLI's + 2 ms
Covered by (interaction): `stash_push_runs_the_cli_commands`, `stash_apply_runs_the_cli_commands`, `stash_pop_runs_the_cli_commands`, `stash_drop_runs_the_cli_commands`, `commit_runs_the_cli_commands`, `branch_create_runs_the_cli_commands`, `branch_rename_runs_the_backend_commands`, `branch_delete_runs_the_cli_commands`, `switch_runs_the_cli_commands`, `tag_create_runs_the_cli_commands`, `tag_delete_runs_the_cli_commands`, `soft_reset_runs_the_cli_commands`, `mixed_reset_runs_the_cli_commands`, `hard_reset_runs_the_cli_commands`, `cherry_pick_runs_the_cli_commands`, `revert_runs_the_cli_commands`, `merge_runs_the_cli_commands`, `fetch_runs_the_cli_commands`, `push_runs_the_cli_commands` (apps/git-good/src/parity_tests.rs); `a_recording_keeps_only_its_own_roots` (crates/git/src/process.rs); `git_processes_start_at_the_chokepoint` (crates/git/tests/chokepoint.rs); nightly: `gui_mutations_cost_what_the_cli_costs` (apps/git-good/src/parity_tests/timing.rs, `.github/workflows/timing.yml`)

### XCT-31 Mutations wait for Git off the UI thread
Fixture: two commits on `main` and a `merged` branch; `tag.gpgsign` set with no signing key
Layer: interaction
- Given Delete branch open on `merged`
- When the user deletes it
- Then frames keep drawing while Git runs: the dialog stays open with its primary disabled, the footer reads `Deleting branch merged…` with a spinner, and stash, branch and commit mutations are disabled with `Wait for the current operation to finish`; a second mutation is refused, not queued
- And when Git returns the receipt `Deleted branch merged` closes the dialog; a failure (Delete tag on a missing tag) leaves its dialog open with Git's error and no footer error. Stash changes, New branch and Rename behave alike; the Stashes picker, Reset, Cherry-pick and Revert close as they start and report in the footer, as before (`Stashing 3 files…`, `Restoring stash@{0}…`, `Dropping stash@{0}…`, `Creating topic…`, `Renaming feature to topic…`, `Deleting tag v1…`, `Resetting to 1a2b3c4…`, `Cherry-picking 1a2b3c4…`, `Reverting 1a2b3c4…`)
- And New tag opens at once, showing the last signing configuration read, and takes the background read when it lands (Sign checked by `tag.gpgsign`, *No signing key is configured.*); Create waits for that read on its worker and refuses to make an unsigned tag. Local Changes reads the configuration the same way; focus and a change to the repository's `config` read it again; the commit worker reads it before committing, as `git good --cli commit` does
Covered by (interaction): `a_dialog_mutation_runs_on_the_worker_and_its_dialog_waits_for_git`, `new_tag_reads_signing_off_the_ui_thread_and_never_tags_unsigned`, `dialogs_take_typing_at_once_and_answer_enter_and_esc` (apps/git-good/src/main.rs); `a_worker_waits_for_the_read_in_flight_and_views_see_it_once` (apps/git-good/src/signing_cache.rs); `signing_readiness_blocks_the_commit_without_unsigned_fallback` (apps/git-good/src/status_view.rs)

### XCT-34 Back and Forward
Fixture: `main` with two commits
Layer: interaction
- When the user selects the first commit (scrolled 40 px), then a commit that is later gone, then the second commit, and opens it in Review
- Then View › Back (`Ctrl+[`, or the mouse's back button) leaves Review for the History tree on the second commit; Back again skips the gone commit and selects the first, scrolled as it was; Back is then disabled (*Nothing to go back to*)
- And View › Forward (`Ctrl+]`) returns to the second commit, then to Review on it
- And moving the selection with the keys moves the current place without adding one; jumping elsewhere drops the places forward of it; at most 256 places are kept, the oldest going first; without a repository both items say *Open a repository first*
Covered by (interaction): `back_and_forward_return_to_commits_and_reviews_jumped_to`, `menus_follow_the_final_design_mockup` (apps/git-good/src/main.rs); `back_and_forward_walk_the_places_jumped_to`, `stepping_moves_the_current_place_without_adding_one`, `gone_places_are_skipped_and_dropped`, `at_most_the_limit_is_kept` (apps/git-good/src/trail.rs)

### XCT-33 Undo the last operation
Fixture: `main` with two commits; a branch `merged` at the first
Layer: interaction
- When the user deletes `merged`, the receipt reads `Deleted branch merged` with **Undo**; Undo creates `merged` again at its old tip and reports `Undone: branch merged is back at …`
- When the user resets `main` hard to its parent, the receipt offers **Undo**; with a file changed, Undo is refused with an error and nothing moves; with a clean tree it restores the old tip and saves the tip it replaces as a backup
- When the user commits in Local Changes, the status line shows **Undo**; Undo moves `main` back to the commit's parent, keeps the commit's changes staged and the files untouched
- And only the last operation can be undone: starting another operation forgets the Undo
Covered by (interaction): `undo_brings_a_deleted_branch_back_at_its_tip`, `undo_of_a_reset_needs_a_clean_tree_and_restores_the_tip`, `a_commit_in_local_changes_offers_undo_to_its_parent`, `undo_of_a_commit_keeps_its_changes_staged` (apps/git-good/src/main.rs)

### XCT-32 Launches and crashes leave a log
Fixture: a configuration folder with a `gg/logs/gg.log` from an earlier run
Layer: product
- When Git Good opens a window
- Then the earlier log becomes `gg.previous.log` and a new `gg.log` starts with the version, commit, build date, target and OS, then Git's version; a failed operation's error, a panic with its thread, place and backtrace, and a launch that fails are appended; URL passwords read `https://user:***@host`; past 16 MiB one line says the log is capped and nothing more is written; CLI commands write no log
- And a launch that fails with no terminal attached shows a native error dialog with the error and the log's path
- And the window starts with no OpenGL backend on Windows and no wgpu indirect-call validation unless `WGPU_BACKEND` or `WGPU_VALIDATION_INDIRECT_CALL` say otherwise
Covered by (interaction): `a_launch_keeps_the_previous_log_and_redacts_and_caps`, `url_passwords_are_redacted_and_the_rest_kept`, `times_are_utc_dates`, `the_os_names_the_system_and_architecture` (apps/git-good/src/diagnostics.rs); `version_info_is_plain_build_git_and_os_lines` (apps/git-good/src/branding.rs)

### XCT-35 The domain language names each thing once
Fixture: `fx-linear` with a modified file
Layer: product
- Given the terms in [domain.md](domain.md)
- When the user goes through the History tree, its menus and search, Review with File history, Local Changes, and every Preferences section
- Then "history" never appears on its own: only as **History tree**, **File history**, **Line history** or **Rewrite history**, spelled in full (the History tree tab, `‹ History tree`, the reader's **File history** button, `H: File history`, *Select a commit in the History tree first*)
- And the working-tree view is **Local Changes** on its tab, in the View menu and in messages (**Go to Local Changes**)
- And no string the app can show uses "history" on its own, whatever state shows it
- And `git good --cli history-tree` prints the History tree; `git good --cli history` still prints the same document, and `--help` lists only `history-tree`
Covered by (interaction, end-to-end): `the_word_history_is_always_part_of_a_full_domain_term`, `no_shown_string_uses_history_outside_the_full_domain_terms` (apps/git-good/src/main.rs), `history_is_a_hidden_alias_of_history_tree` (apps/git-good/tests/cli.rs)

### XCT-36 The desktop's accent color
Fixture: KDE Plasma with `AccentColor=61,174,233` in `kdeglobals`; GNOME 47 with the purple accent (settings portal `accent-color`); Windows with an accent color; macOS with an accent color
Layer: product
- Given Preferences › Appearance › **Use the desktop's accent color** is on (the default)
- When Git Good starts, or regains focus after the accent changed
- Then selected rows, selected buttons, links and primary buttons use the desktop's accent, adjusted so white text on primary buttons and links on the panel keep 4.5:1 contrast and text on selected rows at least 3:1, in the light and the dark theme
- And a desktop without an accent (or the preference off) keeps Git Good's own blue
- Where it comes from: Linux, the settings portal's `org.freedesktop.appearance accent-color`, else KDE's `kdeglobals` `[General] AccentColor` or `[Colors:Selection] BackgroundNormal`, else GNOME's `accent-color` setting; Windows, `HKCU\Software\Microsoft\Windows\DWM\AccentColor`; macOS, `NSColor.controlAccentColor`
Covered by (interaction): `the_portal_accent_reads_gdbus_output_and_rejects_unset`, `gnome_and_kde_accents_read_their_settings`, `the_windows_accent_dword_is_abgr`, `a_kde_session_gives_its_accent_fonts_and_forced_dpi`, `a_gnome_session_reads_the_portal_then_gsettings`, `every_accent_keeps_text_readable_in_both_themes`, `the_accent_reaches_selection_links_and_primary_buttons` (apps/git-good/src/desktop_look.rs), `the_desktop_accent_and_text_size_shape_the_window_and_follow_preferences` (apps/git-good/src/main.rs)

### XCT-37 The desktop's fonts and text size
Fixture: KDE with `font=Noto Sans,10,…`, `fixed=Hack,10,…` and `forceFontDPI=120`; GNOME with `font-name 'Cantarell 11'` and `text-scaling-factor 1.5`; Windows with "Make text bigger" at 125 %; macOS
Layer: product
- Given Preferences › Appearance › **Use the desktop's fonts and text size** is on (the default)
- When Git Good starts
- Then interface text uses the desktop's interface font and code uses its monospace font (KDE `font=`/`fixed=`, GNOME `font-name`/`monospace-font-name` found with `fc-match`; Windows Segoe UI Variable and Cascadia Mono; macOS SF Pro and SF Mono), with the bundled fonts, icons and CJK fallback behind them for any character they lack
- And a font that is not installed, does not parse, or that `fc-match` replaces with another family is not used
- And the interface zoom is multiplied by the desktop's text size (KDE forced DPI ÷ 96, GNOME `text-scaling-factor`, Windows `TextScaleFactor` ÷ 100)
- And with the preference off the bundled fonts and zoom are used; fonts change when Git Good restarts, the text size at once
Covered by (interaction): `font_settings_name_their_family`, `text_scales_read_each_desktop_and_reject_misreads`, `only_font_files_that_parse_with_latin_letters_are_used`, `a_kde_session_gives_its_accent_fonts_and_forced_dpi`, `a_gnome_session_reads_the_portal_then_gsettings`, `a_desktop_that_answers_nothing_keeps_git_goods_look` (apps/git-good/src/desktop_look.rs), `the_desktop_fonts_come_before_the_bundled_ones`, `the_desktop_accent_and_text_size_shape_the_window_and_follow_preferences` (apps/git-good/src/main.rs)

## CLI contract

The contract for scripts using `git good --cli`. *Proposed* marks behavior decided but not yet built ([spec decisions](#spec-decisions--2026-11-04)).

### CLI-C01 Output kind per command
Fixture: `fx-linear`
Layer: product
- Then `--version` prints plain text `git-good <version>` and `--help` (or `-h`) plain usage text, both exit 0 without a repository; `git good` with no argument or with a single FOLDER launches the GUI; every command after `--cli` (`history`, `refs`, `show`, `status`, `stage`, `unstage`, `commit`, `reflog`, `blame`, `signature`, `switch`, `branch`, `tag`, `reset`, `fetch`, `push`, `clone`, `stash`, `merge`, `rebase`, `rewrite`, `cherry-pick`, `revert`, `conflicts`, `conflict`, `worktree`, `submodule`, `lfs`) prints JSON and never opens a window (`clone` does not open the clone either)
- And a bare first word is always a folder; commands follow `--cli`. A missing folder named like a command (`git good status`) and an unknown word after `--cli` (`git good --cli histroy`) are usage errors and open no window
Covered by (end-to-end, partial): `version_matches_package_without_a_repository`, `help_lists_every_cli_command`, `commands_follow_cli_and_a_bare_word_is_a_folder` (apps/git-good/tests/cli.rs)

### CLI-C02 JSON framing
Fixture: every JSON command of CLI-C01 on its usual fixture
Layer: product
- Then stdout holds exactly one UTF-8 JSON document followed by a newline and nothing else; progress and warnings for humans go to stderr; keys are snake_case and every receipt names its `operation` and its `status`; `history`, `status` and `show` stream their one document incrementally and end it with `complete` and `status`
- And keys and their types are stable: later versions only add keys, so consumers ignore unknown keys; every path field comes as `path` (escaped) beside `path_bytes` (exact) (decision 16)
Covered by (end-to-end, partial): every receipt test in apps/git-good/tests/cli.rs parses stdout as one document, e.g. `sync_cli_clones_fetches_pushes_and_reports_failures`, `stash_cli_lists_pushes_applies_pops_and_drops` (no stability check)

### CLI-C03 Distinct outcomes
Fixture: `fx-dirty`, `fx-branchy`, `fx-merge-conflict`, `fx-remote-slow`
Layer: product
- Then exit 0 completed (`status: "completed"`) or stopped as planned (`status: "stopped"`, e.g. a rewrite `--edit` stop); 2 usage error; 4 conflict (`status: "conflict"`); 1 any other failure, with `Error: <message>` on stderr and nothing on stdout. Ctrl+C ends the process by the signal, which shells report as 130
- *Proposed* (decided, not yet built; needs failures typed by the Git layer): 3 refusal before any change (`status: "refused"`); 5 partial completion (`status: "partial"`). Until then both exit 1
Covered by (end-to-end, partial): `a_failure_prints_the_plain_message` (2), `merge_cli_fast_forwards_conflicts_and_recovers`, `rebase_cli_rewrites_records_backup_and_recovers`, `cherry_pick_cli_applies_conflicts_and_recovers`, `rewrite_cli_reorders_squashes_and_recovers` (4, `stopped`) (apps/git-good/tests/cli.rs)

### CLI-C04 Usage error
Fixture: `fx-linear`
Layer: product
- When `git good --cli merge --skip`, `git good --cli worktree list --bogus`, `git good --cli worktree list --repo`, `git good --cli signature`
- Then stderr names the problem (`Unknown merge option`, usage line, `--repo needs a path`), stdout is empty, nothing changes; exit 2 (CLI-06, CLI-24, CLI-25)
Covered by (end-to-end, partial): `merge_cli_fast_forwards_conflicts_and_recovers`, `cherry_pick_cli_applies_conflicts_and_recovers` (apps/git-good/tests/cli.rs); `a_failure_prints_the_plain_message` checks exit 2; `every_usage_error_exits_2_and_a_git_failure_exits_1` covers the other usage errors

### CLI-C05 Refusal
Fixture: `fx-dirty` with a branch `other`; `fx-reset-collision`
Layer: product
- When `git good --cli rebase other`, `git good --cli switch other`, `git good --cli reset --hard c05`
- Then each refuses before changing anything (repository byte-identical), stderr names the reason and the paths involved, stdout is empty; exit 1 today, *Proposed* exit 3
Covered by (end-to-end, partial): `rebase_cli_rewrites_records_backup_and_recovers`, `switch_cli_changes_branches_and_reports_dirty_worktrees` (apps/git-good/tests/cli.rs)

### CLI-C06 Conflict leaves a machine-readable receipt
Fixture: `fx-branchy` set up so `merge`, `rebase` and `cherry-pick` conflict
Layer: product
- When each runs
- Then stdout carries the receipt with `operation`, `in_progress: true`, `conflicts` (exact paths), and `backup` when one was written (rebase, rewrite); the next steps are `--continue`/`--skip`/`--abort`; exit 4 with `status: "conflict"`
Covered by (end-to-end, partial): `merge_cli_fast_forwards_conflicts_and_recovers`, `rebase_cli_rewrites_records_backup_and_recovers`, `cherry_pick_cli_applies_conflicts_and_recovers` (apps/git-good/tests/cli.rs)

### CLI-C07 Cancellation
Fixture: `fx-remote-slow`
Layer: product
- When `git good --cli fetch` receives SIGINT during transfer
- Then the Git child is terminated and reaped (no orphan process), stdout is empty, stderr says the fetch was cancelled and the repository should be re-read before retrying; the shell reports 130. Today no signal handling exists: the default signal ends Git Good and its Git child together
Covered by (Not automated)

### CLI-C08 Partial completion
Fixture: `fx-merge-conflict` with `.git/index.lock` created after the conflict was read; `fx-remote` where setting the upstream fails after a successful push
Layer: product
- When `git good --cli conflict --file src/app.rs --use theirs`, and `git good --cli push origin local-only --set-upstream`
- Then *Proposed*: exit 5 with a receipt `status: "partial"` naming what was done (working file replaced; branch pushed) and what was not (path still unmerged; upstream not set). Today: exit 1, stdout empty, the change only described on stderr
Covered by (Not automated)

### CLI-C09 Writing the output fails
Fixture: `fx-linear` and `fx-dirty`
Layer: product
- When stdout is closed or full (`git good --cli refs >&-`, `git good --cli commit -m x > /dev/full`, `git good --cli history | head -c 10`)
- Then a closed stdout ends the command quietly, with no panic text, and the exit code of what it already did (0 for a read, the receipt's code for a mutation); a full disk is a plain failure (exit 1, `Error:` on stderr)
- And a streamed document whose read fails part way still ends as one valid JSON document, with `complete: false`, `status: "failed"` and `error`, and exits 1 with the reason on stderr
Covered by (end-to-end, partial): `a_closed_stdout_ends_quietly`, `history_that_fails_mid_stream_ends_with_an_error_member` (apps/git-good/tests/cli.rs)

### CLI-C10 Unsuccessful exits that changed the repository always print state
Fixture: as CLI-C06 and CLI-C08
Layer: product
- Then every outcome that changed refs, index or files prints its receipt on stdout, whatever the exit status, so a script can always find `backup`, `conflicts` and `in_progress`; outcomes that changed nothing print nothing on stdout
Covered by (end-to-end, partial): `rebase_cli_rewrites_records_backup_and_recovers` (apps/git-good/tests/cli.rs) (conflict receipt with backup)

### CLI-C11 Paths with spaces
Fixture: `fx-linear` at `/tmp/a b/fx-linear`, with tracked `dir with space/my file.txt` edited
Layer: product
- When `git good --cli status --repo "/tmp/a b/fx-linear" --file "dir with space/my file.txt" --area unstaged`, `git good --cli stage --file "dir with space/my file.txt"`, `git good --cli worktree add "../wt with space" --create x`
- Then each acts on exactly that path and reports it unchanged in `path`
Covered by (Not automated)

### CLI-C12 Leading hyphens
Fixture: `fx-linear` with tracked `-x.txt` and `--weird` edited
Layer: product
- When `git good --cli stage --file -x.txt`, `git good --cli show HEAD --file --weird`, `git good --cli blame -- -x.txt`
- Then option values are taken verbatim after their flag (`--file -x.txt` works), and `--` ends options in every command, so `git good --cli blame -- -x.txt` names the file `-x.txt`
Covered by (end-to-end, partial): `double_dash_ends_options` (apps/git-good/tests/cli.rs) — `blame --`

### CLI-C13 Non-UTF-8 names
Fixture: a commit adding a file named with byte `0xFF` and a newline
Layer: product
- When `git good --cli show HEAD --file <raw bytes>`, `git good --cli status`, `git good --cli conflicts` on a conflicted such path, then `git good --cli conflict --file <raw bytes>` and `--use theirs`
- Then the exact path is accepted as raw bytes; JSON reports `path` escaped (`raw\xff\nname`) and `path_bytes` as the exact byte array (`conflicts`: `paths` escaped and `path_bytes` in the same order); the conflict resolves and stages that file; arguments that must be text (URLs) are refused with `must be UTF-8`
Covered by (backend, end-to-end, partial): `show_file_uses_exact_non_utf8_path_bytes`, `conflict_cli_round_trips_non_utf8_path_bytes` (apps/git-good/tests/cli.rs); `conflict_paths_keep_their_exact_bytes` (crates/git/tests/conflict.rs) (status not checked)

## Spec decisions · 2026-11-04

The owner decided the open spec questions found while writing these cases. Each decision names what users see; the affected cases change with the code that builds it.

1. **Docs follow shipped code (questions 1–4, 10, 12).** View › **Commit details** (`Space`); the Search Files group ships; network Cancel sits where the code puts it; branch delete, rename, tracking and tag creation ship; cli.md lists every parsed flag; user-flows F07 and the toolbar list match the stash picker and the shipped toolbar.
2. **Destructive confirmations (5).** Every destructive action lists what it removes before its primary and reads the repository again on confirm: hunk discard, per-file discard, stash drop, **Delete tag** and **Delete on origin…** join Discard all, Reset and Delete branch (RULE-05). A confirmation never loses an outside edit made while it was open.
3. **Rebase band (6).** The result reads `6 commits → 4 · 1 dropped · 1 stop` when a step stopped. At a reopened stop with unstaged files, the primary stays **Continue rebase**, with a note that unstaged files stay out of the commit.
4. **Reader names (7).** Git's short words everywhere: **Blame**, **File history**, **Compare with HEAD**.
5. **Preferences (8).** A modal, as built. The `settings.json` key list in architecture.md is completed.
6. **CLI parity (9).** Every GUI mutation has a CLI form: `worktree prune|lock|unlock`, `pull`, and hunk and line discard are added, and the GUI/CLI parity tests cover each.
7. **Signing (11).** Git Good keeps no signing setting of its own. Preferences shows and edits Git's `commit.gpgSign` and `tag.gpgSign` (global or this repository); the Sign checkbox in Commit and New tag stays a per-action choice. Git Good and plain `git` always sign alike.
8. **Bare repositories (13).** Open read-only: History tree, refs, Review, Blame and File history work; Local Changes, stash, commit, switch, merge and rebase are disabled with *A bare repository has no working files.*
9. **Refs filter (13).** Groups keep their headers with a count of 0 so the sidebar holds still; when nothing matches anywhere, one *No matching refs* line shows.
10. **Recovery (14).** No Refresh buttons: Backups and Reflog re-read on focus and on watcher changes, like every other tool window.
11. **Restart at a stop (15).** Reopening after quitting at a reopened stop opens Local Changes with the reopened commit's message as the draft. Unstaged conflict-block choices are saved like drafts and restored for the same conflict; they are dropped when the file changed outside.
12. **Exit codes (16).** 0 completed, 1 failure, 2 usage, 3 refused, 4 stopped on conflicts, 5 partial, 130 interrupted. `git good --cli merge x && deploy` never continues into a conflicted tree.
13. **Receipts (17).** Every exit that inspected or changed the repository prints the same receipt schema with a `status` key (`completed`, `conflict`, `partial`, `refused`). A plain failure prints nothing on stdout and its reason on stderr.
14. **Output failures (18).** A closed stdout ends quietly with the command's own code and no panic text. `history` streams; a traversal error ends the document with an `error` member, so the output is always valid JSON.
15. **Command line shape (19).** A bare argument is always a folder: `git good [PATH]` opens the GUI. Commands come after `--cli`: `git good --cli status --repo .`. The repository is always `--repo PATH`; positionals are only a command's own subject (a ref, a file, a URL), and `--` ends options. When PATH does not exist and names a command, Git Good opens no window and exits 2 with *No folder status. Did you mean git good --cli status?* The old forms are not kept.
16. **JSON stability (20).** Additive only: keys are never renamed or removed; new keys may appear. Every path field comes as `path` (escaped for display) beside `path_bytes` (exact).
17. **Zoom (21).** 125 % zoom is required from a 900×600 window up (720×480 logical); smaller windows use 100–110 %.
18. **Accessibility (21).** Through AccessKit every control and row has a name (a History tree row: subject, author, date; a diff row: line kind and text), and every disabled control gives its disabled reason as its description. A screen-reader user can complete every workflow the keyboard can.

Still open (13): the missing-parent incomplete comparison label; the operation band's first line for merge, cherry-pick and revert; pending labels during Continue and Abort; disabled reasons for Fetch, Push, Merge and Recovery.
