//! The SpreadsheetML part graph, over the real fixture and over a package with parts nobody models.
//!
//! MJXOFF-91's "Done when" has two clauses this file carries: *every part kind the fixture carries
//! is classified by `parts.rs`*, and *an unrecognised part is preserved rather than rejected*. They
//! pull in opposite directions on purpose — the first says the classifier must be complete over what
//! a real producer writes, the second says being incomplete must never cost a byte — so both are
//! asserted over the same package rather than in separate suites that could each be green about a
//! different file.

use mjx_fixtures::fixture;
use mjx_opc::{Package, PartName, Relationship, TargetMode};
use mjx_xlsx::{
    parts, PartClassification, PartKind, SheetKind, TargetMode as ReexportedTargetMode, Workbook,
};

/// Every part `sample.xlsx` carries, and what this crate must make of it.
///
/// Written out rather than derived, because the point is to state the expected answer independently
/// of the code that produces it. `docProps/*` and the two `.rels` streams are deliberately
/// `Unclassified`: they are OPC concepts, not SpreadsheetML ones, and this crate classifying them
/// would be it claiming a part kind ECMA-376 §12.3 does not give it.
const EXPECTED: &[(&str, Option<PartKind>)] = &[
    ("/xl/workbook.xml", Some(PartKind::Workbook)),
    ("/xl/worksheets/sheet1.xml", Some(PartKind::Worksheet)),
    ("/xl/sharedStrings.xml", Some(PartKind::SharedStrings)),
    ("/xl/styles.xml", Some(PartKind::Styles)),
    ("/xl/theme/theme1.xml", Some(PartKind::Theme)),
    ("/docProps/core.xml", None),
    ("/docProps/app.xml", None),
    ("/_rels/.rels", None),
    ("/xl/_rels/workbook.xml.rels", None),
];

#[test]
fn every_part_the_fixture_carries_is_classified_as_expected() {
    let workbook = Workbook::open(&fixture("sample.xlsx")).expect("open sample.xlsx");
    let inventory = workbook.part_inventory();

    for (name, expected) in EXPECTED {
        let row = inventory
            .iter()
            .find(|row| row.part.as_str() == *name)
            .unwrap_or_else(|| panic!("{name} is not in the inventory"));
        assert_eq!(
            row.classification.kind(),
            *expected,
            "{name} was classified {:?}",
            row.classification
        );
        assert!(
            row.content_type.is_some(),
            "{name} has no content type at all"
        );
    }

    assert_eq!(
        inventory.len(),
        EXPECTED.len(),
        "the inventory holds {} parts, this suite names {} — a fixture changed under it: {:?}",
        inventory.len(),
        EXPECTED.len(),
        inventory
            .iter()
            .map(|row| row.part.as_str())
            .collect::<Vec<_>>()
    );

    // The four SpreadsheetML parts are exactly the four the schema gate holds under `sml.xsd`, and
    // the theme is exactly the one that is not.
    let spreadsheetml: Vec<&str> = inventory
        .iter()
        .filter(|row| {
            row.content_type.is_some_and(|content_type| {
                content_type
                    .starts_with("application/vnd.openxmlformats-officedocument.spreadsheetml.")
            })
        })
        .map(|row| row.part.as_str())
        .collect();
    assert_eq!(
        spreadsheetml,
        [
            "/xl/workbook.xml",
            "/xl/styles.xml",
            "/xl/worksheets/sheet1.xml",
            "/xl/sharedStrings.xml",
        ]
    );
}

#[test]
fn the_workbook_and_worksheet_graphs_are_resolved_from_the_fixture() {
    let workbook = Workbook::open(&fixture("sample.xlsx")).expect("open sample.xlsx");

    let parts = workbook.parts();
    assert_eq!(
        parts.styles.as_ref().map(PartName::as_str),
        Some("/xl/styles.xml")
    );
    assert_eq!(
        parts.shared_strings.as_ref().map(PartName::as_str),
        Some("/xl/sharedStrings.xml")
    );
    assert_eq!(
        parts.theme.as_ref().map(PartName::as_str),
        Some("/xl/theme/theme1.xml")
    );
    assert_eq!(
        parts
            .worksheets
            .iter()
            .map(PartName::as_str)
            .collect::<Vec<_>>(),
        ["/xl/worksheets/sheet1.xml"]
    );
    // The eight relationship types LibreOffice did not write are absent rather than defaulted.
    assert!(parts.calculation_chain.is_none());
    assert!(parts.connections.is_none());
    assert!(parts.metadata.is_none());
    assert!(parts.volatile_dependencies.is_none());
    assert!(parts.chartsheets.is_empty());
    assert!(parts.dialogsheets.is_empty());
    assert!(parts.external_links.is_empty());
    assert!(parts.pivot_cache_definitions.is_empty());

    assert_eq!(
        parts.sheet_parts(),
        vec![(
            SheetKind::Worksheet,
            PartName::new("/xl/worksheets/sheet1.xml").expect("a valid part name")
        )]
    );

    // The sheet tier: `sample.xlsx`'s one worksheet relates to nothing at all.
    let sheet = workbook
        .worksheet(0)
        .expect("resolve the sheet")
        .expect("the fixture has one");
    assert_eq!(sheet.kind(), Some(SheetKind::Worksheet));
    assert_eq!(sheet.entry().name, "sample");
    let sheet_parts = sheet.parts();
    assert!(sheet_parts.drawing.is_none());
    assert!(sheet_parts.vml_drawing.is_none());
    assert!(sheet_parts.comments.is_none());
    assert!(sheet_parts.printer_settings.is_none());
    assert!(sheet_parts.tables.is_empty());
    assert!(sheet_parts.query_tables.is_empty());
    assert!(sheet_parts.pivot_tables.is_empty());

    assert!(
        workbook.worksheet(1).expect("no error").is_none(),
        "there is no second sheet"
    );
}

