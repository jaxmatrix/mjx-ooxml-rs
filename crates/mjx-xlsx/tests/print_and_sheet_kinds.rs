//! **MJXOFF-129's package gate.** The four sheet kinds at the package tier, the two relationships a
//! print block reaches, and the binary parts at the end of them.
//!
//! `crates/mjx-sml/tests/print_and_sheet_kinds.rs` is the markup half — what a `pageSetup` *says*.
//! This is the package half: which part in this container each `r:id` reaches, that the binary parts
//! survive untouched, and that a reference pointing at the wrong kind of relationship is a defect
//! rather than a silently wrong answer.
//!
//! # The fixture, and the one shape that is assembled here instead
//!
//! `tests/fixtures/print_and_sheet_kinds.xlsx` carries a **worksheet** and a **dialogsheet**, two
//! printer-settings blobs and a background image. It does not carry a chartsheet, and the reason is
//! the schema gate rather than this suite: a legal `CT_Chartsheet` declares `drawing`
//! `minOccurs="1"` (`sml.xsd:2965`), so a schema-valid chartsheet drags a `dml-spreadsheetDrawing`
//! part into the corpus — and that namespace has **no arm** in `mjx_schema_gate::categories`, whose
//! owner is MJXOFF-107 (E3). A committed fixture carrying one would report `UNCATEGORISED` and fail
//! the gate outright.
//!
//! So [`a_workbook_of_three_sheet_kinds_opens_reports_and_round_trips`] assembles that package here,
//! exactly as `crates/mjx-xlsx/tests/worksheet_part.rs` already does for the same reason, and
//! asserts the three-kinds clause against it.
//!
//! # Nothing here opens a binary part
//!
//! A printer-settings blob is a Windows `DEVMODE` ECMA-376 Part 1 §15.2.13 places no requirement on;
//! an image is bytes `mjx-opc` stores verbatim. Both are asserted **byte for byte** and neither is
//! interpreted.

use mjx_opc::{Package, PartName, Relationship, TargetMode};
use mjx_sml::CellReference;
use mjx_sml::{CellRange, CellValue, HeaderFooterSlot, WorksheetTableSpec};
use mjx_xlsx::{HyperlinkTarget, SheetKind, SheetMarkup, SpreadsheetDefect, Workbook, XlsxError};

/// The fixture this suite is written against.
const FIXTURE: &str = "print_and_sheet_kinds.xlsx";

const WORKSHEET_PART: &str = "/xl/worksheets/sheet1.xml";
const PRINTER_SETTINGS_1: &str = "/xl/printerSettings/printerSettings1.bin";
const PRINTER_SETTINGS_2: &str = "/xl/printerSettings/printerSettings2.bin";
const BACKGROUND_IMAGE: &str = "/xl/media/image1.png";

/// A part name, or a panic naming it.
fn part(name: &str) -> PartName {
    PartName::new(name).unwrap_or_else(|error| panic!("{name} is a part name: {error}"))
}

/// Every part of a container, in container order, with its bytes.
fn all_parts(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let package = Package::open(bytes).expect("the container opens");
    package
        .part_names()
        .map(|name| {
            let payload = package
                .part_bytes(&name)
                .unwrap_or_else(|| panic!("{} has bytes", name.as_str()))
                .to_vec();
            (name.as_str().to_owned(), payload)
        })
        .collect()
}

// -------------------------------------------------------------------------------------------
// The fixture: two kinds, two printer-settings blobs and one image
// -------------------------------------------------------------------------------------------

