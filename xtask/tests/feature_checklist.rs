//! The renderer feature checklist gate (MJXOFF-296): every row of `docs/client-platform/data/features.json` is owned or out of scope, names all four stage owners, and every audit id is placed.

#[path = "../src/json.rs"]
mod json;
#[path = "../src/ticket_roster.rs"]
mod ticket_roster;

use json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use ticket_roster::{is_roster_ticket, TICKET_ROSTER};

// The audit-id rosters of the 2026-09-15 renderer audits: prefix, the highest number issued, and the format audited.
const AUDIT_ROSTERS: [(&str, u32, &str); 4] = [
    ("P", 45, "pptx"),
    ("X", 27, "xlsx"),
    ("W", 45, "docx"),
    ("C", 17, "shared"),
];

// The only decisions that may take a row out of scope.
const OUT_OF_SCOPE_DECISIONS: [&str; 2] = ["D24.4", "D21.7"];

// The one ticket whose rows may be `working`: RC46, the render-tier acceptance.
const WORKING_RC: &str = "RC46";

const AUDIT_STATES: [&str; 3] = ["gap", "working", "out-of-scope"];
const FORMATS: [&str; 4] = ["pptx", "xlsx", "docx", "shared"];
const STAGES: [&str; 4] = ["model", "layout", "scene", "paint"];

// The stage only a picture row carries, and the paint API that marks a row as drawing a picture.
const DECODE_STAGE: &str = "decode";
const PICTURE_PAINT_API: &str = "ImageSource";

// Floors that say a parser is still reading rows, not the exact corpus sizes.
const MINIMUM_ROWS: usize = 100;
const MINIMUM_LEDGER_ROWS: usize = 100;

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

fn rows_of(checklist: &Value) -> &[Value] {
    let rows = checklist
        .get("features")
        .and_then(Value::array)
        .expect("the checklist has a `features` array");
    assert!(
        rows.len() >= MINIMUM_ROWS,
        "only {} row(s) were read; the reader has stopped matching",
        rows.len()
    );
    rows
}

fn text<'a>(row: &'a Value, key: &str) -> &'a str {
    row.get(key).and_then(Value::string).unwrap_or("").trim()
}

fn row_id(row: &Value) -> String {
    let id = text(row, "id");
    if id.is_empty() {
        "<row with no id>".to_owned()
    } else {
        id.to_owned()
    }
}

// The parity ledger's row ids, parsed out of the constructor calls in `CAPABILITIES` rather than restated here.
fn ledger_row_ids() -> BTreeSet<String> {
    let path = repository_root().join("xtask/src/ledger/rows.rs");
    let source = std::fs::read_to_string(&path).expect("the ledger rows are committed");
    let (_, body) = source
        .split_once("pub(crate) const CAPABILITIES")
        .expect("rows.rs declares CAPABILITIES");
    let (body, _) = body
        .split_once("\n];")
        .expect("CAPABILITIES closes with `];` at the start of a line");
    // A row written as a struct literal would be invisible to the constructor scan.
    assert!(
        !body.contains("Capability {"),
        "a ledger row is written as a struct literal; this reader only sees constructor calls"
    );
    let mut ids = BTreeSet::new();
    for constructor in ["rendered(", "behaviour(", "excluded("] {
        for (at, _) in body.match_indices(constructor) {
            if body[..at]
                .chars()
                .next_back()
                .is_some_and(|before| before.is_alphanumeric() || before == '_')
            {
                continue;
            }
            let id = body[at + constructor.len()..]
                .trim_start()
                .strip_prefix('"')
                .and_then(|rest| rest.split_once('"'))
                .map(|(id, _)| id)
                .unwrap_or_else(|| panic!("a `{constructor}` call does not open with a string id"));
            assert!(
                ids.insert(id.to_owned()),
                "ledger row `{id}` is declared twice"
            );
        }
    }
    assert!(
        ids.len() >= MINIMUM_LEDGER_ROWS,
        "only {} ledger row id(s) were read; the reader has stopped matching",
        ids.len()
    );
    ids
}

