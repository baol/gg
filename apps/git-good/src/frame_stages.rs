//! A slow-frame log. Each frame is timed in named stages (polling, the
//! toolbar, the sidebar and dialogs, the views); a frame that takes
//! [`SLOW`] or longer writes one line to the diagnostics log naming its
//! slowest stages and the command it ran, if any. At most one line per
//! [`QUIET`]; the slow frames in between are counted on the next line.
//! Source: photocraft `crates/ui-egui/src/frame_stages.rs`.

use std::time::{Duration, Instant};

/// A frame at least this long is logged: three frames at 60 Hz.
pub const SLOW: Duration = Duration::from_millis(50);

/// The least time between two log lines.
pub const QUIET: Duration = Duration::from_secs(2);

#[derive(Debug, Default)]
pub struct Stages {
    /// When the frame began, and when its last stage ended.
    start: Option<Instant>,
    last: Option<Instant>,
    stages: Vec<(&'static str, Duration)>,
    /// The command the frame ran.
    command: Option<String>,
    /// When the last line was written, and slow frames not written since.
    logged: Option<Instant>,
    skipped: usize,
}

impl Stages {
    /// A frame begins.
    pub fn begin(&mut self, now: Instant) {
        self.start = Some(now);
        self.last = Some(now);
        self.stages.clear();
        self.command = None;
    }

    /// The stage `name` ended at `now`: the time since the last mark.
    pub fn mark(&mut self, name: &'static str, now: Instant) {
        let Some(last) = self.last else {
            return;
        };
        self.stages
            .push((name, now.saturating_duration_since(last)));
        self.last = Some(now);
    }

    /// The frame ran `command` (the last one counts).
    pub fn command(&mut self, command: impl Into<String>) {
        self.command = Some(command.into());
    }

    /// The frame ended with stage `name` at `now`: the line to log when it
    /// was slow and the log is not quiet.
    pub fn finish(&mut self, name: &'static str, now: Instant) -> Option<String> {
        self.mark(name, now);
        let start = self.start.take()?;
        let total = now.saturating_duration_since(start);
        if total < SLOW {
            return None;
        }
        if self
            .logged
            .is_some_and(|logged| now.saturating_duration_since(logged) < QUIET)
        {
            self.skipped += 1;
            return None;
        }
        self.logged = Some(now);
        let mut stages = self.stages.clone();
        stages.sort_by_key(|stage| std::cmp::Reverse(stage.1));
        let named: Vec<String> = stages
            .iter()
            .take(3)
            .filter(|(_, time)| !time.is_zero())
            .map(|(name, time)| format!("{name} {}", millis(*time)))
            .collect();
        let mut line = format!("{} ({})", millis(total), named.join(", "));
        if let Some(command) = &self.command {
            line.push_str(&format!("; command {command}"));
        }
        if self.skipped > 0 {
            line.push_str(&format!(
                "; {} more slow frames since the last",
                self.skipped
            ));
            self.skipped = 0;
        }
        Some(line)
    }
}

#[cfg(test)]
impl Stages {
    /// The stages marked so far this frame, in order.
    pub fn names(&self) -> Vec<&'static str> {
        self.stages.iter().map(|(name, _)| *name).collect()
    }
}

fn millis(time: Duration) -> String {
    format!("{} ms", time.as_millis())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(
        stages: &mut Stages,
        at: Instant,
        times: [u64; 3],
        command: Option<&str>,
    ) -> Option<String> {
        let ms = Duration::from_millis;
        stages.begin(at);
        stages.mark("polling", at + ms(times[0]));
        if let Some(command) = command {
            stages.command(command);
        }
        stages.mark("sidebar", at + ms(times[0] + times[1]));
        stages.finish("views", at + ms(times[0] + times[1] + times[2]))
    }

    #[test]
    fn a_slow_frame_names_its_slowest_stages_and_command() {
        let mut stages = Stages::default();
        let start = Instant::now();
        assert_eq!(
            frame(&mut stages, start, [2, 5, 10], None),
            None,
            "17 ms is fine"
        );
        assert_eq!(
            frame(&mut stages, start, [3, 61, 12], Some("Fetch")).as_deref(),
            Some("76 ms (sidebar 61 ms, views 12 ms, polling 3 ms); command Fetch")
        );
    }

    #[test]
    fn slow_frames_close_together_are_counted_on_the_next_line() {
        let mut stages = Stages::default();
        let start = Instant::now();
        assert!(frame(&mut stages, start, [0, 80, 0], None).is_some());
        let soon = start + Duration::from_millis(500);
        assert_eq!(frame(&mut stages, soon, [0, 80, 0], None), None);
        assert_eq!(frame(&mut stages, soon, [0, 80, 0], None), None);
        let later = start + QUIET + Duration::from_millis(1);
        assert_eq!(
            frame(&mut stages, later, [0, 90, 0], None).as_deref(),
            Some("90 ms (sidebar 90 ms); 2 more slow frames since the last")
        );
    }
}
