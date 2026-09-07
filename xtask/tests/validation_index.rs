//! The index gate: **every entry id resolves to a generated artefact, and every generated artefact
//! is named by at least one entry** (MJXOFF-122, "Done when" #5).
//!
//! # The trap this file is written against, in its own terms
//!
//! > *an index test that passes because it compares two lists generated from the same source proves
//! > nothing.*
//!
//! So the two lists come from two places that cannot drift together:
//!
//! * the **entry** side is parsed out of `docs/validation/01-index.md` — hand-written markdown, the
//!   thing a person edits when they add an area;
//! * the **artefact** side is a `read_dir` of the directory the `xtask` binary has just written —
//!   files on a filesystem, produced by running the generator.
//!
//! A third, weaker comparison rides along: the risk level and the area name in each row are checked
//! against `AREAS` in `xtask/src/validation/mod.rs`. That one *is* code against prose, which is
//! exactly why it is not the gate — it is what stops the two statements of a level diverging.
//!
//! # And the floors
//!
//! "Every entry resolves" is green when there are no entries, and "every artefact is named" is green
//! when there are no artefacts. Both sides are therefore floored before either comparison runs, so a
//! parser that silently stopped matching table rows fails here rather than passing an empty
//! comparison. That is MJXOFF-118's shape and it is borrowed deliberately.
//!
//! The floors are stated as *the parser is working* rather than as *the catalogue is this size*, and
//! that distinction was measured rather than guessed. A floor of "at least as many rows as `AREAS`"
//! fires **before** the two-direction comparison does, so deleting one row from the document — the
//! mutation that is supposed to prove the binding — reddens the floor and the assertion it was
//! aimed at never executes. What the floors say instead is that rows were found for all three
//! formats and that there are enough of them for a partial parse to be visible; exactness is the
//! two-direction comparison's job, and it now runs on that mutation and names the missing row.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use xtask::validation::{original_for, Risk, Variant, AREAS};

/// `docs/validation/`.
fn validation_docs() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../docs/validation")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// One row of the index, as the *document* states it.
#[derive(Debug, PartialEq, Eq)]
struct DocumentedEntry {
    id: String,
    risk: String,
    area: String,
    authored: String,
    edited: String,
}

/// The cells of a markdown table row, trimmed, or `None` for a line that is not one.
fn row_cells(line: &str) -> Option<Vec<&str>> {
    let line = line.trim();
    let inner = line.strip_prefix('|')?.strip_suffix('|')?;
    Some(inner.split('|').map(str::trim).collect())
}

/// The text between the first pair of backticks, if the cell is exactly one code span.
fn code_span(cell: &str) -> Option<&str> {
    cell.strip_prefix('`')?.strip_suffix('`')
}

/// Every entry the index document states, in document order.
///
/// A row is one of these when it has five cells and the first is a code span shaped like an entry
/// id. The header row (`| Entry | Risk | … |`) has no code spans and the separator row is not a row
/// by this rule, so neither needs excluding by position.
fn documented_entries() -> Vec<DocumentedEntry> {
    let page = read(&validation_docs().join("01-index.md"));
    let mut entries = Vec::new();
    for line in page.lines() {
        let Some(cells) = row_cells(line) else {
            continue;
        };
        if cells.len() != 5 {
            continue;
        }
        let Some(id) = code_span(cells[0]) else {
            continue;
        };
        if !id.starts_with("V-") {
            continue;
        }
        entries.push(DocumentedEntry {
            id: id.to_owned(),
            risk: cells[1].to_owned(),
            area: code_span(cells[2]).unwrap_or(cells[2]).to_owned(),
            authored: code_span(cells[3]).unwrap_or(cells[3]).to_owned(),
            edited: code_span(cells[4]).unwrap_or(cells[4]).to_owned(),
        });
    }
    entries
}

