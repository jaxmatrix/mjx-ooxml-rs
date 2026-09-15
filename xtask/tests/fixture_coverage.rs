//! **Every owned checklist row names the corporate-fixture element that exercises it** (MJXOFF-300).
//!
//! RC03 commits one corporate-shaped fixture per format. A fixture is only worth committing if a
//! reader can say *which feature each part of it is there to exercise*, so each one carries a README
//! with a coverage table, and this gate holds that table against
//! `docs/client-platform/data/features.json` in both directions.
//!
//! # The form a README must use
//!
//! `tests/render/RC03-corporate/<format>/README.md`, carrying a section that begins with the
//! heading [`COVERAGE_HEADING`] and then a Markdown table of exactly two columns:
//!
//! ```text
//! ## Checklist coverage
//!
//! | features.json row | fixture element |
//! | --- | --- |
//! | pptx-line-breaks | the title's soft line break (`a:br`) |
//! ```
//!
//! The left column is a `features.json` `id`; the right says what in the fixture exercises it, in
//! words, and may not be empty. A table is parsed, never trusted: an id that names no row fails, and
//! a row that no table names fails the other way.
//!
//! # ⚠ The population this gate sweeps, and the one the ticket asked for
//!
//! MJXOFF-300 asks for *"every `features.json` row whose owner is a Wave 0–3 ticket"*. **No wave is
//! defined anywhere in this repository** — not in `docs/`, not in `xtask/`, not in the checklist
//! itself, which carries `owner` and `rc` and no grouping above them. A wave table invented here
//! would be a population this gate made up, and a gate whose corpus is invented proves whatever the
//! invention says.
//!
//! So this sweeps the population that *is* checkable: every row with an owning ticket on the epic's
//! roster that is not out of scope. That is a **superset** of Wave 0–3, which is the safe direction
//! to be wrong in — a row this gate demands and the waves would not is a row somebody must either
//! cover or excuse in writing, and [`NOT_IN_THE_CORPORATE_FIXTURES`] is where that is written down.
//! Narrowing it is a one-line change the day a wave roster exists.

#[path = "../src/json.rs"]
mod json;
#[path = "../src/ticket_roster.rs"]
mod ticket_roster;

use json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use ticket_roster::{closed_reason, is_roster_ticket, TICKET_ROSTER};

/// The heading that opens a README's coverage table.
const COVERAGE_HEADING: &str = "## Checklist coverage";

/// The three corporate fixtures, as (format, fixture file, README directory).
const CORPORATE_FIXTURES: [(&str, &str, &str); 3] = [
    ("pptx", "corporate.pptx", "tests/render/RC03-corporate/pptx"),
    ("xlsx", "corporate.xlsx", "tests/render/RC03-corporate/xlsx"),
    ("docx", "corporate.docx", "tests/render/RC03-corporate/docx"),
];

/// A row the corporate fixtures deliberately do not exercise, and why.
///
/// A row here is a **statement**, not a suppression: it does not turn the sweep off, it answers the
/// question the sweep raised. Writing one means having decided that a corporate-shaped file has no
/// natural place for the feature — not that covering it was inconvenient.
struct Excused {
    /// The `features.json` `id`.
    row: &'static str,
    /// Why no corporate fixture carries an element for it.
    reason: &'static str,
}

/// Every owned row no corporate fixture exercises, each with the reason it does not.
///
/// **Empty on purpose.** RC03's implementation fills both this and the three READMEs; until it does,
/// [`every_owned_row_is_covered_or_excused`] names every row that is neither, which is the red state
/// this gate is supposed to have before the fixtures exist.
const NOT_IN_THE_CORPORATE_FIXTURES: &[Excused] = &[];

/// A floor that says a README's table is still being parsed, not the exact size of any of them.
const MINIMUM_COVERED_ROWS_PER_FIXTURE: usize = 5;

/// A floor that says the checklist reader is still matching rows.
const MINIMUM_CHECKLIST_ROWS: usize = 100;

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask sits one level below the repository root")
        .to_path_buf()
}

