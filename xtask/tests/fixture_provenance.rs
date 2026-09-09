//! **`docProps/app.xml`'s `Application` element is not provenance** (MJXOFF-249).
//!
//! # The trap
//!
//! `docs/validation/06-the-office-pass.md` opens with the sentence this whole programme rests on:
//! *the value of an Office-authored file is entirely its provenance, and a file this library wrote
//! and called "PowerPoint-authored" would be a permanent lie in the one place there is no other
//! defence against one.* The page then names no defence against a **third party** claiming Office
//! authorship in the one element a reader would think to check.
//!
//! Two fixtures do exactly that, and neither was written by Microsoft:
//!
//! * `tests/fixtures/comments_third_party.xlsx` carries `Microsoft Excel` / `12.0000`. It is
//!   XlsxWriter's output; that library emits the element verbatim as a compatibility measure.
//! * `tests/fixtures/charts.pptx` carries `Microsoft Macintosh PowerPoint` / `14.0000`. It is
//!   python-pptx's template deck, and its own `docProps/core.xml` says so in words —
//!   `generated using python-pptx`.
//!
//! MJXOFF-246 nearly concluded the first one was Office-authored. **This repository had already
//! concluded it about the second**: `crates/mjx-pptx/tests/charts.rs` said in a live doc comment
//! that *`charts.pptx` was written by PowerPoint*, and
//! `crates/mjx-ooxml/tests/preservation/deck_cases.rs` drew an inference about *what PowerPoint
//! writes* from that file's markup. Both were corrected under MJXOFF-249; this suite is what stops
//! the third one.
//!
//! # The rule this suite holds, and the rule it deliberately does not
//!
//! The rule H6 established: **provenance is a fact about how a file reached this repository, not
//! about its bytes** — because every element in a file is something a producer *chose* to write,
//! and a producer may choose to write somebody else's name.
//!
//! Its corollary decides the shape of everything below. A gate that read `Application` and
//! classified producers from it would build the very inference the rule forbids, and would be
//! wrong about both fixtures on its first run. So this suite **never concludes anything about who
//! wrote a file.** It asserts a negative against a ledger:
//!
//! > No fixture's `Application` element names Microsoft unless a person has written down, here,
//! > how that file actually reached this repository.
//!
//! [`NAMES_MICROSOFT`] is a test on a **string**, not a judgement about a producer. Matching it
//! proves nothing; it raises a question, and the only thing that answers the question is a
//! [`KNOWN_IMPERSONATORS`] row written by somebody who knows where the file came from.
//!
//! # What each test does
//!
//! * [`no_fixture_claims_microsoft_authorship_off_the_ledger`] — the negative, over the corpus
//!   `mjx-fixtures` enumerates rather than over a list.
//! * [`every_known_impersonator_still_makes_the_claim_the_ledger_records`] — the ledger cannot rot:
//!   a row for a fixture that no longer exists, or no longer carries that exact string, fails.
//! * [`the_ingest_report_never_reads_the_application_element`] — MJXOFF-249's third condition, held
//!   as a property of the source rather than as a promise.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use mjx_opc::{Package, PartName};

/// The part every claim in this suite is read out of.
const APP_PROPERTIES: &str = "/docProps/app.xml";

/// The needle that raises the question — **and answers none of it**.
///
/// Every application Microsoft ships writes its own name into `Application`, and every one of those
/// names contains this word: `Microsoft Excel`, `Microsoft Office Word`,
/// `Microsoft Macintosh PowerPoint`. So a fixture whose value contains it is a fixture somebody has
/// to have looked at. That is the entire content of the match. It is deliberately *not* a list of
/// products, a version test, or anything else that could be mistaken for deciding what wrote a
/// file: both fixtures on the ledger below would pass such a test, which is why one does not exist
/// here.
const NAMES_MICROSOFT: &str = "microsoft";

/// One fixture whose `Application` element names Microsoft, and how the file actually got here.
struct Impersonator {
    /// The fixture, as `mjx-fixtures` names it.
    fixture: &'static str,
    /// The exact string it carries, so the row cannot drift away from the file.
    claims: &'static str,
    /// How this file reached this repository — the only statement of provenance there is.
    ///
    /// Written by a person, from evidence outside the `Application` element. Nothing derives it and
    /// nothing checks it: a fact about history is not recoverable from bytes, which is the whole
    /// point of the ticket this suite closes.
    reached_us_by: &'static str,
}

