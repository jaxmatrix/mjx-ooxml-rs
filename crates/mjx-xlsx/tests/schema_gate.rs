//! Excel's half of the ECMA-376 gate — running before a single line of `sml` model code exists.
//!
//! The `sml.xsd` arm predates this file: an authored chart embeds a whole `.xlsx` workbook, and
//! `mjx-chart` writes its SpreadsheetML. What did *not* exist was anything pointing that arm at
//! `sample.xlsx`, so nothing had ever validated a `.xlsx` the project did not itself write — and
//! two real divergences were sitting in it unnoticed. Both are now recorded as tolerated deviations
//! with their reasons, which is the difference between preserving a defect and not knowing about it.

use mjx_ooxml_core::{Interner, RawDocument, RawElement, RawName, RawNode};
use mjx_opc::{Package, PartName};
use mjx_schema_gate::{
    assert_fixture_is_schema_valid, fixture, harness, inspect_deck, inspect_fixture, outcome_table,
    package_fixtures_with_extension, PartOutcome,
};

/// The SpreadsheetML namespace, as `sml.xsd` declares it.
const SML_NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

#[test]
fn every_xlsx_fixture_is_schema_valid() {
    let fixtures = package_fixtures_with_extension("xlsx");
    assert!(
        !fixtures.is_empty(),
        "no .xlsx fixture — this case would pass vacuously"
    );
    for name in fixtures {
        assert_fixture_is_schema_valid(&name);
    }
}

#[test]
fn the_spreadsheetml_parts_are_validated_and_not_skipped() {
    // As in the Word gate: the verdict is pinned per part and names the schema. The two parts that
    // carry a producer divergence must report `Tolerated`, not `Validated` and not `Failed` —
    // tolerating an input's defect is correct, and silently skipping it is not.
    let rows = inspect_fixture("sample.xlsx");
    if rows.is_empty() {
        return;
    }
    println!("{}", outcome_table("sample.xlsx", &rows));

    for part in ["/xl/styles.xml", "/xl/worksheets/sheet1.xml"] {
        let row = rows
            .iter()
            .find(|row| row.name == part)
            .unwrap_or_else(|| panic!("sample.xlsx: {part} is not in the sweep"));
        assert_eq!(row.namespace.as_deref(), Some(SML_NS));
        assert!(
            matches!(row.outcome, PartOutcome::Validated("sml.xsd")),
            "{part} must be validated against sml.xsd; it reported: {}",
            row.outcome.describe()
        );
    }

    for (part, expected_in_reason) in [
        ("/xl/workbook.xml", "dateCompatibility"),
        ("/xl/sharedStrings.xml", "xml:space"),
    ] {
        let row = rows
            .iter()
            .find(|row| row.name == part)
            .unwrap_or_else(|| panic!("sample.xlsx: {part} is not in the sweep"));
        let PartOutcome::Tolerated { schema, reason } = &row.outcome else {
            panic!(
                "{part} carries a LibreOffice divergence and must be *tolerated* against sml.xsd \
                 with its reason; it reported: {}",
                row.outcome.describe()
            );
        };
        assert_eq!(*schema, "sml.xsd");
        assert!(
            reason.contains(expected_in_reason),
            "{part}: the tolerance must say what it tolerates; it said: {reason}"
        );
    }

    for part in ["/docProps/core.xml", "/docProps/app.xml"] {
        let row = rows
            .iter()
            .find(|row| row.name == part)
            .unwrap_or_else(|| panic!("sample.xlsx: {part} is not in the sweep"));
        assert!(
            matches!(row.outcome, PartOutcome::SkippedPreservedForeign { .. }),
            "{part} must be skipped as preserved foreign markup with its reason; it reported: {}",
            row.outcome.describe()
        );
    }
}

