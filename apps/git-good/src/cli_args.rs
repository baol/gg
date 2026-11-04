//! Command-line arguments with `--`: after it every argument is a value,
//! even one that looks like an option (`git good --cli status --file -- -x.txt`).
use std::{
    ffi::OsString,
    io::Write,
    sync::atomic::{AtomicU8, Ordering},
};

/// Exit codes (decision 12): 0 completed or stopped as planned, 1 failure,
/// 2 usage, 4 stopped on conflicts. 3 (refused) and 5 (partial) are
/// reserved until Git Good's errors say which they are; 130 is the shell's
/// code for the default Ctrl+C.
pub const USAGE: u8 = 2;
pub const CONFLICT: u8 = 4;

/// The exit code of the receipt already written, for a run that ends after it.
static OUTCOME: AtomicU8 = AtomicU8::new(0);

pub fn outcome() -> u8 {
    OUTCOME.load(Ordering::Relaxed)
}

/// Write one JSON receipt and a newline. An object gains `status`:
/// `conflict` when it lists conflicts, `stopped` when an operation is
/// still in progress without them, else `completed`; the exit code follows.
pub fn emit(receipt: &impl serde::Serialize) -> Result<(), Box<dyn std::error::Error>> {
    let mut receipt = serde_json::to_value(receipt)?;
    if let Some(object) = receipt.as_object_mut()
        && !object.contains_key("status")
    {
        let conflicts = object
            .get("conflicts")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|conflicts| !conflicts.is_empty());
        let in_progress = object.get("in_progress") == Some(&serde_json::Value::Bool(true));
        let status = match (conflicts, in_progress) {
            (true, _) => "conflict",
            (false, true) => "stopped",
            (false, false) => "completed",
        };
        OUTCOME.store(if conflicts { CONFLICT } else { 0 }, Ordering::Relaxed);
        object.insert("status".into(), status.into());
    }
    let mut out = std::io::stdout().lock();
    serde_json::to_writer(&mut out, &receipt)?;
    writeln!(out)?;
    out.flush()?;
    Ok(())
}

/// Close a streamed document after its last member: add `"complete"`,
/// `"status"`, and on a read error an `"error"` member, so the output is
/// always one valid JSON document; the read error is then returned.
pub fn end_stream(
    out: &mut impl Write,
    read: Result<(), Box<dyn std::error::Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    match read {
        Ok(()) => {
            writeln!(out, ",\"complete\":true,\"status\":\"completed\"}}")?;
            out.flush()?;
            Ok(())
        }
        Err(error) => {
            writeln!(
                out,
                ",\"complete\":false,\"status\":\"failed\",\"error\":{}}}",
                serde_json::to_string(&error.to_string())?
            )?;
            out.flush()?;
            Err(error)
        }
    }
}

/// Whether the error is stdout closing under the command (`| head`).
pub fn closed_output(error: &(dyn std::error::Error + 'static)) -> bool {
    let kind = if let Some(error) = error.downcast_ref::<std::io::Error>() {
        Some(error.kind())
    } else {
        error
            .downcast_ref::<serde_json::Error>()
            .and_then(serde_json::Error::io_error_kind)
    };
    kind == Some(std::io::ErrorKind::BrokenPipe)
}

/// The exit code for a failure: 2 for a command line Git Good cannot read
/// (its own usage messages), else 1.
pub fn failure_code(message: &str) -> u8 {
    const USAGE_STARTS: [&str; 6] = [
        "Usage:",
        "Unknown ",
        "--",
        "Name a command after --cli",
        "Open one folder",
        "No folder ",
    ];
    if USAGE_STARTS.iter().any(|start| message.starts_with(start)) {
        USAGE
    } else {
        1
    }
}

pub struct Args<I> {
    inner: I,
    literal: bool,
}

impl<I: Iterator<Item = OsString>> Args<I> {
    pub fn new(inner: I) -> Self {
        Self {
            inner,
            literal: false,
        }
    }

    /// The argument as an option name to match, or `None` after `--`, so
    /// it falls to the parser's value arm.
    pub fn option<'a>(&self, arg: &'a OsString) -> Option<&'a str> {
        if self.literal { None } else { arg.to_str() }
    }
}

impl<I: Iterator<Item = OsString>> Iterator for Args<I> {
    type Item = OsString;

    fn next(&mut self) -> Option<OsString> {
        let arg = self.inner.next()?;
        if !self.literal && arg == "--" {
            self.literal = true;
            return self.inner.next();
        }
        Some(arg)
    }
}

/// The repository of a command whose only option is `--repo PATH`; the
/// working folder without it.
pub fn repo_only(
    args: impl Iterator<Item = OsString>,
    usage: &str,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let mut args = Args::new(args);
    let mut path = None;
    while let Some(arg) = args.next() {
        match args.option(&arg) {
            Some("--repo") if path.is_none() => {
                path = Some(std::path::PathBuf::from(
                    args.next().ok_or("--repo needs a path")?,
                ));
            }
            _ => return Err(usage.into()),
        }
    }
    match path {
        Some(path) => Ok(path),
        None => Ok(std::env::current_dir()?),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_messages_exit_2() {
        assert_eq!(
            failure_code("Usage: git good --cli refs [--repo PATH]"),
            USAGE
        );
        assert_eq!(failure_code("Unknown merge option"), USAGE);
        assert_eq!(failure_code("--repo needs a path"), USAGE);
        assert_eq!(
            failure_code("No folder status. Did you mean git good --cli status?"),
            USAGE
        );
        assert_eq!(failure_code("Could not read repository"), 1);
    }

    #[test]
    fn dashes_end_options() {
        let raw = ["--repo", "r", "--", "--repo", "-x"].map(OsString::from);
        let mut args = Args::new(raw.into_iter());
        let mut seen = Vec::new();
        while let Some(arg) = args.next() {
            seen.push((args.option(&arg).map(str::to_string), arg));
        }
        assert_eq!(
            seen,
            [
                (Some("--repo".to_string()), OsString::from("--repo")),
                (Some("r".to_string()), OsString::from("r")),
                (None, OsString::from("--repo")),
                (None, OsString::from("-x")),
            ]
        );
    }
}
