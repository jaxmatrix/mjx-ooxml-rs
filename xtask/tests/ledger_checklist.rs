//! The parity ledger and the feature checklist agree (MJXOFF-297): the two name each other, a rendered row is one format's on both sides, a checklist row's audit state bounds the ledger state it names, and no row the checklist names reads `implemented` without citing a function that draws.

#[path = "../src/json.rs"]
mod json;

// The ledger's own reading of test source, so "draws" has one definition here and in the assessor.
#[allow(dead_code)]
#[path = "../src/ledger/scan.rs"]
mod scan;

use json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

// The three application formats; a `shared` checklist row may name any ledger row.
const APPLICATION_FORMATS: [&str; 3] = ["pptx", "xlsx", "docx"];

// Crates whose suites are one format's own, so a rendered row citing two of these sets spans two formats.
const FORMAT_CRATES: [(&str, &[&str]); 3] = [
    ("pptx", &["mjx-pptx", "mjx-layout-pptx", "mjx-scene-pptx"]),
    (
        "xlsx",
        &["mjx-xlsx", "mjx-sml", "mjx-layout-xlsx", "mjx-scene-xlsx"],
    ),
    ("docx", &["mjx-docx", "mjx-layout-docx", "mjx-scene-docx"]),
];

// Floors that say a reader is still reading rows, not the exact sizes.
const MINIMUM_LEDGER_ROWS: usize = 100;
const MINIMUM_CHECKLIST_ROWS: usize = 100;

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask sits one level below the repository root")
        .to_path_buf()
}

fn read(relative: &str) -> String {
    std::fs::read_to_string(repository_root().join(relative))
        .unwrap_or_else(|error| panic!("reading {relative}: {error}"))
}

// One ledger row as `rows.rs` declares it: its constructor, and the suites it cites.
struct LedgerRow {
    kind: &'static str,
    evidence: Vec<String>,
}

// The ledger's rows, parsed out of the constructor calls in `CAPABILITIES`.
fn ledger_rows() -> BTreeMap<String, LedgerRow> {
    let source = read("xtask/src/ledger/rows.rs");
    let (_, body) = source
        .split_once("pub(crate) const CAPABILITIES")
        .expect("rows.rs declares CAPABILITIES");
    let (body, _) = body
        .split_once("\n];")
        .expect("CAPABILITIES closes with `];` at the start of a line");
    assert!(
        !body.contains("Capability {"),
        "a ledger row is written as a struct literal; this reader only sees constructor calls"
    );
    let mut starts: Vec<(usize, &'static str)> = Vec::new();
    for kind in ["rendered", "behaviour", "excluded"] {
        let call = format!("{kind}(");
        for (at, _) in body.match_indices(&call) {
            let preceded = body[..at]
                .chars()
                .next_back()
                .is_some_and(|before| before.is_alphanumeric() || before == '_');
            if !preceded {
                starts.push((at, kind));
            }
        }
    }
    starts.sort_unstable();
    let mut rows = BTreeMap::new();
    for (index, (at, kind)) in starts.iter().enumerate() {
        let end = starts.get(index + 1).map_or(body.len(), |(next, _)| *next);
        let call = &body[at + kind.len() + 1..end];
        let id = call
            .trim_start()
            .strip_prefix('"')
            .and_then(|rest| rest.split_once('"'))
            .map(|(id, _)| id.to_owned())
            .unwrap_or_else(|| panic!("a `{kind}(` call does not open with a string id"));
        let evidence = call
            .split('"')
            .skip(1)
            .step_by(2)
            .filter(|literal| literal.starts_with("crates/") && literal.contains(".rs"))
            .map(str::to_owned)
            .collect();
        assert!(
            rows.insert(id.clone(), LedgerRow { kind, evidence })
                .is_none(),
            "ledger row `{id}` is declared twice"
        );
    }
    assert!(
        rows.len() >= MINIMUM_LEDGER_ROWS,
        "only {} ledger row(s) were read; the reader has stopped matching",
        rows.len()
    );
    rows
}

// One checklist row: its id, format, audit state and the ledger row it names.
struct ChecklistRow {
    id: String,
    format: String,
    audit_state: String,
    ledger: Option<String>,
}

// Every checklist row.
fn checklist_rows() -> Vec<ChecklistRow> {
    let document = json::parse(&read("docs/client-platform/data/features.json"))
        .expect("the feature checklist is valid JSON");
    let rows: Vec<ChecklistRow> = document
        .get("features")
        .and_then(Value::array)
        .expect("the checklist has a `features` array")
        .iter()
        .map(|row| {
            let field = |key: &str| row.get(key).and_then(Value::string).map(str::to_owned);
            ChecklistRow {
                id: field("id").unwrap_or_default(),
                format: field("format").unwrap_or_default(),
                audit_state: field("audit_state").unwrap_or_default(),
                ledger: field("ledger"),
            }
        })
        .collect();
    assert!(
        rows.len() >= MINIMUM_CHECKLIST_ROWS,
        "only {} checklist row(s) were read; the reader has stopped matching",
        rows.len()
    );
    rows
}

fn crate_of(path: &str) -> &str {
    path.strip_prefix("crates/")
        .and_then(|rest| rest.split_once('/'))
        .map_or("", |(name, _)| name)
}

// A citation naming a test function that reads a display list or pixels, by the assessor's own definition.
fn cites_a_drawing_function(citation: &str) -> bool {
    let (suite, function) = scan::split_citation(citation);
    let Some(function) = function else {
        return false;
    };
    let unit = scan::read_unit(&repository_root(), suite).unwrap_or_else(|error| panic!("{error}"));
    scan::drawing_tests(&unit.code, &unit.unit).contains(function)
}

fn format_of_crate(name: &str) -> Option<&'static str> {
    FORMAT_CRATES
        .iter()
        .find(|(_, crates)| crates.contains(&name))
        .map(|(format, _)| *format)
}

