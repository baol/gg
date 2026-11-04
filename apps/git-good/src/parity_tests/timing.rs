//! RULE-16 timing, run nightly: each mutation through the GUI path and
//! through its CLI handler, on fresh fixtures, interleaved.
//!
//! The GUI is timed from the call its button or menu makes until the
//! receipt is applied to the window's state (the worker reported and the
//! result is published), not until the background refresh after it ends.
//! Between polls it sleeps until a worker asks for a repaint, as the
//! window's event loop does.
//! The CLI is timed around its handler, after waiting as long as the window
//! waits to go quiet. Setup, fixture creation and the quiet time between
//! runs are not timed. A mutation fails when its GUI
//! median exceeds 1.01 × its CLI median + 2 ms.
//!
//! `cargo test --release -p git-good --locked -- --ignored --nocapture
//! gui_mutations_cost_what_the_cli_costs`; `GG_TIMING_RUNS` sets the runs per
//! side (default 30, at least 15) and `GG_TIMING_CASES` keeps the cases whose
//! name contains one of its comma-separated words.
use super::*;
use std::time::{Duration, Instant};

/// How long the window stays quiet before its timed run ([`Gui::quiet`]).
const QUIET: Duration = Duration::from_millis(800);

/// The timed GUI call, prepared by untimed setup.
type GuiRun = Box<dyn FnOnce(&mut Gui) -> Result>;
/// The timed CLI call, prepared by untimed setup.
type CliRun = Box<dyn FnOnce() -> Result>;

struct Case {
    name: &'static str,
    prepare: fn(&Fixture) -> Result,
    /// Untimed setup in the open window; returns the timed call.
    gui: fn(&mut Gui) -> Result<GuiRun>,
    /// The result is published.
    done: fn(&Browser) -> bool,
    /// Untimed setup on the CLI's fixture; returns the timed call.
    cli: fn(&Fixture) -> Result<CliRun>,
}

/// The CLI handler with `args` and `--repo`.
fn handler(
    fixture: &Fixture,
    args: Vec<String>,
    run: fn(std::vec::IntoIter<OsString>) -> Result,
) -> Result<CliRun> {
    let mut args = args;
    args.push("--repo".into());
    args.push(fixture.repo.to_string_lossy().into_owned());
    let args = args.into_iter().map(OsString::from).collect::<Vec<_>>();
    Ok(Box::new(move || run(args.into_iter())))
}

fn operation_done(app: &Browser) -> bool {
    app.operation_worker.is_none()
}

fn failure(app: &Browser) -> Option<String> {
    app.mutation_error
        .clone()
        .or_else(|| app.branch_error.clone())
        .or_else(|| app.network_error.clone())
        .or_else(|| app.stash_picker.error.clone())
        .or_else(|| {
            app.outcome
                .as_ref()
                .filter(|outcome| outcome.severity == OutcomeSeverity::Error)
                .map(|outcome| outcome.summary.clone())
        })
}

fn frame_command(gui: &mut Gui, command: FrameCommand) {
    let (mut local, mut history) = (false, false);
    let ctx = gui.ctx.clone();
    gui.app
        .run_frame_command(&ctx, command, &mut local, &mut history);
}

fn stash_pick(gui: &mut Gui, pick: StashPick) {
    let ctx = gui.ctx.clone();
    gui.app.apply_stash_pick(&ctx, pick);
}

