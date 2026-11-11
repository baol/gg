//! Who commits: Git's `user.name` and `user.email`, read and written through
//! the user's own `git config`.
use crate::process::git;
use std::path::Path;

/// Where an identity is written.
#[derive(Clone, Copy, Debug)]
pub enum Scope<'a> {
    /// The user's global config, as `git config --global` writes it.
    Global,
    /// One config file, as `git config --file` writes it.
    File(&'a Path),
}

/// `user.name` and `user.email`, each when Git has it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Identity {
    pub name: Option<String>,
    pub email: Option<String>,
}

impl Identity {
    /// Both are set, so Git can commit.
    pub fn complete(&self) -> bool {
        self.name.is_some() && self.email.is_some()
    }

    /// `Name <email>`, `Name` or `<email>`; `None` when Git has neither.
    pub fn label(&self) -> Option<String> {
        match (&self.name, &self.email) {
            (Some(name), Some(email)) => Some(format!("{name} <{email}>")),
            (Some(name), None) => Some(name.clone()),
            (None, Some(email)) => Some(format!("<{email}>")),
            (None, None) => None,
        }
    }
}

/// The identity Git would commit with: in `repo` when given (its own config
/// first), otherwise from the global and system config.
pub fn read(repo: Option<&Path>) -> Identity {
    let get = |key: &str| {
        let mut command = git();
        if let Some(repo) = repo {
            command.arg("-C").arg(repo);
        }
        value(command.args(["config", "--get", key]))
    };
    Identity {
        name: get("user.name"),
        email: get("user.email"),
    }
}

/// The identity written in one scope only.
pub fn read_scope(scope: Scope) -> Identity {
    let get = |key: &str| {
        let mut command = git();
        command.arg("config");
        scope_args(&mut command, scope);
        value(command.args(["--get", key]))
    };
    Identity {
        name: get("user.name"),
        email: get("user.email"),
    }
}

/// Write `user.name` and `user.email` to `scope`. Both are required, on one
/// line each.
pub fn set(scope: Scope, name: &str, email: &str) -> Result<(), String> {
    let name = name.trim();
    let email = email.trim();
    if name.is_empty() || email.is_empty() {
        return Err("Enter both a name and an email.".into());
    }
    if name.contains(['\n', '\r']) || email.contains(['\n', '\r']) {
        return Err("A name or email cannot span lines.".into());
    }
    if email.contains(['<', '>']) {
        return Err("Enter the email without < >.".into());
    }
    for (key, value) in [("user.name", name), ("user.email", email)] {
        let mut command = git();
        command.arg("config");
        scope_args(&mut command, scope);
        let output = command
            .args(["--", key, value])
            .output()
            .map_err(|error| format!("Could not run git: {error}"))?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
        }
    }
    Ok(())
}

fn scope_args(command: &mut crate::process::GitCommand, scope: Scope) {
    match scope {
        Scope::Global => {
            command.arg("--global");
        }
        Scope::File(path) => {
            command.arg("--file").arg(path);
        }
    }
}

fn value(command: &mut crate::process::GitCommand) -> Option<String> {
    command
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .filter(|value| !value.is_empty())
}
