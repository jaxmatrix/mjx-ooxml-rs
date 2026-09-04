//! Word's half of the ECMA-376 gate — running before a single line of `wml` model code exists.
//!
//! This file is the reason [`mjx_schema_gate`] was extracted from `mjx-pptx/tests/`: an integration
//! test compiles only into its own crate, so a harness living there could never be reached from
//! here. Every case below is the shared harness pointed at WordprocessingML.
//!
//! Before this, **`sample.docx` had never been schema-validated by anything.** Its four `word/*.xml`
//! parts were reported `SkippedForeignNamespace`, and the sentence "the schema gate covers Word and
//! is green" was true with zero implementation behind it. What made it possible to fix is in
//! `mjx_schema_gate::harness`: `wml.xsd:21` imports the XML namespace with no `schemaLocation`, so
//! the schema could not even *compile* until the gate started pairing it with a committed
//! `xml.xsd` through a generated driver schema.

use mjx_docx::{Document, DocxError, PageOrientation, PageSize};
use mjx_ooxml_core::{Interner, RawDocument, RawElement, RawName, RawNode};
use mjx_opc::{Package, PartName};
use mjx_schema_gate::{
    assert_authored_deck_is_schema_valid, assert_fixture_is_schema_valid, audit_deck_order,
    fixture, harness, inspect_deck, inspect_fixture, outcome_table,
    package_fixtures_with_extension, PartOutcome,
};

/// The WordprocessingML namespace, as `wml.xsd` declares it.
const WML_NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

#[test]
fn every_docx_fixture_is_schema_valid() {
    let fixtures = package_fixtures_with_extension("docx");
    assert!(
        !fixtures.is_empty(),
        "no .docx fixture — this case would pass vacuously"
    );
    for name in fixtures {
        assert_fixture_is_schema_valid(&name);
    }
}

#[test]
fn the_wordprocessingml_parts_are_validated_and_not_skipped() {
    // The clause this child is graded on. "Some part validated" is true of a `.docx` whose every
    // `w:` part was skipped and whose four validated parts were two `.rels`, the content types and
    // a DrawingML theme — that is exactly what the baseline was. So pin the verdict *per part*,
    // naming the schema: `wml.xsd`, not "something".
    let rows = inspect_fixture("sample.docx");
    if rows.is_empty() {
        return;
    }
    println!("{}", outcome_table("sample.docx", &rows));

    for part in [
        "/word/document.xml",
        "/word/styles.xml",
        "/word/fontTable.xml",
        "/word/settings.xml",
    ] {
        let row = rows
            .iter()
            .find(|row| row.name == part)
            .unwrap_or_else(|| panic!("sample.docx: {part} is not in the sweep"));
        assert_eq!(
            row.namespace.as_deref(),
            Some(WML_NS),
            "{part} must be rooted in WordprocessingML"
        );
        assert!(
            matches!(row.outcome, PartOutcome::Validated("wml.xsd")),
            "{part} must be validated against wml.xsd; it reported: {}",
            row.outcome.describe()
        );
    }

    // MJXOFF-149 decided document properties are authored, not merely preserved: the streams are
    // validated for real now, against their own schemas — `opc-coreProperties.xsd` (ECMA-376 Part
    // 2, Dublin Core) and `shared-documentPropertiesExtended.xsd` — not skipped as foreign.
    for (part, schema) in [
        ("/docProps/core.xml", "opc-coreProperties.xsd"),
        ("/docProps/app.xml", "shared-documentPropertiesExtended.xsd"),
    ] {
        let row = rows
            .iter()
            .find(|row| row.name == part)
            .unwrap_or_else(|| panic!("sample.docx: {part} is not in the sweep"));
        assert!(
            matches!(row.outcome, PartOutcome::Validated(s) if s == schema),
            "{part} must be validated against {schema}; it reported: {}",
            row.outcome.describe()
        );
    }
}