fn cases() -> Vec<Case> {
    vec![
        Case {
            name: "stash push",
            prepare: dirty,
            gui: |_| {
                Ok(Box::new(|gui| {
                    let ctx = gui.ctx.clone();
                    gui.app.push_stash(&ctx, "work", false, false)?;
                    Ok(())
                }))
            },
            done: operation_done,
            cli: |fixture| handler(fixture, strings(&["push", "-m", "work"]), stash_cli),
        },
        Case {
            name: "stash apply",
            prepare: stashed,
            gui: |_| {
                Ok(Box::new(|gui| {
                    stash_pick(gui, StashPick::Apply("stash@{0}".into()));
                    Ok(())
                }))
            },
            done: operation_done,
            cli: |fixture| handler(fixture, strings(&["apply", "stash@{0}"]), stash_cli),
        },
        Case {
            name: "stash pop",
            prepare: stashed,
            gui: |_| {
                Ok(Box::new(|gui| {
                    stash_pick(gui, StashPick::Pop("stash@{0}".into()));
                    Ok(())
                }))
            },
            done: operation_done,
            cli: |fixture| handler(fixture, strings(&["pop", "stash@{0}"]), stash_cli),
        },
        Case {
            name: "stash drop",
            prepare: stashed,
            gui: |_| {
                Ok(Box::new(|gui| {
                    stash_pick(gui, StashPick::Drop("stash@{0}".into()));
                    Ok(())
                }))
            },
            done: operation_done,
            cli: |fixture| handler(fixture, strings(&["drop", "stash@{0}"]), stash_cli),
        },
        Case {
            name: "commit",
            prepare: staged,
            gui: |gui| {
                let ctx = gui.ctx.clone();
                gui.app.open_working(&ctx);
                gui.quiet()?;
                if let Some(working) = &mut gui.app.working {
                    working.set_draft("add c".into());
                }
                Ok(Box::new(|gui| {
                    let ctx = gui.ctx.clone();
                    let mut go_local = false;
                    let mut go_history = false;
                    gui.app.run_frame_command(
                        &ctx,
                        FrameCommand::CommitNow,
                        &mut go_local,
                        &mut go_history,
                    );
                    Ok(())
                }))
            },
            done: |app| app.working.as_ref().is_some_and(|working| !working.busy()),
            cli: |fixture| handler(fixture, strings(&["-m", "add c"]), commit_cli),
        },
        Case {
            name: "branch create",
            prepare: none,
            gui: |gui| {
                gui.app.open_new_branch();
                Ok(Box::new(|gui| {
                    let ctx = gui.ctx.clone();
                    gui.app.submit_create_branch(&ctx, "topic".into(), false);
                    Ok(())
                }))
            },
            done: operation_done,
            cli: |fixture| {
                let head = rev(fixture, "HEAD")?;
                handler(
                    fixture,
                    strings(&["--create", "topic", "--at", &head]),
                    branch_cli,
                )
            },
        },
        Case {
            name: "branch rename",
            prepare: none,
            gui: |_| {
                Ok(Box::new(|gui| {
                    let ctx = gui.ctx.clone();
                    gui.app
                        .submit_rename(&ctx, "feature".into(), "topic".into());
                    Ok(())
                }))
            },
            done: operation_done,
            // No CLI command renames: the backend call the CLI would make.
            cli: |fixture| {
                let repo = fixture.repo.clone();
                Ok(Box::new(move || {
                    gg_git::branch::rename(&repo, "feature", "topic")?;
                    Ok(())
                }))
            },
        },
        Case {
            name: "branch delete",
            prepare: none,
            gui: |_| {
                Ok(Box::new(|gui| {
                    let ctx = gui.ctx.clone();
                    gui.app.delete_branch(&ctx, "merged", false)?;
                    Ok(())
                }))
            },
            done: operation_done,
            cli: |fixture| handler(fixture, strings(&["--delete", "merged"]), branch_cli),
        },
        Case {
            name: "switch",
            prepare: none,
            gui: |_| {
                Ok(Box::new(|gui| {
                    let ctx = gui.ctx.clone();
                    gui.app.start_branch(&ctx, "feature".into(), false);
                    Ok(())
                }))
            },
            done: |app| app.branch.is_none(),
            cli: |fixture| handler(fixture, strings(&["feature"]), switch_cli),
        },
        Case {
            name: "tag create",
            prepare: none,
            gui: |_| {
                Ok(Box::new(|gui| {
                    gui.app.open_new_tag();
                    if let Some(dialog) = &mut gui.app.tag_create {
                        dialog.name = "v2".into();
                        dialog.message = "second release".into();
                    }
                    let ctx = gui.ctx.clone();
                    gui.app.start_tag_create(&ctx);
                    Ok(())
                }))
            },
            done: |app| app.tag_create.is_none(),
            cli: |fixture| {
                let head = rev(fixture, "HEAD")?;
                handler(
                    fixture,
                    strings(&["--create", "v2", "--at", &head, "-m", "second release"]),
                    tag_cli,
                )
            },
        },
        Case {
            name: "tag delete",
            prepare: none,
            gui: |_| {
                Ok(Box::new(|gui| {
                    let ctx = gui.ctx.clone();
                    gui.app.delete_tag(&ctx, "v1")?;
                    Ok(())
                }))
            },
            done: operation_done,
            cli: |fixture| handler(fixture, strings(&["--delete", "v1"]), tag_cli),
        },
        Case {
            name: "reset soft",
            prepare: dirty,
            gui: |gui| reset_gui(gui, gg_engine::reset::Mode::Soft),
            done: operation_done,
            cli: |fixture| reset_cli_run(fixture, "--soft"),
        },
        Case {
            name: "reset mixed",
            prepare: dirty,
            gui: |gui| reset_gui(gui, gg_engine::reset::Mode::Mixed),
            done: operation_done,
            cli: |fixture| reset_cli_run(fixture, "--mixed"),
        },
        Case {
            name: "reset hard",
            prepare: dirty,
            gui: |gui| reset_gui(gui, gg_engine::reset::Mode::Hard),
            done: operation_done,
            cli: |fixture| reset_cli_run(fixture, "--hard"),
        },
        Case {
            name: "cherry-pick",
            prepare: none,
            gui: |gui| action_gui(gui, "feature", CommitAction::CherryPick),
            done: operation_done,
            cli: |fixture| handler(fixture, vec![rev(fixture, "feature")?], cherry_pick_cli),
        },
        Case {
            name: "revert",
            prepare: none,
            gui: |gui| action_gui(gui, "HEAD", CommitAction::Revert),
            done: operation_done,
            cli: |fixture| handler(fixture, vec![rev(fixture, "HEAD")?], revert_cli),
        },
        Case {
            name: "merge",
            prepare: none,
            gui: |_| {
                Ok(Box::new(|gui| {
                    let ctx = gui.ctx.clone();
                    gui.app.start_merge(&ctx, "feature", false)?;
                    Ok(())
                }))
            },
            done: operation_done,
            cli: |fixture| handler(fixture, strings(&["feature"]), merge_cli),
        },
        Case {
            name: "fetch",
            prepare: none,
            gui: |_| {
                Ok(Box::new(|gui| {
                    frame_command(gui, FrameCommand::Fetch);
                    Ok(())
                }))
            },
            done: |app| app.network.is_none(),
            cli: |fixture| {
                // The GUI fetches with the default preference.
                let args = if settings::Prefs::default().fetch_prune {
                    strings(&["--prune"])
                } else {
                    Vec::new()
                };
                handler(fixture, args, fetch_cli)
            },
        },
        Case {
            name: "push",
            prepare: none,
            gui: |_| {
                Ok(Box::new(|gui| {
                    frame_command(gui, FrameCommand::Push);
                    Ok(())
                }))
            },
            done: |app| app.network.is_none(),
            cli: |fixture| handler(fixture, Vec::new(), push_cli),
        },
    ]
}

