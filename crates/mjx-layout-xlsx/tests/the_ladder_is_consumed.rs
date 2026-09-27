//! **The `xf` ladder is consumed, never re-derived** — which is the whole reason this crate sits
//! above the format tier instead of inside it.
//!
//! `mjx-sml` resolves a cell's formatting through a walk nobody should write twice: the cell's own
//! `@s`, else the row's when it writes `customFormat="1"`, else a `col` run's `@style`, else
//! `cellXfs[0]`; and then, per aspect, either the `cellXfs` record or the `cellStyleXfs` record
//! beneath it according to that aspect's `applyX` flag, in all three of that flag's states. Six
//! aspects, two tables, four sources.
//!
//! A second walk written here would be a second answer to the same question, free to drift, and
//! invisible to `crates/mjx-sml/tests/effective_cell_format.rs` — the suite that actually pins the
//! behaviour. So this file greps this crate's own source for the identifiers a re-derivation would
//! need, and fails if any appears.
//!
//! # This gate learned its own defect from R15
//!
//! MJXOFF-170 found that `mjx-layout-pptx`'s equivalent matcher excluded `effective_` **by
//! substring**, so `effective_cell_run_properties` read as the declared `run_properties(` and the
//! gate had been passing on a match it should have refused. The fix was to reconstruct the
//! identifier rather than to search inside one, and it is what this file does from the start:
//! [`names`] checks the character on each side of a match.

use std::path::{Path, PathBuf};

/// The `mjx-sml` and `mjx-xlsx` calls this crate is **allowed** to make about formatting.
///
/// Every one of them is an answer the ladder has already produced. Adding to this list is a claim
/// that a new question is being asked of the format tier, which is a decision.
const PERMITTED_FORMAT_CALLS: &[&str] = &[
    "effective_cell_format",
    "resolver",
    "formats",
    "columns",
    "style_index",
    "font",
    "fill",
    "border",
    "alignment",
    "number_format",
    "format_code",
    "format_code_in",
    "properties",
];

/// The identifiers a crate re-deriving the ladder would have to name.
///
/// Each is a *step* of the walk rather than its answer: reaching for `cellXfs` or `cell_style_index`
/// means computing which record applies, which is `mjx-sml`'s job and is already done.
const REDERIVATION: &[&str] = &[
    "cell_style_index",
    "cell_formats",
    "cell_style_formats",
    "cellXfs",
    "cellStyleXfs",
    "apply_attribute",
    "ApplyFlag",
    "FormatLayer",
    "StyleIndexSource",
    "CellFormatTable",
    "NamedCellStyles",
    "custom_format",
    "builtin_format_code",
    "column_style_index",
];

/// The identifiers a crate re-deriving *merge* resolution would have to name, rather than asking
/// `mjx-sml` which range covers a cell.
const MERGE_REDERIVATION: &[&str] = &["MergedCells", "merge_cells", "unmerge_cells", "cell_span"];

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

/// Whether `code` names `needle` as a whole identifier — reconstructed, never a substring match.
fn names(code: &str, needle: &str) -> bool {
    let mut from = 0;
    while let Some(at) = code[from..].find(needle) {
        let start = from + at;
        let end = start + needle.len();
        let before_ends = start == 0
            || !code[..start]
                .chars()
                .next_back()
                .is_some_and(|character| character.is_alphanumeric() || character == '_');
        let after_ends = code[end..]
            .chars()
            .next()
            .is_none_or(|character| !character.is_alphanumeric() && character != '_');
        if before_ends && after_ends {
            return true;
        }
        from = end;
    }
    false
}

/// Every line of `src/` that is code rather than prose.
fn code_lines() -> Vec<(PathBuf, String)> {
    let mut lines = Vec::new();
    for file in rust_files(&source_root()) {
        let text = std::fs::read_to_string(&file).expect("a readable source file");
        for line in text.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") {
                continue;
            }
            lines.push((file.clone(), line.to_owned()));
        }
    }
    lines
}

#[test]
fn no_source_line_re_derives_the_effective_format_walk() {
    let mut offences = Vec::new();
    for (file, line) in code_lines() {
        for identifier in REDERIVATION {
            if names(&line, identifier) {
                offences.push(format!(
                    "{}: `{identifier}` — the ladder is `mjx-sml`'s and has already run\n    {}",
                    file.display(),
                    line.trim()
                ));
            }
        }
    }
    assert!(offences.is_empty(), "{}", offences.join("\n"));
}