/// `sample.xlsx` with a `s:sheetData` nested inside `s:fileVersion` — markup `sml.xsd` rejects,
/// because `CT_FileVersion` is attribute-only and holds no child elements at all.
fn sample_xlsx_with_sheet_data_inside_file_version() -> Vec<u8> {
    let mut package = Package::open(&fixture("sample.xlsx")).expect("open sample.xlsx");
    let part = PartName::new("/xl/workbook.xml").expect("a valid part name");
    let RawDocument { interner, root, .. } =
        package.part_tree_mut(&part).expect("edit xl/workbook.xml");
    let sheet_data = RawElement::new(
        RawName {
            prefix: None,
            local: interner.intern("sheetData"),
            namespace: Some(interner.intern(SML_NS)),
        },
        Vec::new(),
        Vec::new(),
        true,
    );
    assert!(
        plant_in_first(root, interner, "fileVersion", &sheet_data),
        "sample.xlsx has no s:fileVersion to corrupt"
    );
    package.save().expect("save the corrupted workbook")
}

/// Pushes a copy of `payload` into the first SpreadsheetML element named `local`, depth first.
fn plant_in_first(
    element: &mut RawElement,
    interner: &Interner,
    local: &str,
    payload: &RawElement,
) -> bool {
    let matches_target = element
        .name
        .namespace
        .is_some_and(|ns| interner.resolve(ns) == SML_NS)
        && interner.resolve(element.name.local) == local;
    if matches_target {
        element.children.push(RawNode::Element(payload.clone()));
        return true;
    }
    for child in &mut element.children {
        if let RawNode::Element(child) = child {
            if plant_in_first(child, interner, local, payload) {
                return true;
            }
        }
    }
    false
}

#[test]
fn invalid_spreadsheetml_is_caught_and_names_the_part() {
    // The `sml` arm proved live rather than merely present, on the same terms as the `wml` one.
    // Note that the corrupted part is `/xl/workbook.xml`, which also carries a *tolerated*
    // deviation: the tolerance matches error-by-error, so a new defect in the same part still
    // fails. That is the property being demonstrated here as much as the arm itself.
    let Some(harness) = harness() else { return };
    let corrupted = sample_xlsx_with_sheet_data_inside_file_version();
    let tolerances = mjx_schema_gate::tolerances_for("sample.xlsx");
    let rows = inspect_deck(
        &harness,
        "sample.xlsx with s:sheetData inside s:fileVersion",
        &corrupted,
        &tolerances,
    );

    let row = rows
        .iter()
        .find(|row| row.name == "/xl/workbook.xml")
        .expect("xl/workbook.xml is in the sweep");
    let PartOutcome::Failed { schema, report } = &row.outcome else {
        panic!(
            "a s:sheetData inside a s:fileVersion must fail against sml.xsd; it reported: {}",
            row.outcome.describe()
        );
    };
    assert_eq!(*schema, "sml.xsd");
    assert!(
        report.contains("/xl/workbook.xml"),
        "the failure must name the part:\n{report}"
    );
    assert!(
        report.contains("fileVersion") && report.contains("content type is empty"),
        "the failure must name the element whose content model was broken:\n{report}"
    );
    // The tolerance for this same part is still in force and did *not* swallow the new defect:
    // `xl/workbook.xml`'s `dateCompatibility` line is still reported beside it. That is the
    // error-by-error match doing its job.
    assert!(
        report.contains("dateCompatibility"),
        "the tolerated deviation must still be reported when the part fails for another reason:\n\
         {report}"
    );
    println!("the sml arm, proved live:\n{report}");
}

#[test]
fn an_xlsx_the_library_re_emits_unchanged_is_still_schema_valid() {
    // `mjx-opc` rewrites the content types and every `.rels` stream on every save, so this is not a
    // no-op even for a package no `sml` code has touched. The workbook's two known divergences are
    // its own, so the fixture's tolerances travel with it.
    let package = Package::open(&fixture("sample.xlsx")).expect("open");
    let saved = package.save().expect("save");
    mjx_schema_gate::assert_deck_is_in_schema_order("saved unedited sample.xlsx", &saved);
    let Some(harness) = harness() else { return };
    let tolerances = mjx_schema_gate::tolerances_for("sample.xlsx");
    let rows = inspect_deck(&harness, "saved unedited sample.xlsx", &saved, &tolerances);
    mjx_schema_gate::assert_rows_are_valid("saved unedited sample.xlsx", &rows);
}