/// Every fixture that claims Microsoft authorship, with what actually wrote it.
///
/// A row here is a *statement*, not a suppression: it does not turn a check off, it supplies the
/// answer to the question [`NAMES_MICROSOFT`] raised. Adding a row means having found out where a
/// file came from; it cannot be done by looking harder at the file.
const KNOWN_IMPERSONATORS: &[Impersonator] = &[
    Impersonator {
        fixture: "charts.pptx",
        claims: "Microsoft Macintosh PowerPoint",
        reached_us_by:
            "python-pptx's template deck, committed with the chart fixture in Phase A. Its own \
             `docProps/core.xml` states it: `<dc:description>generated using python-pptx` and a \
             `cp:lastModifiedBy` of python-pptx's author. The `Application` element is the one its \
             template was born with and python-pptx does not rewrite; `crates/mjx-chart/src/author.rs`, \
             `crates/mjx-chart/tests/preservation.rs` and `crates/mjx-opc/src/doc_props.rs` all name \
             python-pptx as its producer.",
    },
    Impersonator {
        fixture: "comments_third_party.xlsx",
        claims: "Microsoft Excel",
        reached_us_by:
            "XlsxWriter 3.2.9, run to produce a comments fixture no other producer here writes. \
             XlsxWriter emits `<Application>Microsoft Excel</Application>` verbatim as a \
             compatibility measure, and the module table in `crates/mjx-xlsx/tests/comments.rs` \
             records the real producer.",
    },
];

/// The workspace root — `xtask/`'s parent.
fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask/ has a parent")
        .to_path_buf()
}

/// The `Application` element of one fixture, or `None` when it ships no `docProps/app.xml`.
///
/// Read out of the package rather than out of the zip directly, so the corpus is walked the way
/// every other suite walks it. A fixture that will not open at all is a failure: this suite's
/// corpus is the committed one, and every file in it round-trips elsewhere.
fn application_of(fixture: &str) -> Option<String> {
    let bytes = mjx_fixtures::fixture(fixture);
    let package = Package::open(&bytes).unwrap_or_else(|e| panic!("opening {fixture}: {e}"));
    let part = PartName::new(APP_PROPERTIES).expect("a literal part name");
    let payload = package.part_payload(&part)?;
    let text = String::from_utf8_lossy(&payload).into_owned();
    let start = text.find("<Application>")? + "<Application>".len();
    let end = text[start..].find("</Application>")? + start;
    Some(text[start..end].to_owned())
}

