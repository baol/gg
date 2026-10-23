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
