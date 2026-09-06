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
    package_fixtures_with_extension, PartOutcome, PartRow,
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

/// Pushes a copy of `payload` into the first element named `(namespace, local)`, depth first.
///
/// The namespace is a parameter rather than `SML_NS` because MJXOFF-107 plants into an
/// `xdr:twoCellAnchor`, and a corrupting helper that could only reach one schema would have needed a
/// near-copy of itself to reach the second.
fn plant_in_first_of(
    element: &mut RawElement,
    interner: &Interner,
    namespace: &str,
    local: &str,
    payload: &RawElement,
) -> bool {
    let matches_target = element
        .name
        .namespace
        .is_some_and(|ns| interner.resolve(ns) == namespace)
        && interner.resolve(element.name.local) == local;
    if matches_target {
        element.children.push(RawNode::Element(payload.clone()));
        return true;
    }
    for child in &mut element.children {
        if let RawNode::Element(child) = child {
            if plant_in_first_of(child, interner, namespace, local, payload) {
                return true;
            }
        }
    }
    false
}

/// [`plant_in_first_of`] in the SpreadsheetML namespace, which is where most of this file plants.
fn plant_in_first(
    element: &mut RawElement,
    interner: &Interner,
    local: &str,
    payload: &RawElement,
) -> bool {
    plant_in_first_of(element, interner, SML_NS, local, payload)
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

/// The content types under `xl/` that carry **no XML to validate**, each with the reason it is not
/// a skip anybody should worry about.
///
/// # Why this list exists, and why it is a list rather than a wildcard
///
/// `no_part_under_xl_is_skipped_as_foreign_or_uncategorised` originally rejected *every* outcome
/// that was not `Validated` or `Tolerated`, `PartOutcome::SkippedBinary` included. That conflated
/// two categorically different things:
///
/// * a part whose **payload is not XML** — a printer-settings `DEVMODE` blob, a PNG — has nothing a
///   schema could be applied to, so skipping it is the *correct* verdict and always will be;
/// * a part whose **root namespace has no arm** in `mjx_schema_gate::categories` is the false green
///   MJXOFF-110 exists to close, and reports `SkippedPreservedForeign` or `Uncategorised`.
///
/// MJXOFF-127 (D16) hit the first case, and — rightly — put its fixture's binary part at the package
/// root rather than weaken this gate in the same commit that added the thing the gate would have
/// caught. MJXOFF-129 (D17) is the commit that draws the distinction, because a printer-settings
/// part is `xl/printerSettings/printerSettings1.bin` in every file Excel writes and there is nowhere
/// else to put it.
///
/// The widening is **narrow by construction**: a `SkippedBinary` is accepted only when its content
/// type is on this list. A content type nobody has written a reason for still fails, naming itself,
/// so the next child adding a new kind of binary part under `xl/` adds a row here and states why —
/// exactly as `mce_parts_are_skipped_with_a_named_reason` pins the MCE skips rather than allowing a
/// class of them. **MJXOFF-107 (E3)** is the next one: `xl/media/` grows when a sheet drawing lands.
///
/// Every row is proved live by `every_non_xml_content_type_on_the_allowlist_is_exercised`, so a row
/// that stops matching anything fails rather than rotting.
const NON_XML_CONTENT_TYPES_UNDER_XL: &[(&str, &str)] = &[
    (
        "application/vnd.openxmlformats-officedocument.spreadsheetml.printerSettings",
        "a printer settings part (ECMA-376 Part 1 §15.2.13), on which the specification places no \
         requirement at all. Every file this project has read carries a Windows DEVMODE blob; \
         `mjx-sml` holds the `pageSetup@r:id` that names it and never opens it",
    ),
    (
        "application/vnd.openxmlformats-officedocument.spreadsheetml.customProperty",
        "a Custom Property part (ECMA-376 Part 1 §12.3.5), whose content the specification leaves \
         entirely to the application. `mjx-sml` holds the `customPr@r:id` that names it \
         (MJXOFF-127) and nothing opens it. `hyperlinks.xlsx` carried this part at the *package \
         root* until MJXOFF-129, because this guard rejected it under `xl/`; a Custom Property \
         part's target is relative to the workbook, so `xl/` is where it belongs and where it now \
         is",
    ),
    (
        "image/png",
        "a raster image — a sheet's background picture (`CT_SheetBackgroundPicture`) and, since \
         MJXOFF-107, a picture anchored on a worksheet drawing (`xl/media/imageN.png`, named by an \
         `xdr:pic`'s `a:blip@r:embed`). `mjx-opc` stores the caller's bytes verbatim and \
         `ImageFormat::sniff` reads a magic-byte signature without decoding a pixel. **One row, two \
         kinds of part**: this list is keyed on the content type, not on where the part sits, so a \
         PNG under `xl/media/` needed no row of its own",
    ),
    (
        "image/jpeg",
        "a raster image in the other format `xl/media/` actually carries — the one MJXOFF-107 (E3) \
         did have to add, because no committed fixture held a JPEG before it. \
         `tests/fixtures/worksheet_drawings.xlsx` anchors a PNG on its two-cell and one-cell \
         anchors and a JPEG on its absolute anchor, precisely so that the media path is not \
         proved by one format and assumed for the rest. Nothing here decodes a scan line: \
         `ImageFormat::sniff` reads the `FF D8 FF` signature and stops",
    ),
];

/// The rule `no_part_under_xl_is_skipped_as_foreign_or_uncategorised` applies to one part: `Ok(())`,
/// or the reason it fails.
///
/// Extracted from the loop so that
/// [`the_rule_still_rejects_every_shape_of_false_green`] can feed it the outcomes no committed
/// fixture produces. A guard whose only witness is the corpus is a guard nobody has seen fail.
fn account_for_part_under_xl(row: &PartRow) -> Result<(), String> {
    match &row.outcome {
        PartOutcome::Validated(_) | PartOutcome::Tolerated { .. } => Ok(()),
        // The one widening, and the whole of it: a payload that is not XML, whose content type
        // somebody has written a reason for.
        PartOutcome::SkippedBinary(content_type)
            if NON_XML_CONTENT_TYPES_UNDER_XL
                .iter()
                .any(|(known, _)| known == content_type) =>
        {
            Ok(())
        }
        PartOutcome::SkippedBinary(content_type) => Err(format!(
            "{} is a non-XML part under xl/ whose content type ({content_type}) is on no list. \
             Skipping a binary payload is correct, but only once somebody has said which payload \
             and why: add a row to NON_XML_CONTENT_TYPES_UNDER_XL",
            row.name
        )),
        other => Err(format!(
            "{} is under xl/ and was not validated at all — it reported: {}",
            row.name,
            other.describe()
        )),
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
    // parts are not being validated at all. **That guard is untouched by MJXOFF-129's widening** —
    // see `account_for_part_under_xl`, which still rejects `SkippedPreservedForeign` and
    // `Uncategorised` outright, and `the_rule_still_rejects_every_shape_of_false_green`, which
    // proves it rather than asserting it.
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
            if let Err(reason) = account_for_part_under_xl(row) {
                panic!("{name}: {reason}");
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

/// The widened arm did not open the door it was widened beside.
///
/// MJXOFF-129 relaxed [`account_for_part_under_xl`] to accept a `SkippedBinary` whose content type
/// is on [`NON_XML_CONTENT_TYPES_UNDER_XL`]. This case feeds the rule the three shapes the guard
/// exists for and asserts each is still rejected — including a `SkippedBinary` whose content type is
/// on **no** list, which is the shape a careless widening (`SkippedBinary(_) => Ok(())`) would have
/// let through.
///
/// The rows are built here rather than found in the corpus, deliberately: no committed `.xlsx`
/// produces an `Uncategorised` or a `SkippedPreservedForeign` under `xl/` — that is the point of the
/// guard — so a case that only swept the corpus would prove the rule holds where it is never tested.
///
/// **MJXOFF-133 found this case one-sided and widened it.** See the comment at the top of the body:
/// every row it authored was named `drawing1.xml` in the DrawingML namespace, so a widening scoped
/// to *any* of the twelve preserved part kinds MJXOFF-133 added under `xl/` slipped past it
/// untouched. Each shape now runs in two disguises.
#[test]
fn the_rule_still_rejects_every_shape_of_false_green() {
    // Three *shapes* of row, each in **two disguises**. The disguise matters: MJXOFF-133 mutated the
    // rule to swallow anything whose part name or content type said "pivot", and every case here
    // stayed green — because every row was named `drawing1.xml` in the DrawingML namespace, so the
    // pivot-shaped arm never executed. Proved, not assumed: the arm was instrumented to panic and
    // the suite was still green. A guard whose witnesses all wear one costume tests one costume.
    //
    // MJXOFF-133 added a dozen part kinds under `xl/` — a pivot cache, an external link, a revision
    // log, an XML map. Any of them is a plausible target for the next narrow widening, so the second
    // disguise is one of them.
    const DISGUISES: &[(&str, &str, &str)] = &[
        (
            "/xl/drawings/drawing1.xml",
            "xdr:wsDr",
            "http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing",
        ),
        (
            "/xl/pivotCache/pivotCacheDefinition1.xml",
            "pivotCacheDefinition",
            "http://schemas.openxmlformats.org/spreadsheetml/2006/main",
        ),
    ];

    for (name, root_element, namespace) in DISGUISES {
        let row = |outcome| PartRow {
            name: (*name).to_owned(),
            root_element: Some((*root_element).to_owned()),
            namespace: Some((*namespace).to_owned()),
            outcome,
        };

        // 1. A namespace on no list at all — the original false green, and MJXOFF-107's (E3)
        //    namespace.
        let uncategorised = row(PartOutcome::Uncategorised {
            namespace: Some((*namespace).to_owned()),
        });
        let reason = account_for_part_under_xl(&uncategorised).expect_err("must still be rejected");
        assert!(
            reason.contains("was not validated at all") && reason.contains("UNCATEGORISED"),
            "{name}: the rejection must say what happened: {reason}"
        );

        // 2. A namespace with a *reason* but no schema arm — still a skip, still rejected under xl/.
        let foreign = row(PartOutcome::SkippedPreservedForeign {
            namespace: Some("urn:example:preserved".to_owned()),
            label: "a made-up preserved vocabulary",
            reason: "authored by this test",
        });
        assert!(
            account_for_part_under_xl(&foreign).is_err(),
            "{name}: a part skipped for want of a schema arm is the false green this rule exists to \
             catch"
        );

        // 3. A binary payload nobody has written a reason for — the shape a widening must NOT admit.
        //    Two of them: one generic, and one whose content type names a preserved cluster, so that
        //    a widening scoped to that cluster is caught rather than sailing past.
        for unexplained_content_type in [
            "application/vnd.ms-excel.something",
            "application/vnd.openxmlformats-officedocument.spreadsheetml.pivotCacheRecords",
        ] {
            let unexplained = row(PartOutcome::SkippedBinary(
                unexplained_content_type.to_owned(),
            ));
            let reason = account_for_part_under_xl(&unexplained).expect_err("must be rejected");
            assert!(
                reason.contains("NON_XML_CONTENT_TYPES_UNDER_XL"),
                "{name}: the rejection must say how to fix it: {reason}"
            );
        }

        // …and the ones the widening does admit, so this case cannot pass by rejecting everything.
        for (content_type, _) in NON_XML_CONTENT_TYPES_UNDER_XL {
            let accepted = row(PartOutcome::SkippedBinary((*content_type).to_owned()));
            assert!(
                account_for_part_under_xl(&accepted).is_ok(),
                "{content_type} is on the allowlist and must be accepted"
            );
        }
    }
}

/// Every row of [`NON_XML_CONTENT_TYPES_UNDER_XL`] matches a part in the committed corpus.
///
/// The allowlist rule MJXOFF-110 established, applied to this list: an entry is a claim that a
/// content type is really carried under `xl/` by a file this project keeps, and a claim nothing
/// witnesses is a claim that can quietly become false. `mjx_schema_gate::categories`'
/// `the_allowlist_has_no_dead_entries` is the same test for the namespace lists.
#[test]
fn every_non_xml_content_type_on_the_allowlist_is_exercised() {
    let fixtures = package_fixtures_with_extension("xlsx");
    let mut seen: Vec<&str> = Vec::new();
    for name in &fixtures {
        let rows = inspect_fixture(name);
        if rows.is_empty() {
            return;
        }
        for row in &rows {
            if !row.name.starts_with("/xl/") {
                continue;
            }
            if let PartOutcome::SkippedBinary(content_type) = &row.outcome {
                if let Some((known, _)) = NON_XML_CONTENT_TYPES_UNDER_XL
                    .iter()
                    .find(|(known, _)| known == content_type)
                {
                    if !seen.contains(known) {
                        seen.push(known);
                    }
                }
            }
        }
    }
    let dead: Vec<&str> = NON_XML_CONTENT_TYPES_UNDER_XL
        .iter()
        .map(|(content_type, _)| *content_type)
        .filter(|content_type| !seen.contains(content_type))
        .collect();
    assert!(
        dead.is_empty(),
        "no committed .xlsx carries a part under xl/ with {dead:?}; an allowlist entry nothing \
         witnesses is one that can quietly become false"
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

// -------------------------------------------------------------------------------------------
// The authored workbook (MJXOFF-112)
// -------------------------------------------------------------------------------------------

/// Every part `Workbook::blank` authors validates against the XSDs, and every one of them is in
/// `xsd:sequence` order.
///
/// This is the arm the ticket names: *"an authoring path that emits invalid markup is the exact
/// defect A1 exists to prevent, and it survived 58 releases last time."* Nothing about it is
/// specific to `blank` — the same call validates any container this library produces — but `blank`
/// is the one whose every byte the library wrote, so it is where an invalid authored part would show
/// up first.
#[test]
fn a_blank_workbook_is_schema_valid_and_in_schema_order() {
    let bytes = mjx_xlsx::Workbook::blank()
        .expect("a blank workbook is authored")
        .save()
        .expect("it saves");
    mjx_schema_gate::assert_authored_deck_is_schema_valid("Workbook::blank", &bytes);

    let Some(harness) = harness() else { return };
    let rows = inspect_deck(&harness, "Workbook::blank", &bytes, &[]);
    println!("{}", outcome_table("Workbook::blank", &rows));

    // A green run that never entered the SpreadsheetML parts would prove nothing, so the verdict is
    // pinned per part and names the schema.
    for part in [
        "/xl/workbook.xml",
        "/xl/worksheets/sheet1.xml",
        "/xl/styles.xml",
        "/xl/sharedStrings.xml",
    ] {
        let row = rows
            .iter()
            .find(|row| row.name == part)
            .unwrap_or_else(|| panic!("Workbook::blank: {part} is not in the sweep"));
        assert_eq!(row.namespace.as_deref(), Some(SML_NS), "{part}");
        assert!(
            matches!(row.outcome, PartOutcome::Validated("sml.xsd")),
            "{part} must be validated against sml.xsd; it reported: {}",
            row.outcome.describe()
        );
    }
}

/// A workbook authored from nothing, filled with every cell type and a styled cell, is still
/// schema-valid.
///
/// `blank` alone exercises the *empty* shape of every part. This exercises the populated one: a
/// `sheetData` with rows, a `dimension` with a range in it, a shared-string table with entries, and
/// a `styles.xml` with a second font, fill, border and `xf`.
#[test]
fn a_workbook_authored_from_nothing_and_filled_in_is_schema_valid() {
    use mjx_sml::write::{BorderSpec, CellFormatSpec, CellFormatTarget, PatternFillSpec};
    use mjx_sml::{CellReference, CellValue, FontProperties};

    let mut workbook = mjx_xlsx::Workbook::blank().expect("authored");
    let shared = workbook.intern_shared_string("North").expect("interns");
    let font = workbook
        .append_font(&FontProperties {
            font_name: Some("Calibri".to_owned()),
            size_in_points: Some(11.0),
            bold: Some(true),
            ..FontProperties::default()
        })
        .expect("appends");
    let fill = workbook
        .append_pattern_fill(&PatternFillSpec::solid("FFFF00"))
        .expect("appends");
    let border = workbook
        .append_border(&BorderSpec::all_edges_plain())
        .expect("appends");
    let style = workbook
        .append_cell_format(
            CellFormatTarget::CellFormats,
            &CellFormatSpec {
                font_index: Some(font),
                fill_index: Some(fill),
                border_index: Some(border),
                ..CellFormatSpec::skeleton_cell_format()
            },
        )
        .expect("appends");

    let at = |address: &str| CellReference::parse(address).expect("a literal address");
    for (address, value) in [
        ("A1", CellValue::SharedString(shared)),
        ("B1", CellValue::Number(19.25)),
        ("C1", CellValue::Boolean(false)),
        ("D1", CellValue::Error("#REF!")),
        ("E1", CellValue::InlineString("in the cell")),
    ] {
        workbook
            .set_cell_value(0, at(address), value)
            .expect("the store accepts the value");
    }
    workbook
        .set_cell_style(0, at("B1"), Some(style))
        .expect("the store accepts the style");
    workbook.add_sheet("Data").expect("a second tab");

    let bytes = workbook.save().expect("saves");
    mjx_schema_gate::assert_authored_deck_is_schema_valid("an authored workbook", &bytes);
}

/// A table this library authored is schema-valid, and the **table part itself** is validated rather
/// than skipped.
///
/// MJXOFF-125 creates the first new *part type* of Phase D, so "the package is schema-valid" is a
/// weaker claim here than usual: a table part the sweep never opened would leave the whole gate
/// green with the new markup unexamined. The per-part verdict is therefore pinned by name, which is
/// the guard MJXOFF-110 put in place for exactly this shape.
#[test]
fn an_authored_table_part_is_schema_valid_and_is_not_skipped() {
    use mjx_ooxml_types::spreadsheetml::TotalsRowFunction;
    use mjx_sml::{
        CellRange, CellReference, CellValue, TableStyleReferenceSpec, WorksheetTableSpec,
    };

    let at = |address: &str| CellReference::parse(address).expect("a literal address");
    let range = |text: &str| CellRange::parse(text).expect("a literal range");

    let mut workbook = mjx_xlsx::Workbook::blank().expect("authored");
    for (address, value) in [
        ("A1", CellValue::InlineString("Region")),
        ("B1", CellValue::InlineString("Units")),
        ("A2", CellValue::InlineString("North")),
        ("B2", CellValue::Number(1200.0)),
    ] {
        workbook
            .set_cell_value(0, at(address), value)
            .expect("the store accepts the value");
    }

    let mut spec = WorksheetTableSpec::new("Sales", range("A1:B3"), &["Region", "Units"]);
    spec.totals_row_count = 1;
    spec.style = Some(TableStyleReferenceSpec::named("TableStyleMedium2"));
    spec.columns[1].totals_row_function = Some(TotalsRowFunction::Sum);
    spec.columns[1].calculated_column_formula = Some("Sales[[#This Row],[Units]]*1".to_owned());
    workbook.add_table(0, &spec).expect("the table is created");

    let bytes = workbook.save().expect("saves");
    mjx_schema_gate::assert_authored_deck_is_schema_valid("an authored table", &bytes);

    let Some(harness) = harness() else { return };
    let rows = inspect_deck(&harness, "an authored table", &bytes, &[]);
    println!("{}", outcome_table("an authored table", &rows));
    let row = rows
        .iter()
        .find(|row| row.name == "/xl/tables/table1.xml")
        .expect("the authored table part is in the sweep");
    assert_eq!(row.namespace.as_deref(), Some(SML_NS));
    assert!(
        matches!(row.outcome, PartOutcome::Validated("sml.xsd")),
        "the table part must be validated against sml.xsd; it reported: {}",
        row.outcome.describe()
    );
}

// -------------------------------------------------------------------------------------------
// Worksheet drawings (MJXOFF-107) — both halves of the `xdr` gate, proved live
// -------------------------------------------------------------------------------------------

/// The SpreadsheetDrawingML namespace, as `dml-spreadsheetDrawing.xsd` declares it.
const XDR_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing";

/// A workbook this library authored, carrying one anchor of each of the three kinds.
fn an_authored_drawing() -> Vec<u8> {
    use mjx_dml::spreadsheet_drawing::CellMarker;
    use mjx_dml::{Position, Size};
    use mjx_ooxml_types::spreadsheetdrawing::ResizingBehavior;

    const PNG: &[u8] = &[
        0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, b'I', b'H', b'D',
        b'R', 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90,
        0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, b'I', b'D', b'A', b'T', 0x08, 0xD7, 0x63, 0xF8,
        0xCF, 0xC0, 0x00, 0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB0, 0x00, 0x00, 0x00,
        0x00, b'I', b'E', b'N', b'D', 0xAE, 0x42, 0x60, 0x82,
    ];

    let mut workbook = mjx_xlsx::Workbook::blank().expect("a blank workbook");
    workbook
        .add_two_cell_anchored_picture(
            0,
            PNG,
            "two-cell",
            CellMarker::new(1, 190_500, 2, 47_625),
            CellMarker::new(3, 95_250, 5, 19_050),
            ResizingBehavior::MoveWithCellsButDoNotResize,
        )
        .expect("added");
    workbook
        .add_one_cell_anchored_picture(
            0,
            PNG,
            "one-cell",
            CellMarker::new(4, 76_200, 1, 38_100),
            Size::from_emu(914_400, 457_200),
        )
        .expect("added");
    workbook
        .add_absolute_anchored_picture(
            0,
            PNG,
            "absolute",
            Position::from_emu(1_905_000, 952_500),
            Size::from_emu(685_800, 342_900),
        )
        .expect("added");
    workbook.save().expect("it validates and saves")
}

#[test]
fn an_authored_worksheet_drawing_is_schema_valid_under_the_xdr_arm() {
    // The first half of MJXOFF-107's gate: markup this library wrote, validated against the schema
    // the new `MODELED_SCHEMAS` row names — not merely "not skipped".
    let bytes = an_authored_drawing();
    mjx_schema_gate::assert_authored_deck_is_schema_valid("an authored drawing", &bytes);

    let Some(harness) = harness() else { return };
    let rows = inspect_deck(&harness, "an authored drawing", &bytes, &[]);
    println!("{}", outcome_table("an authored drawing", &rows));
    let row = rows
        .iter()
        .find(|row| row.name == "/xl/drawings/drawing1.xml")
        .expect("the authored drawing part is in the sweep");
    assert_eq!(row.namespace.as_deref(), Some(XDR_NS));
    assert!(
        matches!(
            row.outcome,
            PartOutcome::Validated("dml-spreadsheetDrawing.xsd")
        ),
        "the drawing part must be validated against dml-spreadsheetDrawing.xsd; it reported: {}",
        row.outcome.describe()
    );
}

/// The DrawingML-chart namespace, as `dml-chart.xsd` declares it.
const CHART_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/chart";

/// A workbook with two authored charts on one sheet — one with an embedded workbook, one over a
/// live range (MJXOFF-111, E4).
///
/// Both, deliberately. They are different markup: the first writes a `c:externalData` and a whole
/// `.xlsx` beside it, the second writes `c:f` formulas naming this workbook's own cells and no
/// `c:externalData` at all. A gate that validated one would say nothing about the other.
fn a_workbook_with_authored_charts() -> Vec<u8> {
    use mjx_chart::{ChartData, ChartKind, LegendPosition};
    use mjx_dml::spreadsheet_drawing::CellMarker;
    use mjx_ooxml_types::spreadsheetdrawing::ResizingBehavior;
    use mjx_sml::{CellReference, CellValue};
    use mjx_xlsx::{SheetChartSeries, SheetChartSource};

    let mut workbook = mjx_xlsx::Workbook::blank().expect("a blank workbook");
    workbook.rename_sheet(0, "Data").expect("renamed");
    for (address, value) in [("A1", 10.0), ("A2", 20.0), ("A3", 30.0)] {
        workbook
            .set_cell_value(
                0,
                CellReference::parse(address).expect("a literal address"),
                CellValue::Number(value),
            )
            .expect("the store accepts the value");
    }

    let chart = ChartData::new(ChartKind::Bar)
        .categories(["Q1", "Q2", "Q3"])
        .series("Revenue", [10.0, 20.0, 30.0])
        .title("Quarterly revenue")
        .legend(LegendPosition::Bottom);
    workbook
        .add_chart(
            0,
            &chart,
            CellMarker::new(2, 0, 1, 0),
            CellMarker::new(8, 0, 15, 0),
            "Embedded",
            ResizingBehavior::MoveWithCellsButDoNotResize,
        )
        .expect("a chart with an embedded workbook");

    let source = SheetChartSource {
        categories: None,
        series: vec![SheetChartSeries {
            name_cell: None,
            name: "Live".to_owned(),
            values: "Data!$A$1:$A$3".to_owned(),
        }],
    };
    workbook
        .add_range_chart(
            0,
            ChartKind::Line,
            &source,
            CellMarker::new(2, 0, 17, 0),
            CellMarker::new(8, 0, 31, 0),
            "Live",
            ResizingBehavior::MoveWithCellsButDoNotResize,
        )
        .expect("a live-range chart");

    workbook.save().expect("it validates and saves")
}

#[test]
fn the_authored_charts_and_the_frames_that_hold_them_are_schema_valid() {
    // MJXOFF-111's own *Done when* clause: the authored workbook is schema-valid **including the
    // chart part and the drawing part**. Both are named by hand rather than left to the sweep,
    // because a chart part that were skipped as foreign would satisfy "the workbook is valid" while
    // proving nothing about the markup this child writes.
    let bytes = a_workbook_with_authored_charts();
    mjx_schema_gate::assert_authored_deck_is_schema_valid("authored charts", &bytes);

    let Some(harness) = harness() else { return };
    let rows = inspect_deck(&harness, "authored charts", &bytes, &[]);
    println!("{}", outcome_table("authored charts", &rows));

    for part in ["/xl/charts/chart1.xml", "/xl/charts/chart2.xml"] {
        let row = rows
            .iter()
            .find(|row| row.name == part)
            .unwrap_or_else(|| panic!("{part} is not in the sweep"));
        assert_eq!(row.namespace.as_deref(), Some(CHART_NS));
        assert!(
            matches!(row.outcome, PartOutcome::Validated("dml-chart.xsd")),
            "{part} must be validated against dml-chart.xsd; it reported: {}",
            row.outcome.describe()
        );
    }
    let drawing = rows
        .iter()
        .find(|row| row.name == "/xl/drawings/drawing1.xml")
        .expect("the drawing part that frames both charts is in the sweep");
    assert_eq!(drawing.namespace.as_deref(), Some(XDR_NS));
    assert!(
        matches!(
            drawing.outcome,
            PartOutcome::Validated("dml-spreadsheetDrawing.xsd")
        ),
        "the frames must be validated against dml-spreadsheetDrawing.xsd; it reported: {}",
        drawing.outcome.describe()
    );
}

/// The authored workbook with a `c:ser` planted directly inside `c:chartSpace` — a series where
/// `CT_ChartSpace`'s own `xsd:sequence` allows only `c:date1904`, `c:lang`, `c:roundedCorners`,
/// `c:style`, `c:clrMapOvr`, `c:pivotSource`, `c:protection`, `c:chart`, `c:spPr`, `c:txPr`,
/// `c:externalData`, `c:printSettings` and `c:userShapes`.
fn a_chart_with_a_series_outside_its_plot() -> Vec<u8> {
    let mut package = Package::open(&a_workbook_with_authored_charts()).expect("open");
    let part = PartName::new("/xl/charts/chart1.xml").expect("a valid part name");
    let RawDocument { interner, root, .. } = package.part_tree_mut(&part).expect("edit the chart");
    let stray = RawElement::new(
        RawName {
            prefix: Some(interner.intern("c")),
            local: interner.intern("ser"),
            namespace: Some(interner.intern(CHART_NS)),
        },
        Vec::new(),
        Vec::new(),
        true,
    );
    root.children.push(RawNode::Element(stray));
    root.empty = false;
    package.save().expect("save the corrupted chart")
}

/// The authored workbook with an `xdr:graphicFrame` stripped of its required `xdr:xfrm`.
///
/// The trap MJXOFF-111 found in the schema and had to write around:
/// `CT_GraphicalObjectFrame` declares `xfrm` `minOccurs="1"`, unlike the `a:xfrm` a picture may
/// omit, so a frame that leaves it out is invalid however it is anchored — and a `twoCellAnchor`'s
/// markers make the transform look redundant, which is exactly why it would be dropped by accident.
fn a_chart_frame_without_its_required_transform() -> Vec<u8> {
    let mut package = Package::open(&a_workbook_with_authored_charts()).expect("open");
    let part = PartName::new("/xl/drawings/drawing1.xml").expect("a valid part name");
    let RawDocument { interner, root, .. } =
        package.part_tree_mut(&part).expect("edit the drawing");
    let removed = remove_first_xfrm(root, interner);
    assert!(removed, "the drawing has no xdr:xfrm to remove");
    package.save().expect("save the corrupted drawing")
}

/// Removes the first `xdr:xfrm` found anywhere under `element`, depth first. Answers whether one
/// went.
fn remove_first_xfrm(element: &mut RawElement, interner: &mut Interner) -> bool {
    let at = element.children.iter().position(|node| {
        matches!(node, RawNode::Element(child)
            if child.name.namespace.map(|ns| interner.resolve(ns)) == Some(XDR_NS)
                && interner.resolve(child.name.local) == "xfrm")
    });
    if let Some(at) = at {
        element.children.remove(at);
        return true;
    }
    for node in &mut element.children {
        if let RawNode::Element(child) = node {
            if remove_first_xfrm(child, interner) {
                return true;
            }
        }
    }
    false
}

#[test]
fn invalid_chart_markup_is_caught_and_names_the_chart_part() {
    // The `dml-chart` arm, proved live on a `.xlsx` rather than assumed from PowerPoint's use of it:
    // markup the schema rejects turns this case red, and the failure names the part.
    let Some(harness) = harness() else { return };
    let corrupted = a_chart_with_a_series_outside_its_plot();
    let rows = inspect_deck(&harness, "a chart with a stray c:ser", &corrupted, &[]);

    let row = rows
        .iter()
        .find(|row| row.name == "/xl/charts/chart1.xml")
        .expect("the chart part is in the sweep");
    let PartOutcome::Failed { schema, report } = &row.outcome else {
        panic!(
            "a stray c:ser must fail against dml-chart.xsd; it reported: {}",
            row.outcome.describe()
        );
    };
    assert_eq!(*schema, "dml-chart.xsd");
    assert!(
        report.contains("/xl/charts/chart1.xml"),
        "the failure must name the part:\n{report}"
    );
    assert!(
        report.contains("ser"),
        "the failure must name the element that broke the sequence:\n{report}"
    );

    // The discriminating half: only that part broke. The *other* chart and the drawing that frames
    // both are still valid, so this cannot pass because the corruption broke everything.
    for (part, schema) in [
        ("/xl/charts/chart2.xml", "dml-chart.xsd"),
        ("/xl/drawings/drawing1.xml", "dml-spreadsheetDrawing.xsd"),
    ] {
        let other = rows
            .iter()
            .find(|row| row.name == part)
            .unwrap_or_else(|| panic!("{part} is not in the sweep"));
        assert!(
            matches!(other.outcome, PartOutcome::Validated(s) if s == schema),
            "{part} must be unaffected; it reported: {}",
            other.outcome.describe()
        );
    }
    println!("the dml-chart arm, proved live:\n{report}");
}

#[test]
fn a_chart_frame_missing_its_required_transform_is_caught_and_names_the_drawing_part() {
    // The other half of MJXOFF-111's schema clause, on the *drawing* part rather than the chart:
    // `xdr:xfrm` is `minOccurs="1"` inside `CT_GraphicalObjectFrame`, and a frame written without
    // one is markup `dml-spreadsheetDrawing.xsd` rejects. This is the mutation that proves the
    // authored frame's transform is load-bearing rather than decorative.
    let Some(harness) = harness() else { return };
    let corrupted = a_chart_frame_without_its_required_transform();
    let rows = inspect_deck(&harness, "a chart frame with no xdr:xfrm", &corrupted, &[]);

    let row = rows
        .iter()
        .find(|row| row.name == "/xl/drawings/drawing1.xml")
        .expect("the drawing part is in the sweep");
    let PartOutcome::Failed { schema, report } = &row.outcome else {
        panic!(
            "a graphicFrame with no xfrm must fail against dml-spreadsheetDrawing.xsd; it \
             reported: {}",
            row.outcome.describe()
        );
    };
    assert_eq!(*schema, "dml-spreadsheetDrawing.xsd");
    assert!(
        report.contains("/xl/drawings/drawing1.xml"),
        "the failure must name the part:\n{report}"
    );
    assert!(
        report.contains("xfrm") || report.contains("graphic"),
        "the failure must name what the sequence was missing:\n{report}"
    );

    // …and both chart parts are untouched, so the corruption really was local to the frame.
    for part in ["/xl/charts/chart1.xml", "/xl/charts/chart2.xml"] {
        let other = rows
            .iter()
            .find(|row| row.name == part)
            .unwrap_or_else(|| panic!("{part} is not in the sweep"));
        assert!(
            matches!(other.outcome, PartOutcome::Validated("dml-chart.xsd")),
            "{part} must be unaffected; it reported: {}",
            other.outcome.describe()
        );
    }
    println!("the required-transform rule, proved live:\n{report}");
}

/// `worksheet_drawings.xlsx` with an `xdr:col` planted directly inside `xdr:twoCellAnchor` — a
/// marker child where the anchor's own `xsd:sequence` allows only `from`, `to`, one object and
/// `clientData`.
fn a_drawing_with_a_marker_child_outside_its_marker() -> Vec<u8> {
    let mut package = Package::open(&fixture("worksheet_drawings.xlsx")).expect("open");
    let part = PartName::new("/xl/drawings/drawing1.xml").expect("a valid part name");
    let RawDocument { interner, root, .. } =
        package.part_tree_mut(&part).expect("edit the drawing");
    let stray = RawElement::new(
        RawName {
            prefix: Some(interner.intern("xdr")),
            local: interner.intern("col"),
            namespace: Some(interner.intern(XDR_NS)),
        },
        Vec::new(),
        Vec::new(),
        true,
    );
    assert!(
        plant_in_first_of(root, interner, XDR_NS, "twoCellAnchor", &stray),
        "the fixture has no xdr:twoCellAnchor to corrupt"
    );
    package.save().expect("save the corrupted drawing")
}

#[test]
fn invalid_drawing_markup_is_caught_and_names_the_drawing_part() {
    // The `xdr` arm, proved live rather than assumed: markup the schema rejects turns a case red,
    // and the failure names the part and the element whose content model was broken.
    let Some(harness) = harness() else { return };
    let corrupted = a_drawing_with_a_marker_child_outside_its_marker();
    let rows = inspect_deck(
        &harness,
        "worksheet_drawings.xlsx with a stray xdr:col",
        &corrupted,
        &[],
    );

    let row = rows
        .iter()
        .find(|row| row.name == "/xl/drawings/drawing1.xml")
        .expect("the drawing part is in the sweep");
    let PartOutcome::Failed { schema, report } = &row.outcome else {
        panic!(
            "a stray xdr:col must fail against dml-spreadsheetDrawing.xsd; it reported: {}",
            row.outcome.describe()
        );
    };
    assert_eq!(*schema, "dml-spreadsheetDrawing.xsd");
    assert!(
        report.contains("/xl/drawings/drawing1.xml"),
        "the failure must name the part:\n{report}"
    );
    assert!(
        report.contains("col"),
        "the failure must name the element that broke the sequence:\n{report}"
    );

    // The discriminating half: only that part broke. The worksheet beside it is still valid, so this
    // case cannot pass because the corruption happened to break everything.
    let worksheet = rows
        .iter()
        .find(|row| row.name == "/xl/worksheets/sheet1.xml")
        .expect("the worksheet is in the sweep");
    assert!(
        matches!(worksheet.outcome, PartOutcome::Validated("sml.xsd")),
        "/xl/worksheets/sheet1.xml must be unaffected; it reported: {}",
        worksheet.outcome.describe()
    );
    println!("the xdr arm, proved live:\n{report}");
}

#[test]
fn the_generated_xdr_table_is_what_puts_the_drawing_part_under_the_ordering_gate() {
    // The **second** half of the gate, and the one a schema arm alone leaves open: a table nothing
    // reads passes every test there is. Asserted from both ends — the category table says the part
    // is *required* to be audited (which reads `OrderingCoverage::Generated`, itself checked against
    // the real tables by `the_ordering_gaps_are_exactly_the_declared_ones`), and the walk says it
    // *was*, having descended into real structure rather than recognising a root and none of its
    // children.
    //
    // Drop `"dml-spreadsheetDrawing"` from `CHILD_ORDER_SCHEMAS` and both halves go red here, on top
    // of the two reconciliation cases in `mjx-schema-gate` and the hard codegen error the
    // `TWO_CELL_ANCHOR` export raises.
    let package = Package::open(&fixture("worksheet_drawings.xlsx")).expect("open");
    let saved = package.save().expect("save");

    let required = mjx_schema_gate::parts_that_must_be_audited("worksheet_drawings.xlsx", &saved);
    assert!(
        required
            .iter()
            .any(|name| name == "/xl/drawings/drawing1.xml"),
        "the drawing is rooted in SpreadsheetDrawingML, so the category table must require it to be \
         audited; it required {required:?}"
    );

    let audited = mjx_schema_gate::audit_deck_order("worksheet_drawings.xlsx", &saved);
    let entry = audited
        .iter()
        .find(|entry| entry.name == "/xl/drawings/drawing1.xml")
        .expect("the drawing was required but the ordering walk did not audit it");
    println!(
        "/xl/drawings/drawing1.xml — elements_visited = {}, root_child_elements = {}, floor = {}",
        entry.elements_visited,
        entry.root_child_elements,
        entry.floor()
    );
    assert!(
        entry.elements_visited >= mjx_schema_gate::MINIMUM_ELEMENTS_VISITED,
        "the drawing part visited only {} element(s); the tables knew its root and recognised none \
         of its children, which is a vacuous audit",
        entry.elements_visited
    );
    assert!(
        entry.elements_visited > entry.floor(),
        "elements_visited {} is not past the floor {}",
        entry.elements_visited,
        entry.floor()
    );
}

#[test]
fn an_out_of_sequence_anchor_child_turns_the_ordering_audit_red() {
    // The ordering table made load-bearing, not decorative. `CT_TwoCellAnchor`'s sequence is `from`,
    // `to`, the object, then `clientData`; this moves `clientData` to the front, which no schema
    // *validator* would be needed to catch — the audit alone must.
    let mut package = Package::open(&fixture("worksheet_drawings.xlsx")).expect("open");
    let part = PartName::new("/xl/drawings/drawing1.xml").expect("a valid part name");
    {
        let RawDocument { interner, root, .. } =
            package.part_tree_mut(&part).expect("edit the drawing");
        let anchor = root
            .children
            .iter_mut()
            .find_map(|node| match node {
                RawNode::Element(child)
                    if interner.resolve(child.name.local) == "twoCellAnchor" =>
                {
                    Some(child)
                }
                _ => None,
            })
            .expect("the fixture has an xdr:twoCellAnchor");
        let at = anchor
            .children
            .iter()
            .position(|node| match node {
                RawNode::Element(child) => interner.resolve(child.name.local) == "clientData",
                _ => false,
            })
            .expect("the anchor has an xdr:clientData");
        let client_data = anchor.children.remove(at);
        anchor.children.insert(0, client_data);
    }
    let saved = package.save().expect("save the reordered drawing");

    // `audit_deck_order` panics on a defect, so the red is caught and read rather than asserted
    // around. The default hook is silenced first: this panic is the expected result, and letting it
    // print would make a passing run look like a failing one.
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let outcome =
        std::panic::catch_unwind(|| mjx_schema_gate::audit_deck_order("reordered drawing", &saved));
    std::panic::set_hook(previous);

    let payload = outcome.expect_err("an out-of-sequence anchor child must turn the audit red");
    let message = payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&str>().copied())
        .unwrap_or("<non-string panic>")
        .to_owned();
    assert!(
        message.contains("/xl/drawings/drawing1.xml"),
        "the audit must name the part: {message}"
    );
    assert!(
        message.contains("CT_TwoCellAnchor") && message.contains("clientData"),
        "the audit must name the type whose sequence was broken and the child that broke it: \
         {message}"
    );
    println!("the xdr ordering table, proved load-bearing:\n{message}");
}
