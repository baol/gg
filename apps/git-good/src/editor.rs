//! Open in editor: the editor command from Preferences, or the system's
//! default app for the file when the command is empty.

use std::path::Path;
use std::process::Command;

/// The program and arguments for `command` opening `file`. `{file}` is
/// replaced by the path; without it the path is added at the end. Quotes
/// group words: `"/Applications/My Editor.app/x" --wait`.
pub fn command_line(command: &str, file: &Path) -> Option<(String, Vec<String>)> {
    let file = file.to_string_lossy();
    let mut words = split(command);
    if words.is_empty() {
        return None;
    }
    if words.iter().any(|word| word.contains("{file}")) {
        for word in &mut words {
            *word = word.replace("{file}", &file);
        }
    } else {
        words.push(file.into_owned());
    }
    let program = words.remove(0);
    Some((program, words))
}

/// Split on whitespace, keeping double- or single-quoted runs together.
fn split(command: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    let mut started = false;
    for ch in command.chars() {
        match (quote, ch) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), c) => word.push(c),
            (None, '"' | '\'') => {
                quote = Some(ch);
                started = true;
            }
            (None, c) if c.is_whitespace() => {
                if started || !word.is_empty() {
                    words.push(std::mem::take(&mut word));
                }
                started = false;
            }
            (None, c) => word.push(c),
        }
    }
    if started || !word.is_empty() {
        words.push(word);
    }
    words
}

/// Start the editor for `file` without waiting for it to exit.
pub fn open(command: &str, file: &Path) -> Result<(), String> {
    let (program, args) = command_line(command, file).unwrap_or_else(|| system_opener(file));
    Command::new(&program)
        .args(&args)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Open {} with {program}: {error}", file.display()))
}

/// The platform's own "open with the default app" command.
fn system_opener(file: &Path) -> (String, Vec<String>) {
    let file = file.to_string_lossy().into_owned();
    if cfg!(target_os = "macos") {
        ("open".into(), vec![file])
    } else if cfg!(windows) {
        (
            "cmd".into(),
            vec!["/C".into(), "start".into(), String::new(), file],
        )
    } else {
        ("xdg-open".into(), vec![file])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_goes_where_the_command_says_or_at_the_end() {
        let file = Path::new("/r/a b.rs");
        assert_eq!(
            command_line("code --wait {file}", file),
            Some((
                "code".to_string(),
                vec!["--wait".to_string(), "/r/a b.rs".to_string()]
            ))
        );
        assert_eq!(
            command_line("subl", file),
            Some(("subl".to_string(), vec!["/r/a b.rs".to_string()]))
        );
        assert_eq!(
            command_line("\"/Apps/My Editor\" -n '{file}:1'", file),
            Some((
                "/Apps/My Editor".to_string(),
                vec!["-n".to_string(), "/r/a b.rs:1".to_string()]
            ))
        );
        assert_eq!(
            command_line("   ", file),
            None,
            "empty uses the system opener"
        );
    }
}