/// Runs the generator into its own directory and returns every file it wrote.
fn generated_artefacts() -> (PathBuf, BTreeSet<String>) {
    let directory =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/validation-artefacts-index");
    // A stale file from an earlier run would let "every artefact is named" pass on yesterday's
    // output, and a *missing* one would let "every entry resolves" fail for the wrong reason.
    let _ = std::fs::remove_dir_all(&directory);
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("validation-artefacts")
        .arg("--out")
        .arg(&directory)
        .output()
        .expect("running the xtask binary");
    assert!(
        output.status.success(),
        "validation-artefacts failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let files = std::fs::read_dir(&directory)
        .unwrap_or_else(|e| panic!("reading {}: {e}", directory.display()))
        .map(|entry| {
            entry
                .expect("a directory entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    (directory, files)
}

/// Enough rows that a parser which stopped matching part-way is visible, without pinning the
/// catalogue's exact size — see this file's own header for why the difference matters.
const MINIMUM_ROWS: usize = 12;

/// The floor: rows were found, for all three formats, and enough of them.
///
/// Stated over the three format prefixes as well as over the total, because a parser that stopped
/// matching *one* of the three tables — a heading changed, a column added — would still clear a
/// bare count.
fn assert_parser_is_working(documented: &[DocumentedEntry]) {
    assert!(
        documented.len() >= MINIMUM_ROWS,
        "only {} row(s) were parsed out of docs/validation/01-index.md — the parser is not reaching \
         the tables, and every comparison below would pass on an almost empty list",
        documented.len()
    );
    for format in ["PPTX", "DOCX", "XLSX"] {
        let prefix = format!("V-{format}-");
        assert!(
            documented.iter().any(|entry| entry.id.starts_with(&prefix)),
            "not one {format} row was parsed out of docs/validation/01-index.md; the parser is \
             reaching some of the tables and not that one"
        );
    }
}

#[test]
fn the_index_document_and_the_generated_artefacts_agree_in_both_directions() {
    let documented = documented_entries();
    let (_, produced) = generated_artefacts();

    // ---- The floors, before either comparison ------------------------------------------------
    assert_parser_is_working(&documented);
    assert!(
        produced.len() >= MINIMUM_ROWS,
        "the generator wrote only {} artefact(s); the comparison below would pass on almost nothing",
        produced.len()
    );

    // ---- Direction 1: every documented entry resolves to an artefact ---------------------------
    // The authored artefact must exist. The edited one must exist **if and only if** the Office
    // corpus holds an original for that area — an `iff`, so an empty corpus is still an assertion
    // rather than a hole.
    let by_id: BTreeMap<&str, &xtask::validation::Area> =
        AREAS.iter().map(|area| (area.id, area)).collect();
    for entry in &documented {
        // The artefact assertion comes **first**, deliberately. It is what this direction claims,
        // and an earlier lookup that panicked on the same mutation would leave it unexecuted — the
        // shape MJXOFF-118 found in one of its own mutations.
        assert!(
            produced.contains(&entry.authored),
            "{} names the authored artefact {}, which the generator did not write",
            entry.id,
            entry.authored
        );
        let area = by_id.get(entry.id.as_str()).unwrap_or_else(|| {
            panic!(
                "docs/validation/01-index.md names {}, which is in no area of `AREAS`",
                entry.id
            )
        });
        let has_original = original_for(area)
            .expect("reading the Office-authored corpus")
            .is_some();
        assert_eq!(
            produced.contains(&entry.edited),
            has_original,
            "{}: the edited artefact {} must exist exactly when {} does",
            entry.id,
            entry.edited,
            xtask::validation::original_path(area).display()
        );
    }

    // ---- Direction 2: every artefact is named by an entry ---------------------------------------
    let named: BTreeSet<&str> = documented
        .iter()
        .flat_map(|entry| [entry.authored.as_str(), entry.edited.as_str()])
        .collect();
    for artefact in &produced {
        assert!(
            named.contains(artefact.as_str()),
            "the generator wrote {artefact}, which no row of docs/validation/01-index.md names"
        );
    }

    println!(
        "index: {} documented entr(ies) <-> {} generated artefact(s), both directions",
        documented.len(),
        produced.len()
    );
}

#[test]
fn the_index_document_states_the_same_area_and_risk_as_the_catalogue() {
    let documented = documented_entries();
    assert_parser_is_working(&documented);
    let by_id: BTreeMap<&str, &DocumentedEntry> = documented
        .iter()
        .map(|entry| (entry.id.as_str(), entry))
        .collect();
    assert_eq!(
        by_id.len(),
        documented.len(),
        "docs/validation/01-index.md states an entry id twice"
    );
    for area in AREAS {
        let entry = by_id.get(area.id).unwrap_or_else(|| {
            panic!(
                "`AREAS` has {} and docs/validation/01-index.md has no row for it",
                area.id
            )
        });
        assert_eq!(
            entry.risk,
            area.risk.label(),
            "{}: the index says risk {:?}, the catalogue says {:?}",
            area.id,
            entry.risk,
            area.risk.label()
        );
        assert_eq!(
            entry.area, area.slug,
            "{}: the index calls the area {:?}, the catalogue calls it {:?}",
            area.id, entry.area, area.slug
        );
        assert_eq!(entry.authored, area.artefact_name(Variant::Authored));
        assert_eq!(entry.edited, area.artefact_name(Variant::Edited));
    }
    // Every risk word in the document is one the catalogue can parse, which is what stops a typo in
    // the markdown reading as a level nothing has.
    for entry in &documented {
        Risk::parse(&entry.risk)
            .unwrap_or_else(|| panic!("{}: {:?} is not a risk level", entry.id, entry.risk));
    }
}

#[test]
fn every_entry_id_used_anywhere_in_the_validation_docs_is_in_the_index() {
    // MJXOFF-128 writes per-format check pages beside the index. An id invented there that binds to
    // no artefact would otherwise be invisible until somebody went looking for the file.
    let index: BTreeSet<String> = documented_entries()
        .into_iter()
        .map(|entry| entry.id)
        .collect();
    let mut seen = 0usize;
    let mut pages = 0usize;
    for entry in std::fs::read_dir(validation_docs()).expect("docs/validation") {
        let path = entry.expect("a directory entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        pages += 1;
        let text = read(&path);
        for id in entry_ids_in(&text) {
            seen += 1;
            assert!(
                index.contains(&id),
                "{} mentions the entry id {id}, which docs/validation/01-index.md does not bind to \
                 an artefact",
                path.display()
            );
        }
    }
    assert!(pages >= 2, "only {pages} page(s) in docs/validation");
    assert!(
        seen >= 18,
        "only {seen} entry id(s) were found across the validation docs; the scanner is not matching"
    );
    println!("index: {seen} entry-id mention(s) across {pages} page(s), every one bound");
}

/// Panics when a result line's body carries one of the forbidden verdict words.
fn assert_no_verdict(path: &Path, whole_line: &str, body: &str, forbidden: &[&str]) {
    let lower = body.to_ascii_lowercase();
    for word in forbidden {
        assert!(
            !lower
                .split(|c: char| !c.is_ascii_alphabetic())
                .any(|token| token == *word),
            "{}: a result line says {word:?}. Judging what Office renders is the user's, and no \
             agent may stand in for it:\n  {whole_line}",
            path.display()
        );
    }
}

/// Every `V-FMT-NN` (and `V-FMT-NN.n`) in `text`, reduced to the area id.
///
/// Written out rather than pulled in as a regex dependency: `xtask` carries none, and the shape is
/// fixed by `docs/validation/00-method.md`.
fn entry_ids_in(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut index = 0usize;
    while let Some(offset) = text[index..].find("V-") {
        let start = index + offset;
        index = start + 2;
        let mut cursor = index;
        while cursor < bytes.len() && bytes[cursor].is_ascii_uppercase() {
            cursor += 1;
        }
        if cursor == index || cursor >= bytes.len() || bytes[cursor] != b'-' {
            continue;
        }
        let format_end = cursor;
        cursor += 1;
        let digits_start = cursor;
        while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
            cursor += 1;
        }
        if cursor - digits_start != 2 {
            continue;
        }
        found.push(format!(
            "V-{}-{}",
            &text[index..format_end],
            &text[digits_start..cursor]
        ));
        index = cursor;
    }
    found
}

#[test]
fn no_result_line_anywhere_carries_a_verdict() {
    // The rule this whole phase turns on: an agent builds the harness and marks nothing. A result
    // line ships unfilled, and the verdict vocabulary has no word that reads like a test result —
    // see `docs/validation/00-method.md` §3.
    const FORBIDDEN: [&str; 4] = ["pass", "passed", "passes", "ok"];
    let mut lines = 0usize;
    let mut cells = 0usize;
    for entry in std::fs::read_dir(validation_docs()).expect("docs/validation") {
        let path = entry.expect("a directory entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let text = read(&path);
        for line in text.lines() {
            let trimmed = line.trim();
            if let Some(body) = trimmed.strip_prefix("Result:") {
                lines += 1;
                assert_no_verdict(&path, trimmed, body, &FORBIDDEN);
                continue;
            }
            // A verdict does not have to arrive on a `Result:` line. MJXOFF-108's inherited table
            // records its answers in *columns*, and MJXOFF-128's checks may do the same, so every
            // table cell is swept too: a cell whose whole content is one of these words is a
            // verdict however it is laid out.
            let Some(row) = row_cells(trimmed) else {
                continue;
            };
            for cell in row {
                cells += 1;
                let bare = cell.trim().trim_matches('`').trim_matches('*').trim();
                assert!(
                    !FORBIDDEN.contains(&bare.to_ascii_lowercase().as_str()),
                    "{}: a table cell says {bare:?}. Judging what Office renders is the user's, \
                     and no agent may stand in for it:\n  {trimmed}",
                    path.display()
                );
            }
        }
    }
    // The same floor, for the same reason: a row parser that matched nothing would make the cell
    // sweep above green by finding no cells.
    assert!(
        cells >= 90,
        "only {cells} table cell(s) were swept across docs/validation/; the row parser is matching \
         almost nothing"
    );
    // The floor. "No result line says `pass`" is green precisely when the scanner finds no result
    // lines at all, which is the exact shape of false green §7 of the epic keeps naming. Today the
    // lines it finds are `00-method.md`'s worked examples of the convention; when MJXOFF-128 writes
    // the checks there will be one per check, and the floor grows with them.
    assert!(
        lines >= 1,
        "not one `Result:` line was found across docs/validation/ — the scanner is matching nothing, \
         and this assertion would pass on an empty sweep"
    );
    println!(
        "results: {lines} result line(s) and {cells} table cell(s) inspected, not one carries a \
         verdict word"
    );
}
