//! Screenshots for the documentation site (`website/`), rendered offscreen
//! like the README one: each shot opens the window on a fixture repository
//! that `scripts/docs_screenshots.py` builds, puts it in one state (a view,
//! a dialog, a popup) and draws it through egui_kittest and wgpu. Nothing
//! is committed: the docs workflow renders them before building the site.

use crate::{Browser, FrameCommand, PrefSection};
use eframe::egui;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

/// What the fixture script wrote: repository paths and commit IDs by name.
struct Fixture {
    root: PathBuf,
    commits: serde_json::Value,
}

impl Fixture {
    fn repo(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    fn commit(&self, name: &str) -> Result<String> {
        self.commits[name]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| format!("no commit {name} in the fixture").into())
    }
}

type Harness = egui_kittest::Harness<'static, Browser>;

/// The window on `repo` (or the welcome view for `None`), loaded and
/// settled, at `size` in `theme`.
fn window(repo: Option<&Path>, size: egui::Vec2, theme: egui::Theme) -> Harness {
    let mut app = Browser::new(repo.map_or_else(PathBuf::new, Path::to_path_buf));
    // Tests keep default settings; these shots read the fixture's own
    // configuration folder (recent repositories, clone folder).
    app.prefs = crate::settings::Prefs::load();
    app.recent = crate::settings::recent();
    app.recent_base = app.recent.clone();
    app.chrome.custom = crate::custom_titlebar(
        &crate::settings::Prefs::default(),
        crate::Desktop::OtherLinux,
    );
    let mut harness = egui_kittest::Harness::builder()
        .with_size(size)
        .with_pixels_per_point(2.0)
        .with_theme(theme)
        .with_context_setup(|ctx| {
            crate::phosphor::install(ctx);
            crate::wide_font::install(ctx);
        })
        .allow_missing_glyphs()
        .wgpu()
        .build_ui_state(|ui, app: &mut Browser| app.draw_ui(ui), app);
    let ctx = harness.ctx.clone();
    {
        let app = harness.state_mut();
        app.prefs.theme = match theme {
            egui::Theme::Dark => crate::settings::Theme::Dark,
            egui::Theme::Light => crate::settings::Theme::Light,
        };
        app.apply_prefs(&ctx);
        app.details_visible = true;
        if repo.is_some() {
            app.load(&ctx);
        }
    }
    let loaded = |app: &Browser| {
        repo.is_none()
            || (app.state.count > 0 && app.local_summary.is_some() && app.pending.is_none())
    };
    settle(&mut harness, loaded);
    harness
}

