//! An idle window draws no frames: once a view's reads and animations
//! settle, with the pointer resting on it, it asks for no repaint sooner
//! than [`IDLE`]. Adapted from figcraft `crates/ui-egui/src/idle_tests.rs`.

use super::*;
use std::time::{Duration, Instant};

/// The shortest repaint delay that counts as idle; a blinking text caret
/// wakes twice a second, so views are checked with no text field focused.
const IDLE: Duration = Duration::from_secs(1);
/// How long every frame must stay quiet before a view counts as settled.
const SETTLED: Duration = Duration::from_millis(1500);
/// How long a settled view must then stay idle.
const HOLD: Duration = Duration::from_secs(3);

struct Harness {
    ctx: egui::Context,
    app: Browser,
    start: Instant,
    /// Where the pointer rests; set once, then held still.
    pointer: Option<egui::Pos2>,
    _dir: tempfile::TempDir,
}

impl Harness {
    /// A repository with a few commits and one unstaged edit, opened in
    /// History.
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let path = std::fs::canonicalize(dir.path())?;
        let git = |args: &[&str]| -> Result<(), String> {
            let output = std::process::Command::new("git")
                .arg("-C")
                .arg(&path)
                .args(args)
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env(
                    "GIT_CONFIG_GLOBAL",
                    if cfg!(windows) { "NUL" } else { "/dev/null" },
                )
                .output()
                .map_err(|error| error.to_string())?;
            if output.status.success() {
                Ok(())
            } else {
                Err(String::from_utf8_lossy(&output.stderr).into_owned())
            }
        };
        git(&["init", "-q", "-b", "main"])?;
        git(&["config", "user.name", "Test"])?;
        git(&["config", "user.email", "test@example.org"])?;
        git(&["config", "core.autocrlf", "false"])?;
        for round in 0..5 {
            std::fs::write(path.join("a.txt"), format!("line {round}\n"))?;
            git(&["add", "a.txt"])?;
            git(&["commit", "-qm", &format!("commit {round}")])?;
        }
        std::fs::write(path.join("a.txt"), "edited\n")?;
        let mut harness = Self {
            ctx: egui::Context::default(),
            app: Browser::new(path),
            start: Instant::now(),
            pointer: None,
            _dir: dir,
        };
        harness.app.load(&harness.ctx);
        Ok(harness)
    }

    /// One frame at the real elapsed time; returns the repaint delay it
    /// asked for.
    fn frame(&mut self) -> Duration {
        let events = self
            .pointer
            .take()
            .map(|point| vec![egui::Event::PointerMoved(point)])
            .unwrap_or_default();
        let mut input = tests::frame_input(1120.0, 740.0, events);
        input.time = Some(self.start.elapsed().as_secs_f64());
        let app = &mut self.app;
        let output = tests::run_ui(&self.ctx, input, |ui| app.draw_ui(ui));
        output
            .viewport_output
            .get(&egui::ViewportId::ROOT)
            .map_or(Duration::MAX, |viewport| viewport.repaint_delay)
    }

    /// Draw frames until every frame for [`SETTLED`] of real time asked for
    /// nothing sooner than [`IDLE`]: long enough for the file watcher's
    /// debounce and a refresh it starts, which arrive late on Windows for
    /// files written just before opening. False if that never happens
    /// within 20 s, as with a refresh that keeps starting another.
    fn settle(&mut self) -> bool {
        let deadline = Instant::now() + Duration::from_secs(20);
        let mut quiet_since = Instant::now();
        while Instant::now() < deadline {
            if self.frame() < IDLE {
                quiet_since = Instant::now();
            } else if quiet_since.elapsed() >= SETTLED {
                return true;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        false
    }

    /// Rest the pointer at `point`, settle, then hold idle for [`HOLD`]. One
    /// late wake from outside (on Windows, an antivirus scan of the files
    /// the test just wrote reaches the file watcher seconds later) settles
    /// again and holds again; a second wake fails, as does a refresh that
    /// keeps starting another, however slowly.
    fn assert_idle(&mut self, what: &str, point: egui::Pos2) {
        self.pointer = Some(point);
        let mut wakes = Vec::new();
        loop {
            let settled = self.settle();
            assert!(
                settled,
                "{what}: still asking for frames: {:?}",
                self.ctx.repaint_causes()
            );
            match self.hold() {
                None => return,
                Some(cause) => wakes.push(cause),
            }
            assert!(
                wakes.len() < 2,
                "{what}: woke again after settling: {wakes:?}"
            );
        }
    }

    /// Frames for [`HOLD`] of real time; the first one that asks for a
    /// repaint sooner than [`IDLE`], with egui's causes, or `None`.
    fn hold(&mut self) -> Option<String> {
        let end = Instant::now() + HOLD;
        while Instant::now() < end {
            std::thread::sleep(Duration::from_millis(50));
            let delay = self.frame();
            if delay < IDLE {
                return Some(format!("{delay:?}: {:?}", self.ctx.repaint_causes()));
            }
        }
        None
    }
}

#[test]
fn every_view_is_idle_once_settled() -> Result<(), Box<dyn std::error::Error>> {
    let mut harness = Harness::new()?;
    let over_history = egui::pos2(600.0, 300.0);
    harness.assert_idle("History", over_history);
    assert!(harness.app.state.history.is_some(), "History did not load");

    harness.app.open_working(&harness.ctx);
    harness.assert_idle("Local Changes", egui::pos2(200.0, 200.0));
    harness.app.working = None;

    let commit = harness
        .app
        .state
        .rows
        .get(&1)
        .map(|commit| commit.id.clone());
    assert!(commit.is_some(), "History has no second row");
    harness.app.state.selected = commit;
    let location = harness.app.capture_return_location();
    harness.app.open_review(&harness.ctx, location);
    assert!(harness.app.review.is_some(), "Review did not open");
    harness.assert_idle("Review", egui::pos2(200.0, 200.0));
    harness.app.review = None;

    harness.app.open_preferences(PrefSection::General);
    harness.assert_idle("Preferences", egui::pos2(560.0, 370.0));
    Ok(())
}
