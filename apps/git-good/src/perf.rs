//! Performance budgets. Each benchmark writes its numbers as JSON to
//! `target/perf/<name>.json` (or `GG_PERF_OUT`) with the process's peak
//! resident memory, and fails when a number is over its limit in
//! `perf/budgets.json`, or more than [`BASELINE_SLACK`] over the same
//! number in a baseline folder named by `GG_PERF_BASELINE` (an earlier
//! run's `target/perf`). The benchmarks build their own fixtures and are
//! ignored by default: run them in release, as the nightly Timing workflow
//! does. Source: photocraft `crates/testkit/src/perf.rs`, `perf/budgets.toml`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// A number may exceed its baseline by this factor, plus 2 for jitter on
/// small numbers.
pub const BASELINE_SLACK: f64 = 1.25;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

/// One benchmark's numbers.
#[derive(Debug, Default)]
pub struct Report {
    pub name: String,
    pub metrics: BTreeMap<String, f64>,
}

impl Report {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            metrics: BTreeMap::new(),
        }
    }

    pub fn time(&mut self, metric: &str, time: Duration) {
        self.metrics
            .insert(metric.into(), time.as_secs_f64() * 1000.0);
    }

    /// Add the peak resident memory, write the JSON, and check it against
    /// the budgets and any baseline.
    pub fn finish(mut self) -> Result {
        if let Some(peak) = peak_rss_mib() {
            self.metrics.insert("peak_rss_mib".into(), peak);
        }
        let json = serde_json::to_string_pretty(&serde_json::json!({
            "name": self.name,
            "metrics": self.metrics,
        }))?;
        let out = out_dir();
        std::fs::create_dir_all(&out)?;
        std::fs::write(out.join(format!("{}.json", self.name)), &json)?;
        eprintln!("{json}");
        let budgets: serde_json::Value =
            serde_json::from_str(include_str!("../perf/budgets.json"))?;
        let baseline = std::env::var_os("GG_PERF_BASELINE")
            .map(PathBuf::from)
            .and_then(|dir| std::fs::read(dir.join(format!("{}.json", self.name))).ok())
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok());
        let over = check(
            &self.name,
            &self.metrics,
            &budgets,
            baseline.as_ref().map(|value| &value["metrics"]),
        );
        if over.is_empty() {
            Ok(())
        } else {
            Err(over.join("\n").into())
        }
    }
}

/// What is over its budget or baseline.
fn check(
    name: &str,
    metrics: &BTreeMap<String, f64>,
    budgets: &serde_json::Value,
    baseline: Option<&serde_json::Value>,
) -> Vec<String> {
    let mut over = Vec::new();
    for (metric, value) in metrics {
        if let Some(limit) = budgets[name][metric].as_f64()
            && *value > limit
        {
            over.push(format!(
                "{name}.{metric} = {value:.1}, over its budget {limit}"
            ));
        }
        if let Some(before) = baseline.and_then(|baseline| baseline[metric].as_f64())
            && *value > before * BASELINE_SLACK + 2.0
        {
            over.push(format!(
                "{name}.{metric} = {value:.1}, over its baseline {before:.1} × {BASELINE_SLACK}"
            ));
        }
    }
    over
}

fn out_dir() -> PathBuf {
    std::env::var_os("GG_PERF_OUT").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/perf"),
        PathBuf::from,
    )
}

/// The process's peak resident memory in MiB, where the system says
/// (Linux: `VmHWM` in `/proc/self/status`).
pub fn peak_rss_mib() -> Option<f64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|line| line.starts_with("VmHWM:"))?;
    let kib: f64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kib / 1024.0)
}

/// The 95th percentile of `times`.
pub fn p95(mut times: Vec<Duration>) -> Duration {
    if times.is_empty() {
        return Duration::ZERO;
    }
    times.sort();
    times[(times.len() * 95 / 100).min(times.len() - 1)]
}