fn reset_gui(gui: &mut Gui, mode: gg_engine::reset::Mode) -> Result<GuiRun> {
    let base = rev(&gui.fixture, "HEAD~2")?;
    Ok(Box::new(move |gui| {
        let ctx = gui.ctx.clone();
        gui.app.start_reset(&ctx, &base, mode)?;
        Ok(())
    }))
}

fn reset_cli_run(fixture: &Fixture, flag: &str) -> Result<CliRun> {
    handler(
        fixture,
        vec![flag.to_string(), rev(fixture, "HEAD~2")?],
        reset_cli,
    )
}

fn action_gui(gui: &mut Gui, revision: &str, action: CommitAction) -> Result<GuiRun> {
    let commit = rev(&gui.fixture, revision)?;
    Ok(Box::new(move |gui| {
        let ctx = gui.ctx.clone();
        gui.app.run_commit_action(&ctx, commit, action);
        Ok(())
    }))
}

/// Publish what the workers reported, as a frame does, without drawing.
fn publish(gui: &mut Gui) {
    let ctx = gui.ctx.clone();
    gui.app.poll_jobs(&ctx);
    gui.app.apply_tag_create(&ctx);
    if let Some(working) = &mut gui.app.working {
        working.poll_commit(&ctx);
    }
}

fn time_gui(case: &Case) -> Result<Duration> {
    let mut gui = Gui::open(case.prepare)?;
    let run = (case.gui)(&mut gui)?;
    // As the window's event loop does: sleep until a worker asks for a
    // repaint, then publish what arrived.
    let (wake, woken) = std::sync::mpsc::channel();
    let wake = std::sync::Mutex::new(wake);
    gui.ctx.set_request_repaint_callback(move |_| {
        if let Ok(wake) = wake.lock() {
            let _ = wake.send(());
        }
    });
    let started = Instant::now();
    run(&mut gui)?;
    let deadline = started + Duration::from_secs(60);
    loop {
        publish(&mut gui);
        if (case.done)(&gui.app) {
            break;
        }
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(format!("{}: the GUI never published a result", case.name).into());
        }
        // A missed wake costs at most this poll, never a result.
        let _ = woken.recv_timeout(left.min(Duration::from_millis(50)));
        // egui reports a repaint request once per pass; an empty pass lets
        // the next request through, as the window's next frame would.
        let _ = gui.ctx.run_ui(egui::RawInput::default(), |_| {});
    }
    let elapsed = started.elapsed();
    if let Some(error) = failure(&gui.app) {
        return Err(format!("{}: the GUI failed: {error}", case.name).into());
    }
    // The refresh after the result runs before the next timed run.
    gui.finish()?;
    Ok(elapsed)
}

