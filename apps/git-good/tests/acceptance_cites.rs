//! Every test docs/acceptance.md cites as coverage exists, in the file it
//! names: a renamed or deleted test cannot leave a case looking covered.
//!
//! On a `Covered by` line, each backticked `snake_case` name is a test. The
//! next parenthesis that names `.rs` files or folders (`(apps/git-good/src/
//! main.rs)`, `(diff_view.rs)`, `(crates/engine)`) is where its names must
//! be defined; a name with no such parenthesis after it must be defined
//! somewhere in the workspace.
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every `.rs` file under `dir`, as a path relative to the workspace.
fn sources(root: &Path, dir: &Path, found: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        if path.is_dir() {
            if name != "target" && !name.to_string_lossy().starts_with('.') {
                sources(root, &path, found);
            }
        } else if path.extension().is_some_and(|extension| extension == "rs")
            && let Ok(relative) = path.strip_prefix(root)
        {
            found.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
}

/// The functions each file defines: `fn name`.
fn definitions(root: &Path) -> BTreeMap<String, BTreeSet<String>> {
    let mut files = Vec::new();
    for dir in ["apps", "crates"] {
        sources(root, &root.join(dir), &mut files);
    }
    let mut defined: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for file in files {
        let Ok(text) = std::fs::read_to_string(root.join(&file)) else {
            continue;
        };
        for piece in text.split("fn ").skip(1) {
            let name: String = piece
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                defined.entry(name).or_default().insert(file.clone());
            }
        }
    }
    defined
}

/// A test name: lowercase snake case with at least one underscore.
fn test_name(text: &str) -> bool {
    text.contains('_')
        && text.starts_with(|c: char| c.is_ascii_lowercase())
        && text
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// The `.rs` files and folders a parenthesis names.
fn places(text: &str) -> Vec<String> {
    text.split(|c: char| c.is_whitespace() || c == ',' || c == ';')
        .map(|word| word.trim_matches('`'))
        .filter(|word| {
            word.ends_with(".rs") || word.starts_with("apps/") || word.starts_with("crates/")
        })
        .map(|word| word.trim_end_matches('/').to_owned())
        .collect()
}

fn defined_in(files: &BTreeSet<String>, place: &str) -> bool {
    files.iter().any(|file| {
        file == place
            || file.ends_with(&format!("/{place}"))
            || file.starts_with(&format!("{place}/"))
    })
}

/// The cited names that are not defined where the line says: (line, name,
/// places).
fn missing(document: &str, defined: &BTreeMap<String, BTreeSet<String>>) -> Vec<String> {
    let none = BTreeSet::new();
    let mut missing = Vec::new();
    for (number, line) in document.lines().enumerate() {
        if !line.starts_with("Covered by") {
            continue;
        }
        let mut pending: Vec<&str> = Vec::new();
        let mut check = |pending: &mut Vec<&str>, at: &[String]| {
            for name in pending.drain(..) {
                let files = defined.get(name).unwrap_or(&none);
                let found = if at.is_empty() {
                    !files.is_empty()
                } else {
                    at.iter().any(|place| defined_in(files, place))
                };
                if !found {
                    let at = if at.is_empty() {
                        "the workspace".to_owned()
                    } else {
                        at.join(", ")
                    };
                    missing.push(format!("line {}: `{name}` is not in {at}", number + 1));
                }
            }
        };
        let mut rest = line;
        loop {
            let tick = rest.find('`');
            let paren = rest.find('(');
            match (tick, paren) {
                (Some(t), p) if p.is_none_or(|p| t < p) => {
                    let after = &rest[t + 1..];
                    let Some(end) = after.find('`') else { break };
                    let name = &after[..end];
                    if test_name(name) {
                        pending.push(name);
                    }
                    rest = &after[end + 1..];
                }
                (_, Some(p)) => {
                    let after = &rest[p + 1..];
                    let end = after.find(')').unwrap_or(after.len());
                    let at = places(&after[..end]);
                    if !at.is_empty() {
                        check(&mut pending, &at);
                    }
                    rest = &after[(end + 1).min(after.len())..];
                }
                _ => break,
            }
        }
        check(&mut pending, &[]);
    }
    missing
}

#[test]
fn every_test_acceptance_cites_exists_where_it_says() -> Result<(), Box<dyn std::error::Error>> {
    let root = workspace();
    let document = std::fs::read_to_string(root.join("docs/acceptance.md"))?;
    let defined = definitions(&root);
    assert!(defined.len() > 1000, "the workspace sources were read");
    let missing = missing(&document, &defined);
    assert!(
        missing.is_empty(),
        "acceptance.md cites tests that do not exist there:\n{}",
        missing.join("\n")
    );
    Ok(())
}

#[test]
fn a_renamed_or_misplaced_test_is_reported() {
    let defined = BTreeMap::from([
        (
            "draws_the_rows".to_owned(),
            BTreeSet::from(["apps/git-good/src/diff_view.rs".to_owned()]),
        ),
        (
            "parses_a_pointer".to_owned(),
            BTreeSet::from(["crates/git/tests/lfs.rs".to_owned()]),
        ),
    ]);
    let ok = "Covered by (interaction): `draws_the_rows` (diff_view.rs); `parses_a_pointer` (crates/git), `Ctrl+F` and `x`";
    assert!(
        missing(ok, &defined).is_empty(),
        "{:?}",
        missing(ok, &defined)
    );
    let bad = "Covered by (interaction): `draws_the_rows` (apps/git-good/src/main.rs), `gone_test`";
    assert_eq!(
        missing(bad, &defined),
        [
            "line 1: `draws_the_rows` is not in apps/git-good/src/main.rs",
            "line 1: `gone_test` is not in the workspace",
        ]
    );
}