fn checklist() -> Value {
    let path = repository_root().join("docs/client-platform/data/features.json");
    let text = std::fs::read_to_string(&path).expect("the feature checklist is committed");
    json::parse(&text).expect("the feature checklist is valid JSON")
}

fn text<'a>(row: &'a Value, key: &str) -> &'a str {
    row.get(key).and_then(Value::string).unwrap_or("").trim()
}

/// Every checklist row, as (id, format), for the rows this gate demands coverage of.
///
/// A row is demanded when it has an owning ticket on the epic's roster and is not out of scope. A
/// closed owner is not demanded: its work is merged and the checklist gate already refuses it.
fn demanded_rows() -> BTreeMap<String, String> {
    let checklist = checklist();
    let rows = checklist
        .get("features")
        .and_then(Value::array)
        .expect("the checklist has a `features` array");
    assert!(
        rows.len() >= MINIMUM_CHECKLIST_ROWS,
        "only {} checklist row(s) were read; the reader has stopped matching",
        rows.len()
    );

    let mut demanded = BTreeMap::new();
    for row in rows {
        let (id, owner, format) = (text(row, "id"), text(row, "owner"), text(row, "format"));
        if id.is_empty() {
            continue;
        }
        if row.get("out_of_scope").is_some() || owner.is_empty() {
            continue;
        }
        if closed_reason(owner).is_some() || !is_roster_ticket(owner) {
            continue;
        }
        demanded.insert(id.to_owned(), format.to_owned());
    }
    assert!(
        !demanded.is_empty(),
        "no checklist row is owned by a roster ticket; this gate's population reader has broken"
    );
    demanded
}

/// Every id in the whole checklist, so an unknown id in a README can be told from an excused one.
fn every_row_id() -> BTreeSet<String> {
    let checklist = checklist();
    let rows = checklist
        .get("features")
        .and_then(Value::array)
        .expect("the checklist has a `features` array");
    rows.iter()
        .map(|row| text(row, "id").to_owned())
        .filter(|id| !id.is_empty())
        .collect()
}

/// One README's coverage table, as (row id, the element that exercises it).
///
/// # Panics
/// If the README is missing, carries no coverage heading, or its table is malformed — each of which
/// is a fixture that cannot say what it is for.
fn coverage_table(directory: &str) -> Vec<(String, String)> {
    let path = repository_root().join(directory).join("README.md");
    let markdown = std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "{}: {error}. RC03 commits one README per corporate fixture, carrying the `{}` table \
             this gate reads.",
            path.display(),
            COVERAGE_HEADING
        )
    });

    let (_, after) = markdown.split_once(COVERAGE_HEADING).unwrap_or_else(|| {
        panic!(
            "{} carries no `{COVERAGE_HEADING}` heading, so it does not say which checklist rows \
             the fixture exercises",
            path.display()
        )
    });

    let mut covered = Vec::new();
    for line in after.lines() {
        let line = line.trim();
        if line.starts_with("##") {
            break; // The next section; the table has ended.
        }
        if !line.starts_with('|') {
            continue;
        }
        let cells: Vec<&str> = line
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect::<Vec<_>>();
        if cells.len() != 2 {
            panic!(
                "{}: the coverage table must have exactly two columns, and this row has {}: {line}",
                path.display(),
                cells.len()
            );
        }
        let (id, element) = (cells[0], cells[1]);
        // The header row and the `| --- | --- |` separator carry no id.
        if id.eq_ignore_ascii_case("features.json row") || id.chars().all(|c| c == '-' || c == ':')
        {
            continue;
        }
        assert!(
            !element.is_empty(),
            "{}: row `{id}` names no fixture element. A coverage claim with nothing beside it is \
             the suppression this table exists not to be.",
            path.display()
        );
        covered.push((id.to_owned(), element.to_owned()));
    }
    covered
}

