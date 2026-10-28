//! Signature verification and explicit signing configuration.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Unsigned,
    Good,
    Bad,
    UnknownKey,
    Unavailable,
    Error,
}

impl State {
    pub fn label(self) -> &'static str {
        match self {
            Self::Unsigned => "Unsigned",
            Self::Good => "Good signature",
            Self::Bad => "Bad signature",
            Self::UnknownKey => "Unknown key",
            Self::Unavailable => "Verification unavailable",
            Self::Error => "Verification error",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Verification {
    pub reference: String,
    pub kind: Option<String>,
    pub state: State,
    pub format: Option<String>,
    pub signer: Option<String>,
    pub key: Option<String>,
    pub trust: Option<String>,
    pub detail: Option<String>,
}

impl Verification {
    pub fn signed(&self) -> bool {
        !matches!(self.state, State::Unsigned)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub format: String,
    pub signing_key: Option<String>,
    pub gpg_program: Option<String>,
    pub commit_signing: bool,
    pub tag_signing: bool,
    pub allowed_signers: Option<String>,
    pub tool_available: bool,
    pub note: Option<String>,
}

impl Config {
    pub fn ready(&self) -> bool {
        self.signing_key.is_some() && self.tool_available
    }
}

/// Parse Git's `--raw` verification output into a stable result.
pub fn parse_raw(
    reference: &str,
    kind: Option<&str>,
    success: bool,
    output: &str,
    format: Option<&str>,
) -> Verification {
    let mut signer = None;
    let mut key = None;
    let mut trust = None;
    let mut bad = false;
    let mut unknown = false;
    let mut good = false;
    let mut unsigned = false;
    let mut detail = Vec::new();
    for line in output.lines() {
        if let Some(value) = line.strip_prefix("[GNUPG:] GOODSIG ") {
            good = true;
            let (key_id, rest) = value.split_once(' ').unwrap_or((value, ""));
            key.get_or_insert_with(|| key_id.to_string());
            signer = Some(rest.to_string());
        } else if let Some(value) = line.strip_prefix("[GNUPG:] VALIDSIG ") {
            good = true;
            key.get_or_insert_with(|| {
                value
                    .split_whitespace()
                    .next()
                    .unwrap_or_default()
                    .to_string()
            });
        } else if let Some(value) = line.strip_prefix("[GNUPG:] KEY_CONSIDERED ") {
            key.get_or_insert_with(|| {
                value
                    .split_whitespace()
                    .next()
                    .unwrap_or_default()
                    .to_string()
            });
        } else if line.starts_with("[GNUPG:] BADSIG ") {
            bad = true;
        } else if line.starts_with("[GNUPG:] ERRSIG ")
            || line.starts_with("[GNUPG:] NO_PUBKEY ")
            || line.contains("Can't check signature")
        {
            unknown = true;
        } else if let Some(value) = line.strip_prefix("[GNUPG:] TRUST_") {
            trust = Some(
                value
                    .split_whitespace()
                    .next()
                    .unwrap_or_default()
                    .to_lowercase(),
            );
        } else if line.contains("no signature found") {
            unsigned = true;
        } else if line.contains("Good \"git\" signature") {
            good = true;
            signer = Some(line.to_string());
        } else if line.starts_with("[GNUPG:]") {
        } else if !line.trim().is_empty() {
            detail.push(line.to_string());
        }
    }
    let state = if bad {
        State::Bad
    } else if unsigned {
        State::Unsigned
    } else if unknown {
        State::UnknownKey
    } else if good {
        State::Good
    } else if !success && detail.is_empty() {
        State::Unsigned
    } else if !success {
        State::Error
    } else {
        State::Good
    };
    Verification {
        reference: reference.to_string(),
        kind: kind.map(str::to_owned),
        state,
        format: format.map(str::to_owned),
        signer,
        key,
        trust,
        detail: (!unsigned && !detail.is_empty()).then(|| detail.join("\n")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_good_bad_unknown_and_unsigned_outputs() {
        let good = parse_raw(
            "HEAD",
            Some("commit"),
            true,
            "[GNUPG:] GOODSIG ABC123 Test User <test@example.org>\n[GNUPG:] VALIDSIG DEF456 2026-10-28\n[GNUPG:] TRUST_ULTIMATE 0 pgp\n",
            Some("openpgp"),
        );
        assert_eq!(good.state, State::Good);
        assert_eq!(good.signer.as_deref(), Some("Test User <test@example.org>"));
        assert_eq!(good.trust.as_deref(), Some("ultimate"));
        assert!(good.detail.is_none());

        let bad = parse_raw(
            "HEAD",
            Some("commit"),
            true,
            "[GNUPG:] BADSIG ABC123 Test\n",
            None,
        );
        assert_eq!(bad.state, State::Bad);
        let unknown = parse_raw(
            "HEAD",
            Some("tag"),
            true,
            "[GNUPG:] NO_PUBKEY ABC123\n",
            None,
        );
        assert_eq!(unknown.state, State::UnknownKey);
        let unsigned = parse_raw("HEAD", Some("commit"), false, "", None);
        assert_eq!(unsigned.state, State::Unsigned);
        assert!(!unsigned.signed());
        let unsigned_tag = parse_raw(
            "v1",
            Some("tag"),
            false,
            "error: no signature found\n",
            None,
        );
        assert_eq!(unsigned_tag.state, State::Unsigned);
        assert!(unsigned_tag.detail.is_none());
    }
}