/// `sample.docx` with a `w:tbl` nested directly inside a `w:rPr` — markup `wml.xsd` rejects, because
/// `CT_RPr`'s content model holds run properties and no block-level content at all.
fn sample_docx_with_a_table_inside_run_properties() -> Vec<u8> {
    let mut package = Package::open(&fixture("sample.docx")).expect("open sample.docx");
    let part = PartName::new("/word/document.xml").expect("a valid part name");
    let RawDocument { interner, root, .. } = package
        .part_tree_mut(&part)
        .expect("edit word/document.xml");
    let table = RawElement::new(
        RawName {
            prefix: Some(interner.intern("w")),
            local: interner.intern("tbl"),
            namespace: Some(interner.intern(WML_NS)),
        },
        Vec::new(),
        Vec::new(),
        true,
    );
    assert!(
        plant_in_first_run_properties(root, interner, &table),
        "sample.docx has no w:rPr to corrupt"
    );
    package.save().expect("save the corrupted document")
}

/// Pushes a copy of `payload` into the first `w:rPr` found, depth first. Returns whether it landed.
fn plant_in_first_run_properties(
    element: &mut RawElement,
    interner: &Interner,
    payload: &RawElement,
) -> bool {
    let is_run_properties = element
        .name
        .namespace
        .is_some_and(|ns| interner.resolve(ns) == WML_NS)
        && interner.resolve(element.name.local) == "rPr";
    if is_run_properties {
        element.children.push(RawNode::Element(payload.clone()));
        return true;
    }
    for child in &mut element.children {
        if let RawNode::Element(child) = child {
            if plant_in_first_run_properties(child, interner, payload) {
                return true;
            }
        }
    }
    false
}

#[test]
fn invalid_wordprocessingml_is_caught_and_names_the_part() {
    // The proof that the arm is *live* rather than merely present. An arm that ships unproved is
    // the defect this child exists to close: a `w:` part with no arm is reported skipped, and a
    // suite that only asserts "no failures" is greenest when everything is skipped.
    //
    // This is a negative test in the strong sense — it fails if the arm is deleted, because the
    // part then reports `Uncategorised` or a skip instead of `INVALID (wml.xsd)`.
    let Some(harness) = harness() else { return };
    let corrupted = sample_docx_with_a_table_inside_run_properties();
    let rows = inspect_deck(
        &harness,
        "sample.docx with w:tbl inside w:rPr",
        &corrupted,
        &[],
    );

    let row = rows
        .iter()
        .find(|row| row.name == "/word/document.xml")
        .expect("word/document.xml is in the sweep");
    let PartOutcome::Failed { schema, report } = &row.outcome else {
        panic!(
            "a w:tbl inside a w:rPr must fail against wml.xsd; it reported: {}",
            row.outcome.describe()
        );
    };
    assert_eq!(*schema, "wml.xsd");
    assert!(
        report.contains("/word/document.xml"),
        "the failure must name the part:\n{report}"
    );
    assert!(
        report.contains("tbl"),
        "the failure must name the element that broke it:\n{report}"
    );
    println!("the wml arm, proved live:\n{report}");
}

#[test]
fn a_docx_the_library_re_emits_unchanged_is_still_schema_valid() {
    // The save path itself, for Word: open and re-emit, touching nothing. `mjx-opc` rewrites the
    // content types and every `.rels` stream on every save, so this is not a no-op even for a
    // package no `wml` code has touched — and it is the case Phase C inherits.
    let package = Package::open(&fixture("sample.docx")).expect("open");
    let saved = package.save().expect("save");
    // MJXOFF-90 flipped WordprocessingML's `OrderingCoverage` from `Pending` to `Generated` in
    // `mjx-schema-gate::categories` — this is the ordering half of the gate turning on for `wml` for
    // the first time, on the real `sample.docx` fixture rather than a synthetic one (the mirror of
    // `mjx-xlsx`'s own `an_xlsx_the_library_re_emits_unchanged_is_still_schema_valid`, which already
    // calls this for `sml`).
    mjx_schema_gate::assert_deck_is_in_schema_order("saved unedited sample.docx", &saved);
    mjx_schema_gate::assert_authored_deck_is_schema_valid("saved unedited sample.docx", &saved);
}

// -------------------------------------------------------------------------------------------------
// MJXOFF-98 — `Document::blank`: a document built from nothing, not opened from a file
// -------------------------------------------------------------------------------------------------

