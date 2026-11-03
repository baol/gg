//! Every production Git process starts at `gg_git::process::git` (RULE-16).
use std::path::{Path, PathBuf};

fn sources(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            sources(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
    Ok(())
}

/// Modules declared `#[cfg(test)] mod name;`: whole files of tests.
fn test_modules(text: &str) -> Vec<String> {
    let lines = text.lines().map(str::trim).collect::<Vec<_>>();
    lines
        .windows(2)
        .filter(|pair| pair[0] == "#[cfg(test)]")
        .filter_map(|pair| {
            pair[1]
                .strip_prefix("mod ")
                .and_then(|rest| rest.strip_suffix(';'))
                .map(str::to_string)
        })
        .collect()
}

/// The text before the file's `#[cfg(test)] mod … {` block.
fn production(text: &str) -> &str {
    let mut offset = 0;
    let mut previous_was_cfg_test = false;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim();
        if previous_was_cfg_test && trimmed.starts_with("mod ") && trimmed.ends_with('{') {
            return &text[..offset];
        }
        previous_was_cfg_test = trimmed == "#[cfg(test)]";
        offset += line.len();
    }
    text
}

#[test]
fn git_processes_start_at_the_chokepoint() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    for dir in ["crates/git/src", "crates/engine/src", "apps/git-good/src"] {
        sources(&root.join(dir), &mut files)?;
    }
    let mut test_only = Vec::new();
    for file in &files {
        let text = std::fs::read_to_string(file)?;
        let dir = file.parent().ok_or("source without a folder")?;
        for module in test_modules(&text) {
            test_only.push(dir.join(format!("{module}.rs")));
        }
    }
    let needle = concat!("Command::new(\"", "git");
    let mut offenders = Vec::new();
    for file in &files {
        if file.ends_with("crates/git/src/process.rs") || test_only.contains(file) {
            continue;
        }
        let text = std::fs::read_to_string(file)?;
        for (number, line) in production(&text).lines().enumerate() {
            if line.contains(needle) {
                offenders.push(format!("{}:{}", file.display(), number + 1));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "start Git with gg_git::process::git() so the RULE-16 parity tests see it:\n{}",
        offenders.join("\n")
    );
    // The lint itself must see the tree: the chokepoint and its callers exist.
    assert!(files.iter().any(|file| file.ends_with("process.rs")));
    assert!(files.len() > 50);
    Ok(())
}
