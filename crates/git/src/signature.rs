//! Verify commit/tag signatures and read explicit signing configuration.
use gg_engine::signature::{Config, Verification, parse_raw};
use std::{
    path::Path,
    process::{Command, Stdio},
};

fn command(path: &Path) -> Command {
    let mut command = Command::new("git");
    command
        .current_dir(path)
        .stdin(Stdio::null())
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE");
    command
}

fn config_value(path: &Path, key: &str) -> Option<String> {
    command(path)
        .args(["config", "--get", key])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|value| !value.is_empty())
}

fn config_bool(path: &Path, key: &str) -> bool {
    config_value(path, key).is_some_and(|value| {
        matches!(
            value.trim().to_lowercase().as_str(),
            "true" | "yes" | "on" | "1"
        )
    })
}

/// Read signing configuration without modifying it.
pub fn config(path: &Path) -> Config {
    let format = config_value(path, "gpg.format").unwrap_or_else(|| "openpgp".into());
    let signing_key = config_value(path, "user.signingkey");
    let gpg_program = config_value(path, "gpg.program");
    let commit_signing = config_bool(path, "commit.gpgsign");
    let tag_signing = config_bool(path, "tag.gpgsign");
    let allowed_signers = config_value(path, "gpg.ssh.allowedSignersFile");
    let program = match format.as_str() {
        "ssh" => config_value(path, "gpg.ssh.program").unwrap_or_else(|| "ssh-keygen".into()),
        "x509" => config_value(path, "gpg.x509.program").unwrap_or_else(|| "gpgsm".into()),
        _ => gpg_program.clone().unwrap_or_else(|| "gpg".into()),
    };
    let mut command = Command::new(&program);
    if format == "ssh" {
        command.arg("-?");
    } else {
        command.arg("--version");
    }
    let tool_available = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| format == "ssh" || status.success());
    let note = if signing_key.is_none() {
        Some("No signing key is configured.".into())
    } else if !tool_available {
        Some(format!("Signing tool {program} is unavailable."))
    } else {
        None
    };
    Config {
        format,
        signing_key,
        gpg_program,
        commit_signing,
        tag_signing,
        allowed_signers,
        tool_available,
        note,
    }
}

/// Verify one commit or tag object through Git's configured signature tool.
pub fn verify(path: &Path, reference: &str) -> Result<Verification, String> {
    let reference = reference.trim();
    if reference.is_empty() || reference.starts_with('-') || reference.contains('\0') {
        return Err("Signature reference is invalid.".into());
    }
    let kind = command(path)
        .args(["cat-file", "-t", reference])
        .output()
        .map_err(|error| format!("Could not inspect {reference}: {error}"))?;
    if !kind.status.success() {
        return Err(format!("Could not resolve {reference}."));
    }
    let kind = String::from_utf8_lossy(&kind.stdout).trim().to_string();
    let verb = match kind.as_str() {
        "commit" => "verify-commit",
        "tag" => "verify-tag",
        other => return Err(format!("{reference} is a {other}, not a commit or tag.")),
    };
    let format = config_value(path, "gpg.format").unwrap_or_else(|| "openpgp".into());
    let output = command(path)
        .args([verb, "--raw", reference])
        .output()
        .map_err(|error| format!("Could not verify {reference}: {error}"))?;
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    Ok(parse_raw(
        reference,
        Some(&kind),
        output.status.success(),
        &text,
        Some(&format),
    ))
}
