//! Paths Git prints, kept as the bytes Git wrote. Git paths are bytes on
//! Unix and UTF-8 on Windows, so decoding them lossily, or trimming spaces
//! from them, names another file.
use std::{ffi::OsString, path::PathBuf};

/// `bytes` as an argument or file name, unchanged on Unix.
pub fn os_string(bytes: &[u8]) -> OsString {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        std::ffi::OsStr::from_bytes(bytes).to_os_string()
    }
    #[cfg(not(unix))]
    {
        OsString::from(String::from_utf8_lossy(bytes).into_owned())
    }
}

/// `bytes` as a path, unchanged on Unix.
pub fn path(bytes: &[u8]) -> PathBuf {
    PathBuf::from(os_string(bytes))
}

/// The path a one-line Git answer names (`rev-parse --show-toplevel`,
/// `--git-dir`, `--git-path`, …): its one line ending is removed, and any
/// spaces the path itself ends with are kept.
pub fn line(stdout: &[u8]) -> PathBuf {
    path(without_line_ending(stdout))
}

/// `bytes` without the one line ending Git printed after it: `\n`, or
/// `\r\n` on Windows.
pub fn without_line_ending(bytes: &[u8]) -> &[u8] {
    let Some(bytes) = bytes.strip_suffix(b"\n") else {
        return bytes;
    };
    if cfg!(windows) {
        bytes.strip_suffix(b"\r").unwrap_or(bytes)
    } else {
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_line_ending_is_removed() {
        assert_eq!(without_line_ending(b"/r/repo \n"), b"/r/repo ");
        assert_eq!(without_line_ending(b"/r/repo"), b"/r/repo");
        assert_eq!(without_line_ending(b"/r/a\n\n"), b"/r/a\n");
        let crlf: &[u8] = if cfg!(windows) { b"C:/r" } else { b"C:/r\r" };
        assert_eq!(without_line_ending(b"C:/r\r\n"), crlf);
    }

    #[cfg(unix)]
    #[test]
    fn bytes_that_are_not_utf8_are_kept() {
        use std::os::unix::ffi::OsStrExt;
        assert_eq!(line(b"/r/raw\xff\n").as_os_str().as_bytes(), b"/r/raw\xff");
    }
}