#[test]
fn no_source_line_re_derives_merge_resolution() {
    let mut offences = Vec::new();
    for (file, line) in code_lines() {
        for identifier in MERGE_REDERIVATION {
            if names(&line, identifier) {
                offences.push(format!(
                    "{}: `{identifier}`\n    {}",
                    file.display(),
                    line.trim()
                ));
            }
        }
    }
    assert!(offences.is_empty(), "{}", offences.join("\n"));
}

#[test]
fn the_permitted_calls_are_actually_made() {
    // The instrument's own instrument. A list of forbidden identifiers is satisfied by a crate that
    // asks the format tier nothing at all — which would mean the layout was inventing its
    // formatting rather than consuming it. This asserts the *permitted* calls are present, so the
    // gate above is measuring a crate that really does consume the ladder.
    let all: String = code_lines()
        .into_iter()
        .map(|(_, line)| line)
        .collect::<Vec<_>>()
        .join("\n");
    let missing: Vec<&&str> = PERMITTED_FORMAT_CALLS
        .iter()
        .filter(|call| !names(&all, call))
        .collect();
    assert!(
        missing.is_empty(),
        "these consumption points vanished, which means the ladder is being answered somewhere \
         else: {missing:?}"
    );
}

#[test]
fn nothing_here_evaluates_a_formula() {
    // There is no calculation engine in this loop at all — a cached value is rendered as stored,
    // which is correct for a viewer — and it is easy to start one by accident while making a cell's
    // text "look right".
    //
    // ⚠ This gate used to forbid `evaluate` as well, because until MJXOFF-172 a `numFmt` evaluator
    // was the other subject this crate deliberately did not have. R17 **is** that evaluator, so the
    // identifier is now expected rather than refused, and what replaced the refusal is
    // `the_number_format_engine_carries_no_table_of_its_own` below: the evaluator is welcome, a
    // second copy of §18.8.30 is not.
    let forbidden = [
        "recalculate",
        "calc_chain",
        "CalcChain",
        "shared_formula_expand",
    ];
    let mut offences = Vec::new();
    for (file, line) in code_lines() {
        for identifier in forbidden {
            if names(&line, identifier) {
                offences.push(format!("{}: `{identifier}`", file.display()));
            }
        }
    }
    assert!(offences.is_empty(), "{}", offences.join("\n"));
}

#[test]
fn the_number_format_engine_carries_no_table_of_its_own() {
    // ⚠ The defect MJXOFF-172's ticket names by name: **the built-in formats are a table nobody
    // wrote down**. Ids 0–49 are defined by ECMA-376 Part 1 §18.8.30 and are *not* present in the
    // file — a cell that says `numFmtId="14"` carries no format string anywhere — so an evaluator
    // that kept its own copy would look right and drift, and a *short* copy would fall back to
    // `General` for the missing ids and look plausible while doing it.
    //
    // `mjx-sml` already holds that table (`builtin_format_code`, twenty-eight ids under *All
    // Languages* plus five locale tables), and `REDERIVATION` above already refuses this crate to
    // name the function. This refuses the other half: the **codes themselves** must not appear in
    // this crate's source, because a second table pasted in as string literals would satisfy the
    // identifier gate perfectly.
    //
    // The check asks `mjx-sml` for the list rather than restating it, so a code added on either side
    // is covered without editing this file.
    let all: String = code_lines()
        .into_iter()
        .map(|(file, line)| format!("{}\u{1}{line}", file.display()))
        .collect::<Vec<_>>()
        .join("\n");
    let mut offences = Vec::new();
    for id in 0..=49_u32 {
        let Some(code) = mjx_sml::builtin_format_code(id) else {
            continue;
        };
        // Short codes are tokens of the language as well as table entries — `0`, `@`, `0.00`,
        // `h:mm` — and refusing those would refuse the parser's own vocabulary. Everything longer is
        // a table entry and nothing else.
        if code.chars().count() < 5 || code == "General" {
            continue;
        }
        for line in all.lines() {
            if line.contains(code) {
                let (file, text) = line.split_once('\u{1}').unwrap_or(("?", line));
                offences.push(format!(
                    "{file}: the §18.8.30 code for id {id} — `{code}`\n    {text}"
                ));
            }
        }
    }
    assert!(
        offences.is_empty(),
        "a second copy of the built-in format table:\n{}",
        offences.join("\n")
    );
}
