//! The renderer feature checklist gate (MJXOFF-296): every row of `docs/client-platform/data/features.json` is owned or out of scope, names all four stage owners, and every audit id is placed.

#[path = "../src/json.rs"]
mod json;

use json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

// The audit-id rosters of the 2026-09-15 renderer audits: prefix and the highest number issued.
const AUDIT_ROSTERS: [(&str, u32); 4] = [("P", 45), ("X", 27), ("W", 45), ("C", 17)];

const FORMATS: [&str; 4] = ["pptx", "xlsx", "docx", "shared"];
const STAGES: [&str; 4] = ["model", "layout", "scene", "paint"];

// A floor that says the parser is still reading rows, not the exact corpus size.
const MINIMUM_ROWS: usize = 100;

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

fn is_ticket(candidate: &str) -> bool {
    candidate
        .strip_prefix("MJXOFF-")
        .is_some_and(|number| !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit()))
}

#[test]
fn every_row_has_an_owning_ticket_or_an_out_of_scope_reason() {
    let document = checklist();
    let rows = rows_of(&document);
    let mut failures = Vec::new();
    let (mut owned, mut excluded) = (0, 0);
    for row in rows {
        let owner = text(row, "owner");
        let out_of_scope = row.get("out_of_scope");
        let decision = out_of_scope.map_or("", |value| text(value, "decision"));
        let reason = out_of_scope.map_or("", |value| text(value, "reason"));
        match (owner.is_empty(), out_of_scope.is_some()) {
            (false, false) if is_ticket(owner) => owned += 1,
            (false, false) => failures.push(format!(
                "{}: owner {owner:?} is not an MJXOFF id",
                row_id(row)
            )),
            (true, true) if !decision.is_empty() && !reason.is_empty() => excluded += 1,
            (true, true) => failures.push(format!(
                "{}: out_of_scope needs a decision and a reason",
                row_id(row)
            )),
            (false, true) => failures.push(format!(
                "{}: has both an owner and an out-of-scope reason",
                row_id(row)
            )),
            (true, false) => failures.push(format!(
                "{}: has neither an owning ticket nor an out-of-scope reason",
                row_id(row)
            )),
        }
    }
    assert!(
        failures.is_empty(),
        "{} row(s) unowned:\n{}",
        failures.len(),
        failures.join("\n")
    );
    println!(
        "{} rows: {owned} owned, {excluded} out of scope",
        rows.len()
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
        for stage in STAGES {
            let Some(owner) = row.get("stages").and_then(|stages| stages.get(stage)) else {
                failures.push(format!("{}: stage `{stage}` is missing", row_id(row)));
                continue;
            };
            checked += 1;
            let krate = text(owner, "crate");
            let api = text(owner, "api");
            let problem = match krate {
                "" => Some("names no crate".to_owned()),
                "none" if !is_ticket(text(owner, "created_by")) => {
                    Some("is `none` but names no MJXOFF ticket in `created_by`".to_owned())
                }
                "not-applicable" if text(owner, "why").is_empty() => {
                    Some("is `not-applicable` but says nothing in `why`".to_owned())
                }
                "none" | "not-applicable" => None,
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
        .flat_map(|(prefix, highest)| {
            (1..=*highest).map(move |number| format!("{prefix}{number:02}"))
        })
        .collect();
    let mut referenced = BTreeSet::new();
    let mut unknown = Vec::new();
    for row in rows {
        for id in row.get("audit").and_then(Value::array).unwrap_or(&[]) {
            let id = id.string().unwrap_or("").to_owned();
            if roster.contains(&id) {
                referenced.insert(id);
            } else {
                unknown.push(format!("{}: `{id}` is in no audit roster", row_id(row)));
            }
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
        if row
            .get("ledger")
            .is_some_and(|ledger| ledger.string().is_some_and(|l| l.trim().is_empty()))
        {
            failures.push(format!("{id}: ledger is an empty string; write null"));
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