/// Step until `done` holds (60 s at most), then for another half second so
/// background reads and fades finish.
fn settle(harness: &mut Harness, done: impl Fn(&Browser) -> bool) {
    let started = Instant::now();
    while !done(harness.state()) && started.elapsed() < Duration::from_secs(60) {
        harness.step();
        std::thread::sleep(Duration::from_millis(10));
    }
    let calm = Instant::now();
    while calm.elapsed() < Duration::from_millis(600) {
        harness.step();
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn save(harness: &mut Harness, out: &Path, name: &str) -> Result {
    let image = harness.render()?;
    let path = out.join(format!("{name}.png"));
    image.save(&path)?;
    eprintln!("wrote {}", path.display());
    Ok(())
}

fn key(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}

const WIDE: egui::Vec2 = egui::vec2(1280.0, 800.0);

/// An animation recorded a frame at a time: `NAME.frames/NNN.png` and
/// `frames.json` (how long each frame shows, in ms), which
/// `scripts/docs_screenshots.py` encodes as `NAME.gif`.
struct Recording {
    dir: PathBuf,
    durations: Vec<u32>,
}

impl Recording {
    fn new(out: &Path, name: &str) -> Result<Self> {
        let dir = out.join(format!("{name}.frames"));
        std::fs::create_dir_all(&dir)?;
        Ok(Self {
            dir,
            durations: Vec::new(),
        })
    }

    /// Add the window as it is now, shown for `ms`.
    fn shot(&mut self, harness: &mut Harness, ms: u32) -> Result {
        let image = harness.render()?;
        image.save(self.dir.join(format!("{:03}.png", self.durations.len())))?;
        self.durations.push(ms);
        Ok(())
    }

    fn finish(self) -> Result {
        std::fs::write(
            self.dir.join("frames.json"),
            serde_json::to_vec(&self.durations)?,
        )?;
        eprintln!(
            "recorded {} ({} frames)",
            self.dir.display(),
            self.durations.len()
        );
        Ok(())
    }
}

/// Step for `ms` so background work and animations move on.
fn wait(harness: &mut Harness, ms: u64) {
    let until = Instant::now() + Duration::from_millis(ms);
    while Instant::now() < until {
        harness.step();
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn press(harness: &mut Harness, pressed: egui::Key, modifiers: egui::Modifiers) {
    harness.input_mut().events.push(key(pressed, modifiers));
    harness.step();
}

/// Render every documentation screenshot into `GG_DOCS_SCREENS` from the
/// fixtures described by `GG_DOCS_FIXTURES` (a JSON file).
#[test]
#[ignore = "renders the documentation screenshots: scripts/docs_screenshots.py runs it"]
fn docs_screenshots() -> Result {
    let out = PathBuf::from(std::env::var_os("GG_DOCS_SCREENS").ok_or("set GG_DOCS_SCREENS")?);
    let manifest =
        PathBuf::from(std::env::var_os("GG_DOCS_FIXTURES").ok_or("set GG_DOCS_FIXTURES")?);
    let described: serde_json::Value = serde_json::from_slice(&std::fs::read(&manifest)?)?;
    let fixture = Fixture {
        root: PathBuf::from(described["root"].as_str().ok_or("no root")?),
        commits: described["commits"].clone(),
    };
    std::fs::create_dir_all(&out)?;
    let only = std::env::var("GG_DOCS_ONLY").ok();
    let wanted = |name: &str| {
        only.as_deref()
            .is_none_or(|only| only.split(',').any(|o| o == name))
    };
    let api = fixture.repo("acme-api");
    let dark = egui::Theme::Dark;

    if wanted("welcome") {
        let mut harness = window(None, WIDE, dark);
        // The example project's committer, not the empty config the shots run with.
        harness
            .state_mut()
            .welcome
            .set_identity(gg_git::identity::Identity {
                name: Some("Ada Lovelace".into()),
                email: Some("ada@acme.test".into()),
            });
        for _ in 0..3 {
            harness.step();
        }
        save(&mut harness, &out, "welcome")?;
    }
    if wanted("history") {
        let mut harness = window(Some(&api), WIDE, dark);
        save(&mut harness, &out, "history")?;
    }
    if wanted("history-light") {
        let mut harness = window(Some(&api), WIDE, egui::Theme::Light);
        save(&mut harness, &out, "history-light")?;
    }
    if wanted("history-narrow") {
        let mut harness = window(Some(&api), egui::vec2(760.0, 620.0), dark);
        save(&mut harness, &out, "history-narrow")?;
    }
    if wanted("review") {
        let mut harness = window(Some(&api), WIDE, dark);
        let commit = fixture.commit("not_found_json")?;
        let ctx = harness.ctx.clone();
        harness
            .state_mut()
            .open_review_at(&ctx, commit, "Return 404 as JSON".into(), None);
        settle(&mut harness, |app| app.review.is_some());
        settle(&mut harness, |_| true);
        save(&mut harness, &out, "review")?;
    }
    if wanted("local-changes") {
        let mut harness = window(Some(&api), WIDE, dark);
        let ctx = harness.ctx.clone();
        harness.state_mut().open_working(&ctx);
        settle(&mut harness, |app| app.working.is_some());
        settle(&mut harness, |_| true);
        save(&mut harness, &out, "local-changes")?;
    }
    if wanted("search") {
        let mut harness = window(Some(&api), WIDE, dark);
        harness
            .input_mut()
            .events
            .push(key(egui::Key::K, egui::Modifiers::COMMAND));
        harness.step();
        harness
            .input_mut()
            .events
            .push(egui::Event::Text("rebase".into()));
        settle(&mut harness, |app| app.search.is_some());
        save(&mut harness, &out, "search")?;
    }
    if wanted("stashes") {
        let mut harness = window(Some(&api), WIDE, dark);
        let ctx = harness.ctx.clone();
        let (mut local, mut history) = (false, false);
        harness.state_mut().run_frame_command(
            &ctx,
            FrameCommand::Stashes,
            &mut local,
            &mut history,
        );
        settle(&mut harness, |_| true);
        save(&mut harness, &out, "stashes")?;
    }
    if wanted("rebase") {
        let mut harness = window(Some(&api), WIDE, dark);
        let from = fixture.commit("rebase_from")?;
        harness.state_mut().open_rewrite_dialog(&from)?;
        settle(&mut harness, |_| true);
        // Fold "WIP fix db typo" into the commit before it.
        for pressed in [egui::Key::J, egui::Key::F] {
            harness
                .input_mut()
                .events
                .push(key(pressed, egui::Modifiers::NONE));
            harness.step();
        }
        settle(&mut harness, |_| true);
        save(&mut harness, &out, "rebase")?;
    }
    if wanted("conflict") {
        let mut harness = window(Some(&fixture.repo("acme-api-merge")), WIDE, dark);
        let ctx = harness.ctx.clone();
        harness.state_mut().open_conflict_view(&ctx);
        settle(&mut harness, |app| app.conflict_view.is_some());
        settle(&mut harness, |_| true);
        save(&mut harness, &out, "conflict")?;
    }
    if wanted("recovery") {
        let mut harness = window(Some(&api), WIDE, dark);
        let ctx = harness.ctx.clone();
        harness.state_mut().open_recovery_window(&ctx);
        settle(&mut harness, |app| app.recovery.is_some());
        settle(&mut harness, |_| true);
        save(&mut harness, &out, "recovery")?;
    }
    if wanted("ignore") {
        // An untracked log, and ⋯ File actions open on it.
        std::fs::create_dir_all(api.join("logs"))?;
        std::fs::write(api.join("logs/requests.log"), "GET /orders 200\n")?;
        let mut harness = window(Some(&api), WIDE, dark);
        let ctx = harness.ctx.clone();
        harness.state_mut().open_working(&ctx);
        settle(&mut harness, |app| {
            app.working.as_ref().is_some_and(|working| !working.busy())
        });
        let started = Instant::now();
        while !harness
            .state_mut()
            .working
            .as_mut()
            .is_some_and(|working| working.select_unstaged(b"logs/requests.log", &ctx))
            && started.elapsed() < Duration::from_secs(30)
        {
            harness.step();
        }
        settle(&mut harness, |_| true);
        {
            use egui_kittest::kittest::Queryable;
            harness.get_by_label(crate::phosphor::DOTS_THREE).click();
        }
        settle(&mut harness, |_| true);
        save(&mut harness, &out, "ignore")?;
        std::fs::remove_dir_all(api.join("logs"))?;
    }
    for (name, pressed) in [("file-history", egui::Key::H), ("blame", egui::Key::B)] {
        if !wanted(name) {
            continue;
        }
        let mut harness = window(Some(&api), WIDE, dark);
        let commit = fixture.commit("not_found_json")?;
        let ctx = harness.ctx.clone();
        harness
            .state_mut()
            .open_review_at(&ctx, commit, "Return 404 as JSON".into(), None);
        settle(&mut harness, |app| app.review.is_some());
        settle(&mut harness, |_| true);
        harness
            .input_mut()
            .events
            .push(key(pressed, egui::Modifiers::NONE));
        settle(&mut harness, |_| true);
        save(&mut harness, &out, name)?;
    }
    for (name, command, row) in [
        ("new-tag", FrameCommand::NewTag, Some(2)),
        ("reset", FrameCommand::ResetHere, Some(4)),
        ("worktrees", FrameCommand::Worktrees, None),
    ] {
        if !wanted(name) {
            continue;
        }
        let mut harness = window(Some(&api), WIDE, dark);
        let ctx = harness.ctx.clone();
        if let Some(row) = row {
            harness.state_mut().state.selected_row = Some(row);
            settle(&mut harness, |_| true);
        }
        let (mut local, mut history) = (false, false);
        harness
            .state_mut()
            .run_frame_command(&ctx, command, &mut local, &mut history);
        settle(&mut harness, |_| true);
        save(&mut harness, &out, name)?;
    }
    for (name, command) in [
        ("clone", FrameCommand::Clone),
        ("new-repository", FrameCommand::NewRepository(None)),
    ] {
        if !wanted(name) {
            continue;
        }
        let mut harness = window(None, WIDE, dark);
        let ctx = harness.ctx.clone();
        let (mut local, mut history) = (false, false);
        harness
            .state_mut()
            .run_frame_command(&ctx, command, &mut local, &mut history);
        settle(&mut harness, |_| true);
        save(&mut harness, &out, name)?;
    }
    // The animations come last: two of them commit and rewrite.
    if wanted("demo-history") {
        // Scrolling a 20,000-commit History tree, then to its end and back.
        let mut harness = window(Some(&fixture.repo("monorepo")), WIDE, dark);
        let mut film = Recording::new(&out, "demo-history")?;
        film.shot(&mut harness, 900)?;
        for _ in 0..10 {
            press(&mut harness, egui::Key::PageDown, egui::Modifiers::NONE);
            wait(&mut harness, 60);
            film.shot(&mut harness, 140)?;
        }
        press(&mut harness, egui::Key::End, egui::Modifiers::NONE);
        wait(&mut harness, 300);
        film.shot(&mut harness, 1_400)?;
        press(&mut harness, egui::Key::Home, egui::Modifiers::NONE);
        wait(&mut harness, 300);
        film.shot(&mut harness, 1_000)?;
        film.finish()?;
    }
    if wanted("demo-rebase") {
        // Fold "WIP fix db typo" into the commit it fixes, start, then get
        // the old branch tip back from Recovery.
        let demo = fixture.repo("acme-api-demo");
        let mut harness = window(Some(&demo), WIDE, dark);
        let mut film = Recording::new(&out, "demo-rebase")?;
        film.shot(&mut harness, 900)?;
        let from = fixture.commit("rebase_from")?;
        harness.state_mut().open_rewrite_dialog(&from)?;
        settle(&mut harness, |_| true);
        film.shot(&mut harness, 1_200)?;
        press(&mut harness, egui::Key::J, egui::Modifiers::NONE);
        wait(&mut harness, 150);
        film.shot(&mut harness, 700)?;
        press(&mut harness, egui::Key::F, egui::Modifiers::NONE);
        wait(&mut harness, 150);
        film.shot(&mut harness, 1_600)?;
        press(&mut harness, egui::Key::Enter, egui::Modifiers::COMMAND);
        settle(&mut harness, |app| {
            app.rewrite_dialog.is_none() && !app.operation_busy()
        });
        film.shot(&mut harness, 2_000)?;
        let ctx = harness.ctx.clone();
        harness.state_mut().open_recovery_window(&ctx);
        settle(&mut harness, |app| app.recovery.is_some());
        settle(&mut harness, |_| true);
        film.shot(&mut harness, 2_400)?;
        film.finish()?;
    }
    if wanted("demo-stage") {
        // Stage one hunk of src/routes.rs and commit it.
        let demo = fixture.repo("acme-api-demo");
        git_stash_pop(&demo)?;
        let mut harness = window(Some(&demo), WIDE, dark);
        let ctx = harness.ctx.clone();
        harness.state_mut().open_working(&ctx);
        settle(&mut harness, |app| app.working.is_some());
        settle(&mut harness, |_| true);
        let started = Instant::now();
        while !harness
            .state_mut()
            .working
            .as_mut()
            .is_some_and(|working| working.select_unstaged(b"src/routes.rs", &ctx))
            && started.elapsed() < Duration::from_secs(30)
        {
            harness.step();
        }
        settle(&mut harness, |_| true);
        let mut film = Recording::new(&out, "demo-stage")?;
        film.shot(&mut harness, 1_400)?;
        // Hunk buttons ignore clicks for a moment after they appear.
        wait(&mut harness, 700);
        {
            use egui_kittest::kittest::Queryable;
            harness
                .query_all_by_label("Stage hunk")
                .next()
                .ok_or("no Stage hunk button")?
                .click();
        }
        settle(&mut harness, |_| true);
        film.shot(&mut harness, 1_400)?;
        let message = "Route /orders through the orders module";
        for end in (4..=message.len()).step_by(4).chain([message.len()]) {
            if let Some(working) = harness.state_mut().working.as_mut() {
                working.set_draft(message[..end].to_string());
            }
            harness.step();
            film.shot(&mut harness, 70)?;
        }
        film.shot(&mut harness, 600)?;
        if let Some(working) = harness.state_mut().working.as_mut() {
            working.commit_now(&ctx);
        }
        settle(&mut harness, |_| true);
        film.shot(&mut harness, 2_400)?;
        film.finish()?;
    }
    if wanted("preferences") {
        let mut harness = window(Some(&api), WIDE, dark);
        harness.state_mut().open_preferences(PrefSection::Keyboard);
        settle(&mut harness, |_| true);
        save(&mut harness, &out, "preferences")?;
    }
    Ok(())
}

/// Bring the demo copy's work in progress back for the staging demo.
fn git_stash_pop(repo: &Path) -> Result {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["stash", "pop", "-q"])
        .output()?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned().into());
    }
    Ok(())
}