/// **No fixture claims Microsoft authorship unless a person has written down how it got here.**
///
/// The corpus comes from `mjx-fixtures` rather than from a list in this file, so a fixture
/// committed tomorrow is inside it the moment it lands — which is the only way a gate like this
/// stays true.
#[test]
fn no_fixture_claims_microsoft_authorship_off_the_ledger() {
    let ledger: BTreeSet<&str> = KNOWN_IMPERSONATORS
        .iter()
        .map(|entry| entry.fixture)
        .collect();

    let fixtures = mjx_fixtures::package_fixtures();
    // A floor phrased as *the walk is still matching*, never as the corpus's size: pinned to the
    // real total it would fire before the comparison it guards.
    assert!(
        fixtures.len() > 20,
        "only {} package fixture(s) were walked — `mjx_fixtures::package_fixtures` has stopped \
         matching, and the comparison below would pass on almost nothing",
        fixtures.len()
    );

    let mut examined = 0usize;
    let mut with_properties = 0usize;
    let mut failures: Vec<String> = Vec::new();
    for fixture in &fixtures {
        examined += 1;
        let Some(application) = application_of(fixture) else {
            continue;
        };
        with_properties += 1;
        if !application.to_ascii_lowercase().contains(NAMES_MICROSOFT) {
            continue;
        }
        if ledger.contains(fixture.as_str()) {
            continue;
        }
        failures.push(format!(
            "`tests/fixtures/{fixture}` carries `<Application>{application}</Application>` and is \
             on no ledger row. That element is **not** provenance — it is something its producer \
             chose to write, and two fixtures here already carry Microsoft's name without \
             Microsoft having touched them. Find out how this file reached the repository and add \
             an entry to `KNOWN_IMPERSONATORS` saying so. Do not answer the question from the \
             file: if it really came out of Office it belongs in `tests/office-authored/` by the \
             hand-off in `docs/validation/06-the-office-pass.md`, and that is a decision about \
             history rather than about bytes."
        ));
    }

    assert!(
        with_properties > 10,
        "only {with_properties} of {examined} fixture(s) yielded a `docProps/app.xml` — the part \
         reader has stopped matching"
    );
    assert!(
        failures.is_empty(),
        "{} fixture(s) claim Microsoft authorship off the ledger:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );

    println!(
        "provenance: {examined} package fixture(s), {with_properties} with `docProps/app.xml`, {} \
         naming Microsoft and every one of them on the ledger with its real producer",
        ledger.len()
    );
}

/// **The ledger cannot rot.** Every row names a fixture that is still here and still makes the
/// claim the row records.
///
/// Same shape as `doc_gate.rs`'s `every_retired_path_entry_is_still_needed`, and for the same
/// reason: a permission nothing uses is a permission nobody notices going wrong. A row kept after
/// its fixture was regenerated by something else would be a second false statement about
/// provenance, in the file written to prevent the first.
#[test]
fn every_known_impersonator_still_makes_the_claim_the_ledger_records() {
    assert!(
        !KNOWN_IMPERSONATORS.is_empty(),
        "`KNOWN_IMPERSONATORS` is empty, so the negative above compared nothing"
    );
    let committed: BTreeSet<String> = mjx_fixtures::package_fixtures().into_iter().collect();
    for entry in KNOWN_IMPERSONATORS {
        assert!(
            committed.contains(entry.fixture),
            "`KNOWN_IMPERSONATORS` names `{}`, which `mjx-fixtures` does not enumerate. Delete the \
             row: a ledger entry for a file that is gone is a sentence nothing holds.",
            entry.fixture
        );
        let application = application_of(entry.fixture).unwrap_or_else(|| {
            panic!(
                "`{}` is on the ledger for its `Application` element and ships no {APP_PROPERTIES}",
                entry.fixture
            )
        });
        assert_eq!(
            application, entry.claims,
            "`{}` now carries `{application}` where the ledger records `{}`. Whatever regenerated \
             it may also have changed its producer, so the `reached_us_by` sentence beside it can \
             no longer be trusted either.",
            entry.fixture, entry.claims
        );
        assert!(
            entry.reached_us_by.len() > 40,
            "`{}`'s ledger row states no evidence for how the file got here, which is the only \
             thing a row is for",
            entry.fixture
        );
        println!(
            "ledger: `{}` claims `{}` — {}",
            entry.fixture, entry.claims, entry.reached_us_by
        );
    }
}

/// **`validation-artefacts --ingest` never reads the `Application` element** — MJXOFF-249's third
/// condition.
///
/// The ingest report is where a candidate Office file is described back to the person who supplied
/// it, and it is exactly the place a provenance inference would be tempting: the element is right
/// there, and printing "written by Microsoft Excel" would look like a service. It would be a lie
/// about two of the fixtures in this very repository.
///
/// Held as a property of the source rather than as a promise, under the same same-block rule
/// `doc_gate.rs`'s `RETIRED_PATHS` uses: the word may appear only in a line that also names this
/// ticket, so an explanation of why the element is not read passes and a use of it does not.
#[test]
fn the_ingest_report_never_reads_the_application_element() {
    let directory = repository_root().join("xtask/src/validation");
    let mut scanned = 0usize;
    let mut offences: Vec<String> = Vec::new();
    let mut files: Vec<PathBuf> = Vec::new();
    collect_rust_sources(&directory, &mut files);
    files.sort();
    for file in &files {
        let text = std::fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("reading {}: {e}", file.display()));
        scanned += 1;
        for (index, line) in text.lines().enumerate() {
            if line.contains("Application") && !line.contains("MJXOFF-249") {
                offences.push(format!(
                    "{}:{}: {}",
                    file.strip_prefix(repository_root())
                        .unwrap_or(file)
                        .display(),
                    index + 1,
                    line.trim()
                ));
            }
        }
    }

    assert!(
        scanned > 3,
        "only {scanned} source file(s) were found under xtask/src/validation — the walk has \
         stopped matching"
    );
    assert!(
        offences.is_empty(),
        "the ingest report names `Application` in {} place(s) without naming MJXOFF-249:\n  {}\n\n\
         That element is not provenance. `tests/fixtures/charts.pptx` and \
         `tests/fixtures/comments_third_party.xlsx` both carry Microsoft's name and neither was \
         written by Microsoft, so a report that repeated it would be telling the person who \
         supplied a file the one thing they cannot check for themselves, wrongly.",
        offences.len(),
        offences.join("\n  ")
    );
    println!("ingest: {scanned} source file(s), none reading `Application`");
}

/// Every `.rs` file under a directory, recursively.
fn collect_rust_sources(directory: &Path, into: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(directory)
        .unwrap_or_else(|e| panic!("reading {}: {e}", directory.display()));
    for entry in entries {
        let entry = entry.unwrap_or_else(|e| panic!("reading {}: {e}", directory.display()));
        let path = entry.path();
        if path.is_dir() {
            collect_rust_sources(&path, into);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            into.push(path);
        }
    }
}