// The generated ledger's state per row id, read off the committed document.
fn ledger_states() -> BTreeMap<String, String> {
    let document = read("docs/client-platform/PARITY_LEDGER.md");
    let (_, table) = document
        .split_once("## The rows")
        .expect("the ledger has a rows section");
    let mut states = BTreeMap::new();
    for line in table.lines().filter(|line| line.starts_with("| `")) {
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        let id = cells.get(1).map_or("", |cell| cell.trim_matches('`'));
        let state = cells.iter().skip(2).find_map(|cell| {
            let bare = cell.trim_matches('`');
            [
                "implemented",
                "partial",
                "preserved-not-rendered",
                "not-started",
                "out-of-scope",
            ]
            .contains(&bare)
            .then(|| bare.to_owned())
        });
        if let Some(state) = state {
            states.insert(id.to_owned(), state);
        }
    }
    assert!(
        states.len() >= MINIMUM_LEDGER_ROWS,
        "only {} row state(s) were read out of PARITY_LEDGER.md",
        states.len()
    );
    states
}

// (c) Both directions: a checklist row names a real ledger row, and a rendered ledger row is named by a checklist row.
#[test]
fn the_checklist_and_the_ledger_name_each_other() {
    let ledger = ledger_rows();
    let checklist = checklist_rows();
    let mut failures = Vec::new();
    let mut named = BTreeSet::new();
    for ChecklistRow {
        id, ledger: target, ..
    } in &checklist
    {
        if let Some(target) = target {
            if !ledger.contains_key(target) {
                failures.push(format!(
                    "checklist row `{id}` names ledger row `{target}`, which rows.rs does not declare"
                ));
            }
            named.insert(target.clone());
        }
    }
    for (id, row) in &ledger {
        if row.kind == "rendered" && !named.contains(id) {
            failures.push(format!(
                "rendered ledger row `{id}` is named by no row of features.json"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} disagreement(s) between the checklist and the ledger:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

// A rendered ledger row is one format's: named by one application format's checklist rows, citing one format's suites.
#[test]
fn every_rendered_ledger_row_is_split_per_format() {
    let ledger = ledger_rows();
    let mut named_by: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for row in checklist_rows() {
        if let Some(target) = row.ledger {
            if APPLICATION_FORMATS.contains(&row.format.as_str()) {
                named_by.entry(target).or_default().insert(row.format);
            }
        }
    }
    let mut failures = Vec::new();
    for (id, row) in &ledger {
        if row.kind != "rendered" {
            continue;
        }
        if let Some(formats) = named_by.get(id).filter(|formats| formats.len() > 1) {
            failures.push(format!("`{id}` is named by checklist rows of {formats:?}"));
        }
        let cited: BTreeSet<&str> = row
            .evidence
            .iter()
            .filter_map(|path| format_of_crate(crate_of(path)))
            .collect();
        if cited.len() > 1 {
            failures.push(format!("`{id}` cites suites of {cited:?}"));
        }
        // Where both a naming format and a cited format exist, they are the same format.
        if let Some(named) = named_by.get(id) {
            let named: BTreeSet<&str> = named.iter().map(String::as_str).collect();
            if !cited.is_empty() && !named.is_empty() && named != cited {
                failures.push(format!(
                    "`{id}` is named by checklist rows of {named:?} and cites suites of {cited:?}"
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} rendered ledger row(s) span more than one format:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

// (d) Word has no scene companion, so a Word rendered row that reads `implemented` must cite a suite that draws.
#[test]
fn no_word_row_reads_implemented_without_a_rendering_tier_suite() {
    let ledger = ledger_rows();
    let states = ledger_states();
    let mut word: BTreeSet<String> = checklist_rows()
        .into_iter()
        .filter(|row| row.format == "docx")
        .filter_map(|row| row.ledger)
        .collect();
    word.extend(ledger.keys().filter(|id| id.starts_with("word-")).cloned());
    let mut failures = Vec::new();
    for id in &word {
        let Some(row) = ledger.get(id) else { continue };
        if row.kind != "rendered" || states.get(id).map(String::as_str) != Some("implemented") {
            continue;
        }
        if !row
            .evidence
            .iter()
            .any(|citation| cites_a_drawing_function(citation))
        {
            failures.push(format!(
                "`{id}` reads `implemented` and cites no function that draws: {:?}",
                row.evidence
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} Word row(s) overstated:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

// A ledger row any checklist row names answers the drawn question whatever its kind, so `implemented` needs a function that draws.
#[test]
fn no_checklist_named_row_reads_implemented_without_drawing_evidence() {
    let ledger = ledger_rows();
    let states = ledger_states();
    let named: BTreeSet<String> = checklist_rows()
        .into_iter()
        .filter_map(|row| row.ledger)
        .collect();
    let mut failures = Vec::new();
    for id in &named {
        let Some(row) = ledger.get(id) else { continue };
        if states.get(id).map(String::as_str) != Some("implemented") {
            continue;
        }
        if !row
            .evidence
            .iter()
            .any(|citation| cites_a_drawing_function(citation))
        {
            failures.push(format!(
                "`{id}` ({}) reads `implemented`, is named by the checklist, and cites no function that draws: {:?}",
                row.kind, row.evidence
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} checklist-named row(s) overstated:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

// A checklist row's audit state bounds the state of the ledger row it names: `gap` is never `implemented`, and `working` is `implemented` or `partial`.
#[test]
fn the_checklist_state_bounds_the_ledger_state() {
    let states = ledger_states();
    let mut failures = Vec::new();
    for row in checklist_rows() {
        let Some(target) = &row.ledger else { continue };
        let Some(state) = states.get(target) else {
            continue;
        };
        let allowed = match row.audit_state.as_str() {
            "gap" => state != "implemented",
            "working" => state == "implemented" || state == "partial",
            _ => true,
        };
        if !allowed {
            failures.push(format!(
                "checklist row `{}` is `{}` and names `{target}`, which reads `{state}`",
                row.id, row.audit_state
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} checklist state(s) disagree with the ledger:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
