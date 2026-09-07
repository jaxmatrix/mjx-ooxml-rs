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

// ---------------------------------------------------------------------------------------------
// The seam the first Office-authored workbook will hit
// ---------------------------------------------------------------------------------------------

/// The SpreadsheetML namespace, as every part of this reproduction spells it.
const SML_NAMESPACE: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

/// One worksheet in **three** views: as a producer writes it, with only the markup-compatibility
/// *attributes* taken off, and as markup-compatibility resolution leaves it.
///
/// Authored here, for this reproduction, and **not** presented as anything Office wrote — that
/// distinction is the whole value of `tests/office-authored/`. The shape is the one every modern
/// Office workbook and every modern Office chart carries: an `<ext>` whose only child is in a
/// namespace the root declares `mc:Ignorable`.
///
/// The middle view is what makes the diagnosis complete rather than a bare failure. It is the same
/// document with the `mc:Ignorable` attribute and the `xmlns:mc` binding removed and the ignorable
/// *content* left in place — and it validates, because `CT_Extension`'s wildcard is
/// `processContents="lax"` and no schema for that namespace is loaded. So the schema does not object
/// to the extension; it objects to the **hole** resolution leaves where the extension was.
fn the_three_views() -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let extension = "<extLst><ext uri=\"{2C3FCC01-B0D6-4A2A-9C1A-000000000001}\">\
                     <demo:note weight=\"3\">an extension only its author understands</demo:note>\
                     </ext></extLst>";
    let as_written = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
         <worksheet xmlns=\"{SML_NAMESPACE}\" \
         xmlns:mc=\"http://schemas.openxmlformats.org/markup-compatibility/2006\" \
         xmlns:demo=\"urn:mjx:demo\" mc:Ignorable=\"demo\">\
         <sheetData><row r=\"1\"><c r=\"A1\"><v>1</v>{extension}</c></row></sheetData></worksheet>\n"
    )
    .into_bytes();
    let without_compatibility_attributes = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
         <worksheet xmlns=\"{SML_NAMESPACE}\" xmlns:demo=\"urn:mjx:demo\">\
         <sheetData><row r=\"1\"><c r=\"A1\"><v>1</v>{extension}</c></row></sheetData></worksheet>\n"
    )
    .into_bytes();

    let document = mjx_xml::fidelity::parse(&as_written).expect("the authored worksheet parses");
    let resolved = mjx_schema_gate::markup_compatibility_resolved(&document)
        .expect("nothing here is an unsatisfied mc:MustUnderstand");
    (as_written, without_compatibility_attributes, resolved)
}

#[test]
fn markup_compatibility_resolution_empties_the_extension_it_was_meant_to_ignore() {
    let (as_written, _, resolved) = the_three_views();
    let text = String::from_utf8(resolved).expect("the resolved view is UTF-8");
    assert!(
        String::from_utf8_lossy(&as_written).contains("demo:note"),
        "the reproduction no longer carries an ignorable child, so it reproduces nothing"
    );
    assert!(
        !text.contains("demo:note"),
        "markup-compatibility resolution kept the ignorable child, so the seam this test documents \
         has changed shape:\n{text}"
    );
    assert!(
        text.contains("<ext"),
        "the extension itself was removed as well as its content, which is not what MCE says and \
         not the defect recorded here:\n{text}"
    );
    println!("MCE resolution leaves: {}", text.trim());
}

/// The composition defect itself: the view MCE mandates is the view the schema rejects.
///
/// **This test fails when somebody fixes the seam, and that is the point.** It is written against a
/// defect rather than against a feature, so its own red is the signal that the defect is gone and
/// this file, `ingest.rs`'s module documentation and the corpus README can all lose a paragraph.
///
/// It is *not* a tolerance. A tolerance in `crates/mjx-schema-gate/src/tolerances.rs` is for one
/// file and one message, and never for markup we author — and this is neither file-specific nor the
/// producer's fault. `sml.xsd` and `dml-chart.xsd` both declare `CT_Extension`'s wildcard as a bare
/// `<xsd:any processContents="lax"/>`, whose `minOccurs` therefore defaults to 1; `pml.xsd`'s copy
/// and `dml-main.xsd`'s `CT_OfficeArtExtension` say `minOccurs="0"` and are unaffected. So the
/// defect reaches every format, through charts, and not Excel alone.
///
/// The three views together are the diagnosis:
///
/// | View | Verdict | What it establishes |
/// |---|---|---|
/// | as a producer writes it | rejected — `mc:Ignorable` *is not allowed* | why the gate resolves at all |
/// | compatibility attributes removed, content kept | validates | the schema does not object to the extension |
/// | fully resolved | rejected — *Missing child element(s)* | it objects to the hole resolution leaves |
#[test]
fn the_resolved_view_of_an_ignorable_extension_is_rejected_by_the_schema_that_admits_the_original()
{
    let Some(harness) = mjx_schema_gate::harness() else {
        println!(
            "skipped: no References/ tree or no xmllint. MJX_REQUIRE_SCHEMA=1 makes that a failure."
        );
        return;
    };
    let schema = mjx_schema_gate::schema_for_namespace(SML_NAMESPACE)
        .expect("SpreadsheetML is a modelled schema");
    let work = mjx_schema_gate::WorkDir::new("office-corpus-mce-seam");
    let (as_written, kept, resolved) = the_three_views();

    let verdict = |name: &str, bytes: &[u8]| -> Option<String> {
        let path = work.path().join(name);
        std::fs::write(&path, bytes).expect("writing a view");
        harness.validate(schema, SML_NAMESPACE, &path)
    };

    let written_report = verdict("as-written.xml", &as_written).unwrap_or_else(|| {
        panic!(
            "the worksheet as a producer writes it validates with `mc:Ignorable` still on it, so \
             the gate has no reason to resolve markup compatibility and this whole seam is gone"
        )
    });
    assert!(
        written_report.contains("Ignorable"),
        "the authored view fails for a reason other than its compatibility \
         attribute:\n{written_report}"
    );

    assert!(
        verdict("content-kept.xml", &kept).is_none(),
        "the schema rejects the extension itself, not the hole resolution leaves — the diagnosis \
         recorded here is wrong:\n{}",
        verdict("content-kept.xml", &kept).unwrap_or_default()
    );

    let resolved_report = verdict("mce-resolved.xml", &resolved).unwrap_or_else(|| {
        panic!(
            "the resolved view validates. The MCE/CT_Extension seam defect is fixed — delete this \
             test, the paragraph in xtask/src/validation/ingest.rs and the one in \
             tests/office-authored/README.md, and close MJXOFF-196."
        )
    });
    assert!(
        resolved_report.contains("Missing child element(s)"),
        "the resolved view fails for a different reason than the one recorded \
         here:\n{resolved_report}"
    );
    println!(
        "the MCE/CT_Extension seam, reproduced:\n  as written:        {}\n  content kept:      \
         validates\n  MCE-resolved:      {}",
        written_report.trim(),
        resolved_report.trim()
    );
}

/// Where the reproduction above says the corpus lives, so a reader of this file can find it.
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