#[test]
fn the_fixture_reports_two_sheet_kinds_and_hands_back_the_markup_for_each() {
    let workbook = Workbook::open(&mjx_fixtures::fixture(FIXTURE)).expect("it opens");

    assert_eq!(workbook.sheets().len(), 2);
    assert_eq!(workbook.sheets()[0].name, "Printed");
    assert_eq!(workbook.sheets()[0].kind, Some(SheetKind::Worksheet));
    assert_eq!(workbook.sheets()[1].name, "Dialog");
    assert_eq!(workbook.sheets()[1].kind, Some(SheetKind::Dialogsheet));

    // `worksheet_markup` still answers `None` for the dialogsheet — that surface is about cells.
    assert!(workbook.worksheet_markup(0).expect("read").is_some());
    assert!(workbook.worksheet_markup(1).expect("no error").is_none());

    // `sheet_markup` is the other question, and answers it for both.
    let sheet = workbook.sheet_markup(0).expect("read").expect("a part");
    assert_eq!(sheet.root_element(), "worksheet");
    let SheetMarkup::Worksheet(worksheet) = &sheet else {
        panic!("tab 0 is a worksheet; it reported {}", sheet.root_element());
    };
    assert_eq!(worksheet.cells().count(), 9);

    let dialog = workbook.sheet_markup(1).expect("read").expect("a part");
    assert_eq!(dialog.root_element(), "dialogsheet");
    let SheetMarkup::DialogSheet(dialogsheet) = &dialog else {
        panic!(
            "tab 1 is a dialogsheet; it reported {}",
            dialog.root_element()
        );
    };
    assert_eq!(
        dialogsheet
            .header_footer()
            .expect("x:headerFooter")
            .string(HeaderFooterSlot::OddHeader)
            .expect("an oddHeader")
            .text(),
        "&LDialog && Controls&R&A",
        "a dialogsheet carries the same print block a worksheet does"
    );
}

#[test]
fn each_sheets_own_page_setup_and_picture_resolve_to_the_parts_they_name() {
    let workbook = Workbook::open(&mjx_fixtures::fixture(FIXTURE)).expect("it opens");

    // Read off the *markup* — the `r:id` the `pageSetup` writes — and resolved against the sheet
    // part's own `.rels`. The two sheets name two different blobs, so a lookup that answered from
    // the workbook rather than from the sheet gives the same one twice and fails.
    assert_eq!(
        workbook
            .sheet_printer_settings(0)
            .expect("resolves")
            .map(|name| name.as_str().to_owned()),
        Some(PRINTER_SETTINGS_1.to_owned())
    );
    assert_eq!(
        workbook
            .sheet_printer_settings(1)
            .expect("resolves")
            .map(|name| name.as_str().to_owned()),
        Some(PRINTER_SETTINGS_2.to_owned())
    );
    assert_eq!(
        workbook
            .sheet_background_image(0)
            .expect("resolves")
            .map(|name| name.as_str().to_owned()),
        Some(BACKGROUND_IMAGE.to_owned())
    );
    assert_eq!(
        workbook.sheet_background_image(1).expect("no error"),
        None,
        "CT_Dialogsheet declares no picture slot at all"
    );

    // The part-graph view answers the *other* question, and agrees here because this fixture's
    // sheets each relate to exactly what their markup names.
    let sheet = workbook
        .worksheet(0)
        .expect("resolves")
        .expect("it reaches a part");
    assert_eq!(
        sheet
            .parts()
            .printer_settings
            .as_ref()
            .map(PartName::as_str),
        Some(PRINTER_SETTINGS_1)
    );
    assert_eq!(
        sheet
            .parts()
            .background_image
            .as_ref()
            .map(PartName::as_str),
        Some(BACKGROUND_IMAGE)
    );

    // Neither part is opened. Both are bytes, asserted as bytes.
    let package = workbook.package();
    assert_eq!(
        package
            .part_bytes(&part(PRINTER_SETTINGS_1))
            .expect("bytes"),
        &(0u8..64).collect::<Vec<u8>>()[..],
        "the DEVMODE stand-in, carried and never interpreted"
    );
    assert!(package
        .part_bytes(&part(BACKGROUND_IMAGE))
        .expect("bytes")
        .starts_with(&[0x89, b'P', b'N', b'G']));
}

