//! Every public function and constant this crate exports has a caller.
//!
//! # Why this gate, and why here
//!
//! MJXOFF-155 §9 item 10 records *dead public API* as a **pattern** rather than an instance —
//! twenty-three `pub fn` in `mjx-text` with zero readers, including the one its own report names as
//! the mitigation for the single risk it identifies — and says *"the cheap systemic fix is a gate,
//! not a list"*. This is that gate, over this crate.
//!
//! It matters more here than anywhere else in the workspace, because this crate **is** a contract.
//! An exported method with no caller and no test is an unverified claim about the seam every layer
//! above is written against: it compiles, so it looks like a decision, and nothing has ever run it.
//! Four independent box models will be written against these signatures, and the first one to use
//! an untested corner finds out that it was a guess.
//!
//! # What "reachable" means, and what this gate does not claim
//!
//! A symbol is reachable when its name appears somewhere other than its own definition — in this
//! crate's source, or in one of its test suites. That is a **name** check, not a call-graph
//! analysis: it cannot tell a call from a mention in a `use`, and it would be satisfied by a test
//! that named a function without asserting anything about it. What it *does* catch is the thing that
//! actually happens — a symbol produced, exported, and then never written down again anywhere — and
//! it catches it automatically, on the next `cargo test`, rather than on the next audit.
//!
//! The complement is deliberate too: **the gate is a reason to keep the surface small.** Every
//! `pub fn` here either has a caller or has to be deleted, so the crate cannot accumulate a
//! complete-looking API nobody has run.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Names that are allowed to have no caller, each with the reason.
///
/// **Kept empty on purpose.** A list is what this gate exists to replace; an entry here is a
/// standing exception and should be argued for in the same breath as it is added.
const ALLOWED_WITHOUT_A_CALLER: &[(&str, &str)] = &[];

fn crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn rust_files(directory: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![directory.to_path_buf()];
    while let Some(current) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|suffix| suffix == "rs") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// The identifier that follows `keyword` on `line`, if the line declares one publicly.
fn declared_name<'a>(line: &'a str, keyword: &str) -> Option<&'a str> {
    let code = line.trim_start();
    let rest = code.strip_prefix("pub ")?;
    let rest = rest.strip_prefix(keyword).or_else(|| {
        // `pub const fn` is a function, not a constant, and would otherwise be counted twice.
        rest.strip_prefix("const ")?.strip_prefix(keyword)
    })?;
    let name: &str = rest
        .split(|character: char| !character.is_alphanumeric() && character != '_')
        .find(|piece| !piece.is_empty())?;
    Some(name)
}

/// Whether `haystack` contains `name` as a whole identifier rather than as a substring.
fn mentions(haystack: &str, name: &str) -> bool {
    let mut from = 0;
    while let Some(offset) = haystack[from..].find(name) {
        let start = from + offset;
        let end = start + name.len();
        let before = haystack[..start]
            .chars()
            .next_back()
            .is_none_or(|character| !character.is_alphanumeric() && character != '_');
        let after = haystack[end..]
            .chars()
            .next()
            .is_none_or(|character| !character.is_alphanumeric() && character != '_');
        if before && after {
            return true;
        }
        from = end;
    }
    false
}

#[test]
fn every_public_function_and_constant_has_a_caller() {
    let root = crate_root();
    let sources = rust_files(&root.join("src"));
    let tests = rust_files(&root.join("tests"));
    assert!(
        sources.len() >= 8 && tests.len() >= 6,
        "the walk found {} source and {} test file(s), which cannot be this crate",
        sources.len(),
        tests.len()
    );

    // Where each name was declared, and every line of code that could mention it.
    let mut declarations: Vec<(String, PathBuf, usize)> = Vec::new();
    let mut corpus = String::new();
    for path in sources.iter().chain(tests.iter()) {
        let text = std::fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
        for (number, line) in text.lines().enumerate() {
            let is_declaration = ["fn ", "const "]
                .iter()
                .find_map(|keyword| declared_name(line, keyword).map(|name| (name, *keyword)));
            match is_declaration {
                Some((name, _)) if path.starts_with(root.join("src")) => {
                    declarations.push((name.to_owned(), path.clone(), number + 1));
                }
                _ => {}
            }
            // The declaration line itself is never evidence of a caller; every other line is,
            // including a comment, because a symbol a doc comment links to is a symbol somebody has
            // at least read.
            if is_declaration.is_none() {
                corpus.push_str(line);
                corpus.push('\n');
            }
        }
    }

    assert!(
        declarations.len() > 80,
        "only {} public functions and constants were found; the scan is not reading the crate",
        declarations.len()
    );

    let exempt: BTreeSet<&str> = ALLOWED_WITHOUT_A_CALLER
        .iter()
        .map(|(name, _)| *name)
        .collect();
    let mut orphans: Vec<String> = Vec::new();
    for (name, path, line) in &declarations {
        if exempt.contains(name.as_str()) {
            continue;
        }
        if !mentions(&corpus, name) {
            orphans.push(format!(
                "{}:{line} — `{name}` is exported and named nowhere else",
                path.display()
            ));
        }
    }

    assert!(
        orphans.is_empty(),
        "this crate is a contract, and an exported symbol with no caller is an unverified claim \
         about it. Either give each of these a caller and a test, or delete it:\n  {}",
        orphans.join("\n  ")
    );
}

#[test]
fn the_gate_can_tell_a_named_symbol_from_an_unnamed_one() {
    // The gate rests on `declared_name` and `mentions`, so both are checked directly — otherwise a
    // parser that recognised nothing would report no orphans and pass for ever.
    assert_eq!(
        declared_name("    pub fn column_count(self) -> u16 {", "fn "),
        Some("column_count")
    );
    assert_eq!(
        declared_name("    pub const fn number(self) -> u32 {", "fn "),
        Some("number")
    );
    assert_eq!(
        declared_name("pub const MAXIMUM_CELL_SPAN: usize = 16;", "const "),
        Some("MAXIMUM_CELL_SPAN")
    );
    assert_eq!(declared_name("    fn private_helper() {", "fn "), None);
    assert_eq!(
        declared_name("    pub(crate) fn from_index(index: usize)", "fn "),
        None,
        "only a fully public symbol is a claim about the seam"
    );

    assert!(mentions("a.column_count()", "column_count"));
    assert!(mentions("use crate::column_count;", "column_count"));
    assert!(
        !mentions("a.column_counts()", "column_count"),
        "a substring is not a mention, or every short name would look reachable"
    );
    assert!(!mentions("my_column_count", "column_count"));
    assert!(mentions("column_count", "column_count"));

    // And the exception list is empty, which is what makes this a gate rather than a list.
    assert!(
        ALLOWED_WITHOUT_A_CALLER.is_empty(),
        "an exception must be argued for: {ALLOWED_WITHOUT_A_CALLER:?}"
    );
}
