//! The cell store's fidelity contract (MJXOFF-95): what survives a read, an edit and a write.
//!
//! # The trap these cases are written against
//!
//! Two of this child's clauses are the shape that passes without doing anything.
//!
//! * *"A worksheet round-trips byte for byte"* is satisfied by a store that reads nothing and writes
//!   the bytes back. So every case here reads the values out as well, and the byte comparison is
//!   made against the part's own `sheetData` range rather than against a string written here.
//! * *"Rows the caller never touched re-emit verbatim"* is satisfied trivially **if nothing is ever
//!   edited**. So the edit-isolation cases below edit a cell, and then assert at the byte level over
//!   the *untouched rows specifically* — one row at a time, never over the whole part, which would
//!   pass on a store that rebuilt every row identically for unrelated reasons.
//!
//! The authored worksheet in [`DISCRIMINATING`] exists for the second point in particular. It
//! deliberately carries markup that a *rebuild* cannot reproduce — an end tag with whitespace inside
//! it, `</x:v >`, which XML allows and which only a preserved byte range brings back. Without it the
//! byte assertion would still pass with copy-on-write switched off, because this store rebuilds so
//! faithfully; with it, the assertion measures the mechanism it is named after.

use std::sync::Arc;

use mjx_ooxml_core::{RawDocument, RawNode};
use mjx_ooxml_types::spreadsheetml::CellType;
use mjx_opc::{Package, PartName};
use mjx_sml::{CellReference, CellValue, SheetData, SheetDataAnomaly};

/// An authored worksheet carrying, on purpose, the constructs a rebuild loses.
///
/// * `</x:v >` on `C1` — whitespace inside an end tag, which `ETag ::= '</' Name S? '>'` permits and
///   which nothing but the original bytes reproduces.
/// * `<x:c t="s" r="B1">` — `t` written before `r`, and `foo='bar'` in single quotes with two spaces
///   before it, so that regenerating the start tag from the decoded fields would change it.
/// * A `<x:c>` with no `r` at all on row 3, whose address is its position.
/// * A row-level `<x:extLst>`, and a comment between two cells.
const DISCRIMINATING: &[u8] = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<x:worksheet xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><x:sheetData>
  <x:row r="1" spans="1:3"  x14ac:dyDescent="0.25" xmlns:x14ac="http://schemas.microsoft.com/office/spreadsheetml/2009/9/ac"><x:c r="A1" t="inlineStr"><x:is><x:t>Alpha</x:t></x:is></x:c><!-- between --><x:c t="s"  foo='bar' r="B1"><x:v>0</x:v></x:c><x:c r="C1"><x:v>2.5</x:v ></x:c></x:row>
  <x:row r="2"><x:c r="A2"><x:f t="shared" si="0">SUM(A1:C1)</x:f><x:v>3</x:v></x:c><x:c r="B2" s="7"><x:v>4</x:v><x:extLst><x:ext uri="{ABC}"><q:keep xmlns:q="urn:q" weight="3">held</q:keep></x:ext></x:extLst></x:c></x:row>
  <x:row r="3"><x:c><x:v>9</x:v></x:c><x:c/><x:extLst><x:ext uri="{ROW}"><q:rowlevel xmlns:q="urn:q"/></x:ext></x:extLst></x:row>
</x:sheetData></x:worksheet>"#;

/// The `sheetData` element's own bytes in the part it was parsed from.
fn sheet_data_source(document: &RawDocument) -> &[u8] {
    let source = document.source().expect("the document kept its buffer");
    for child in document.root.children.iter() {
        let RawNode::Element(element) = child else {
            continue;
        };
        if document.interner.resolve(element.name.local) != "sheetData" {
            continue;
        }
        let span = element
            .source_span()
            .expect("an unmodified sheetData carries its range");
        return &source[span.start as usize..span.end as usize];
    }
    panic!("the worksheet has no sheetData");
}

/// Parses `markup` and reads its `sheetData`.
fn read(markup: &[u8]) -> (RawDocument, SheetData) {
    let document =
        mjx_xml::fidelity::parse_shared(Arc::from(markup)).expect("the worksheet parses");
    let sheet = SheetData::read_worksheet(&document)
        .expect("the sheet reads")
        .expect("the worksheet has a sheetData");
    (document, sheet)
}