/// Tier 1 at the package tier: opened and saved untouched, byte-identical part for part — the two
/// binary parts included.
#[test]
fn the_fixture_round_trips_untouched_binary_parts_included() {
    let original = mjx_fixtures::fixture(FIXTURE);
    let workbook = Workbook::open(&original).expect("it opens");
    let saved = workbook.save().expect("a checked save writes");

    let names: Vec<String> = all_parts(&original)
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    for expected in [PRINTER_SETTINGS_1, PRINTER_SETTINGS_2, BACKGROUND_IMAGE] {
        assert!(
            names.contains(&expected.to_owned()),
            "{expected} is in the corpus"
        );
    }
    assert_eq!(all_parts(&saved), all_parts(&original));
}

/// An edit to one cell leaves the whole print block, both blobs and the image byte-identical.
#[test]
fn an_edit_to_one_cell_leaves_the_print_block_and_the_binary_parts_untouched() {
    let original = mjx_fixtures::fixture(FIXTURE);
    let mut workbook = Workbook::open(&original).expect("it opens");
    workbook
        .set_cell_value(
            0,
            CellReference::parse("B2").expect("B2"),
            CellValue::Number(99.0),
        )
        .expect("the store accepts the value");
    let saved = workbook.save().expect("a checked save writes");

    let before = Package::open(&original).expect("opens");
    let after = Package::open(&saved).expect("opens");
    let changed: Vec<String> = before
        .part_names()
        .filter(|name| before.part_bytes(name) != after.part_bytes(name))
        .map(|name| name.as_str().to_owned())
        .collect();
    assert_eq!(
        changed,
        vec![WORKSHEET_PART.to_owned()],
        "only the edited worksheet changed"
    );

    // …and inside that part, every print-block slot is still the file's own bytes.
    let sheet = String::from_utf8(
        after
            .part_bytes(&part(WORKSHEET_PART))
            .expect("bytes")
            .to_vec(),
    )
    .expect("UTF-8");
    for untouched in [
        r#"<printOptions horizontalCentered="1" verticalCentered="0" headings="1" gridLines="1" gridLinesSet="0"/>"#,
        r#"<pageSetup paperSize="9" paperHeight="297mm" paperWidth="210mm" scale="85""#,
        r#"<firstHeader>&amp;C&#65;lpha&amp;LFirst</firstHeader>"#,
        r#"<firstFooter><![CDATA[&Rdraft]]></firstFooter>"#,
        r#"<customSheetView guid="{2F1C7B44-9E30-4D6A-8B15-A7C3E0D95F82}""#,
        r#"<picture r:id="rId2"/>"#,
    ] {
        assert!(
            sheet.contains(untouched),
            "a cell edit must leave {untouched} byte-identical"
        );
    }
}

// -------------------------------------------------------------------------------------------
// Three sheet kinds in one workbook
// -------------------------------------------------------------------------------------------

