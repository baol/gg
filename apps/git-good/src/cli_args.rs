//! Command-line arguments with `--`: after it every argument is a value,
//! even one that looks like an option (`git good --cli status --file -- -x.txt`).
use std::{
    ffi::OsString,
    io::Write,
    sync::atomic::{AtomicU8, Ordering},
};

/// Exit codes (decision 12): 0 completed or stopped as planned, 1 failure,
/// 2 usage, 4 stopped on conflicts, 5 a result cut short (`complete:
/// false`). 3 (refused) is reserved until Git Good's errors say which they
/// are; 130 is the shell's code for the default Ctrl+C.
pub const USAGE: u8 = 2;
pub const CONFLICT: u8 = 4;
pub const PARTIAL: u8 = 5;

/// The exit code of the receipt already written, for a run that ends after it.
static OUTCOME: AtomicU8 = AtomicU8::new(0);

pub fn outcome() -> u8 {
    OUTCOME.load(Ordering::Relaxed)
}

/// Write one JSON receipt and a newline. An object gains `status`:
/// `conflict` when it lists conflicts, `partial` when it says it is not
/// `complete`, `stopped` when an operation is still in progress, else
/// `completed`; the exit code follows.
pub fn emit(receipt: &impl serde::Serialize) -> Result<(), Box<dyn std::error::Error>> {
    let mut receipt = serde_json::to_value(receipt)?;
    if let Some(object) = receipt.as_object_mut()
        && !object.contains_key("status")
    {
        let (status, code) = receipt_status(object);
        OUTCOME.store(code, Ordering::Relaxed);
        object.insert("status".into(), status.into());
    }
    let mut out = std::io::stdout().lock();
    serde_json::to_writer(&mut out, &receipt)?;
    writeln!(out)?;
    out.flush()?;
    Ok(())
}

fn receipt_status(object: &serde_json::Map<String, serde_json::Value>) -> (&'static str, u8) {
    let conflicts = object
        .get("conflicts")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|conflicts| !conflicts.is_empty());
    let partial = object.get("complete") == Some(&serde_json::Value::Bool(false));
    let in_progress = object.get("in_progress") == Some(&serde_json::Value::Bool(true));
    if conflicts {
        ("conflict", CONFLICT)
    } else if partial {
        ("partial", PARTIAL)
    } else if in_progress {
        ("stopped", 0)
    } else {
        ("completed", 0)
    }
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

/// A command line Git Good cannot read: an unknown command or option, a
/// missing or malformed value, options that do not go together. Exits 2.
#[derive(Debug)]
pub struct Usage(pub String);

impl std::fmt::Display for Usage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Usage {}

pub fn usage_error(message: impl Into<String>) -> Box<dyn std::error::Error> {
    Box::new(Usage(message.into()))
}

/// The exit code for a failure: 2 for a [`Usage`] error, else 1.
pub fn failure_code(error: &(dyn std::error::Error + 'static)) -> u8 {
    if error.is::<Usage>() { USAGE } else { 1 }
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

    /// The value of the option just read, taken as it is: `--` here is the
    /// value, not the end of options (`commit -m --`).
    pub fn value(&mut self) -> Option<OsString> {
        self.inner.next()
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
                    args.value()
                        .ok_or_else(|| usage_error("--repo needs a path"))?,
                ));
            }
            _ => return Err(usage_error(usage)),
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
    fn usage_errors_exit_2_whatever_their_words() {
        assert_eq!(
            failure_code(usage_error("Branch switch requires a branch name").as_ref()),
            USAGE
        );
        let failure: Box<dyn std::error::Error> = "Unknown revision".into();
        assert_eq!(
            failure_code(failure.as_ref()),
            1,
            "a Git failure is not usage"
        );
    }

    #[test]
    fn a_receipt_cut_short_is_partial() {
        let status = |value: serde_json::Value| {
            value
                .as_object()
                .map_or(("not an object", 1), receipt_status)
        };
        assert_eq!(
            status(serde_json::json!({"complete": true})),
            ("completed", 0)
        );
        assert_eq!(
            status(serde_json::json!({"complete": false})),
            ("partial", PARTIAL)
        );
        assert_eq!(
            status(serde_json::json!({"in_progress": true})),
            ("stopped", 0)
        );
        assert_eq!(
            status(serde_json::json!({"complete": false, "conflicts": ["a"]})),
            ("conflict", CONFLICT)
        );
    }

    #[test]
    fn an_option_value_may_be_two_dashes() {
        let raw = ["-m", "--", "--amend"].map(OsString::from);
        let mut args = Args::new(raw.into_iter());
        let first = args.next();
        assert_eq!(first.as_ref().and_then(|arg| args.option(arg)), Some("-m"));
        assert_eq!(args.value(), Some(OsString::from("--")));
        let next = args.next();
        assert_eq!(
            next.as_ref().and_then(|arg| args.option(arg)),
            Some("--amend"),
            "still an option"
        );
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