/// Every worksheet part of every committed `.xlsx` fixture, derived from the corpus rather than
/// listed here.
fn worksheet_parts() -> Vec<(String, Vec<u8>)> {
    let mut found = Vec::new();
    for name in mjx_fixtures::all_fixture_files() {
        if !name.ends_with(".xlsx") {
            continue;
        }
        let bytes = mjx_fixtures::fixture(&name);
        let package = Package::open(&bytes).expect("a committed fixture opens");
        let parts: Vec<PartName> = package
            .part_names()
            .filter(|part| part.as_str().starts_with("/xl/worksheets/"))
            .collect();
        for part in parts {
            let markup = package
                .part_bytes(&part)
                .expect("the worksheet part is there")
                .to_vec();
            found.push((format!("{name}::{}", part.as_str()), markup));
        }
    }
    assert!(
        found.len() >= 2,
        "only {} worksheet parts found in the committed corpus — a sweep that finds nothing passes \
         every assertion below",
        found.len()
    );
    found
}

// -------------------------------------------------------------------------------------------
// Round-trip
// -------------------------------------------------------------------------------------------

/// Every committed worksheet's `sheetData` re-emits byte for byte, and its cells read back.
///
/// The second half is what stops this passing on a store that read nothing: the cell count, the
/// addresses and at least one value are checked against the markup as well.
#[test]
fn every_committed_worksheet_re_emits_its_sheet_data_byte_for_byte() {
    let parts = worksheet_parts();
    let mut cells_seen = 0usize;
    for (name, markup) in &parts {
        let (document, sheet) = read(markup);
        let original = sheet_data_source(&document);
        assert_eq!(
            String::from_utf8_lossy(&sheet.to_markup()),
            String::from_utf8_lossy(original),
            "{name}: the sheetData did not re-emit byte for byte"
        );
        assert_eq!(
            sheet.edited_bytes(),
            0,
            "{name}: an unedited worksheet must own no bytes of its own"
        );
        assert!(sheet.is_verbatim(), "{name}: nothing was edited");
        assert!(
            sheet.anomalies().is_empty(),
            "{name}: a committed fixture should be well-formed: {:?}",
            sheet.anomalies()
        );
        for row in sheet.rows() {
            for cell in row.cells() {
                cells_seen += 1;
                // Every address parses and re-renders as the file spelled it.
                if let Some(written) = cell.raw_attribute("r") {
                    assert_eq!(
                        cell.reference().text().as_str(),
                        written.as_ref(),
                        "{name}: {} did not re-render its own address",
                        cell.reference()
                    );
                }
            }
        }
    }
    assert!(
        cells_seen >= 9,
        "only {cells_seen} cells were read across the corpus; the assertions above are vacuous"
    );
}

/// The authored worksheet round-trips too, whitespace inside an end tag and all.
#[test]
fn the_discriminating_worksheet_round_trips_before_anything_is_edited() {
    let (document, sheet) = read(DISCRIMINATING);
    assert_eq!(
        String::from_utf8_lossy(&sheet.to_markup()),
        String::from_utf8_lossy(sheet_data_source(&document))
    );
    assert_eq!(sheet.row_count(), 3);
    assert_eq!(sheet.cell_count(), 7);
    assert_eq!(sheet.edited_bytes(), 0);
}

// -------------------------------------------------------------------------------------------
// Edit isolation
// -------------------------------------------------------------------------------------------