/// The ticket's first clause: **a worksheet, a chartsheet and a dialogsheet** in one workbook, which
/// opens, reports three sheets of three kinds, and round-trips byte-identically through all three
/// tiers.
///
/// The three tiers, each asserted here:
///
/// 1. **`mjx_opc::Package`** — opened and saved, part for part;
/// 2. **`mjx_xlsx::Workbook`** — opened, checked and saved, part for part;
/// 3. **the sheet models** — each of the three read into its `mjx-sml` model and re-emitted, whose
///    bytes must equal the part's own.
#[test]
fn a_workbook_of_three_sheet_kinds_opens_reports_and_round_trips() {
    let bytes = workbook_with_all_three_kinds();
    let workbook = Workbook::open(&bytes).expect("a three-kind workbook opens");

    assert_eq!(workbook.sheets().len(), 3);
    assert_eq!(
        workbook
            .sheets()
            .iter()
            .map(|sheet| (sheet.name.as_str(), sheet.kind))
            .collect::<Vec<_>>(),
        vec![
            ("Printed", Some(SheetKind::Worksheet)),
            ("Dialog", Some(SheetKind::Dialogsheet)),
            ("Picture", Some(SheetKind::Chartsheet)),
        ]
    );

    // Three kinds, three models — and the chartsheet's is the one with no cell accessor.
    let roots: Vec<&str> = (0..3)
        .map(|index| {
            workbook
                .sheet_markup(index)
                .expect("read")
                .expect("a part")
                .root_element()
        })
        .collect();
    assert_eq!(roots, vec!["worksheet", "dialogsheet", "chartsheet"]);

    let chart = workbook.sheet_markup(2).expect("read").expect("a part");
    let SheetMarkup::ChartSheet(chartsheet) = &chart else {
        panic!("tab 2 is a chartsheet");
    };
    assert_eq!(
        chartsheet
            .drawing()
            .expect("x:drawing")
            .relationship_id(chartsheet.interner(), chartsheet.relationship_prefix())
            .expect("decodes")
            .as_deref(),
        Some("rId1"),
        "the relationship the chart hangs off is reported; the part it names is E3's"
    );

    // Tier 1 — the container.
    let package = Package::open(&bytes).expect("opens");
    assert_eq!(
        all_parts(&package.save().expect("saves")),
        all_parts(&bytes)
    );

    // Tier 2 — the workbook surface, checked.
    assert_eq!(
        all_parts(&workbook.save().expect("a checked save writes")),
        all_parts(&bytes)
    );

    // Tier 3 — each sheet's own model, re-emitted.
    for index in 0..3 {
        let markup = workbook.sheet_markup(index).expect("read").expect("a part");
        assert!(
            markup.is_verbatim(),
            "sheet {index} was only read, so it must still write from its own buffer"
        );
        let name = workbook.sheets()[index]
            .part
            .clone()
            .expect("the tab reaches a part");
        assert_eq!(
            markup.to_markup(),
            workbook.package().part_bytes(&name).expect("bytes"),
            "sheet {index} ({}) does not re-emit its own bytes",
            markup.root_element()
        );
    }
}

/// A macrosheet is reported as an unknown **kind** and still hands back its markup.
///
/// The two halves are different questions and `crates/mjx-xlsx/tests/worksheet_part.rs` pins the
/// first: [`SheetKind`] is defined by content type and ECMA-376 declares none for a macrosheet, so
/// the entry's `kind` is `None`. [`Workbook::sheet_markup`] dispatches on the **root element**
/// instead, so the markup is there to be read either way.
#[test]
fn a_macrosheet_reports_no_kind_and_still_hands_back_its_markup() {
    let bytes = workbook_with_a_macrosheet();
    let workbook = Workbook::open(&bytes).expect("it opens");

    assert_eq!(workbook.sheets()[2].name, "Macros");
    assert_eq!(
        workbook.sheets()[2].kind,
        None,
        "a macrosheet is not one of ECMA-376's three sheet kinds"
    );

    let markup = workbook.sheet_markup(2).expect("read").expect("a part");
    assert_eq!(markup.root_element(), "macrosheet");
    let SheetMarkup::MacroSheet(macro_sheet) = &markup else {
        panic!("tab 2 is a macrosheet");
    };
    assert_eq!(
        macro_sheet
            .page_margins()
            .expect("x:pageMargins")
            .top_inches(macro_sheet.interner()),
        Ok(0.5)
    );
    assert_eq!(
        markup.to_markup(),
        workbook
            .package()
            .part_bytes(&part("/xl/macrosheets/sheet1.xml"))
            .expect("bytes")
    );
    assert_eq!(
        all_parts(&workbook.save().expect("saves")),
        all_parts(&bytes)
    );
}

// -------------------------------------------------------------------------------------------
// The reference and the part it names are one thing
// -------------------------------------------------------------------------------------------