#[test]
fn every_row_has_an_owning_ticket_or_an_out_of_scope_reason() {
    let document = checklist();
    let rows = rows_of(&document);
    let mut failures = Vec::new();
    for row in rows {
        let (owner, rc) = (text(row, "owner"), text(row, "rc"));
        let out_of_scope = row.get("out_of_scope");
        let decision = out_of_scope.map_or("", |value| text(value, "decision"));
        let reason = out_of_scope.map_or("", |value| text(value, "reason"));
        let problem = match (owner.is_empty(), out_of_scope.is_some()) {
            (false, false) if TICKET_ROSTER.contains(&(rc, owner)) => None,
            (false, false) if is_roster_ticket(owner) => Some(format!(
                "rc {rc:?} is not the epic's label for owner {owner}"
            )),
            (false, false) => Some(format!("owner {owner:?} is not a ticket of the epic")),
            (true, true) if !rc.is_empty() => {
                Some(format!("is out of scope but carries rc {rc:?}"))
            }
            (true, true) if decision.is_empty() || reason.is_empty() => {
                Some("out_of_scope needs a decision and a reason".to_owned())
            }
            (true, true) if !OUT_OF_SCOPE_DECISIONS.contains(&decision) => Some(format!(
                "out-of-scope decision {decision:?} is not one of {OUT_OF_SCOPE_DECISIONS:?}"
            )),
            (true, true) => None,
            (false, true) => Some("has both an owner and an out-of-scope reason".to_owned()),
            (true, false) => {
                Some("has neither an owning ticket nor an out-of-scope reason".to_owned())
            }
        };
        if let Some(problem) = problem {
            failures.push(format!("{}: {problem}", row_id(row)));
        }
    }
    assert!(
        failures.is_empty(),
        "{} row(s) unowned:\n{}",
        failures.len(),
        failures.join("\n")
    );
    for format in FORMATS {
        let count = |state: &str| {
            rows.iter()
                .filter(|row| text(row, "format") == format && text(row, "audit_state") == state)
                .count()
        };
        println!(
            "{format}: {} gap, {} working, {} out of scope",
            count("gap"),
            count("working"),
            count("out-of-scope")
        );
    }
}

