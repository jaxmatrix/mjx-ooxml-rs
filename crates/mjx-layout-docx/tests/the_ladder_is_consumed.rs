//! **The effective-property ladder is consumed, never re-derived** — held by grepping this crate's
//! own source for the identifiers a re-derivation would need.
//!
//! # Why an identifier gate rather than a value assertion
//!
//! A second implementation of the ladder would *agree* with the first on every fixture anyone
//! bothered to write, and would drift the first time `mjx-docx` fixed something. The failure is not
//! a wrong answer today; it is two answers free to diverge tomorrow. So what is asserted is that the
//! machinery is not here: no `w:basedOn` walk, no `w:docDefaults` read, no style index, no toggle
//! recombination.
//!
//! MJXOFF-172 learned the sharper version of this — a table pasted in as string literals satisfies
//! an identifier gate perfectly — so the second half of this file checks the other direction: the
//! resolved types **are** named, which is what says the ladder is being read rather than avoided.

use std::path::{Path, PathBuf};

/// Machinery that only a crate resolving the ladder itself would need.
const RE_DERIVATION: &[&str] = &[
    "StyleIndex",
    "ChainCache",
    "based_on_chain",
    "document_defaults",
    "DocumentDefaults",
    "DefaultRunProperties",
    "DefaultParagraphProperties",
    "StyleSheet",
    "StyleDefinition",
    "merge_under",
    "recombine_toggles",
    "combine_toggle",
    "resolve_numbering",
    "NumberingIndex",
    "effective_run_properties",
    "effective_paragraph_properties",
];

/// The resolved values this crate must be reading, which is what says it consumes the ladder rather
/// than working around it.
const CONSUMED: &[&str] = &[
    "EffectiveParagraphProperties",
    "EffectiveCharacterProperties",
    "EffectiveTabStop",
    "DocumentFormatting",
    "ParagraphFormatting",
    "DocumentLayoutSettings",
];

fn source_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn rust_files(directory: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![directory.to_path_buf()];
    while let Some(current) = pending.pop() {
        for entry in std::fs::read_dir(&current).expect("a readable directory") {
            let path = entry.expect("a directory entry").path();
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

/// Every line of `src/` that is code rather than prose.
fn code_lines() -> Vec<(PathBuf, String)> {
    let mut lines = Vec::new();
    for file in rust_files(&source_root()) {
        let code = std::fs::read_to_string(&file).expect("a readable source file");
        for line in code.lines() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            lines.push((file.clone(), line.to_owned()));
        }
    }
    lines
}

#[test]
fn the_ladder_is_not_re_derived_here() {
    let mut offences = Vec::new();
    for (file, line) in code_lines() {
        for identifier in RE_DERIVATION {
            if line.contains(identifier) {
                offences.push(format!("{}: {}", file.display(), line.trim()));
            }
        }
    }
    assert!(
        offences.is_empty(),
        "`mjx-docx` resolves the ladder and this crate reads the answer:\n{}",
        offences.join("\n")
    );
}

#[test]
fn the_resolved_values_are_actually_read() {
    let code: String = code_lines()
        .into_iter()
        .map(|(_, line)| line)
        .collect::<Vec<_>>()
        .join("\n");
    for identifier in CONSUMED {
        assert!(
            code.contains(identifier),
            "`{identifier}` is what the ladder answers with; a crate that never named it would be \
             getting its values from somewhere else"
        );
    }
}

/// The residency is what makes laying a long document out affordable, and a crate that had quietly
/// gone back to the per-paragraph reader would be quadratic again without failing anything else.
#[test]
fn the_residency_is_what_is_read_and_not_the_per_paragraph_reader() {
    let code: String = code_lines()
        .into_iter()
        .map(|(_, line)| line)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        code.contains("document.formatting()") || code.contains(".formatting()?"),
        "`Document::formatting` — the read-once surface — must be what this crate calls"
    );
}