/// **The mutation the ticket names, in the form that discriminates.**
///
/// Retyping the printer-settings relationship — keeping its `Id`, changing its `Type` — leaves the
/// package internally consistent: `mjx_opc`'s
/// [`UndeclaredRelationshipReference`](mjx_opc::PackageDefect::UndeclaredRelationshipReference)
/// check sees a declared id and is satisfied. Only a reader that knows what a `pageSetup` *means*
/// can tell that the sheet is now pointing at a comments part, and that reader is
/// [`Workbook::validate`].
#[test]
fn a_page_setup_naming_a_relationship_of_the_wrong_type_is_a_defect() {
    let bytes = fixture_with_the_printer_settings_relationship_retyped();
    let mut workbook = Workbook::open(&bytes).expect("the container still opens");

    // Author the sheet part: this crate faults markup **this library will write**, never markup it
    // merely preserved. Writing the model back unchanged is what makes the part ours.
    let markup = workbook.sheet_markup(0).expect("read").expect("a part");
    workbook
        .write_sheet_markup(0, &markup)
        .expect("the part is replaced");

    let error = workbook.validate().expect_err("the reference is wrong");
    let XlsxError::InvalidWorkbook(defect) = &error else {
        panic!("expected a SpreadsheetML defect; got {error}");
    };
    let SpreadsheetDefect::SheetReferenceHasTheWrongRelationshipType {
        part,
        element,
        relationship_id,
        expected_type,
        found_type,
    } = defect.as_ref()
    else {
        panic!("expected a wrong-relationship-type defect; got {defect}");
    };
    assert_eq!(part, WORKSHEET_PART);
    assert_eq!(*element, "pageSetup");
    assert_eq!(relationship_id, "rId1");
    assert!(expected_type.ends_with("/printerSettings"));
    assert!(found_type.ends_with("/comments"), "{found_type}");

    // `save_unchecked` still writes the container: reporting a defect is not repairing one.
    assert!(workbook.save_unchecked().is_ok());
}

/// The same sheet, unedited, is **not** faulted — the scope that keeps the fidelity promise.
///
/// A workbook somebody else wrote is preserved, not corrected, so the check above runs only over
/// parts this library will write. Without this half, the case above would pass just as well with the
/// check applied to every part in every package, and that would make `Workbook::save` refuse to
/// write back a file it had only opened.
#[test]
fn a_wrong_reference_in_a_part_this_library_did_not_write_is_preserved_not_faulted() {
    let bytes = fixture_with_the_printer_settings_relationship_retyped();
    let workbook = Workbook::open(&bytes).expect("the container opens");
    workbook
        .validate()
        .expect("a container this library did not author is not ours to fault");
    assert_eq!(
        all_parts(&workbook.save().expect("a checked save writes")),
        all_parts(&bytes)
    );
}

/// Dropping the relationship outright is the *other* half, and it is `mjx_opc`'s to report.
///
/// The ticket asks for this mutation by name. It is recorded here rather than in `mjx-xlsx`'s own
/// defect list because the check that catches it — every attribute in the relationship-reference
/// namespace naming a declared relationship — is generic, predates this child, and needs no help:
/// duplicating it would make one defect two. What this case pins is that the `pageSetup@r:id` is
/// **reached** by it, which is a property of the markup this child added rather than of that check.
///
/// Since MJXOFF-238 the removal is caught **twice**, at two different moments, and the first of them
/// is new: `mjx_opc::Package::save` now refuses the removal itself, because the worksheet's `.rels`
/// is one this library edited and `rId1` is an id it removed. Before that, this case had to write
/// the broken container out and reopen it — laundering the worksheet's provenance — for the check to
/// reach the markup at all. Both moments are asserted below, in the order they happen.
#[test]
fn dropping_the_printer_settings_relationship_is_caught_by_the_packaging_check() {
    let mut package = Package::open(&mjx_fixtures::fixture(FIXTURE)).expect("opens");
    assert!(package
        .remove_relationship(Some(&part(WORKSHEET_PART)), "rId1")
        .expect("the .rels parses"));
    // The first moment (MJXOFF-238): the removal alone is refused, and the worksheet body was never
    // touched on the way there.
    let refusal = package
        .save()
        .expect_err("the pageSetup still names the relationship that has just gone")
        .to_string();
    assert!(
        refusal.contains("pageSetup") && refusal.contains("rId1"),
        "the refusal must name the element and the reference: {refusal}"
    );
    // The second moment, which is what the rest of this case is about: the container is written
    // anyway — `save_unchecked` is what exists for a package a caller knows to be inconsistent — and
    // the same defect is reported again once the worksheet is re-authored on the other side.
    let bytes = package.save_unchecked().expect("saves");

    let mut workbook = Workbook::open(&bytes).expect("the container still opens");
    let markup = workbook.sheet_markup(0).expect("read").expect("a part");
    workbook
        .write_sheet_markup(0, &markup)
        .expect("the part is replaced");

    let error = workbook
        .validate()
        .expect_err("the pageSetup names a relationship nothing declares");
    let report = error.to_string();
    assert!(
        report.contains("pageSetup") && report.contains("rId1"),
        "the failure must name the element and the reference: {report}"
    );
    assert!(
        workbook
            .sheet_printer_settings(0)
            .expect("no error")
            .is_none(),
        "and the resolution answers `None` rather than an unrelated part"
    );
}

