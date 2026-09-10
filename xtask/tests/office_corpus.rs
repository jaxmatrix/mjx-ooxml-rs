//! The Office-authored corpus: whatever `tests/office-authored/` holds, held to this library's own
//! promise (MJXOFF-130).
//!
//! # The weakness this retires
//!
//! Every fixture in `tests/fixtures/` was written by this project or by LibreOffice. Phase A said so
//! in every child's report — *the gate proves our reader agrees with our writer* — and the gaps page
//! of all three formats still carries the row. Nothing here has ever read a file real Microsoft
//! Office wrote, so the reader has only ever been held to markup shaped the way our own writer
//! shapes it. **This suite is the road; the files are traffic only a person with Office can supply.**
//!
//! # The trap this suite is written against, in its own terms
//!
//! *A corpus suite over an empty directory passes trivially, and will keep passing after files
//! arrive if the walk is wrong.* Three things answer that, and none of them is a comment:
//!
//! 1. [`the_corpus_is_walked_and_its_size_reported`] prints the count on every run, so "green" never
//!    silently means "nothing ran", and `MJX_REQUIRE_OFFICE_CORPUS=1` turns an empty corpus into a
//!    failure the moment CI should expect one.
//! 2. [`a_package_this_suite_corrupts_fails_the_checks_that_matter`] runs the very same engine over
//!    bytes deliberately broken in four different ways and asserts each one is caught. It is the
//!    permanent form of "drop a corrupted file in and confirm red": the engine is proved able to
//!    fail on every run, not once by hand.
//! 3. [`the_walk_binds_a_name_to_an_entry_and_refuses_anything_else`] holds the naming convention,
//!    because a file the walk cannot bind is a file no edit variant reads.
//!
//! # What may fail a build, and what may only be reported
//!
//! An ingested file is **not ours**, and `xtask/src/validation/ingest.rs` draws the line in full.
//! Byte identity, the fidelity tree, the facade's own re-save, a package defect *we* introduced and
//! a part out of `xsd:sequence` are this library's and fail. A defect the file arrived with, and a
//! part its producer wrote that the XSDs reject, are the file's and are printed instead — A7b's
//! scope rule in one direction, and MJXOFF-103's Apache POI measurement in the other.

use std::path::PathBuf;

use xtask::validation::{corpus_directory, corpus_files, ingest, report, ArtefactFormat, Verdict};

/// The corpus, or a hard failure when `MJX_REQUIRE_OFFICE_CORPUS` says there must be one.
///
/// The gate is the same shape as `MJX_REQUIRE_SCHEMA` and `MJX_REQUIRE_SOFFICE`: the expectation
/// lives in the code and CI decides whether absence is tolerable. It is **not** set today, because
/// the corpus is empty and no agent may fill it — the files come out of a person's Office.
fn corpus() -> Vec<xtask::validation::CorpusFile> {
    let files = corpus_files().expect("walking the Office-authored corpus");
    if files.is_empty() && std::env::var_os("MJX_REQUIRE_OFFICE_CORPUS").is_some() {
        panic!(
            "MJX_REQUIRE_OFFICE_CORPUS is set and {} is empty. Set it only once the corpus has \
             files: an empty corpus is the honest state of this project until somebody re-saves \
             something out of Microsoft Office.",
            corpus_directory().display()
        );
    }
    files
}

#[test]
fn the_corpus_is_walked_and_its_size_reported() {
    let files = corpus();
    println!(
        "office corpus: {} file(s) in {}",
        files.len(),
        corpus_directory().display()
    );
    for file in &files {
        println!(
            "  {:<24} {:<5} {}",
            file.name,
            file.format.extension(),
            match file.area {
                Some(area) => area.id,
                None => "—",
            }
        );
    }
    if files.is_empty() {
        println!(
            "the corpus is empty, which is the honest state of this project: no agent may fill it, \
             because the value of an Office-authored file is entirely its provenance. \
             docs/validation/06-the-office-pass.md is how it gets filled."
        );
    }
}

#[test]
fn the_walk_binds_a_name_to_an_entry_and_refuses_anything_else() {
    for file in corpus() {
        assert!(
            file.area.is_some(),
            "{} is in the corpus and binds to no validation entry. One file per area, named by the \
             area's own id — `{}.{}` — because an unbound file is one no edit variant reads and no \
             result line refers to. `cargo run -p xtask -- validation-artefacts --list` prints \
             every id.",
            file.name,
            "v-xlsx-02",
            file.format.extension()
        );
        let area = file.area.expect("just asserted");
        assert_eq!(
            area.format,
            file.format,
            "{} binds to {}, which is a {} area — the extension and the entry id disagree",
            file.name,
            area.id,
            area.format.extension()
        );
    }
}

