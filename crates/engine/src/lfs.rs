//! Read-only Git LFS pointer identity and explicit fetch receipts.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pointer {
    pub oid: String,
    pub size: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Parse {
    NotPointer,
    Pointer(Pointer),
    Malformed(String),
}

/// Parse a canonical Git LFS v1 pointer without accepting arbitrary text.
pub fn parse(bytes: &[u8]) -> Parse {
    if bytes.len() > 1024 {
        return Parse::NotPointer;
    }
    let Ok(text) = std::str::from_utf8(bytes) else {
        return Parse::NotPointer;
    };
    let mut lines = text.lines();
    let Some(version) = lines.next() else {
        return Parse::NotPointer;
    };
    if version != "version https://git-lfs.github.com/spec/v1" {
        return Parse::NotPointer;
    }
    let Some(oid_line) = lines.next() else {
        return Parse::Malformed("LFS pointer is missing oid.".into());
    };
    let Some(oid) = oid_line.strip_prefix("oid sha256:") else {
        return Parse::Malformed("LFS pointer oid is not sha256.".into());
    };
    if oid.len() != 64 || !oid.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Parse::Malformed("LFS pointer oid is not 64 hexadecimal digits.".into());
    }
    let Some(size_line) = lines.next() else {
        return Parse::Malformed("LFS pointer is missing size.".into());
    };
    let Some(size) = size_line.strip_prefix("size ") else {
        return Parse::Malformed("LFS pointer size is invalid.".into());
    };
    let Ok(size) = size.parse::<u64>() else {
        return Parse::Malformed("LFS pointer size is invalid.".into());
    };
    for line in lines {
        if !line.is_empty() && !line.starts_with("ext-") {
            return Parse::Malformed("LFS pointer has an unknown line.".into());
        }
    }
    Parse::Pointer(Pointer {
        oid: oid.to_lowercase(),
        size,
    })
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SidePointer {
    pub oid: String,
    pub size: u64,
    pub available: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Details {
    pub path: String,
    pub path_bytes: Vec<u8>,
    pub before: Option<SidePointer>,
    pub after: Option<SidePointer>,
    pub lfs_installed: bool,
    pub remote: Option<String>,
    pub note: Option<String>,
}

impl Details {
    pub fn target(&self) -> Option<&SidePointer> {
        self.after.as_ref().or(self.before.as_ref())
    }

    pub fn can_fetch(&self) -> Result<(), String> {
        if !self.lfs_installed {
            return Err("git-lfs is not installed.".into());
        }
        let Some(target) = self.target() else {
            return Err("No LFS pointer is selected.".into());
        };
        if target.available {
            return Err("The LFS object is already available locally.".into());
        }
        Ok(())
    }

    pub fn status_label(&self) -> &'static str {
        match self.target() {
            None => "Not an LFS pointer",
            Some(target) if target.available => "Object available",
            Some(_) if !self.lfs_installed => "LFS unavailable",
            Some(_) => "Object missing",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FetchReceipt {
    pub path: String,
    pub path_bytes: Vec<u8>,
    pub oid: String,
    pub size: u64,
    pub remote: Option<String>,
    pub available: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    const OID: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn parses_canonical_pointers_and_rejects_other_text() {
        let pointer =
            format!("version https://git-lfs.github.com/spec/v1\noid sha256:{OID}\nsize 12345\n");
        assert_eq!(
            parse(pointer.as_bytes()),
            Parse::Pointer(Pointer {
                oid: OID.into(),
                size: 12345
            })
        );
        assert_eq!(parse(b"ordinary text\n"), Parse::NotPointer);
        assert!(matches!(
            parse(b"version https://git-lfs.github.com/spec/v1\noid sha256:nope\nsize 1\n"),
            Parse::Malformed(_)
        ));
        assert!(matches!(
            parse(b"version https://git-lfs.github.com/spec/v1\noid sha256:0123\nsize 1\n"),
            Parse::Malformed(_)
        ));
    }

    #[test]
    fn details_name_unavailable_and_available_objects() {
        let mut details = Details {
            path: "asset.bin".into(),
            path_bytes: b"asset.bin".to_vec(),
            before: None,
            after: Some(SidePointer {
                oid: OID.into(),
                size: 10,
                available: false,
            }),
            lfs_installed: true,
            remote: Some("origin".into()),
            note: None,
        };
        assert_eq!(details.status_label(), "Object missing");
        assert!(details.can_fetch().is_ok());
        if let Some(after) = details.after.as_mut() {
            after.available = true;
        }
        assert_eq!(details.status_label(), "Object available");
        assert!(details.can_fetch().is_err());
        if let Some(after) = details.after.as_mut() {
            after.available = false;
        }
        details.lfs_installed = false;
        assert_eq!(details.status_label(), "LFS unavailable");
        assert!(details.can_fetch().is_err());
    }
}