#[test]
fn audit_state_agrees_with_ownership() {
    let document = checklist();
    let rows = rows_of(&document);
    let mut failures = Vec::new();
    for row in rows {
        let state = text(row, "audit_state");
        let excluded = row.get("out_of_scope").is_some();
        let problem = if !AUDIT_STATES.contains(&state) {
            Some(format!(
                "audit_state {state:?} is not one of {AUDIT_STATES:?}"
            ))
        } else if (state == "out-of-scope") != excluded {
            Some(format!(
                "audit_state {state:?} disagrees with out_of_scope being {}",
                if excluded { "present" } else { "absent" }
            ))
        } else if state == "working" && text(row, "rc") != WORKING_RC {
            Some(format!(
                "is `working` but owned by {:?}; only {WORKING_RC} rows may be",
                text(row, "rc")
            ))
        } else {
            None
        };
        if let Some(problem) = problem {
            failures.push(format!("{}: {problem}", row_id(row)));
        }
    }
    assert!(
        failures.is_empty(),
        "{} row(s) with an inconsistent audit_state:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn no_stage_owner_is_blank() {
    let document = checklist();
    let rows = rows_of(&document);
    let root = repository_root();
    let mut failures = Vec::new();
    let mut checked = 0;
    for row in rows {
        let stages = row.get("stages");
        let paint_api = stages
            .and_then(|stages| stages.get("paint"))
            .map_or("", |paint| text(paint, "api"));
        let has_decode = stages.and_then(|stages| stages.get(DECODE_STAGE)).is_some();
        if paint_api.contains(PICTURE_PAINT_API) && !has_decode {
            failures.push(format!(
                "{}: paints through {PICTURE_PAINT_API} but names no `{DECODE_STAGE}` stage",
                row_id(row)
            ));
        }
        for stage in STAGES.iter().chain(has_decode.then_some(&DECODE_STAGE)) {
            let Some(owner) = stages.and_then(|stages| stages.get(stage)) else {
                failures.push(format!("{}: stage `{stage}` is missing", row_id(row)));
                continue;
            };
            checked += 1;
            let krate = text(owner, "crate");
            let api = text(owner, "api");
            let created_by = text(owner, "created_by");
            let problem = match krate {
                "" => Some("names no crate".to_owned()),
                "none" if !is_roster_ticket(created_by) => Some(format!(
                    "is `none` but `created_by` {created_by:?} is not a ticket of the epic"
                )),
                "not-applicable" if text(owner, "why").is_empty() => {
                    Some("is `not-applicable` but says nothing in `why`".to_owned())
                }
                "none" | "not-applicable" => None,
                // A crate is one directory name, so `..` or a separator cannot reach a manifest outside crates/.
                _ if krate.contains(['/', '\\']) || krate == ".." || krate == "." => Some(format!(
                    "names `{krate}`, which is not a single path segment"
                )),
                _ if !root.join("crates").join(krate).join("Cargo.toml").is_file() => Some(
                    format!("names `{krate}`, which is not a crate under crates/"),
                ),
                _ if api.is_empty() => Some(format!("names `{krate}` but no API")),
                _ => None,
            };
            if let Some(problem) = problem {
                failures.push(format!("{}: stage `{stage}` {problem}", row_id(row)));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} blank stage owner(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
    println!(
        "{checked} stage owners over {} rows, none blank",
        rows.len()
    );
}

#[test]
fn every_audit_id_is_referenced_by_a_row() {
    let document = checklist();
    let rows = rows_of(&document);
    let roster: BTreeSet<String> = AUDIT_ROSTERS
        .iter()
        .flat_map(|(prefix, highest, _)| {
            (1..=*highest).map(move |number| format!("{prefix}{number:02}"))
        })
        .collect();
    let mut referenced = BTreeSet::new();
    let mut unknown = Vec::new();
    for row in rows {
        let format = text(row, "format");
        for id in row.get("audit").and_then(Value::array).unwrap_or(&[]) {
            let id = id.string().unwrap_or("").to_owned();
            if !roster.contains(&id) {
                unknown.push(format!("{}: `{id}` is in no audit roster", row_id(row)));
                continue;
            }
            // A format row cites its own audit; a shared row may cite any of the four.
            let audited = AUDIT_ROSTERS
                .iter()
                .find(|(prefix, _, _)| id.starts_with(prefix))
                .map_or("", |(_, _, audited)| *audited);
            if format != "shared" && audited != format {
                unknown.push(format!(
                    "{}: `{id}` is from the {audited} audit, not the {format} one",
                    row_id(row)
                ));
            }
            referenced.insert(id);
        }
    }
    let unplaced: Vec<&String> = roster.difference(&referenced).collect();
    assert!(
        unknown.is_empty(),
        "{} unknown audit id(s):\n{}",
        unknown.len(),
        unknown.join("\n")
    );
    assert!(
        unplaced.is_empty(),
        "{} audit id(s) referenced by no row: {unplaced:?}",
        unplaced.len()
    );
    println!("{} audit ids, all placed", roster.len());
}

#[test]
fn every_row_is_well_formed() {
    let document = checklist();
    let rows = rows_of(&document);
    let ledger_ids = ledger_row_ids();
    let mut ids = BTreeSet::new();
    let mut failures = Vec::new();
    for row in rows {
        let id = row_id(row);
        if !ids.insert(id.clone()) {
            failures.push(format!("{id}: duplicate id"));
        }
        if !FORMATS.contains(&text(row, "format")) {
            failures.push(format!(
                "{id}: format {:?} is not one of {FORMATS:?}",
                text(row, "format")
            ));
        }
        if text(row, "feature").is_empty() {
            failures.push(format!("{id}: states no feature"));
        }
        // `get` reads a null ledger as absent, which is the spelling for a row with no ledger row.
        match row.get("ledger").map(Value::string) {
            None => {}
            Some(Some(ledger)) if ledger_ids.contains(ledger) => {}
            Some(Some(ledger)) => failures.push(format!(
                "{id}: ledger {ledger:?} is not a row of xtask/src/ledger/rows.rs"
            )),
            Some(None) => failures.push(format!("{id}: ledger is neither a string nor null")),
        }
    }
    assert!(
        failures.is_empty(),
        "{} malformed row(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
    for format in FORMATS {
        let count = rows
            .iter()
            .filter(|row| text(row, "format") == format)
            .count();
        assert!(count > 0, "no row for format `{format}`");
        println!("{format}: {count} rows");
    }
}
