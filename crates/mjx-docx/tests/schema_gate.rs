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

use mjx_ooxml_core::{Interner, RawDocument, RawElement, RawName, RawNode};
use mjx_opc::{Package, PartName};
use mjx_schema_gate::{
    assert_fixture_is_schema_valid, fixture, harness, inspect_deck, inspect_fixture, outcome_table,
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
    mjx_schema_gate::assert_authored_deck_is_schema_valid("saved unedited sample.docx", &saved);
}
