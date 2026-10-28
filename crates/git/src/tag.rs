//! Delete local tags.
use std::{
    path::Path,
    process::{Command, Stdio},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeleteReceipt {
    pub tag: String,
}

pub fn delete(path: &Path, tag: &str) -> Result<DeleteReceipt, String> {
    let tag = tag.trim();
    if tag.is_empty() || tag.starts_with('-') || tag.contains('\0') {
        return Err("Tag name is invalid.".into());
    }
    let mut command = Command::new("git");
    command
        .current_dir(path)
        .arg("tag")
        .arg("-d")
        .arg(tag)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE");
    let output = command.output().map_err(|e| {
        format!("Could not run tag deletion using Git: {e}. Install Git and retry.")
    })?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!(
            "Delete tag {tag} failed ({}).{}",
            output.status,
            if detail.is_empty() {
                String::new()
            } else {
                format!("\n{detail}")
            }
        ));
    }
    Ok(DeleteReceipt {
        tag: tag.to_string(),
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateReceipt {
    pub tag: String,
    pub target: String,
    pub id: String,
    pub signed: bool,
}

/// Create one annotated or signed tag at an explicit target without moving HEAD.
pub fn create(
    path: &Path,
    tag: &str,
    target: &str,
    message: &str,
    signed: bool,
) -> Result<CreateReceipt, String> {
    let tag = tag.trim();
    let target = target.trim();
    let message = message.trim_end();
    if tag.is_empty() || tag.starts_with('-') || tag.contains('\0') {
        return Err("Tag name is invalid.".into());
    }
    if target.is_empty() || target.starts_with('-') || target.contains('\0') {
        return Err("Tag target is invalid.".into());
    }
    if message.trim().is_empty() {
        return Err("Tag message is empty. Add a subject before creating the tag.".into());
    }
    let output = Command::new("git")
        .current_dir(path)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .arg("tag")
        .arg(if signed { "-s" } else { "-a" })
        .arg("-m")
        .arg(message)
        .arg("--")
        .arg(tag)
        .arg(target)
        .output()
        .map_err(|e| {
            format!("Could not run tag creation using Git: {e}. Install Git and retry.")
        })?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr);
        let detail = detail.trim();
        return Err(format!(
            "Create tag {tag} failed ({}). Nothing was created; review the name, target, and signing setup.{}",
            output.status,
            if detail.is_empty() {
                String::new()
            } else {
                format!("\n{detail}")
            }
        ));
    }
    let id = Command::new("git")
        .current_dir(path)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .args(["rev-parse", &format!("refs/tags/{tag}")])
        .output()
        .map_err(|e| format!("Read created tag: {e}"))?;
    let id = if id.status.success() {
        String::from_utf8_lossy(&id.stdout).trim().to_string()
    } else {
        String::new()
    };
    Ok(CreateReceipt {
        tag: tag.to_string(),
        target: target.to_string(),
        id,
        signed,
    })
}