// -------------------------------------------------------------------------------------------
// The packages assembled here
// -------------------------------------------------------------------------------------------

/// The fixture with the worksheet's printer-settings relationship retyped to `comments` — same
/// `Id`, same `Target`, different `Type`.
fn fixture_with_the_printer_settings_relationship_retyped() -> Vec<u8> {
    let mut package = Package::open(&mjx_fixtures::fixture(FIXTURE)).expect("opens");
    let sheet = part(WORKSHEET_PART);
    let target = package
        .relationships_for(Some(&sheet))
        .and_then(|rels| rels.by_id("rId1"))
        .map(|rel| rel.target.clone())
        .expect("the worksheet declares rId1");
    package
        .remove_relationship(Some(&sheet), "rId1")
        .expect("the .rels parses");
    package
        .add_relationship(
            Some(&sheet),
            Relationship {
                id: "rId1".to_owned(),
                rel_type:
                    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments"
                        .to_owned(),
                target,
                mode: TargetMode::Internal,
            },
        )
        .expect("the relationship is re-added");
    package.save().expect("saves")
}

/// The fixture plus a chartsheet and the drawing part it requires.
///
/// `CT_Chartsheet` declares `drawing` `minOccurs="1"`, so a chartsheet that would validate needs
/// one. The drawing part is a bare `xdr:wsDr` — SpreadsheetDrawingML, whose model is MJXOFF-107's
/// (E3) and which is why this package is not a committed fixture.
fn workbook_with_all_three_kinds() -> Vec<u8> {
    const CHARTSHEET: &[u8] = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<chartsheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheetPr codeName="Chart1"><tabColor indexed="13"/></sheetPr><sheetViews><sheetView zoomToFit="1" workbookViewId="0"/></sheetViews><pageMargins left="0.7" right="0.7" top="0.75" bottom="0.75" header="0.3" footer="0.3"/><pageSetup paperSize="9" orientation="landscape"/><headerFooter><oddHeader>&amp;C&amp;&quot;Arial,Bold&quot;Revenue</oddHeader></headerFooter><drawing r:id="rId1"/></chartsheet>"#;
    const DRAWING: &[u8] = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"/>"#;

    let mut package = Package::open(&mjx_fixtures::fixture(FIXTURE)).expect("opens");
    let chartsheet = part("/xl/chartsheets/sheet1.xml");
    let drawing = part("/xl/drawings/drawing1.xml");
    let workbook_part = part("/xl/workbook.xml");

    package
        .insert_part(
            &chartsheet,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.chartsheet+xml",
            CHARTSHEET.to_vec(),
        )
        .expect("the chartsheet part is inserted");
    package
        .insert_part(
            &drawing,
            "application/vnd.openxmlformats-officedocument.drawing+xml",
            DRAWING.to_vec(),
        )
        .expect("the drawing part is inserted");
    package
        .add_relationship(
            Some(&chartsheet),
            Relationship {
                id: "rId1".to_owned(),
                rel_type:
                    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing"
                        .to_owned(),
                target: "../drawings/drawing1.xml".to_owned(),
                mode: TargetMode::Internal,
            },
        )
        .expect("the chartsheet reaches its drawing");
    package
        .add_relationship(
            Some(&workbook_part),
            Relationship {
                id: "rId9".to_owned(),
                rel_type:
                    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/chartsheet"
                        .to_owned(),
                target: "chartsheets/sheet1.xml".to_owned(),
                mode: TargetMode::Internal,
            },
        )
        .expect("the workbook reaches its chartsheet");

    append_sheet_entry(&mut package, "Picture", 3, "rId9");
    package.save().expect("the assembled package saves")
}