#[test]
fn a_blank_document_is_schema_valid() {
    // `Document::blank` writes `word/document.xml` and both `docProps` parts from nothing, on top of
    // `Package::empty`. Nothing here came from a file, so every byte of it is this project's to
    // answer for. Both named page sizes, both orientations: `w:pgSz`'s `orient` attribute changes
    // which branch of `document_bytes` runs.
    for size in [
        PageSize::a4(),
        PageSize::a4().landscape(),
        PageSize::us_letter(),
        PageSize::us_letter().landscape(),
    ] {
        let document = Document::blank(size).expect("blank");
        let saved = document.save().expect("save");
        assert_authored_deck_is_schema_valid(&format!("blank document ({size:?})"), &saved);
    }
}

#[test]
fn the_blank_document_validates_every_part_it_ships() {
    // A classification bug that skipped a new part would let invalid markup through as a pass, so
    // pin the verdicts: all five entries are accounted for, and the three markup streams
    // (`word/document.xml` plus both `docProps` parts, MJXOFF-149) are genuinely validated, not
    // skipped. See `crates/mjx-docx/src/blank.rs`'s own module doc for why this is five parts, not
    // `sample.docx`'s ten.
    let Some(harness) = harness() else { return };
    let document = Document::blank(PageSize::a4()).expect("blank");
    let saved = document.save().expect("save");
    let rows = inspect_deck(&harness, "blank document coverage", &saved, &[]);

    let validated: Vec<&str> = rows
        .iter()
        .filter(|row| matches!(row.outcome, PartOutcome::Validated(_)))
        .map(|row| row.name.as_str())
        .collect();
    assert_eq!(
        validated,
        [
            "/[Content_Types].xml",
            "/_rels/.rels",
            "/word/document.xml",
            "/docProps/core.xml",
            "/docProps/app.xml",
        ],
        "every entry of a blank document must be validated, none skipped"
    );
    assert_eq!(rows.len(), validated.len());

    for (part, schema) in [
        ("/word/document.xml", "wml.xsd"),
        ("/docProps/core.xml", "opc-coreProperties.xsd"),
        ("/docProps/app.xml", "shared-documentPropertiesExtended.xsd"),
    ] {
        let row = rows
            .iter()
            .find(|row| row.name == part)
            .unwrap_or_else(|| panic!("blank document: {part} is not in the sweep"));
        assert!(
            matches!(row.outcome, PartOutcome::Validated(s) if s == schema),
            "{part} must be validated against {schema}; it reported: {}",
            row.outcome.describe()
        );
    }
}

#[test]
fn the_blank_document_child_order_audit_reaches_word_document_xml_and_is_not_vacuous() {
    // The mirror of `mjx-pptx`'s own `the_child_order_audit_reaches_every_authored_markup_part…` —
    // an audit that visits nothing passes for the wrong reason.
    let document = Document::blank(PageSize::a4()).expect("blank");
    let saved = document.save().expect("save");
    let audited = audit_deck_order("blank document order coverage", &saved);
    let parts: Vec<&str> = audited.iter().map(|part| part.name.as_str()).collect();
    assert_eq!(
        parts,
        ["/word/document.xml"],
        "the only authored WordprocessingML part a blank document ships must be audited"
    );
    assert!(
        audited[0].elements_visited >= 3,
        "word/document.xml: the walk checked only {} elements — it is not descending",
        audited[0].elements_visited
    );
}

#[test]
fn two_successive_blank_calls_produce_identical_bytes() {
    // Determinism: no timestamp, no random id. `DocumentTimestamp` has no `now()` for exactly this
    // reason (`mjx_opc::doc_props`'s own module doc), and relationship ids are the fixed `rId1..3`
    // `blank::package` writes every time.
    let first = Document::blank(PageSize::a4())
        .expect("blank")
        .save()
        .expect("save");
    let second = Document::blank(PageSize::a4())
        .expect("blank")
        .save()
        .expect("save");
    assert_eq!(
        first, second,
        "two blank() calls must produce identical bytes"
    );
}