/// **The edit-isolation clause.** One cell changes; every other row comes back byte-identical, and
/// the assertion is made over each untouched row on its own rather than over the whole part.
#[test]
fn rows_the_caller_never_touched_re_emit_verbatim_after_an_edit_elsewhere() {
    let (_document, before) = read(DISCRIMINATING);
    let untouched: Vec<(u32, Vec<u8>)> = before
        .rows()
        .map(|row| (row.number().expect("every row is numbered"), row.markup()))
        .collect();

    let (_document, mut after) = read(DISCRIMINATING);
    after
        .set_cell_value(
            CellReference::parse("A2").expect("a reference"),
            CellValue::Number(99.0),
        )
        .expect("A2 exists");

    // Row 2 is the one that changed. Rows 1 and 3 must be byte-identical *and* still verbatim: the
    // first says the output is right, the second says it was produced by copying rather than by a
    // rebuild that happened to agree.
    for (number, original) in &untouched {
        let row = after.row(*number).expect("the row is still there");
        if *number == 2 {
            assert!(
                !row.is_verbatim(),
                "the edited row must have lost its range"
            );
            assert_ne!(&row.markup(), original, "the edited row must have changed");
            continue;
        }
        assert_eq!(
            String::from_utf8_lossy(&row.markup()),
            String::from_utf8_lossy(original),
            "row {number} was not touched and must come back byte for byte"
        );
        assert!(
            row.is_verbatim(),
            "row {number} must still be written from the part's own bytes, not rebuilt"
        );
    }

    // And inside the row that *was* rewritten, the cell nobody touched is still copied from its own
    // range — the third level of the copy-on-write rule.
    let row = after.row(2).expect("row 2");
    let b2 = row.cell(1).expect("B2");
    assert!(
        b2.is_verbatim(),
        "an untouched cell inside a rewritten row must still be copied verbatim"
    );
    let a2 = row.cell(0).expect("A2");
    assert!(!a2.is_verbatim(), "the edited cell was rebuilt");
    assert_eq!(a2.number(), Some(99.0));
}

