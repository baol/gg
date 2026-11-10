//! Ignore an untracked path by adding a pattern to the repository's root
//! `.gitignore`, as a person would by hand. Patterns are anchored to the
//! root (`/build.log`, `/target/`) unless they name an extension
//! (`*.log`), and Git's special characters are escaped.
use std::path::{Path, PathBuf};

/// What Ignore wrote.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Receipt {
    /// The `.gitignore` written, absolute.
    pub file: PathBuf,
    pub pattern: String,
    /// False when the file already held the pattern.
    pub added: bool,
}

/// The three ways to ignore an untracked file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// Only this file.
    File,
    /// Every file with its extension, anywhere.
    Extension,
    /// The folder that holds it.
    Folder,
}

/// The pattern that ignores `relative` (a repository-relative path with
/// `/` separators) in `scope`, or `None` when the scope does not apply: no
/// extension, or a file at the root.
pub fn pattern(relative: &[u8], scope: Scope) -> Result<Option<String>, String> {
    let relative = std::str::from_utf8(relative)
        .map_err(|_| "Ignore needs a UTF-8 path; edit .gitignore by hand.".to_string())?
        .trim_end_matches('/');
    if relative.is_empty()
        || relative
            .split('/')
            .any(|part| part.is_empty() || part == "..")
    {
        return Err("Ignore needs a repository-relative file path.".into());
    }
    Ok(match scope {
        Scope::File => Some(format!("/{}", escape(relative))),
        Scope::Extension => {
            let name = relative.rsplit('/').next().unwrap_or(relative);
            name.rfind('.')
                .filter(|&dot| dot > 0 && dot + 1 < name.len())
                .map(|dot| format!("*.{}", escape(&name[dot + 1..])))
        }
        Scope::Folder => relative
            .rfind('/')
            .map(|slash| format!("/{}/", escape(&relative[..slash]))),
    })
}

/// Escape what `.gitignore` reads specially: `\`, `*`, `?`, `[`, and a
/// trailing space.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if matches!(c, '\\' | '*' | '?' | '[') {
            out.push('\\');
        }
        out.push(c);
    }
    if out.ends_with(' ') {
        out.pop();
        out.push_str("\\ ");
    }
    out
}

/// Add `pattern` to the root `.gitignore` of the repository at `path`,
/// creating the file when there is none and keeping what it holds.
pub fn add(path: &Path, pattern: &str) -> Result<Receipt, String> {
    let pattern = pattern.trim_end_matches(['\n', '\r']);
    if pattern.is_empty() || pattern.contains(['\n', '\r']) {
        return Err("An ignore pattern is one line.".into());
    }
    let repo = gix::discover(path).map_err(|e| format!("Open the repository: {e}"))?;
    let root = repo
        .workdir()
        .ok_or("A bare repository has no working files to ignore.")?;
    let file = root.join(".gitignore");
    let existing = match std::fs::read(&file) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(error) => return Err(format!("Read {}: {error}", file.display())),
    };
    let present = existing
        .split(|&b| b == b'\n')
        .any(|line| line.strip_suffix(b"\r").unwrap_or(line) == pattern.as_bytes());
    if !present {
        let mut text = existing;
        if !text.is_empty() && !text.ends_with(b"\n") {
            text.push(b'\n');
        }
        text.extend_from_slice(pattern.as_bytes());
        text.push(b'\n');
        std::fs::write(&file, text).map_err(|e| format!("Write {}: {e}", file.display()))?;
    }
    Ok(Receipt {
        file,
        pattern: pattern.to_string(),
        added: !present,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patterns_anchor_files_and_folders_and_escape_specials() -> Result<(), String> {
        assert_eq!(
            pattern(b"build.log", Scope::File)?.as_deref(),
            Some("/build.log")
        );
        assert_eq!(
            pattern(b"logs/a.log", Scope::Extension)?.as_deref(),
            Some("*.log")
        );
        assert_eq!(
            pattern(b"logs/a.log", Scope::Folder)?.as_deref(),
            Some("/logs/")
        );
        assert_eq!(pattern(b"build.log", Scope::Folder)?, None, "at the root");
        assert_eq!(pattern(b"Makefile", Scope::Extension)?, None);
        assert_eq!(
            pattern(b".env", Scope::Extension)?,
            None,
            "a dotfile has no extension"
        );
        assert_eq!(
            pattern(b"odd [1]*?.txt", Scope::File)?.as_deref(),
            Some(r"/odd \[1]\*\?.txt")
        );
        assert_eq!(
            pattern(b"trail ", Scope::File)?.as_deref(),
            Some(r"/trail\ ")
        );
        assert_eq!(pattern(b"#notes", Scope::File)?.as_deref(), Some("/#notes"));
        assert!(pattern(b"../x", Scope::File).is_err());
        assert!(pattern(b"\xff", Scope::File).is_err());
        Ok(())
    }
}
