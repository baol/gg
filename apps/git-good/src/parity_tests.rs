//! RULE-16 parity: for every mutation the GUI starts exactly the Git
//! processes `git good <cmd>` starts, in the same order, and its follow-up
//! refreshes are background reads (XCT-30).
//!
//! Each case builds two identical fixtures. The GUI run drives the real
//! [`Browser`] method the button or menu calls and frames until the result is
//! shown and the window is quiet; the CLI run calls the command's handler
//! in-process. [`Recording`] keeps the Git command lines of each fixture
//! with the paths replaced by `<repo>` and `<remote>`.
//!
//! The GUI's records must start with the CLI's records exactly: those are
//! the mutation's own commands. Every one runs on a worker, and whatever
//! follows is a refresh read, also off the UI thread: no GUI mutation runs a
//! Git process on the UI thread.
use super::*;
use gg_git::process::{Record, Recording};
use std::ffi::OsString;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

mod timing;

struct Fixture {
    _dir: tempfile::TempDir,
    repo: PathBuf,
    remote: PathBuf,
}

/// Run fixture Git directly, outside the chokepoint, so setup never shows
/// in a recording.
fn git(path: &Path, args: &[&str]) -> Result<String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.org")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.org")
        .env("GIT_AUTHOR_DATE", "2026-01-01T00:00:00Z")
        .env("GIT_COMMITTER_DATE", "2026-01-01T00:00:00Z")
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// `main`: base, second, local (unpushed); `feature` off base with one
/// commit; `merged` at base; annotated tag `v1` at base; `origin` is a
/// local bare repository holding `main` at second, tracked by `main`.
/// Commit dates are fixed, so both fixtures have the same IDs.
fn fixture(prepare: fn(&Fixture) -> Result) -> Result<Fixture> {
    let dir = tempfile::tempdir()?;
    // Canonical, but without Windows' `\\?\` prefix, which Git reads as a
    // network host in the remote's URL.
    let root = std::fs::canonicalize(dir.path())?;
    let root = match root.to_str().and_then(|text| text.strip_prefix(r"\\?\")) {
        Some(plain) => std::path::PathBuf::from(plain),
        None => root,
    };
    let repo = root.join("repo");
    let remote = root.join("remote.git");
    std::fs::create_dir(&repo)?;
    std::fs::create_dir(&remote)?;
    git(&remote, &["init", "-q", "--bare", "-b", "main"])?;
    git(&repo, &["init", "-q", "-b", "main"])?;
    for (key, value) in [
        ("user.name", "Test"),
        ("user.email", "test@example.org"),
        ("core.autocrlf", "false"),
        ("commit.gpgsign", "false"),
        ("tag.gpgsign", "false"),
    ] {
        git(&repo, &["config", key, value])?;
    }
    std::fs::write(repo.join("a"), b"base\n")?;
    git(&repo, &["add", "a"])?;
    git(&repo, &["commit", "-qm", "base"])?;
    git(&repo, &["tag", "-a", "v1", "-m", "v1"])?;
    git(&repo, &["branch", "merged"])?;
    git(&repo, &["switch", "-q", "-c", "feature"])?;
    std::fs::write(repo.join("b"), b"feature\n")?;
    git(&repo, &["add", "b"])?;
    git(&repo, &["commit", "-qm", "feature"])?;
    git(&repo, &["switch", "-q", "main"])?;
    std::fs::write(repo.join("a"), b"second\n")?;
    git(&repo, &["commit", "-qam", "second"])?;
    let url = remote.to_string_lossy().into_owned();
    git(&repo, &["remote", "add", "origin", &url])?;
    git(&repo, &["push", "-q", "-u", "origin", "main"])?;
    std::fs::write(repo.join("a"), b"local\n")?;
    git(&repo, &["commit", "-qam", "local"])?;
    let fixture = Fixture {
        _dir: dir,
        repo,
        remote,
    };
    prepare(&fixture)?;
    Ok(fixture)
}

fn recording(fixture: &Fixture) -> Recording {
    Recording::start(&[(&fixture.repo, "<repo>"), (&fixture.remote, "<remote>")])
}

/// The CLI run: `args` after `git good <cmd>`, with `--repo` added.
fn cli(
    prepare: fn(&Fixture) -> Result,
    args: impl FnOnce(&Fixture) -> Result<Vec<String>>,
    run: fn(std::vec::IntoIter<OsString>) -> Result,
) -> Result<Vec<Record>> {
    let fixture = fixture(prepare)?;
    let mut args = args(&fixture)?;
    args.push("--repo".into());
    args.push(fixture.repo.to_string_lossy().into_owned());
    let args = args.into_iter().map(OsString::from).collect::<Vec<_>>();
    let recording = recording(&fixture);
    run(args.into_iter())?;
    Ok(recording.take())
}

struct Gui {
    ctx: egui::Context,
    app: Browser,
    recording: Recording,
    fixture: Fixture,
}

impl Gui {
    /// Open the fixture in a window and wait until it is quiet.
    fn open(prepare: fn(&Fixture) -> Result) -> Result<Self> {
        let fixture = fixture(prepare)?;
        let recording = recording(&fixture);
        let ctx = egui::Context::default();
        let mut app = Browser::new(fixture.repo.clone());
        app.prefs = settings::Prefs::load_from(&fixture.repo.join("../settings.json"));
        app.load(&ctx);
        let mut gui = Self {
            ctx,
            app,
            recording,
            fixture,
        };
        gui.until(|app| app.pending.is_none() && app.state.history.is_some())?;
        gui.quiet()?;
        let _ = gui.recording.take();
        Ok(gui)
    }

    fn frame(&mut self, events: Vec<egui::Event>) {
        let app = &mut self.app;
        let _ = crate::tests::run_ui(
            &self.ctx,
            crate::tests::frame_input(1120.0, 740.0, events),
            |ui| app.draw_ui(ui),
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    fn until(&mut self, done: impl Fn(&Browser) -> bool) -> Result {
        for _ in 0..2000 {
            if done(&self.app) {
                return Ok(());
            }
            self.frame(vec![]);
        }
        Err("the GUI did not finish".into())
    }

    fn idle(&self) -> bool {
        // A history read waits while Local Changes hides History, so
        // `pending` is not part of quiet; its processes still are.
        let app = &self.app;
        app.branch.is_none()
            && app.network.is_none()
            && app.operation_worker.is_none()
            && app.local_summary_job.is_none()
            && app.working.as_ref().is_none_or(|working| !working.busy())
    }

    /// Frame until no worker runs and no Git process has started for a
    /// while (longer than the watcher's settle time).
    fn quiet(&mut self) -> Result {
        let mut seen = self.recording.records().len();
        let mut still = std::time::Instant::now();
        for _ in 0..4000 {
            self.frame(vec![]);
            let now = self.recording.records().len();
            if now != seen || !self.idle() {
                seen = now;
                still = std::time::Instant::now();
            } else if still.elapsed() > std::time::Duration::from_millis(800) {
                return Ok(());
            }
        }
        let app = &self.app;
        Err(format!(
            "the GUI never went quiet: pending {} branch {} network {} operation {} summary {} working busy {:?}; last {:?}",
            app.pending.is_some(),
            app.branch.is_some(),
            app.network.is_some(),
            app.operation_worker.is_some(),
            app.local_summary_job.is_some(),
            app.working.as_ref().map(|working| working.busy()),
            self.recording.records().last().map(Record::line),
        )
        .into())
    }

    /// Everything the GUI started since it opened, once quiet again.
    fn finish(mut self) -> Result<Vec<Record>> {
        self.quiet()?;
        Ok(self.recording.take())
    }
}

/// Command lines with the backup refs' creation time (nanoseconds since
/// the epoch, `refs/gg/backup/<kind>/<time>/…`) replaced by `<time>`.
fn lines(records: &[Record]) -> Vec<String> {
    records
        .iter()
        .map(|record| untimed(&record.line()))
        .collect()
}

fn untimed(line: &str) -> String {
    const BACKUP: &str = "refs/gg/backup/";
    let mut out = String::new();
    let mut rest = line;
    while let Some(start) = rest.find(BACKUP) {
        let (before, after) = rest.split_at(start + BACKUP.len());
        out.push_str(before);
        rest = after;
        if let Some(slash) = rest.find('/') {
            let (kind, after) = rest.split_at(slash + 1);
            out.push_str(kind);
            let digits = after.bytes().take_while(u8::is_ascii_digit).count();
            if digits > 0 {
                out.push_str("<time>");
            }
            rest = &after[digits..];
        }
    }
    out.push_str(rest);
    out
}

/// The GUI starts with the CLI's commands, then only background reads, and
/// runs none of them on the UI thread.
fn assert_parity(case: &str, gui: &[Record], cli: &[Record]) {
    let report = format!(
        "{case}\nCLI:\n  {}\nGUI (* = UI thread):\n  {}",
        lines(cli).join("\n  "),
        gui.iter()
            .map(|record| format!(
                "{}{}",
                if record.on_recording_thread { "* " } else { "" },
                record.line()
            ))
            .collect::<Vec<_>>()
            .join("\n  ")
    );
    assert!(!cli.is_empty(), "the recorder saw no CLI command: {report}");
    assert!(
        gui.len() >= cli.len() && lines(&gui[..cli.len()]) == lines(cli),
        "the GUI's commands differ from the CLI's: {report}"
    );
    let (mutation, refresh) = gui.split_at(cli.len());
    for record in mutation {
        assert!(
            !record.on_recording_thread,
            "{} runs on the UI thread: {report}",
            record.line()
        );
    }
    for record in refresh {
        assert!(
            !record.on_recording_thread,
            "refresh read {} runs on the UI thread: {report}",
            record.line()
        );
    }
}

fn none(_: &Fixture) -> Result {
    Ok(())
}

fn dirty(fixture: &Fixture) -> Result {
    std::fs::write(fixture.repo.join("a"), b"changed\n")?;
    Ok(())
}

fn stashed(fixture: &Fixture) -> Result {
    dirty(fixture)?;
    git(&fixture.repo, &["stash", "push", "-qm", "work"])?;
    Ok(())
}

fn staged(fixture: &Fixture) -> Result {
    std::fs::write(fixture.repo.join("c"), b"new\n")?;
    git(&fixture.repo, &["add", "c"])?;
    Ok(())
}

fn rev(fixture: &Fixture, revision: &str) -> Result<String> {
    git(&fixture.repo, &["rev-parse", revision])
}

fn stash_cli(args: std::vec::IntoIter<OsString>) -> Result {
    stash_cli::run(args)
}

fn branch_cli(args: std::vec::IntoIter<OsString>) -> Result {
    branch_cli::manage(args)
}

fn switch_cli(args: std::vec::IntoIter<OsString>) -> Result {
    branch_cli::run(args)
}

fn tag_cli(args: std::vec::IntoIter<OsString>) -> Result {
    tag_cli::run(args)
}

fn reset_cli(args: std::vec::IntoIter<OsString>) -> Result {
    reset_cli::run(args)
}

fn commit_cli(args: std::vec::IntoIter<OsString>) -> Result {
    commit_cli::run(args)
}

fn cherry_pick_cli(args: std::vec::IntoIter<OsString>) -> Result {
    mutation_cli::cherry_pick(args)
}

fn revert_cli(args: std::vec::IntoIter<OsString>) -> Result {
    mutation_cli::revert(args)
}

fn merge_cli(args: std::vec::IntoIter<OsString>) -> Result {
    merge_cli::run(args)
}

fn fetch_cli(args: std::vec::IntoIter<OsString>) -> Result {
    sync_cli::fetch(args)
}

fn push_cli(args: std::vec::IntoIter<OsString>) -> Result {
    sync_cli::push(args)
}

fn strings(args: &[&str]) -> Vec<String> {
    args.iter().map(|arg| (*arg).to_string()).collect()
}

fn enter() -> egui::Event {
    egui::Event::Key {
        key: egui::Key::Enter,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }
}

#[test]
fn stash_push_runs_the_cli_commands() -> Result {
    let mut gui = Gui::open(dirty)?;
    gui.app.push_stash(&gui.ctx, "work", false, false)?;
    let gui = gui.finish()?;
    let cli = cli(dirty, |_| Ok(strings(&["push", "-m", "work"])), stash_cli)?;
    assert_parity("stash push", &gui, &cli);
    Ok(())
}

fn stash_pick(case: &str, pick: StashPick, args: &'static [&'static str]) -> Result {
    let mut gui = Gui::open(stashed)?;
    gui.app.apply_stash_pick(&gui.ctx, pick);
    assert!(
        gui.app.stash_picker.error.is_none(),
        "{:?}",
        gui.app.stash_picker.error
    );
    let gui = gui.finish()?;
    let cli = cli(stashed, |_| Ok(strings(args)), stash_cli)?;
    assert_parity(case, &gui, &cli);
    Ok(())
}

#[test]
fn stash_apply_runs_the_cli_commands() -> Result {
    stash_pick(
        "stash apply",
        StashPick::Apply("stash@{0}".into()),
        &["apply", "stash@{0}"],
    )
}

#[test]
fn stash_pop_runs_the_cli_commands() -> Result {
    stash_pick(
        "stash pop",
        StashPick::Pop("stash@{0}".into()),
        &["pop", "stash@{0}"],
    )
}

#[test]
fn stash_drop_runs_the_cli_commands() -> Result {
    stash_pick(
        "stash drop",
        StashPick::Drop("stash@{0}".into()),
        &["drop", "stash@{0}"],
    )
}

#[test]
fn commit_runs_the_cli_commands() -> Result {
    let mut gui = Gui::open(staged)?;
    gui.app.open_working(&gui.ctx);
    gui.quiet()?;
    let _ = gui.recording.take();
    if let Some(working) = &mut gui.app.working {
        working.set_draft("add c".into());
    }
    let mut go_local = false;
    let mut go_history = false;
    let ctx = gui.ctx.clone();
    gui.app.run_frame_command(
        &ctx,
        FrameCommand::CommitNow,
        &mut go_local,
        &mut go_history,
    );
    gui.until(|app| app.working.as_ref().is_some_and(|working| !working.busy()))?;
    let gui = gui.finish()?;
    let cli = cli(staged, |_| Ok(strings(&["-m", "add c"])), commit_cli)?;
    assert_parity("commit", &gui, &cli);
    Ok(())
}

#[test]
fn branch_create_runs_the_cli_commands() -> Result {
    let mut gui = Gui::open(none)?;
    gui.app.open_new_branch();
    let start = match &mut gui.app.create_branch {
        Some(dialog) => {
            dialog.switch = false;
            dialog.commit.clone()
        }
        None => return Err("New branch did not open".into()),
    };
    gui.frame(vec![]);
    gui.frame(vec![egui::Event::Text("topic".into())]);
    gui.frame(vec![enter()]);
    // The dialog stays open until the worker reports back.
    gui.until(|app| app.operation_worker.is_none())?;
    assert!(gui.app.create_branch.is_none(), "Enter created the branch");
    let gui = gui.finish()?;
    let cli = cli(
        none,
        move |_| Ok(strings(&["--create", "topic", "--at", &start])),
        branch_cli,
    )?;
    assert_parity("branch create", &gui, &cli);
    Ok(())
}

/// Git Good has no rename command; the GUI must run what the backend's
/// rename runs, and nothing more.
#[test]
fn branch_rename_runs_the_backend_commands() -> Result {
    let mut gui = Gui::open(none)?;
    gui.app.rename_dialog = Some(RenameDialog {
        old: "feature".into(),
        new: "topic".into(),
        error: None,
    });
    gui.frame(vec![]);
    gui.frame(vec![enter()]);
    // The dialog stays open until the worker reports back.
    gui.until(|app| app.operation_worker.is_none())?;
    assert!(gui.app.rename_dialog.is_none(), "Enter renamed the branch");
    let gui = gui.finish()?;
    let fixture = fixture(none)?;
    let recording = recording(&fixture);
    gg_git::branch::rename(&fixture.repo, "feature", "topic")?;
    let cli = recording.take();
    assert_parity("branch rename", &gui, &cli);
    Ok(())
}

#[test]
fn branch_delete_runs_the_cli_commands() -> Result {
    let mut gui = Gui::open(none)?;
    gui.app.delete_branch(&gui.ctx, "merged", false)?;
    let gui = gui.finish()?;
    let cli = cli(none, |_| Ok(strings(&["--delete", "merged"])), branch_cli)?;
    assert_parity("branch delete", &gui, &cli);
    Ok(())
}

#[test]
fn switch_runs_the_cli_commands() -> Result {
    let mut gui = Gui::open(none)?;
    gui.app.start_branch(&gui.ctx, "feature".into(), false);
    gui.until(|app| app.branch.is_none())?;
    assert!(gui.app.branch_error.is_none(), "{:?}", gui.app.branch_error);
    let gui = gui.finish()?;
    let cli = cli(none, |_| Ok(strings(&["feature"])), switch_cli)?;
    assert_parity("switch", &gui, &cli);
    Ok(())
}

#[test]
fn tag_create_runs_the_cli_commands() -> Result {
    let mut gui = Gui::open(none)?;
    gui.app.open_new_tag();
    let target = match &mut gui.app.tag_create {
        Some(dialog) => {
            dialog.name = "v2".into();
            dialog.message = "second release".into();
            dialog.target.clone()
        }
        None => return Err("New tag did not open".into()),
    };
    gui.app.start_tag_create(&gui.ctx);
    gui.until(|app| {
        app.tag_create
            .as_ref()
            .is_none_or(|dialog| dialog.pending.is_none())
    })?;
    let gui = gui.finish()?;
    let cli = cli(
        none,
        move |_| {
            Ok(strings(&[
                "--create",
                "v2",
                "--at",
                &target,
                "-m",
                "second release",
            ]))
        },
        tag_cli,
    )?;
    assert_parity("tag create", &gui, &cli);
    Ok(())
}

#[test]
fn tag_delete_runs_the_cli_commands() -> Result {
    let mut gui = Gui::open(none)?;
    gui.app.delete_tag(&gui.ctx, "v1")?;
    let gui = gui.finish()?;
    let cli = cli(none, |_| Ok(strings(&["--delete", "v1"])), tag_cli)?;
    assert_parity("tag delete", &gui, &cli);
    Ok(())
}

fn reset(case: &str, mode: gg_engine::reset::Mode, flag: &'static str) -> Result {
    let mut gui = Gui::open(dirty)?;
    let base = rev(&gui.fixture, "HEAD~2")?;
    gui.app.start_reset(&gui.ctx, &base, mode)?;
    let gui = gui.finish()?;
    let cli = cli(
        dirty,
        move |fixture| Ok(vec![flag.to_string(), rev(fixture, "HEAD~2")?]),
        reset_cli,
    )?;
    assert_parity(case, &gui, &cli);
    Ok(())
}

#[test]
fn soft_reset_runs_the_cli_commands() -> Result {
    reset("reset soft", gg_engine::reset::Mode::Soft, "--soft")
}

#[test]
fn mixed_reset_runs_the_cli_commands() -> Result {
    reset("reset mixed", gg_engine::reset::Mode::Mixed, "--mixed")
}

#[test]
fn hard_reset_runs_the_cli_commands() -> Result {
    reset("reset hard", gg_engine::reset::Mode::Hard, "--hard")
}

fn commit_action(
    case: &str,
    revision: &'static str,
    action: CommitAction,
    run: fn(std::vec::IntoIter<OsString>) -> Result,
) -> Result {
    let mut gui = Gui::open(none)?;
    let commit = rev(&gui.fixture, revision)?;
    gui.app.run_commit_action(&gui.ctx, commit, action);
    assert!(
        gui.app.mutation_error.is_none(),
        "{:?}",
        gui.app.mutation_error
    );
    let gui = gui.finish()?;
    let cli = cli(none, move |fixture| Ok(vec![rev(fixture, revision)?]), run)?;
    assert_parity(case, &gui, &cli);
    Ok(())
}

#[test]
fn cherry_pick_runs_the_cli_commands() -> Result {
    commit_action(
        "cherry-pick",
        "feature",
        CommitAction::CherryPick,
        cherry_pick_cli,
    )
}

/// A second commit on `feature`, so two can be picked.
fn two_on_feature(fixture: &Fixture) -> Result {
    git(&fixture.repo, &["switch", "-q", "feature"])?;
    std::fs::write(fixture.repo.join("c"), b"more\n")?;
    git(&fixture.repo, &["add", "c"])?;
    git(&fixture.repo, &["commit", "-qm", "more feature"])?;
    git(&fixture.repo, &["switch", "-q", "main"])?;
    Ok(())
}

#[test]
fn cherry_picking_two_picked_commits_runs_the_cli_commands() -> Result {
    let mut gui = Gui::open(two_on_feature)?;
    let ids = [
        rev(&gui.fixture, "feature~1")?,
        rev(&gui.fixture, "feature")?,
    ];
    let rows: Vec<usize> = ids
        .iter()
        .filter_map(|id| {
            gui.app
                .state
                .rows
                .iter()
                .find(|(_, commit)| &commit.id == id)
                .map(|(row, _)| *row)
        })
        .collect();
    assert_eq!(rows.len(), 2, "both feature commits are listed");
    // Click the newer, then Ctrl/Cmd-click the older.
    let _ = gui.app.state.click_row(rows[1], false, false);
    let _ = gui.app.state.click_row(rows[0], false, true);
    let picked = gui.app.state.picks_oldest_first().map(|commits| {
        commits
            .iter()
            .map(|commit| commit.id.clone())
            .collect::<Vec<_>>()
    });
    assert_eq!(picked.as_deref(), Some(&ids[..]), "oldest first");
    let selected = gui.app.state.selected.clone().ok_or("no selection")?;
    gui.app
        .run_commit_action(&gui.ctx, selected, CommitAction::CherryPick);
    assert!(
        gui.app.mutation_error.is_none(),
        "{:?}",
        gui.app.mutation_error
    );
    let gui = gui.finish()?;
    let cli = cli(
        two_on_feature,
        |fixture| Ok(vec![rev(fixture, "feature~1")?, rev(fixture, "feature")?]),
        cherry_pick_cli,
    )?;
    assert_parity("cherry-pick two", &gui, &cli);
    Ok(())
}

#[test]
fn revert_runs_the_cli_commands() -> Result {
    commit_action("revert", "HEAD", CommitAction::Revert, revert_cli)
}

#[test]
fn merge_runs_the_cli_commands() -> Result {
    let mut gui = Gui::open(none)?;
    gui.app.start_merge(&gui.ctx, "feature", false)?;
    gui.until(|app| app.operation_worker.is_none())?;
    let gui = gui.finish()?;
    let cli = cli(none, |_| Ok(strings(&["feature"])), merge_cli)?;
    assert_parity("merge", &gui, &cli);
    Ok(())
}

#[test]
fn fetch_runs_the_cli_commands() -> Result {
    let mut gui = Gui::open(none)?;
    let prune = gui.app.prefs.fetch_prune;
    let mut go_local = false;
    let mut go_history = false;
    let ctx = gui.ctx.clone();
    gui.app
        .run_frame_command(&ctx, FrameCommand::Fetch, &mut go_local, &mut go_history);
    gui.until(|app| app.network.is_none())?;
    assert!(
        gui.app.network_error.is_none(),
        "{:?}",
        gui.app.network_error
    );
    let gui = gui.finish()?;
    let args = if prune {
        strings(&["--prune"])
    } else {
        Vec::new()
    };
    let cli = cli(none, move |_| Ok(args), fetch_cli)?;
    assert_parity("fetch", &gui, &cli);
    Ok(())
}

#[test]
fn push_runs_the_cli_commands() -> Result {
    let mut gui = Gui::open(none)?;
    let mut go_local = false;
    let mut go_history = false;
    let ctx = gui.ctx.clone();
    gui.app
        .run_frame_command(&ctx, FrameCommand::Push, &mut go_local, &mut go_history);
    gui.until(|app| app.network.is_none())?;
    assert!(
        gui.app.network_error.is_none(),
        "{:?}",
        gui.app.network_error
    );
    let gui = gui.finish()?;
    let cli = cli(none, |_| Ok(Vec::new()), push_cli)?;
    assert_parity("push", &gui, &cli);
    Ok(())
}