/// The fixture plus a macrosheet — a part with no ECMA-376 content type and no ECMA-376
/// relationship type, reached by its root element alone.
fn workbook_with_a_macrosheet() -> Vec<u8> {
    const MACROSHEET: &[u8] = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<macrosheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><dimension ref="A1:A2"/><sheetData><row r="1"><c r="A1" t="str"><f>RESULT(1)</f></c></row></sheetData><pageMargins left="0.5" right="0.5" top="0.5" bottom="0.5" header="0.2" footer="0.2"/></macrosheet>"#;

    let mut package = Package::open(&mjx_fixtures::fixture(FIXTURE)).expect("opens");
    let macrosheet = part("/xl/macrosheets/sheet1.xml");
    let workbook_part = part("/xl/workbook.xml");

    package
        .insert_part(
            &macrosheet,
            "application/vnd.ms-excel.macrosheet+xml",
            MACROSHEET.to_vec(),
        )
        .expect("the macrosheet part is inserted");
    package
        .add_relationship(
            Some(&workbook_part),
            Relationship {
                id: "rId9".to_owned(),
                rel_type: "http://schemas.microsoft.com/office/2006/relationships/xlMacrosheet"
                    .to_owned(),
                target: "macrosheets/sheet1.xml".to_owned(),
                mode: TargetMode::Internal,
            },
        )
        .expect("the workbook reaches its macrosheet");

    append_sheet_entry(&mut package, "Macros", 3, "rId9");
    package.save().expect("the assembled package saves")
}