/// Every README's table parses, names only real rows, and names each of them once.
#[test]
fn every_readme_covers_real_checklist_rows() {
    let known = every_row_id();
    for (format, fixture, directory) in CORPORATE_FIXTURES {
        let covered = coverage_table(directory);
        assert!(
            covered.len() >= MINIMUM_COVERED_ROWS_PER_FIXTURE,
            "{fixture}'s README claims {} covered row(s); a table this short means the parser has \
             stopped matching or the fixture is not corporate-shaped",
            covered.len()
        );

        let mut seen = BTreeSet::new();
        for (id, _) in &covered {
            assert!(
                known.contains(id),
                "{fixture}'s README claims checklist row `{id}`, which is in no \
                 `docs/client-platform/data/features.json` row. A coverage claim against a row that \
                 does not exist is a claim nothing can check."
            );
            assert!(
                seen.insert(id.clone()),
                "{fixture}'s README claims row `{id}` twice"
            );
        }
        assert!(
            !format.is_empty(),
            "every corporate fixture names the format it is for"
        );
    }
}

/// A README may only claim rows of its own format, or rows shared across all three.
#[test]
fn a_readme_claims_only_its_own_formats_rows() {
    let checklist = checklist();
    let rows = checklist
        .get("features")
        .and_then(Value::array)
        .expect("the checklist has a `features` array");
    let formats: BTreeMap<String, String> = rows
        .iter()
        .map(|row| (text(row, "id").to_owned(), text(row, "format").to_owned()))
        .collect();

    for (format, fixture, directory) in CORPORATE_FIXTURES {
        for (id, _) in coverage_table(directory) {
            let row_format = formats.get(&id).map(String::as_str).unwrap_or("");
            assert!(
                row_format == format || row_format == "shared",
                "{fixture}'s README claims row `{id}`, whose format is `{row_format}`. A \
                 `.{format}` fixture cannot exercise another format's feature."
            );
        }
    }
}

/// **Every owned, in-scope checklist row is either exercised by a corporate fixture or excused in
/// writing.**
///
/// This is the gate MJXOFF-300 is really asking for. See this module's own documentation for why the
/// population is every roster-owned row rather than a wave.
#[test]
fn every_owned_row_is_covered_or_excused() {
    let demanded = demanded_rows();
    let known = every_row_id();

    let mut covered: BTreeSet<String> = BTreeSet::new();
    for (_, _, directory) in CORPORATE_FIXTURES {
        for (id, _) in coverage_table(directory) {
            covered.insert(id);
        }
    }

    let excused: BTreeMap<&str, &str> = NOT_IN_THE_CORPORATE_FIXTURES
        .iter()
        .map(|entry| (entry.row, entry.reason))
        .collect();
    for (row, reason) in &excused {
        assert!(
            known.contains(*row),
            "`{row}` is excused from the corporate fixtures and is in no checklist row"
        );
        assert!(
            !reason.trim().is_empty(),
            "`{row}` is excused with no reason written"
        );
        assert!(
            !covered.contains(*row),
            "`{row}` is both claimed by a fixture README and excused from the fixtures. It is one \
             or the other."
        );
    }

    let missing: Vec<&String> = demanded
        .keys()
        .filter(|id| !covered.contains(*id) && !excused.contains_key(id.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "{} owned checklist row(s) are neither exercised by a corporate fixture nor excused from \
         them. Add the element to the fixture and a line to its README's `{COVERAGE_HEADING}` \
         table, or add the row to `NOT_IN_THE_CORPORATE_FIXTURES` with the reason a \
         corporate-shaped file has no place for it. The rows: {missing:?}",
        missing.len()
    );
}

/// The roster this gate reads its ownership from is the epic's own, and it is not empty.
///
/// Without this, a `TICKET_ROSTER` that had emptied would make `demanded_rows` empty and every
/// sweep above pass over nothing.
#[test]
fn the_ticket_roster_is_the_epics_roster() {
    assert!(
        TICKET_ROSTER.len() >= 40,
        "the epic's ticket roster holds {} pair(s); a roster this short means this gate's ownership \
         reader is looking at the wrong table",
        TICKET_ROSTER.len()
    );
    assert!(
        TICKET_ROSTER.iter().any(|(rc, _)| *rc == "RC03"),
        "RC03 is not on the roster this gate reads, so the ticket that commits the corporate \
         fixtures is not one it can recognise"
    );
}