/// The edited row loses only what was edited: its formula, its unmodelled attributes and its
/// extension all come back unchanged.
#[test]
fn a_rewritten_row_keeps_everything_about_it_that_was_not_edited() {
    let (_document, before) = read(DISCRIMINATING);
    let b2_before = before
        .cell(CellReference::parse("B2").expect("a reference"))
        .expect("B2")
        .markup();

    let (_document, mut sheet) = read(DISCRIMINATING);
    sheet
        .set_cell_value(
            CellReference::parse("A2").expect("a reference"),
            CellValue::Number(99.0),
        )
        .expect("A2 exists");
    let written = String::from_utf8(sheet.to_markup()).expect("UTF-8");

    assert!(
        written.contains(&String::from_utf8_lossy(&b2_before).into_owned()),
        "B2, with its extLst, must survive its row being rewritten:\n{written}"
    );
    assert!(
        written.contains(r#"<x:f t="shared" si="0">SUM(A1:C1)</x:f>"#),
        "the formula in the edited cell is preserved opaquely:\n{written}"
    );
    assert!(
        written
            .contains(r#"<x:c r="A2"><x:f t="shared" si="0">SUM(A1:C1)</x:f><x:v>99</x:v></x:c>"#),
        "the edited cell keeps its address and its formula and changes only its value:\n{written}"
    );
}

// -------------------------------------------------------------------------------------------
// The unknown bucket
// -------------------------------------------------------------------------------------------

/// **The unknown-bucket clause, against a real package.** A `c/extLst` full of foreign markup comes
/// back byte-identically after the row around it is rewritten — in its original order, with its
/// original prefixes.
#[test]
fn an_unmodelled_cell_extension_survives_a_rewritten_row_in_a_real_workbook() {
    let bytes = mjx_fixtures::fixture("row_spans_and_extensions.xlsx");
    let package = Package::open(&bytes).expect("the fixture opens");
    let part = PartName::new("/xl/worksheets/sheet1.xml").expect("a part name");
    let markup = package
        .part_bytes(&part)
        .expect("the worksheet is there")
        .to_vec();

    let (_document, before) = read(&markup);
    let b2 = CellReference::parse("B2").expect("a reference");
    let extension = before.cell(b2).expect("B2").markup_after_value().to_vec();
    assert_eq!(
        String::from_utf8_lossy(&extension),
        r#"<extLst><ext uri="{2C3FCC01-B0D6-4A2A-9C1A-000000000001}"><demo:note xmlns:demo="urn:mjx:demo" weight="3">kept</demo:note></ext></extLst>"#,
        "the extension is held as the bytes the file wrote"
    );

    // Edit the *other* cell in that row, so the row is rewritten and the extension has to survive a
    // rebuild rather than a copy of the whole row.
    let (_document, mut sheet) = read(&markup);
    sheet
        .set_cell_value(
            CellReference::parse("A2").expect("a reference"),
            CellValue::Number(11.0),
        )
        .expect("A2 exists");
    let row = sheet.row(2).expect("row 2");
    assert!(!row.is_verbatim(), "the row was rewritten");
    let b2_after = sheet.cell(b2).expect("B2");
    assert_eq!(
        String::from_utf8_lossy(b2_after.markup_after_value()),
        String::from_utf8_lossy(&extension),
        "the extension must come back byte for byte"
    );
    let written = String::from_utf8(sheet.to_markup()).expect("UTF-8");
    assert!(
        written.contains(&String::from_utf8_lossy(&extension).into_owned()),
        "and it must be in the markup that is written:\n{written}"
    );
}

/// An unknown *attribute* on a cell survives its row being rewritten — and survives an edit to that
/// same cell, which is the harder half.
#[test]
fn an_unknown_cell_attribute_survives_both_a_rewritten_row_and_an_edit_to_its_own_cell() {
    let (_document, mut sheet) = read(DISCRIMINATING);
    let b1 = CellReference::parse("B1").expect("a reference");
    assert_eq!(
        sheet.cell(b1).expect("B1").raw_attribute("foo").as_deref(),
        Some("bar"),
        "the unmodelled attribute is readable"
    );

    // An edit to A1 rewrites row 1. B1 is untouched, so it is copied.
    sheet
        .set_cell_value(
            CellReference::parse("A1").expect("a reference"),
            CellValue::InlineString("Changed"),
        )
        .expect("A1 exists");
    assert_eq!(
        String::from_utf8_lossy(&sheet.cell(b1).expect("B1").markup()),
        r#"<x:c t="s"  foo='bar' r="B1"><x:v>0</x:v></x:c>"#,
        "an untouched cell in a rewritten row keeps every byte, quoting and spacing included"
    );

    // Now edit B1 itself. Its start tag has to be rewritten, and the unmodelled attribute — and the
    // order, and the single quotes, and the double space — all have to come through it.
    sheet
        .set_cell_style(b1, Some(4))
        .expect("setting a style on an existing cell");
    let after = sheet.cell(b1).expect("B1");
    assert_eq!(
        String::from_utf8_lossy(&after.markup()),
        r#"<x:c t="s"  foo='bar' r="B1" s="4"><x:v>0</x:v></x:c>"#,
        "editing a modelled attribute rewrites the run in place; the unmodelled one is untouched"
    );
    assert_eq!(after.raw_attribute("foo").as_deref(), Some("bar"));
    assert_eq!(after.style(), 4);
}

/// A row-level `extLst` — which `CT_Row` declares and this store does not model — survives.
#[test]
fn a_row_level_extension_survives_the_row_being_rewritten() {
    let (_document, mut sheet) = read(DISCRIMINATING);
    sheet
        .set_cell_value(
            CellReference::parse("A3").expect("a reference"),
            CellValue::Number(5.0),
        )
        .expect("A3 exists");
    let row = sheet.row(3).expect("row 3");
    assert!(!row.is_verbatim());
    assert_eq!(
        String::from_utf8_lossy(&row.markup()),
        r#"<x:row r="3"><x:c><x:v>5</x:v></x:c><x:c/><x:extLst><x:ext uri="{ROW}"><q:rowlevel xmlns:q="urn:q"/></x:ext></x:extLst></x:row>"#,
        "the row's trailing extension, and the self-closing cell beside it, both survive"
    );
}

// -------------------------------------------------------------------------------------------
// `row@spans`, against a real workbook
// -------------------------------------------------------------------------------------------

/// **`row@spans` read through a modelled row, from a real package** — MJXOFF-93 could only assert
/// the positive half against authored markup, because `sample.xlsx` is LibreOffice-authored and
/// carries no `spans` anywhere.
///
/// The fixture is written the way Excel writes: `spans` on some rows, and no `spans` on a row that
/// does not need one. Both halves of the rule are asserted — **never dropped** when the file had
/// one, **never derived** when it did not, including after the row without one is rewritten.
#[test]
fn row_spans_are_read_from_a_real_workbook_and_never_derived_for_a_row_that_had_none() {
    let bytes = mjx_fixtures::fixture("row_spans_and_extensions.xlsx");
    let package = Package::open(&bytes).expect("the fixture opens");
    let part = PartName::new("/xl/worksheets/sheet1.xml").expect("a part name");
    let markup = package
        .part_bytes(&part)
        .expect("the worksheet is there")
        .to_vec();
    let (_document, mut sheet) = read(&markup);

    let spans: Vec<(u32, Option<String>)> = sheet
        .rows()
        .map(|row| {
            (
                row.number().expect("numbered"),
                row.spans()
                    .expect("the spans parse")
                    .map(|spans| spans.to_string()),
            )
        })
        .collect();
    assert_eq!(
        spans,
        vec![
            (1, Some("1:3".to_owned())),
            (2, None),
            (3, Some("1:2".to_owned())),
        ],
        "the fixture's spans are pinned: a reader that stopped finding them would pass a bare \
         `every span round-trips` assertion"
    );
    // The zero-based columns behind the wire form, so this is a parse and not a string copy.
    let first = sheet
        .row(1)
        .expect("row 1")
        .spans()
        .expect("parse")
        .expect("present");
    assert_eq!(first.spans()[0].first_column(), 0);
    assert_eq!(first.spans()[0].last_column(), 2);

    // Rewrite row 2 — the one with no `spans` — and confirm it does not gain one.
    sheet
        .set_cell_value(
            CellReference::parse("A2").expect("a reference"),
            CellValue::Number(11.0),
        )
        .expect("A2 exists");
    let row = sheet.row(2).expect("row 2");
    assert!(!row.is_verbatim(), "row 2 was rewritten");
    assert_eq!(
        row.spans().expect("parse"),
        None,
        "a row whose source carried no `spans` must not gain one when it is rewritten"
    );
    assert!(
        !String::from_utf8_lossy(&row.markup()).contains("spans="),
        "and the attribute must not appear in the bytes either: {}",
        String::from_utf8_lossy(&row.markup())
    );

    // Setting one explicitly does write it, so the absence above is a rule and not an inability.
    let wanted = mjx_sml::CellSpans::parse("1:2").expect("a spans list");
    sheet.set_row_spans(2, Some(&wanted)).expect("set");
    assert_eq!(
        sheet.row(2).expect("row 2").spans().expect("parse"),
        Some(wanted)
    );
}

/// The other eleven `CT_Row` attributes read back from the bytes the file wrote.
#[test]
fn every_row_attribute_is_readable_and_writable_without_disturbing_its_neighbours() {
    const MARKUP: &[u8] = br#"<x:worksheet xmlns:x="urn:x"><x:sheetData><x:row r="4" spans="1:2" s="6" customFormat="true" ht="18.75" hidden="1" customHeight="true" outlineLevel="2" collapsed="false" thickTop="1" thickBot="0" ph="true" x14ac:dyDescent="0.25" xmlns:x14ac="urn:x14ac"><x:c r="A4"><x:v>1</x:v></x:c></x:row></x:sheetData></x:worksheet>"#;
    let (_document, mut sheet) = read(MARKUP);
    let row = sheet.row(4).expect("row 4");
    assert_eq!(row.number(), Some(4));
    assert_eq!(row.style(), 6);
    assert_eq!(row.height(), Some(18.75));
    assert_eq!(row.outline_level(), 2);
    assert!(row.uses_custom_format());
    assert!(row.is_hidden());
    assert!(row.uses_custom_height());
    assert!(!row.is_collapsed());
    assert!(row.has_thick_top_border());
    assert!(!row.has_thick_bottom_border());
    assert!(row.shows_phonetic());
    assert_eq!(row.raw_attribute("x14ac:dyDescent"), Some("0.25"));

    // One write, in place: everything else keeps its spelling — `true` where the file said `true`,
    // `1` where it said `1`, and the unmodelled attribute exactly where it was.
    sheet.set_row_height(4, Some(21.0)).expect("set the height");
    assert_eq!(
        String::from_utf8_lossy(&sheet.row(4).expect("row 4").markup()),
        r#"<x:row r="4" spans="1:2" s="6" customFormat="true" ht="21" hidden="1" customHeight="true" outlineLevel="2" collapsed="false" thickTop="1" thickBot="0" ph="true" x14ac:dyDescent="0.25" xmlns:x14ac="urn:x14ac"><x:c r="A4"><x:v>1</x:v></x:c></x:row>"#
    );
}

// -------------------------------------------------------------------------------------------
// Untrusted input
// -------------------------------------------------------------------------------------------

/// A `c@r` that is not a cell reference is a typed error, never a panic.
#[test]
fn a_cell_reference_that_does_not_parse_is_a_typed_error() {
    for bad in [
        r#"<x:c r="A0"><x:v>1</x:v></x:c>"#,
        r#"<x:c r="XFE1"/>"#,
        r#"<x:c r="1A"/>"#,
        r#"<x:c r=""/>"#,
        r#"<x:c r="A1048577"/>"#,
    ] {
        let markup = format!(
            "<x:worksheet xmlns:x=\"urn:x\"><x:sheetData><x:row r=\"1\">{bad}</x:row>\
             </x:sheetData></x:worksheet>"
        );
        let document = mjx_xml::fidelity::parse_shared(Arc::from(markup.as_bytes()))
            .expect("the markup is well-formed XML");
        let error = SheetData::read_worksheet(&document)
            .expect_err("a key the store cannot parse is refused");
        assert!(
            matches!(error, mjx_sml::SmlError::Address(_)),
            "{bad} produced {error:?} rather than an address error"
        );
    }
}

/// Everything else a worksheet can get wrong is preserved as read and reported, never repaired.
#[test]
fn a_worksheet_that_lies_is_preserved_and_reported_rather_than_corrected() {
    const MARKUP: &[u8] = br#"<x:worksheet xmlns:x="urn:x"><x:sheetData><x:row r="7"><x:c r="C7"><x:v>1</x:v></x:c><x:c r="A7"><x:v>2</x:v></x:c><x:c r="A7"><x:v>3</x:v></x:c><x:c r="B9"><x:v>4</x:v></x:c><x:c r="D7" t="inlineStr"><x:v>5</x:v></x:c></x:row><x:row r="2"><x:c r="A2"/></x:row><x:row r="2"><x:c r="B2"/></x:row><x:row><x:c r="A9"/></x:row></x:sheetData></x:worksheet>"#;
    let (document, sheet) = read(MARKUP);

    // Nothing was repaired: the bytes come back exactly.
    assert_eq!(
        String::from_utf8_lossy(&sheet.to_markup()),
        String::from_utf8_lossy(sheet_data_source(&document))
    );
    // And the order was not changed on the way in.
    let addresses: Vec<String> = sheet
        .row(7)
        .expect("row 7")
        .cells()
        .map(|cell| cell.reference().to_string())
        .collect();
    assert_eq!(addresses, ["C7", "A7", "A7", "B9", "D7"]);

    let anomalies = sheet.anomalies();
    let expect = |wanted: SheetDataAnomaly| {
        assert!(
            anomalies.contains(&wanted),
            "{wanted:?} was not reported; found {anomalies:?}"
        );
    };
    expect(SheetDataAnomaly::CellsOutOfOrder { row: 7 });
    expect(SheetDataAnomaly::DuplicateCellReference {
        cell: CellReference::parse("A7").expect("a reference"),
    });
    expect(SheetDataAnomaly::CellRowDisagreesWithRow {
        cell: CellReference::parse("B9").expect("a reference"),
        row: 7,
    });
    expect(SheetDataAnomaly::CellTypeDisagreesWithContent {
        cell: CellReference::parse("D7").expect("a reference"),
    });
    expect(SheetDataAnomaly::RowsOutOfOrder {
        row: 2,
        previous_row: 7,
    });
    expect(SheetDataAnomaly::DuplicateRowNumber { row: 2 });
    expect(SheetDataAnomaly::RowWithoutNumber { position: 3 });

    // A lookup in a sheet whose order was not repaired still answers correctly.
    assert_eq!(
        sheet
            .cell(CellReference::parse("C7").expect("a reference"))
            .expect("C7 is found by scan, the rows being out of order")
            .number(),
        Some(1.0)
    );
}

// -------------------------------------------------------------------------------------------
// Values
// -------------------------------------------------------------------------------------------

/// Each `ST_CellType` reads back through the accessor its `t` names, and writes back with the `t`
/// that goes with it — including no `t` at all for a number, whose type is the schema default.
#[test]
fn every_cell_type_writes_the_attribute_that_belongs_to_it_and_reads_back() {
    let mut sheet = SheetData::authored(None);
    let at = |text: &str| CellReference::parse(text).expect("a reference");
    sheet
        .set_cell_value(at("A1"), CellValue::Number(1.5))
        .expect("set");
    sheet
        .set_cell_value(at("B1"), CellValue::NumberText("1.500"))
        .expect("set");
    sheet
        .set_cell_value(at("C1"), CellValue::SharedString(42))
        .expect("set");
    sheet
        .set_cell_value(at("D1"), CellValue::Boolean(true))
        .expect("set");
    sheet
        .set_cell_value(at("E1"), CellValue::Error("#DIV/0!"))
        .expect("set");
    sheet
        .set_cell_value(at("F1"), CellValue::FormulaString("text & more"))
        .expect("set");
    sheet
        .set_cell_value(at("G1"), CellValue::InlineString("a < b"))
        .expect("set");
    sheet
        .set_cell_value(at("H1"), CellValue::Blank)
        .expect("set");

    assert_eq!(
        String::from_utf8_lossy(&sheet.to_markup()),
        concat!(
            "<sheetData><row r=\"1\">",
            "<c r=\"A1\"><v>1.5</v></c>",
            "<c r=\"B1\"><v>1.500</v></c>",
            "<c r=\"C1\" t=\"s\"><v>42</v></c>",
            "<c r=\"D1\" t=\"b\"><v>1</v></c>",
            "<c r=\"E1\" t=\"e\"><v>#DIV/0!</v></c>",
            "<c r=\"F1\" t=\"str\"><v>text &amp; more</v></c>",
            "<c r=\"G1\" t=\"inlineStr\"><is><t>a &lt; b</t></is></c>",
            "<c r=\"H1\"></c>",
            "</row></sheetData>"
        ),
        "a number writes no `t`, because `n` is the schema default and a file that would not have \
         written the attribute must not gain one"
    );

    assert_eq!(sheet.cell(at("A1")).expect("A1").number(), Some(1.5));
    assert_eq!(sheet.cell(at("A1")).expect("A1").written_cell_type(), None);
    assert_eq!(
        sheet.cell(at("A1")).expect("A1").cell_type(),
        CellType::Number
    );
    assert_eq!(sheet.cell(at("B1")).expect("B1").number(), Some(1.5));
    assert_eq!(
        sheet.cell(at("C1")).expect("C1").shared_string_index(),
        Some(42)
    );
    assert_eq!(sheet.cell(at("D1")).expect("D1").boolean(), Some(true));
    assert_eq!(
        sheet
            .cell(at("E1"))
            .expect("E1")
            .value()
            .expect("decodes")
            .as_deref(),
        Some("#DIV/0!")
    );
    assert_eq!(
        sheet
            .cell(at("F1"))
            .expect("F1")
            .value()
            .expect("decodes")
            .as_deref(),
        Some("text & more"),
        "the entity written into the file is decoded on the way back out"
    );
    assert_eq!(
        sheet.cell(at("G1")).expect("G1").inline_string_markup(),
        Some(&b"<is><t>a &lt; b</t></is>"[..])
    );
    assert_eq!(sheet.cell(at("H1")).expect("H1").raw_value(), None);
    assert!(sheet.anomalies().is_empty());
}

/// An authored sheet, built in the order a file is written, and the values read back out of it.
#[test]
fn an_authored_sheet_can_be_built_edited_and_written() {
    let mut sheet = SheetData::authored(Some("x"));
    for row in 1..=3u32 {
        for column in 0..3u16 {
            let reference = CellReference::relative(column, row - 1).expect("inside the grid");
            sheet
                .set_cell_value(
                    reference,
                    CellValue::Number(f64::from(row * 10 + u32::from(column))),
                )
                .expect("set");
        }
    }
    assert_eq!(sheet.row_count(), 3);
    assert_eq!(sheet.cell_count(), 9);

    sheet.set_row_height(2, Some(30.0)).expect("set the height");
    sheet
        .set_cell_style(CellReference::parse("B2").expect("a reference"), Some(3))
        .expect("set the style");
    sheet
        .remove_cell(CellReference::parse("C3").expect("a reference"))
        .then_some(())
        .expect("C3 was there");

    assert_eq!(
        String::from_utf8_lossy(&sheet.to_markup()),
        concat!(
            "<x:sheetData>",
            "<x:row r=\"1\"><x:c r=\"A1\"><x:v>10</x:v></x:c><x:c r=\"B1\"><x:v>11</x:v></x:c>",
            "<x:c r=\"C1\"><x:v>12</x:v></x:c></x:row>",
            "<x:row r=\"2\" ht=\"30\"><x:c r=\"A2\"><x:v>20</x:v></x:c>",
            "<x:c r=\"B2\" s=\"3\"><x:v>21</x:v></x:c><x:c r=\"C2\"><x:v>22</x:v></x:c></x:row>",
            "<x:row r=\"3\"><x:c r=\"A3\"><x:v>30</x:v></x:c><x:c r=\"B3\"><x:v>31</x:v></x:c></x:row>",
            "</x:sheetData>"
        )
    );
    assert!(sheet.remove_row(2), "the row was there");
    assert_eq!(sheet.row_count(), 2);
    assert_eq!(sheet.cell_count(), 5);
    assert!(sheet.row(2).is_none());
    assert_eq!(
        sheet
            .cell(CellReference::parse("A3").expect("a reference"))
            .expect("A3")
            .number(),
        Some(30.0),
        "removing a row must not disturb the cell slices of the rows after it"
    );
}

/// A cell with no `c@r` takes its address from its position, and does not gain the attribute.
#[test]
fn a_cell_without_a_reference_is_placed_by_position_and_does_not_gain_one() {
    let (_document, sheet) = read(DISCRIMINATING);
    let row = sheet.row(3).expect("row 3");
    let addresses: Vec<String> = row
        .cells()
        .map(|cell| cell.reference().to_string())
        .collect();
    assert_eq!(addresses, ["A3", "B3"]);
    for cell in row.cells() {
        assert!(
            !cell.has_written_reference(),
            "{} wrote no `r` and must not be given one",
            cell.reference()
        );
    }
    assert!(!String::from_utf8_lossy(&row.markup()).contains(" r=\"A3\""));
}

/// `<v></v>` is a value that is present and empty; a cell with no `<v>` at all is not the same
/// thing, and the two must not converge.
#[test]
fn an_empty_value_and_an_absent_one_stay_different() {
    const MARKUP: &[u8] = br#"<x:worksheet xmlns:x="urn:x"><x:sheetData><x:row r="1"><x:c r="A1"><x:v></x:v></x:c><x:c r="B1"/><x:c r="C1"></x:c></x:row></x:sheetData></x:worksheet>"#;
    let (document, sheet) = read(MARKUP);
    let at = |text: &str| CellReference::parse(text).expect("a reference");
    assert_eq!(
        sheet.cell(at("A1")).expect("A1").raw_value(),
        Some(&b""[..])
    );
    assert_eq!(sheet.cell(at("B1")).expect("B1").raw_value(), None);
    assert_eq!(sheet.cell(at("C1")).expect("C1").raw_value(), None);
    assert_eq!(
        String::from_utf8_lossy(&sheet.to_markup()),
        String::from_utf8_lossy(sheet_data_source(&document)),
        "`<v></v>`, `<c/>` and `<c></c>` are three different spellings and all three come back"
    );

    // The same three spellings survive a rebuild, not only a copy.
    let (_document, mut sheet) = read(MARKUP);
    sheet
        .set_cell_value(at("A1"), CellValue::NumberText(""))
        .expect("set");
    assert_eq!(
        String::from_utf8_lossy(&sheet.row(1).expect("row 1").markup()),
        r#"<x:row r="1"><x:c r="A1"><x:v></x:v></x:c><x:c r="B1"/><x:c r="C1"></x:c></x:row>"#
    );
}
