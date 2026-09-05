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
            .unwrap_or_else(|| panic!("sample.xlsx: {part} is not in the sweep"));
        assert!(
            matches!(row.outcome, PartOutcome::Validated(s) if s == schema),
            "{part} must be validated against {schema}; it reported: {}",
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

/// The `sml` child-order table is what puts the SpreadsheetML parts under the ordering gate
/// (MJXOFF-132).
///
/// Before this child, `assert_deck_is_in_schema_order` on a `.xlsx` was very nearly the vacuous pass
/// `mjx_schema_gate::order`'s own module doc warns about: `sml` had no generated table, so the only
/// part of `sample.xlsx` the walk recognised was `/xl/theme/theme1.xml` — a DrawingML part that
/// happens to live in a workbook. Every `x:`-rooted part was invisible to it, and the case was green
/// anyway.
///
/// So this asserts the fact the ticket's own "the row must be load-bearing" clause is about, and
/// asserts it from **both** ends: the category table says these parts are *required* to be audited
/// (which reads `OrderingCoverage::Generated`), and the walk says they *were* audited, each having
/// descended into real structure rather than recognising a root and none of its children. Drop
/// `"sml"` from `CHILD_ORDER_SCHEMAS` and both halves go red here, on top of the two reconciliation
/// cases in `mjx-schema-gate` and the hard codegen error the `WORKSHEET` export raises.
#[test]
fn the_generated_sml_table_is_what_puts_the_worksheet_parts_under_the_ordering_gate() {
    // The four SpreadsheetML parts of `sample.xlsx`. `/xl/theme/theme1.xml` is deliberately not in
    // this list: it is the part that was already audited, and the one that made the old assertion
    // look like it covered a workbook.
    const SPREADSHEETML_PARTS: &[&str] = &[
        "/xl/workbook.xml",
        "/xl/worksheets/sheet1.xml",
        "/xl/sharedStrings.xml",
        "/xl/styles.xml",
    ];

    let package = Package::open(&fixture("sample.xlsx")).expect("open sample.xlsx");
    let saved = package.save().expect("save sample.xlsx");

    let required = mjx_schema_gate::parts_that_must_be_audited("sample.xlsx", &saved);
    let audited = mjx_schema_gate::audit_deck_order("sample.xlsx", &saved);

    for part in SPREADSHEETML_PARTS {
        assert!(
            required.iter().any(|name| name == part),
            "{part} is rooted in SpreadsheetML, so the category table must require it to be \
             audited; it required {required:?}"
        );
        let entry = audited
            .iter()
            .find(|entry| entry.name == *part)
            .unwrap_or_else(|| {
                panic!("{part} was required but the ordering walk did not audit it")
            });
        assert!(
            entry.elements_visited >= mjx_schema_gate::MINIMUM_ELEMENTS_VISITED,
            "{part} visited only {} element(s); the tables knew its root and recognised none of \
             its children, which is a vacuous audit",
            entry.elements_visited
        );
    }

    println!("the sml ordering table, proved live: {audited:#?}");
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

/// `sample.xlsx` with a `s:c` planted directly inside `s:sheetData` — a cell outside its row, which
/// `CT_SheetData` rejects: its `xsd:sequence` holds exactly one element, `row`.
///
/// MJXOFF-91's own mutation, and deliberately in a *different part* from
/// [`sample_xlsx_with_sheet_data_inside_file_version`]'s. That case proves the `sml` arm reaches
/// `/xl/workbook.xml`; a part being validated is a property of the part, not of the schema, and
/// `/xl/worksheets/sheet1.xml` is the part every later Phase D child actually writes into.
fn sample_xlsx_with_a_cell_outside_its_row() -> Vec<u8> {
    let mut package = Package::open(&fixture("sample.xlsx")).expect("open sample.xlsx");
    let part = PartName::new("/xl/worksheets/sheet1.xml").expect("a valid part name");
    let RawDocument { interner, root, .. } = package
        .part_tree_mut(&part)
        .expect("edit xl/worksheets/sheet1.xml");
    let cell = RawElement::new(
        RawName {
            prefix: None,
            local: interner.intern("c"),
            namespace: Some(interner.intern(SML_NS)),
        },
        Vec::new(),
        Vec::new(),
        true,
    );
    assert!(
        plant_in_first(root, interner, "sheetData", &cell),
        "sample.xlsx's worksheet has no s:sheetData to corrupt"
    );
    package.save().expect("save the corrupted worksheet")
}

#[test]
fn invalid_worksheet_markup_is_caught_and_names_the_worksheet_part() {
    let Some(harness) = harness() else { return };
    let corrupted = sample_xlsx_with_a_cell_outside_its_row();
    let tolerances = mjx_schema_gate::tolerances_for("sample.xlsx");
    let rows = inspect_deck(
        &harness,
        "sample.xlsx with a s:c outside its s:row",
        &corrupted,
        &tolerances,
    );

    let row = rows
        .iter()
        .find(|row| row.name == "/xl/worksheets/sheet1.xml")
        .expect("xl/worksheets/sheet1.xml is in the sweep");
    let PartOutcome::Failed { schema, report } = &row.outcome else {
        panic!(
            "a s:c outside its s:row must fail against sml.xsd; it reported: {}",
            row.outcome.describe()
        );
    };
    assert_eq!(*schema, "sml.xsd");
    assert!(
        report.contains("/xl/worksheets/sheet1.xml"),
        "the failure must name the part:\n{report}"
    );
    assert!(
        report.contains("sheetData") || report.contains("}c'"),
        "the failure must name the element whose content model was broken:\n{report}"
    );

    // The discriminating half: only that part broke. `/xl/workbook.xml`'s own tolerated deviation is
    // still tolerated, and `/xl/styles.xml` is still clean — so this case cannot pass because the
    // corruption happened to break everything.
    let styles = rows
        .iter()
        .find(|row| row.name == "/xl/styles.xml")
        .expect("xl/styles.xml is in the sweep");
    assert!(
        matches!(styles.outcome, PartOutcome::Validated("sml.xsd")),
        "/xl/styles.xml must be unaffected; it reported: {}",
        styles.outcome.describe()
    );
    println!("the sml arm on the worksheet part, proved live:\n{report}");
}

/// **MJXOFF-102's ordering gate, reached rather than assumed.**
///
/// The extracted harness (`mjx_schema_gate::audit_deck_order`) is run over a workbook this crate has
/// *edited*, so the worksheet the walk audits is markup this library wrote out rather than markup it
/// merely copied. Three separate facts are asserted, and the third is the one that stops the case
/// passing vacuously:
///
/// 1. the category table **requires** `/xl/worksheets/sheet1.xml` to be audited;
/// 2. the walk **did** audit it, with no ordering defect;
/// 3. its `elements_visited` count is well past the floor — a count of zero, or the part missing
///    from the audited list, would mean the walk never entered the worksheet and the mutation gate
///    below would prove nothing.
///
/// The count is printed, because MJXOFF-102's report has to quote it.
#[test]
fn the_edited_worksheet_is_reached_by_the_order_audit_and_the_count_is_quoted() {
    for name in ["sample.xlsx", "worksheet_spine.xlsx"] {
        let mut workbook = mjx_xlsx::Workbook::open(&fixture(name)).expect("open");
        workbook
            .set_cell_value(
                0,
                mjx_sml::CellReference::parse("B2").expect("B2"),
                mjx_sml::CellValue::Number(7.5),
            )
            .expect("B2 is inside the grid");
        let saved = workbook.save().expect("save");
        let label = format!("{name} with one cell edited");

        let required = mjx_schema_gate::parts_that_must_be_audited(&label, &saved);
        assert!(
            required.iter().any(|part| part == "/xl/worksheets/sheet1.xml"),
            "the worksheet is rooted in SpreadsheetML, so the category table must require it to be \
             audited; it required {required:?}"
        );

        let audited = mjx_schema_gate::audit_deck_order(&label, &saved);
        let worksheet = audited
            .iter()
            .find(|part| part.name == "/xl/worksheets/sheet1.xml")
            .expect("the worksheet was required but the ordering walk did not audit it");
        println!(
            "{label}: /xl/worksheets/sheet1.xml — elements_visited = {}, root_child_elements = {}, \
             floor = {}",
            worksheet.elements_visited,
            worksheet.root_child_elements,
            worksheet.floor()
        );
        assert!(
            worksheet.elements_visited > worksheet.floor(),
            "{label}: the worksheet audit visited {} element(s) against a floor of {} — a walk that \
             recognised the root and none of its structure proves nothing",
            worksheet.elements_visited,
            worksheet.floor()
        );

        // …and the edited workbook is still schema-valid, so the ordering walk is not the only thing
        // watching this part.
        mjx_schema_gate::assert_deck_is_in_schema_order(&label, &saved);
        let Some(harness) = harness() else { continue };
        let tolerances = mjx_schema_gate::tolerances_for(name);
        let rows = inspect_deck(&harness, &label, &saved, &tolerances);
        mjx_schema_gate::assert_rows_are_valid(&label, &rows);
    }
}

/// The worksheet spine fixture's own parts are validated, `/xl/tables/table1.xml` included.
///
/// It is the first committed `.xlsx` to carry a part under `xl/` that is not one of the four
/// `sample.xlsx` has, and the first with a worksheet-level `.rels`. Both are the kind of thing that
/// joins a sweep as a *skip* if nobody looks, which is the false green MJXOFF-110 exists to close.
#[test]
fn the_worksheet_spine_fixtures_parts_are_all_validated() {
    let rows = inspect_fixture("worksheet_spine.xlsx");
    if rows.is_empty() {
        return;
    }
    println!("{}", outcome_table("worksheet_spine.xlsx", &rows));

    for expected in [
        "/xl/workbook.xml",
        "/xl/worksheets/sheet1.xml",
        "/xl/tables/table1.xml",
        "/xl/styles.xml",
    ] {
        let row = rows
            .iter()
            .find(|row| row.name == expected)
            .unwrap_or_else(|| panic!("{expected} is not in the sweep at all"));
        assert!(
            matches!(row.outcome, PartOutcome::Validated("sml.xsd")),
            "{expected} must be validated against sml.xsd; it reported: {}",
            row.outcome.describe()
        );
    }
}

#[test]
fn no_part_under_xl_is_skipped_as_foreign_or_uncategorised() {
    // MJXOFF-91's schema clause in its general form. `the_spreadsheetml_parts_are_validated_and_not_skipped`
    // pins four parts by name; this pins the *rule* those four are instances of, so a part a later
    // Phase D child adds under `xl/` cannot quietly join the sweep as a skip.
    //
    // This is the exact false-green MJXOFF-110 exists to close: a part in a namespace with no arm
    // reports a *skip*, and `assert_outcomes_are_valid` fails on neither a skip nor a tolerance. So
    // "schema validity covers the .xlsx fixtures and is green" is satisfied precisely when the Excel
    // parts are not being validated at all.
    // Swept over **every** committed `.xlsx`, not over `sample.xlsx` alone (MJXOFF-102). A later
    // child adding a fixture with a new kind of part under `xl/` is exactly the case this rule is
    // for, and pinning one fixture would have let `worksheet_spine.xlsx`'s `/xl/tables/table1.xml`
    // join the sweep as a skip.
    let fixtures = package_fixtures_with_extension("xlsx");
    assert!(!fixtures.is_empty(), "no .xlsx fixture to sweep");
    let mut checked = 0usize;
    for name in &fixtures {
        let rows = inspect_fixture(name);
        if rows.is_empty() {
            return;
        }
        println!("{}", outcome_table(name, &rows));
        for row in &rows {
            if !row.name.starts_with("/xl/") || row.name.ends_with(".rels") {
                continue;
            }
            checked += 1;
            match &row.outcome {
                PartOutcome::Validated(_) | PartOutcome::Tolerated { .. } => {}
                other => panic!(
                    "{name}: {} is under xl/ and was not validated at all — it reported: {}",
                    row.name,
                    other.describe()
                ),
            }
        }
    }
    assert!(
        checked >= 12,
        "only {checked} part(s) under xl/ were checked across {} fixture(s); sample.xlsx alone \
         carries five",
        fixtures.len()
    );
}

#[test]
fn a_workbook_opened_and_saved_through_this_crate_is_still_schema_valid() {
    // The gate applied to *this crate's* entry point rather than to `mjx-opc`'s.
    // `an_xlsx_the_library_re_emits_unchanged_is_still_schema_valid` proves the container layer does
    // not corrupt a workbook; this proves `Workbook::open`/`Workbook::save` — which parse
    // `xl/workbook.xml`, resolve the whole part graph and run the SpreadsheetML validator on the way
    // out — do not either.
    let workbook = mjx_xlsx::Workbook::open(&fixture("sample.xlsx")).expect("open");
    let saved = workbook.save().expect("save");
    mjx_schema_gate::assert_deck_is_in_schema_order("sample.xlsx through Workbook", &saved);
    let Some(harness) = harness() else { return };
    let tolerances = mjx_schema_gate::tolerances_for("sample.xlsx");
    let rows = inspect_deck(
        &harness,
        "sample.xlsx through Workbook",
        &saved,
        &tolerances,
    );
    mjx_schema_gate::assert_rows_are_valid("sample.xlsx through Workbook", &rows);
    println!("{}", outcome_table("sample.xlsx through Workbook", &rows));
}