/// Appends one `x:sheet` entry to `xl/workbook.xml`'s sheet list.
///
/// The sheet list is the tab order and the only place it exists, so a part added to the package is
/// not a *tab* until the list names it.
fn append_sheet_entry(package: &mut Package, name: &str, sheet_id: u32, relationship_id: &str) {
    let workbook_part = part("/xl/workbook.xml");
    let bytes = package
        .part_bytes(&workbook_part)
        .expect("the workbook part has bytes")
        .to_vec();
    let markup = String::from_utf8(bytes).expect("UTF-8");
    let entry =
        format!(r#"<sheet name="{name}" sheetId="{sheet_id}" r:id="{relationship_id}"/></sheets>"#);
    let rewritten = markup.replacen("</sheets>", &entry, 1);
    assert_ne!(rewritten, markup, "the workbook part has an x:sheets list");
    package
        .replace_part_bytes(&workbook_part, rewritten.into_bytes())
        .expect("the workbook part is replaced");
}

// -------------------------------------------------------------------------------------------
// MJXOFF-241 — a refusal that says what the tab is, not that its part is gone
// -------------------------------------------------------------------------------------------

/// An edit only a worksheet can carry, aimed at a tab that is not one, is refused **by kind**.
///
/// Until MJXOFF-241 every one of these answered `MissingWorkbookPart("sheet N")`, which displays as
/// *"workbook part sheet 1 is missing from the package"* — about a part that is present, correct and
/// exactly what its `x:sheet` entry says it is. A caller who read that went looking for a broken
/// container; the real answer is that a dialogsheet has no cell to address and a chartsheet is one
/// chart over a whole tab.
///
/// Six calls across six modules rather than one, because the refusal was never in one place: it came
/// out of whichever helper reached for `x:worksheet` markup first. They now share
/// `require_worksheet_markup`, and a seventh call added to the crate cannot answer differently
/// without going around it.
#[test]
fn an_edit_aimed_at_a_chartsheet_or_a_dialogsheet_names_the_kind_rather_than_a_missing_part() {
    let bytes = workbook_with_all_three_kinds();

    // The premise, asserted rather than assumed: a package that stops carrying these two kinds fails
    // here instead of turning the loop below into a green statement about nothing.
    let kinds: Vec<Option<SheetKind>> = Workbook::open(&bytes)
        .expect("a three-kind workbook opens")
        .sheets()
        .iter()
        .map(|sheet| sheet.kind)
        .collect();
    assert_eq!(
        kinds,
        vec![
            Some(SheetKind::Worksheet),
            Some(SheetKind::Dialogsheet),
            Some(SheetKind::Chartsheet)
        ]
    );

    let cell = CellReference::parse("A1").expect("A1 parses");
    let range = CellRange::parse("A1:B2").expect("A1:B2 parses");

    for (index, kind, word) in [
        (1usize, SheetKind::Dialogsheet, "dialogsheet"),
        (2usize, SheetKind::Chartsheet, "chartsheet"),
    ] {
        // Each call gets its own workbook, so a refusal cannot be the shadow of an earlier one.
        let refusal = |name: &'static str,
                       call: &dyn Fn(&mut Workbook) -> Result<(), XlsxError>|
         -> (&'static str, XlsxError) {
            let mut workbook = Workbook::open(&bytes).expect("a three-kind workbook opens");
            match call(&mut workbook) {
                Ok(()) => panic!("{name} must refuse tab {index}, which is a {word}"),
                Err(error) => (name, error),
            }
        };

        let refusals = [
            refusal("set_cell_value", &|workbook| {
                workbook.set_cell_value(index, cell, CellValue::Number(1.0))
            }),
            refusal("set_cell_style", &|workbook| {
                workbook.set_cell_style(index, cell, Some(0))
            }),
            refusal("merge_cells", &|workbook| {
                workbook.merge_cells(index, range)
            }),
            refusal("set_cell_hyperlink", &|workbook| {
                workbook.set_cell_hyperlink(
                    index,
                    range,
                    &HyperlinkTarget::Url("https://example.invalid/".to_owned()),
                )
            }),
            refusal("add_table", &|workbook| {
                workbook
                    .add_table(
                        index,
                        &WorksheetTableSpec::new("Codes", range, &["Code", "Name"]),
                    )
                    .map(|_| ())
            }),
            refusal("add_comment", &|workbook| {
                workbook
                    .add_comment(index, cell, "Reviewer", "a remark")
                    .map(|_| ())
            }),
        ];

        for (name, error) in refusals {
            assert!(
                matches!(
                    error,
                    XlsxError::SheetIsNotAWorksheet { index: at, kind: Some(reported) }
                        if at == index && reported == kind
                ),
                "{name} must refuse tab {index} as the {word} it is; it said: {error:?}"
            );
            let text = error.to_string();
            assert!(
                text.contains(word),
                "{name}'s message must name the kind; it said: {text}"
            );
            assert!(
                !text.contains("missing"),
                "{name} must not report a part that is present as missing; it said: {text}"
            );
        }
    }
}

/// The same calls on the worksheet still work, so the guard above is a guard and not a wall.
#[test]
fn the_worksheet_of_the_three_kinds_still_takes_the_edits_the_other_two_refuse() {
    let bytes = workbook_with_all_three_kinds();
    let mut workbook = Workbook::open(&bytes).expect("a three-kind workbook opens");
    let cell = CellReference::parse("A1").expect("A1 parses");

    workbook
        .set_cell_value(0, cell, CellValue::Number(1.0))
        .expect("tab 0 is a worksheet");
    workbook
        .add_comment(0, cell, "Reviewer", "a remark")
        .expect("tab 0 is a worksheet");
    workbook.save().expect("the edited workbook saves");
}