#[test]
fn a_part_kind_this_crate_does_not_classify_is_carried_through_a_save_untouched() {
    // A part with a content type no `PartKind` claims — an add-in's private data, a producer's own
    // sidecar — inserted into the real fixture beside the parts this crate does know. Both halves
    // of the clause are then asserted on the same package: the classifier is complete about what it
    // knows, and silent about what it does not.
    const PAYLOAD: &[u8] =
        b"<vendor:blob xmlns:vendor=\"urn:example:vendor\">\xE2\x9C\x93</vendor:blob>";

    let mut package = Package::open(&fixture("sample.xlsx")).expect("open sample.xlsx");
    let workbook_part = PartName::new("/xl/workbook.xml").expect("a valid part name");
    let stranger = PartName::new("/xl/vendor/blob.xml").expect("a valid part name");
    package
        .insert_part(&stranger, "application/x-vendor-blob+xml", PAYLOAD.to_vec())
        .expect("insert a part nothing here classifies");
    package
        .add_relationship(
            Some(&workbook_part),
            Relationship {
                id: "rIdVendor".to_owned(),
                rel_type: "urn:example:vendor/relationships/blob".to_owned(),
                target: "vendor/blob.xml".to_owned(),
                mode: TargetMode::Internal,
            },
        )
        .expect("relate it from the workbook");

    let workbook = Workbook::from_package(package).expect("the workbook still opens");
    let row = workbook
        .part_inventory()
        .into_iter()
        .find(|row| row.part == stranger)
        .expect("the stranger is in the inventory");
    assert_eq!(row.classification, PartClassification::Unclassified);
    assert_eq!(row.classification.kind(), None);

    let saved = workbook
        .save()
        .expect("an unclassified part does not block a save");
    let reopened = Package::open(&saved).expect("reopen");
    assert_eq!(
        reopened.part_bytes(&stranger),
        Some(PAYLOAD),
        "the part this crate could not classify came back byte for byte"
    );
    // …and it did not cost the parts around it anything either.
    for name in ["/xl/workbook.xml", "/xl/worksheets/sheet1.xml"] {
        let part = PartName::new(name).expect("a valid part name");
        let original = Package::open(&fixture("sample.xlsx")).expect("reopen the fixture");
        assert_eq!(
            reopened.part_bytes(&part),
            original.part_bytes(&part),
            "{name} changed while an unrelated part was added"
        );
    }
}

#[test]
fn the_relationship_and_content_type_constants_are_the_crates_public_vocabulary() {
    // The constants are `pub` because a caller authoring a part needs them and because MJXOFF-112
    // will move `mjx-chart`'s copies onto them. This pins the two the fixture proves are right — the
    // ones actually written into `xl/_rels/workbook.xml.rels` — against the graph they resolved.
    let package = Package::open(&fixture("sample.xlsx")).expect("open");
    let workbook_part = PartName::new("/xl/workbook.xml").expect("a valid part name");
    let rels = package
        .relationships_for(Some(&workbook_part))
        .expect("the workbook's own .rels");

    for (rel_type, expected_target) in [
        (parts::REL_WORKSHEET, "worksheets/sheet1.xml"),
        (parts::REL_SHARED_STRINGS, "sharedStrings.xml"),
        (parts::REL_STYLES, "styles.xml"),
        (parts::REL_THEME, "theme/theme1.xml"),
    ] {
        let rel = rels
            .by_type(rel_type)
            .next()
            .unwrap_or_else(|| panic!("the fixture declares no {rel_type} relationship"));
        assert_eq!(rel.target, expected_target);
        assert_eq!(rel.mode, ReexportedTargetMode::Internal);
    }

    assert_eq!(
        PartKind::Workbook.relationship_type(),
        parts::REL_OFFICE_DOCUMENT
    );
    assert_eq!(
        PartKind::Worksheet.content_types(),
        [parts::CONTENT_TYPE_WORKSHEET]
    );
}