fn time_cli(case: &Case) -> Result<Duration> {
    let fixture = fixture(case.prepare)?;
    let run = (case.cli)(&fixture)?;
    // Both sides hold a recording, so both pay its bookkeeping.
    let _recording = recording(&fixture);
    // The GUI's run starts after the window went quiet; a Git process
    // started right after the setup's runs at a different speed, so the CLI
    // waits as long.
    std::thread::sleep(QUIET);
    let started = Instant::now();
    run()?;
    Ok(started.elapsed())
}

fn median(mut times: Vec<Duration>) -> Duration {
    times.sort();
    let middle = times.len() / 2;
    if times.len().is_multiple_of(2) {
        (times[middle - 1] + times[middle]) / 2
    } else {
        times[middle]
    }
}

fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

#[test]
#[ignore = "timing: run nightly, in release, on one fixed runner"]
fn gui_mutations_cost_what_the_cli_costs() -> Result {
    let runs = std::env::var("GG_TIMING_RUNS")
        .ok()
        .and_then(|runs| runs.parse::<usize>().ok())
        .unwrap_or(30)
        .max(15);
    let only = std::env::var("GG_TIMING_CASES").unwrap_or_default();
    let skipped = |name: &str| {
        let mut words = only
            .split(',')
            .map(str::trim)
            .filter(|word| !word.is_empty());
        !only.trim().is_empty() && !words.any(|word| name.contains(word))
    };
    let mut report = vec![format!(
        "{:<14} {:>9} {:>9} {:>7} {:>9}  ({runs} runs each, interleaved)",
        "mutation", "CLI ms", "GUI ms", "ratio", "budget"
    )];
    let mut ratios = Vec::new();
    let mut over = Vec::new();
    for case in cases() {
        if skipped(case.name) {
            continue;
        }
        let mut gui = Vec::with_capacity(runs);
        let mut cli = Vec::with_capacity(runs);
        for run in 0..runs {
            // Alternate which side goes first.
            if run % 2 == 0 {
                gui.push(time_gui(&case)?);
                cli.push(time_cli(&case)?);
            } else {
                cli.push(time_cli(&case)?);
                gui.push(time_gui(&case)?);
            }
        }
        let (gui, cli) = (median(gui), median(cli));
        let budget = cli.mul_f64(1.01) + Duration::from_millis(2);
        let ratio = gui.as_secs_f64() / cli.as_secs_f64().max(f64::EPSILON);
        ratios.push(ratio);
        let fails = gui > budget;
        if fails {
            over.push(case.name);
        }
        report.push(format!(
            "{:<14} {:>9.2} {:>9.2} {:>7.3} {:>9.2}{}",
            case.name,
            ms(cli),
            ms(gui),
            ratio,
            ms(budget),
            if fails { "  OVER" } else { "" }
        ));
    }
    ratios.sort_by(f64::total_cmp);
    if let Some(ratio) = ratios.get(ratios.len() / 2) {
        report.push(format!("median GUI/CLI ratio {ratio:.3}"));
    }
    let report = report.join("\n");
    eprintln!("{report}");
    assert!(
        over.is_empty(),
        "GUI median over 1.01 × CLI median + 2 ms for {over:?}:\n{report}"
    );
    Ok(())
}
