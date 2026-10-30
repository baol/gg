//! Network-operation progress and receipts shared by CLI and native views.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Clone,
    Fetch,
    Push,
}

impl Operation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Clone => "clone",
            Self::Fetch => "fetch",
            Self::Push => "push",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Progress {
    pub operation: Operation,
    pub phase: String,
    pub percent: Option<u8>,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FetchReceipt {
    pub remote: String,
    pub pruned: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PushReceipt {
    pub remote: String,
    pub branch: String,
    pub upstream: Option<String>,
    pub set_upstream: bool,
    pub force_with_lease: bool,
    pub lease: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloneReceipt {
    pub path: PathBuf,
    pub head: Option<String>,
    pub branch: Option<String>,
}

/// Return a trimmed repository URL when the text can be cloned.
pub fn repository_url(text: &str) -> Option<String> {
    let text = text.trim();
    if text.is_empty()
        || text.len() > 4096
        || text
            .chars()
            .any(|character| character.is_whitespace() || character.is_control())
    {
        return None;
    }
    if let Some((scheme, rest)) = text.split_once("://") {
        if !matches!(scheme, "https" | "http" | "ssh" | "git" | "file") || rest.is_empty() {
            return None;
        }
        if scheme == "file" {
            if !rest.starts_with('/') {
                return None;
            }
        } else {
            let (host, path) = rest.split_once('/')?;
            if host.is_empty() || path.is_empty() {
                return None;
            }
        }
    } else {
        let (authority, path) = text.split_once(':')?;
        if !authority.contains('@') || authority.contains('/') || path.is_empty() {
            return None;
        }
    }
    repository_name(text).map(|_| text.to_string())
}

/// Derive the directory name for a clone URL.
pub fn repository_name(url: &str) -> Option<String> {
    let trimmed = url.trim().trim_end_matches('/');
    let last = trimmed.rsplit(['/', ':']).next()?;
    let name = last.strip_suffix(".git").unwrap_or(last);
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.contains(['/', '\\'])
        || name.chars().any(char::is_whitespace)
    {
        return None;
    }
    Some(name.to_string())
}

/// True when a destination or project name is one usable path segment.
pub fn repository_folder_name(name: &str) -> bool {
    let name = name.trim();
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains(['/', '\\'])
        && !name.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_clone_urls_without_treating_paths_as_urls() {
        for url in [
            "https://github.com/baol/gg.git",
            "http://example.org/team/repo",
            "ssh://git@example.org/team/repo.git",
            "git://example.org/team/repo.git",
            "file:///srv/git/repo.git",
            "git@github.com:baol/gg.git",
            "  https://github.com/baol/gg.git\n",
        ] {
            assert_eq!(
                repository_url(url).as_deref(),
                Some(url.trim()),
                "{url} should be a clone URL"
            );
        }
        for text in [
            "",
            "hello world",
            "/home/user/repo",
            "https://github.com",
            "https://github.com/",
            "git@github.com:",
            "C:\\Users\\me\\repo",
            "https://example.org/a b",
            "ftp://example.org/repo.git",
        ] {
            assert_eq!(
                repository_url(text),
                None,
                "{text} should not be a clone URL"
            );
        }
    }

    #[test]
    fn derives_project_names_and_rejects_separators() {
        assert_eq!(
            repository_name("https://github.com/baol/gg.git"),
            Some("gg".into())
        );
        assert_eq!(
            repository_name("git@github.com:baol/gg.git"),
            Some("gg".into())
        );
        assert_eq!(
            repository_name("https://example.org/team/repo/"),
            Some("repo".into())
        );
        assert_eq!(
            repository_name("https://example.org"),
            Some("example.org".into())
        );
        assert!(repository_folder_name("gg"));
        assert!(repository_folder_name("gg good"));
        for name in ["", ".", "..", "a/b", "a\\b", "a\nb"] {
            assert!(!repository_folder_name(name), "{name}");
        }
    }
}
