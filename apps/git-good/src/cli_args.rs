//! Command-line arguments with `--`: after it every argument is a value,
//! even one that looks like an option (`git good --cli status --file -- -x.txt`).
use std::ffi::OsString;

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