#[test]
fn text_added_through_the_body_api_reads_back_and_stays_schema_valid() {
    // The story MJXOFF-92's API tells against a document built from nothing rather than opened from
    // a file: append a paragraph, put a run in it, read the text back, and the result must still be
    // schema-valid — proving `Document::blank`'s body is not a special case the rest of this crate's
    // API cannot reach.
    let mut document = Document::blank(PageSize::us_letter()).expect("blank");
    assert_eq!(document.paragraph_count().expect("paragraph_count"), 1);

    document.append_paragraph().expect("append_paragraph");
    document
        .append_run(1, "Hello, document.")
        .expect("append_run");
    assert_eq!(
        document.paragraph_text(1).expect("paragraph_text"),
        "Hello, document."
    );

    let saved = document.save().expect("save");
    assert_authored_deck_is_schema_valid("blank document with an added paragraph and run", &saved);

    let mut reopened = Document::open(&saved).expect("reopen");
    assert_eq!(reopened.paragraph_count().expect("paragraph_count"), 2);
    assert_eq!(
        reopened.paragraph_text(1).expect("paragraph_text"),
        "Hello, document."
    );
}

/// `Document::blank`'s `word/document.xml`, with one of `w:pgMar`'s seven attributes stripped out —
/// syntactically valid XML, invalid WordprocessingML (`CT_PageMar` declares every one of the seven
/// `use="required"`).
fn blank_document_missing_pg_mar_attribute(attribute: &str, value: &str) -> Vec<u8> {
    let document = Document::blank(PageSize::a4()).expect("blank");
    let saved = document.save().expect("save");
    let mut package = Package::open(&saved).expect("reopen the saved bytes");
    let part = PartName::new("/word/document.xml").expect("a valid part name");
    let bytes = package
        .part_bytes(&part)
        .expect("blank document always carries word/document.xml");
    let xml = std::str::from_utf8(bytes).expect("authored markup is UTF-8");
    let needle = format!(r#" w:{attribute}="{value}""#);
    assert!(
        xml.contains(&needle),
        "word/document.xml does not carry {needle:?} to strip — the fixed margins changed:\n{xml}"
    );
    let corrupted = xml.replacen(&needle, "", 1).into_bytes();
    package
        .replace_part_bytes(&part, corrupted)
        .expect("replace word/document.xml");
    package
        .save_unchecked()
        .expect("save the corrupted document")
}

#[test]
fn dropping_any_pg_mar_attribute_turns_the_schema_gate_red() {
    // The mutation gate this child's own module doc owes: `CT_PageMar`'s seven attributes are the
    // *only* genuinely `use="required"` claim `blank.rs` makes about `word/document.xml` (`w:pgSz`,
    // `w:pgMar` and `w:sectPr` themselves are all `minOccurs="0"` — see that module's doc comment).
    // Proved once per attribute, not asserted in aggregate, so a single passing case cannot hide six
    // failing ones.
    let Some(harness) = harness() else { return };
    // Matches `PageMargins::NORMAL` exactly (`crates/mjx-docx/src/page.rs`).
    let attributes: &[(&str, &str)] = &[
        ("top", "1440"),
        ("right", "1440"),
        ("bottom", "1440"),
        ("left", "1440"),
        ("header", "720"),
        ("footer", "720"),
        ("gutter", "0"),
    ];
    for (attribute, value) in attributes {
        let corrupted = blank_document_missing_pg_mar_attribute(attribute, value);
        let rows = inspect_deck(
            &harness,
            &format!("blank document missing w:pgMar@{attribute}"),
            &corrupted,
            &[],
        );
        let row = rows
            .iter()
            .find(|row| row.name == "/word/document.xml")
            .expect("word/document.xml is in the sweep");
        let PartOutcome::Failed { schema, report } = &row.outcome else {
            panic!(
                "dropping w:pgMar@{attribute} must fail against wml.xsd; it reported: {}",
                row.outcome.describe()
            );
        };
        assert_eq!(*schema, "wml.xsd");
        assert!(
            report.contains("pgMar"),
            "the failure must name the element that broke it ({attribute}):\n{report}"
        );
    }
}

#[test]
fn an_out_of_range_page_size_is_a_typed_error_not_a_written_file() {
    // The refusal `PageSize::validate` performs happens before any byte is written — the degenerate
    // size never reaches the schema gate at all, matching `mjx_pptx::Presentation::blank`'s own
    // `PptxError::InvalidSlideSize` contract.
    let degenerate = PageSize::from_twips(0, 16_838, PageOrientation::Portrait);
    assert!(matches!(
        Document::blank(degenerate),
        Err(DocxError::InvalidPageSize { .. })
    ));
}