/// A repository of `commits` commits on `main` and `branches` branches at
/// its tip, in folders of 100 (`team00/topic00000`), written by one
/// `git fast-import`.
pub fn fixture(commits: usize, branches: usize) -> Result<tempfile::TempDir> {
    use std::io::Write;
    use std::process::Stdio;
    let dir = tempfile::tempdir()?;
    let git = |args: &[&str]| -> Result {
        let status = gg_git::process::git()
            .arg("-C")
            .arg(dir.path())
            .args(args)
            .status()?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("git {args:?} failed").into())
        }
    };
    git(&["init", "-q", "-b", "main"])?;
    let mut child = gg_git::process::git()
        .arg("-C")
        .arg(dir.path())
        .args(["fast-import", "--quiet"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;
    {
        let mut input = std::io::BufWriter::new(child.stdin.take().ok_or("no fast-import input")?);
        for n in 0..commits {
            let message = format!("Commit {n} with a subject as long as most");
            write!(
                input,
                "commit refs/heads/main\ncommitter Ada <ada@example.org> {} +0000\ndata {}\n{message}\n",
                1_700_000_000 + n,
                message.len()
            )?;
            if n == 0 {
                write!(input, "M 644 inline README\ndata 6\nhello\n")?;
            }
            writeln!(input)?;
        }
        for n in 0..branches {
            writeln!(
                input,
                "reset refs/heads/team{:02}/topic{n:05}\nfrom refs/heads/main\n",
                n / 100
            )?;
        }
    }
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned().into());
    }
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui;

    #[test]
    fn numbers_over_their_budget_or_baseline_are_named() {
        let budgets = serde_json::json!({"history": {"frame_p95_ms": 16}});
        let metrics = BTreeMap::from([
            ("frame_p95_ms".to_owned(), 20.0),
            ("complete_ms".to_owned(), 900.0),
        ]);
        let baseline = serde_json::json!({"complete_ms": 500.0, "frame_p95_ms": 30.0});
        assert_eq!(
            check("history", &metrics, &budgets, Some(&baseline)),
            [
                "history.complete_ms = 900.0, over its baseline 500.0 × 1.25",
                "history.frame_p95_ms = 20.0, over its budget 16",
            ]
        );
        let fine = BTreeMap::from([("frame_p95_ms".to_owned(), 12.0)]);
        assert!(check("history", &fine, &budgets, None).is_empty());
        assert_eq!(
            p95((1..=100).map(Duration::from_millis).collect()),
            Duration::from_millis(96)
        );
        if cfg!(target_os = "linux") {
            assert!(peak_rss_mib().is_some_and(|mib| mib > 1.0));
        }
    }

    #[test]
    fn every_budget_names_a_number_a_benchmark_reports() -> Result {
        let budgets: serde_json::Value =
            serde_json::from_str(include_str!("../perf/budgets.json"))?;
        let reported = BTreeMap::from([
            (
                "history",
                [
                    "first_rows_ms",
                    "complete_ms",
                    "frame_p95_ms",
                    "idle_frame_p95_ms",
                    "deep_jump_ms",
                    "peak_rss_mib",
                ]
                .as_slice(),
            ),
            ("refs_sidebar", ["frame_p95_ms", "rebuild_ms"].as_slice()),
        ]);
        for (name, metrics) in budgets.as_object().ok_or("an object")? {
            if name.starts_with('_') {
                continue;
            }
            let known = reported
                .get(name.as_str())
                .ok_or(format!("no benchmark {name}"))?;
            for metric in metrics.as_object().ok_or("an object")?.keys() {
                assert!(known.contains(&metric.as_str()), "{name}.{metric}");
            }
        }
        Ok(())
    }

    /// History of 100,000 commits and a sidebar of 10,000 branches, timed
    /// and checked against `perf/budgets.json`.
    #[test]
    #[ignore = "a benchmark: run in release with --ignored (the Timing workflow does)"]
    fn performance_stays_within_budgets() -> Result {
        let repo = fixture(100_000, 10_000)?;
        let ctx = egui::Context::default();
        crate::phosphor::install(&ctx);
        let input = || crate::tests::frame_input(1280.0, 800.0, vec![]);

        let mut history = Report::new("history");
        let mut app = crate::Browser::new(repo.path().into());
        let started = Instant::now();
        app.load(&ctx);
        let mut frames = Vec::new();
        let mut first = None;
        while !app.state.finished {
            let before = Instant::now();
            let _ = crate::tests::run_ui(&ctx, input(), |ctx| app.draw_ui(ctx));
            frames.push(before.elapsed());
            if app.state.count > 0 && first.is_none() {
                first = Some(started.elapsed());
            }
            if let Some(error) = &app.state.error {
                return Err(error.clone().into());
            }
            if started.elapsed() > Duration::from_secs(120) {
                return Err("history did not finish in 120 s".into());
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(app.state.count, 100_000);
        history.time("first_rows_ms", first.unwrap_or_default());
        history.time("complete_ms", started.elapsed());
        // Frames once loaded: what a frame costs with nothing arriving.
        let mut idle = Vec::new();
        for _ in 0..60 {
            let before = Instant::now();
            let _ = crate::tests::run_ui(&ctx, input(), |ctx| app.draw_ui(ctx));
            idle.push(before.elapsed());
        }
        history.time("idle_frame_p95_ms", p95(idle));
        history.time("frame_p95_ms", p95(frames));
        let jump = Instant::now();
        app.state.select_row(99_999);
        app.scroll_to_selection = true;
        while !app.state.rows.contains_key(&99_999) {
            let _ = crate::tests::run_ui(&ctx, input(), |ctx| app.draw_ui(ctx));
            if jump.elapsed() > Duration::from_secs(30) {
                return Err("the last row did not arrive in 30 s".into());
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        history.time("deep_jump_ms", jump.elapsed());
        history.finish()?;

        let mut sidebar = Report::new("refs_sidebar");
        let refs = std::sync::Arc::new(gg_git::refs::read(repo.path())?);
        assert!(refs.items.len() >= 10_000);
        let mut frames = Vec::new();
        let mut filter = String::new();
        for n in 0..120 {
            // Every 40th frame a new snapshot, as after a fetch: the list
            // is built again.
            let refs = if n % 40 == 0 {
                std::sync::Arc::new((*refs).clone())
            } else {
                refs.clone()
            };
            let before = Instant::now();
            let _ = crate::tests::run_ui(&ctx, input(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    crate::refs_view::show(
                        ui,
                        &refs,
                        &[],
                        &[],
                        &mut filter,
                        false,
                        &BTreeMap::new(),
                    );
                });
            });
            let time = before.elapsed();
            if n % 40 == 0 {
                sidebar.metrics.insert(
                    "rebuild_ms".into(),
                    sidebar
                        .metrics
                        .get("rebuild_ms")
                        .copied()
                        .unwrap_or(0.0)
                        .max(time.as_secs_f64() * 1000.0),
                );
            } else {
                frames.push(time);
            }
        }
        sidebar.time("frame_p95_ms", p95(frames));
        sidebar.finish()
    }
}
