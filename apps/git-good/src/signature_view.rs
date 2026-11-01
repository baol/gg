//! Signature details as the inspect proposal draws it: the object kind in
//! the title (`commit 6b14dc55`, `tag v0.4.0`), the result as a coloured
//! banner (green good, red bad or error, amber signed but untrusted, grey
//! unsigned or unavailable), a label grid of only the rows Git reported,
//! and **Signing settings…** in the footer; the signing configuration
//! itself lives in Preferences › Commit.
use crate::inspect_ui::{self, Tone};
use eframe::egui;
use gg_engine::signature::{State, Verification};

pub struct SignatureWindow {
    pub reference: String,
    pub verification: Option<Verification>,
    pub error: Option<String>,
    pub busy: Option<String>,
}

impl SignatureWindow {
    pub fn new(reference: String) -> Self {
        Self {
            reference,
            verification: None,
            error: None,
            busy: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Close,
    /// Open Preferences › Commit, where the signing configuration is.
    Settings,
}

/// `commit 6b14dc55` or `tag v0.4.0`.
pub fn object_title(reference: &str, verification: Option<&Verification>) -> String {
    let hex = reference.len() >= 7 && reference.bytes().all(|byte| byte.is_ascii_hexdigit());
    let kind = verification
        .and_then(|verification| verification.kind.as_deref())
        .unwrap_or(if hex { "commit" } else { "tag" });
    let name = if hex {
        inspect_ui::short(reference)
    } else {
        reference
    };
    format!("{kind} {name}")
}

/// The banner's tone, icon, title, and detail for a verification.
pub fn banner(verification: &Verification) -> (Tone, &'static str, &'static str, String) {
    let kind = verification.kind.as_deref().unwrap_or("commit");
    let ssh = verification.format.as_deref() == Some("ssh");
    let detail = verification
        .detail
        .as_deref()
        .and_then(|detail| detail.lines().next());
    match verification.state {
        State::Good => (
            Tone::Green,
            inspect_ui::CHECK_CIRCLE,
            "Good signature",
            match (&verification.trust, ssh) {
                (_, true) => "Signed by an allowed signer.".to_string(),
                (Some(trust), false) => format!("Signed by a key with {trust} trust."),
                (None, false) => format!("The signature matches this {kind}\u{2019}s content."),
            },
        ),
        State::Bad => (
            Tone::Red,
            inspect_ui::X_CIRCLE,
            "Bad signature",
            format!("The signature does not match this {kind}\u{2019}s content."),
        ),
        State::UnknownKey => (
            Tone::Amber,
            inspect_ui::WARNING_CIRCLE,
            "Unknown key",
            if ssh {
                "Valid signature, but the key is not in allowed signers.".to_string()
            } else {
                "Signed, but the public key is not in your keyring.".to_string()
            },
        ),
        State::Unsigned => (
            Tone::Neutral,
            inspect_ui::MINUS_CIRCLE,
            "Unsigned",
            format!("This {kind} has no signature."),
        ),
        State::Unavailable => (
            Tone::Neutral,
            inspect_ui::WARNING_CIRCLE,
            "Verification unavailable",
            detail
                .unwrap_or("The signing tool cannot verify here.")
                .to_string(),
        ),
        State::Error => (
            Tone::Red,
            inspect_ui::X_CIRCLE,
            "Verification error",
            detail
                .unwrap_or("Git could not verify the signature.")
                .to_string(),
        ),
    }
}

pub fn draw(ctx: &egui::Context, window: &SignatureWindow) -> Option<Action> {
    let mut action = None;
    let max_width = (ctx.content_rect().width() - 24.0).max(360.0);
    let id = egui::Id::unique("signature-details");
    egui::Window::new("Signature details")
        .id(id)
        .title_bar(false)
        .collapsible(false)
        .resizable(true)
        .default_width(max_width.min(560.0))
        .max_width(max_width)
        .default_pos(ctx.content_rect().center())
        .pivot(egui::Align2::CENTER_CENTER)
        .show(ctx, |ui| {
            let title = format!(
                "Signature · {}",
                object_title(&window.reference, window.verification.as_ref())
            );
            if inspect_ui::header(ui, &title, None, |_| {}) {
                action = Some(Action::Close);
            }
            if let Some(error) = &window.error {
                inspect_ui::banner(
                    ui,
                    Tone::Red,
                    inspect_ui::X_CIRCLE,
                    "Verification error",
                    Some(error.lines().next().unwrap_or_default()),
                    None,
                );
            } else if let Some(verification) = window
                .verification
                .as_ref()
                .filter(|_| window.busy.is_none())
            {
                let (tone, icon, title, detail) = banner(verification);
                let link =
                    (verification.state == State::Unavailable).then_some("Signing settings…");
                if inspect_ui::banner(ui, tone, icon, title, Some(&detail), link)
                    .is_some_and(|link| link.clicked())
                {
                    action = Some(Action::Settings);
                }
                inspect_ui::grid(ui, "signature-grid", |ui| {
                    if let Some(signer) = &verification.signer {
                        inspect_ui::row(ui, "SIGNER", |ui| {
                            ui.add(egui::Label::new(signer).truncate());
                        });
                    }
                    if let Some(key) = &verification.key {
                        inspect_ui::row(ui, "KEY", |ui| {
                            ui.add(egui::Label::new(inspect_ui::mono(key)).truncate());
                            inspect_ui::copy_button(ui, key, "key fingerprint");
                        });
                    }
                    if let Some(trust) = &verification.trust {
                        inspect_ui::row(ui, "TRUST", |ui| {
                            ui.label(trust);
                        });
                    }
                    if let Some(format) = &verification.format {
                        inspect_ui::row(ui, "FORMAT", |ui| {
                            ui.label(format);
                        });
                    }
                });
            } else {
                inspect_ui::banner(
                    ui,
                    Tone::Neutral,
                    inspect_ui::CIRCLE_NOTCH,
                    "Verifying signature…",
                    None,
                    None,
                );
            }
            inspect_ui::footer(
                ui,
                Some((
                    "Read-only check; nothing is changed.",
                    ui.visuals().weak_text_color(),
                )),
                |ui| {
                    if ui.link("Signing settings…").clicked() {
                        action = Some(Action::Settings);
                    }
                },
            );
        });
    if action.is_none()
        && inspect_ui::owns_keys(ctx, id)
        && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
    {
        action = Some(Action::Close);
    }
    action
}

#[cfg(test)]
mod tests {
    use super::*;

    fn verification(state: State) -> Verification {
        Verification {
            reference: "6b14dc55aa".into(),
            kind: Some("commit".into()),
            state,
            format: Some("ssh".into()),
            signer: Some("Ada Lovelace <ada@example.org>".into()),
            key: Some("SHA256:Xq3m0c1C5v8yR7nA2wP9kD4tL6hJ0sF3gB8eZ1uN5oQ".into()),
            trust: None,
            detail: None,
        }
    }

    #[test]
    fn titles_name_the_object_kind() {
        assert_eq!(object_title(&"6b14dc55".repeat(5), None), "commit 6b14dc55");
        assert_eq!(object_title("v0.4.0", None), "tag v0.4.0");
        let mut tag = verification(State::Good);
        tag.kind = Some("tag".into());
        assert_eq!(object_title("v0.4.0", Some(&tag)), "tag v0.4.0");
    }

    #[test]
    fn banners_take_the_state_colour() {
        for (state, tone, title) in [
            (State::Good, Tone::Green, "Good signature"),
            (State::Bad, Tone::Red, "Bad signature"),
            (State::Error, Tone::Red, "Verification error"),
            (State::UnknownKey, Tone::Amber, "Unknown key"),
            (State::Unsigned, Tone::Neutral, "Unsigned"),
            (
                State::Unavailable,
                Tone::Neutral,
                "Verification unavailable",
            ),
        ] {
            let (shown, _, label, _) = banner(&verification(state));
            assert_eq!((shown, label), (tone, title));
        }
        assert_eq!(
            banner(&verification(State::Good)).3,
            "Signed by an allowed signer."
        );
        assert_eq!(
            banner(&verification(State::Unsigned)).3,
            "This commit has no signature."
        );
    }

    #[test]
    fn the_window_lists_only_reported_rows_and_links_the_settings() {
        let mut window = SignatureWindow::new("6b14dc55".repeat(5));
        let mut good = verification(State::Good);
        good.format = None;
        window.verification = Some(good);
        let ctx = egui::Context::default();
        let input = || egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(720.0, 480.0),
            )),
            ..Default::default()
        };
        let _ = crate::tests::run_ui(&ctx, input(), |ui| {
            draw(ui.ctx(), &window);
        });
        let output = crate::tests::run_ui(&ctx, input(), |ui| {
            draw(ui.ctx(), &window);
        });
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        for expected in [
            "Signature · commit 6b14dc55",
            "Good signature",
            "SIGNER",
            "Ada Lovelace <ada@example.org>",
            "KEY",
            "SHA256:Xq3m0c1C5v8yR7nA2wP9kD4tL6hJ0sF3gB8eZ1uN5oQ",
            "Read-only check; nothing is changed.",
            "Signing settings…",
        ] {
            assert!(
                text.iter().any(|line| line.contains(expected)),
                "missing {expected}: {text:?}"
            );
        }
        for absent in ["TRUST", "FORMAT", "Signing configuration", "Signing key"] {
            assert!(
                !text.iter().any(|line| line == absent),
                "{absent} shown: {text:?}"
            );
        }
    }
}