#[test]
fn every_corpus_file_holds_this_librarys_own_promise() {
    let files = corpus();
    let mut failures = Vec::new();
    for file in &files {
        let report = ingest(&file.path, file.area).expect("ingesting a corpus file");
        print!("{}", report.render());
        assert!(
            report.checks_that_held() >= 4,
            "{}: only {} check(s) held. A report that skipped its way to green is the failure mode \
             this suite exists to prevent",
            file.name,
            report.checks_that_held()
        );
        for finding in report.failures() {
            failures.push(format!(
                "{}: {} — {}",
                file.name, finding.check, finding.detail
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "the Office-authored corpus found {} defect(s) in this library:\n{}",
        failures.len(),
        failures.join("\n")
    );
    println!(
        "office corpus: {} file(s) round-tripped, validated and audited",
        files.len()
    );
}

// ---------------------------------------------------------------------------------------------
// Proving the engine can fail
// ---------------------------------------------------------------------------------------------

/// The four ways a corpus file can be broken, each with the check that must catch it.
///
/// Written as data rather than as four cases so the list is the assertion: a check that stopped
/// catching its own corruption fails here, naming both.
fn corruptions() -> Vec<(&'static str, &'static str, Vec<u8>)> {
    let sound = mjx_fixtures::fixture("sample.xlsx");

    // Not a ZIP at all.
    let rubbish = b"PK\x03\x04 this is not a package".to_vec();

    // A real package with its last kilobyte cut off: the central directory is gone.
    let mut truncated = sound.clone();
    truncated.truncate(sound.len() - 1024);

    // A package whose worksheet still parses and is no longer SpreadsheetML. Renaming the root
    // leaves well-formed XML in the `x:` namespace whose local name the generated tables do not
    // know, so `parts_that_must_be_audited` still *requires* the part — it categorises by namespace
    // — and the walk cannot reach it. That is a `child order` failure, which is the point of
    // choosing this shape: the schema half would also report it, but only where `References/` and
    // `xmllint` are present, and a corruption caught by a check that *skips* on CI is a corruption
    // nothing catches.
    let mut package = mjx_opc::Package::open(&sound).expect("opening sample.xlsx");
    let part = mjx_opc::PartName::new("/xl/worksheets/sheet1.xml").expect("a part name");
    {
        let tree = package.part_tree_mut(&part).expect("the worksheet tree");
        let renamed = tree.interner.intern("worksheetish");
        tree.root.name.local = renamed;
    }
    let renamed_root = package.save_unchecked().expect("saving the edited package");

    // A package with a relationship pointing at a part that is not there. `theme99` and not
    // `theme1`: `sample.xlsx` *has* a `xl/theme/theme1.xml`, so the first spelling of this
    // corruption was not a corruption at all and `Package::validate` was right to hold. A mutation
    // has to be reachable before its verdict means anything.
    let mut package = mjx_opc::Package::open(&sound).expect("opening sample.xlsx");
    package
        .add_relationship(
            None,
            mjx_opc::Relationship {
                id: "rIdMissing".to_owned(),
                rel_type:
                    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme"
                        .to_owned(),
                target: "xl/theme/theme99.xml".to_owned(),
                mode: mjx_opc::TargetMode::Internal,
            },
        )
        .expect("adding a dangling relationship");
    let dangling = package.save_unchecked().expect("saving the edited package");

    vec![
        ("not a package at all", "opens", rubbish),
        ("a package cut in half", "opens", truncated),
        (
            "a worksheet renamed at the root",
            "child order",
            renamed_root,
        ),
        ("a relationship with no target", "package", dangling),
    ]
}

/// MJXOFF-273's third clause: the report a person reads must say which of the two empty roots it
/// saw, and must not call either one a finding.
///
/// `legacy_form_control.xlsx` is the emptied case — an `xdr:wsDr` whose only child is an
/// `mc:AlternateContent` with a losing `a14` choice and no `mc:Fallback` — and `charts.pptx` is the
/// empty one. The verdict assertion is as load-bearing as the text: auditing the losing choice would
/// mean faulting a producer's extension markup against schemas that do not describe it, so a
/// `Reported` here would put a permanent yellow on essentially every Office-authored drawing. It is
/// recorded, not reported.
#[test]
fn the_ingest_report_tells_an_emptied_root_from_an_empty_one() {
    let emptied = report(
        "legacy_form_control.xlsx",
        ArtefactFormat::Workbook,
        None,
        &mjx_fixtures::fixture("legacy_form_control.xlsx"),
    );
    let finding = emptied
        .finding("child order")
        .expect("the workbook is audited, so the report has a child-order finding");
    assert!(
        matches!(finding.verdict, Verdict::Held),
        "an emptied root is not a defect and must not be dressed as one — the verdict came back \
         `{:?}` with: {}",
        finding.verdict,
        finding.detail
    );
    assert!(
        finding.detail.contains("/xl/drawings/drawing1.xml"),
        "but the report must still name it, or a reader cannot tell this complete audit was \
         complete over nothing the file contains: {}",
        finding.detail
    );

    let empty = report(
        "charts.pptx",
        ArtefactFormat::Presentation,
        None,
        &mjx_fixtures::fixture("charts.pptx"),
    );
    let finding = empty
        .finding("child order")
        .expect("the deck is audited, so the report has a child-order finding");
    assert!(
        matches!(finding.verdict, Verdict::Held),
        "and a deck whose only bare root is a genuinely empty a:tblStyleLst holds: {}",
        finding.detail
    );
    assert!(
        !finding.detail.contains("resolution removed"),
        "with nothing said about resolution removing anything — a clause on every empty part is \
         noise, not a distinction: {}",
        finding.detail
    );
}

#[test]
fn a_package_this_suite_corrupts_fails_the_checks_that_matter() {
    for (what, check, bytes) in corruptions() {
        let report = report(
            &format!("corrupted-{check}.xlsx"),
            ArtefactFormat::Workbook,
            None,
            &bytes,
        );
        let finding = report.finding(check).unwrap_or_else(|| {
            panic!(
                "{what}: the report has no `{check}` finding at all:\n{}",
                report.render()
            )
        });
        // Not merely "did not hold". A `Skipped` verdict would satisfy that and mean the check
        // never ran — which is the false green this whole phase is written against, and exactly
        // what a corruption aimed at the schema half would have got on any machine without
        // `References/`. All four are aimed at checks that run everywhere.
        assert!(
            matches!(finding.verdict, Verdict::Failed | Verdict::Reported),
            "{what}: the `{check}` check came back `{}`. A corruption that a check *held* on, or \
             that only a check which skipped could have caught, is the non-discriminating pass this \
             suite is written against:\n{}",
            finding.verdict.label(),
            report.render()
        );
        println!(
            "corruption `{what}` was caught by `{check}`: {} — {}",
            finding.verdict.label(),
            finding.detail.lines().next().unwrap_or_default()
        );
    }
}

/// The check the corpus makes of the *sound* fixture, so the case above is not green merely because
/// every report is red. A corruption test with no control proves the engine is broken, not that it
/// is discriminating.
#[test]
fn the_same_engine_holds_a_package_that_is_not_corrupted() {
    let report = report(
        "sample.xlsx",
        ArtefactFormat::Workbook,
        None,
        &mjx_fixtures::fixture("sample.xlsx"),
    );
    assert!(
        report.failures().is_empty(),
        "the engine faults a committed fixture, so its verdicts on a corpus file mean \
         nothing:\n{}",
        report.render()
    );
    assert!(
        report.checks_that_held() >= 6,
        "only {} check(s) held on a sound package:\n{}",
        report.checks_that_held(),
        report.render()
    );
}

/// Where every document in this repository says the corpus lives, so a reader of this file can
/// find it.
#[test]
fn the_corpus_directory_is_where_every_document_says_it_is() {
    let directory = corpus_directory();
    assert!(
        directory.ends_with("tests/office-authored"),
        "the corpus moved to {}, and docs/validation/, tests/office-authored/README.md and \
         xtask/src/validation/corpus.rs all name the old place",
        directory.display()
    );
    assert!(
        directory.join("README.md").is_file(),
        "{} has no README, and the redistribution rule is the first thing a person adding a file \
         needs to read",
        directory.display()
    );
    let _: PathBuf = directory;
}
