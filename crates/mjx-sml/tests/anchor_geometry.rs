//! MJXOFF-107 (E3) — resolving a `xdr:` anchor to a rectangle on a sheet, and saying which half of
//! the answer the sheet stated.
//!
//! # Where the expected numbers come from
//!
//! **Not from this crate.** The worksheet below reproduces the geometry of
//! `tests/fixtures/worksheet_drawings.xlsx` — column widths 3.5, 20.75 and 12 characters, a
//! 42-point row 3, `defaultRowHeight="15"` and **no** `defaultColWidth` — and the expected extent of
//! its two-cell anchor is **the one Apache POI wrote into that fixture's own
//! `xdr:pic/xdr:spPr/a:xfrm/a:ext`**: `cx="2085975" cy="885825"`. POI computed those from the same
//! column widths through its own implementation of ECMA-376 Part 1 §18.3.1.13, so a conversion that
//! is wrong in either direction disagrees with a number this project did not produce.
//!
//! Every column width and row height differs from every other, and none is the default, so a
//! resolver that used one width for every column, or crossed the two axes, fails.

use mjx_dml::spreadsheet_drawing::{Anchor, CellMarker, WorksheetDrawing};
use mjx_dml::{Position, Size};
use mjx_sml::{ColumnMetrics, GeometrySource, SheetAnchors, WorksheetPart};
use mjx_xml::fidelity;

const SML: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const XDR: &str = "http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing";

/// The geometry `tests/fixtures/worksheet_drawings.xlsx` carries, as markup.
fn sheet_markup(extra_format: &str, cols: &str) -> String {
    format!(
        r#"<worksheet xmlns="{SML}">
  <sheetFormatPr defaultRowHeight="15"{extra_format}/>
  {cols}
  <sheetData>
    <row r="1"><c r="A1"/></row>
    <row r="2"><c r="A2"/></row>
    <row r="3" ht="42" customHeight="1"><c r="A3"/></row>
    <row r="4"><c r="A4"/></row>
    <row r="5"><c r="A5"/></row>
    <row r="6"><c r="A6"/></row>
  </sheetData>
</worksheet>"#
    )
}

fn poi_geometry() -> WorksheetPart {
    let cols = r#"<cols><col min="1" max="1" width="3.5" customWidth="1"/><col min="2" max="2" width="20.75" customWidth="1"/><col min="3" max="3" width="12" customWidth="1"/></cols>"#;
    WorksheetPart::read_part(sheet_markup("", cols).as_bytes())
        .expect("it parses")
        .expect("its root is an x:worksheet")
}

fn two_cell_anchor(from: &str, to: &str) -> (Anchor, mjx_ooxml_core::Interner) {
    let part = format!(
        r#"<xdr:wsDr xmlns:xdr="{XDR}">
  <xdr:twoCellAnchor>
    <xdr:from>{from}</xdr:from>
    <xdr:to>{to}</xdr:to>
    <xdr:sp><xdr:nvSpPr><xdr:cNvPr id="1" name="s"/><xdr:cNvSpPr/></xdr:nvSpPr><xdr:spPr/></xdr:sp>
    <xdr:clientData/>
  </xdr:twoCellAnchor>
</xdr:wsDr>"#
    );
    let document = fidelity::parse(part.as_bytes()).expect("it parses");
    let drawing = WorksheetDrawing::read_part(&document)
        .expect("it reads")
        .expect("an xdr:wsDr");
    let anchor = drawing.anchor(&document.interner, 0).expect("one anchor");
    (anchor, document.interner)
}

const MARKER_FROM: &str =
    "<xdr:col>1</xdr:col><xdr:colOff>190500</xdr:colOff><xdr:row>2</xdr:row><xdr:rowOff>47625</xdr:rowOff>";
const MARKER_TO: &str =
    "<xdr:col>3</xdr:col><xdr:colOff>95250</xdr:colOff><xdr:row>5</xdr:row><xdr:rowOff>19050</xdr:rowOff>";

#[track_caller]
fn assert_emu(actual: i64, expected: i64) {
    assert_eq!(actual, expected, "expected {expected} EMU, got {actual}");
}

// -------------------------------------------------------------------------------------------
// The conversions, against ECMA-376's own worked example
// -------------------------------------------------------------------------------------------

#[test]
fn the_column_width_formula_reproduces_the_specifications_own_example() {
    // §18.3.1.13: "if the cell width is 8 characters wide, the value of this attribute must be
    // Truncate([8*7+5]/7*256)/256 = 8.7109375" and "Truncate(((256*8.7109375+Truncate(128/7))/256)*7)
    // = 61 pixels".
    let metrics = ColumnMetrics::CALIBRI_11_AT_96_DPI;
    let characters = metrics.base_column_width_characters(8);
    assert!(
        (characters - 8.710_937_5).abs() < 1e-9,
        "expected 8.7109375 characters, got {characters}"
    );
    // 61 pixels at 96 dpi is 61 × 914400/96 EMU.
    assert_emu(
        metrics.characters_to_emu(characters).round() as i64,
        61 * 9525,
    );
}

#[test]
fn a_row_height_is_exact_and_a_column_width_goes_through_the_metrics() {
    let part = poi_geometry();
    let anchors = SheetAnchors::new(&part, ColumnMetrics::CALIBRI_11_AT_96_DPI);

    // A point is 12,700 EMU, by definition. Row 3 (zero-based 2) states 42 points.
    let (height, source) = anchors.row_height(2).expect("row 3 states its own height");
    assert_emu(height.round() as i64, 42 * 12_700);
    assert_eq!(source, GeometrySource::Stated);

    // Row 1 states none, so the sheet's own default answers — and says it defaulted.
    let (height, source) = anchors.row_height(0).expect("the sheet states a default");
    assert_emu(height.round() as i64, 15 * 12_700);
    assert_eq!(source, GeometrySource::SheetDefault);

    // Three stated column widths, three different answers, none of them the default.
    for (column, expected) in [(0u16, 24 * 9525), (1, 145 * 9525), (2, 84 * 9525)] {
        let (width, source) = anchors.column_width(column);
        assert_emu(width.round() as i64, expected);
        assert_eq!(source, GeometrySource::Stated, "column {column}");
    }

    // Column D states nothing and the sheet states no `defaultColWidth` either, so the answer comes
    // from `baseColWidth` — and says so rather than pretending it was measured.
    let (width, source) = anchors.column_width(3);
    assert_emu(width.round() as i64, 61 * 9525);
    assert_eq!(source, GeometrySource::BaseColumnWidth);
}

// -------------------------------------------------------------------------------------------
// The anchor, against a number Apache POI computed
// -------------------------------------------------------------------------------------------

#[test]
fn a_two_cell_anchor_resolves_to_the_extent_apache_poi_wrote_for_the_same_geometry() {
    let part = poi_geometry();
    let anchors = SheetAnchors::new(&part, ColumnMetrics::CALIBRI_11_AT_96_DPI);
    let (anchor, interner) = two_cell_anchor(MARKER_FROM, MARKER_TO);

    let bounds = anchors
        .resolve(&anchor, &interner)
        .expect("the sheet places it");

    // The top-left corner: column A's width (24 px) plus the marker's own 190,500 EMU offset into
    // column B; two default rows plus 47,625 EMU into row 3.
    assert_emu(bounds.position.x.emu(), 24 * 9525 + 190_500);
    assert_emu(bounds.position.y.emu(), 2 * 15 * 12_700 + 47_625);

    // …and the size, which is the number `tests/fixtures/worksheet_drawings.xlsx` carries in its own
    // `a:ext`, written by Apache POI from the same column widths.
    assert_emu(bounds.size.width.emu(), 2_085_975);
    assert_emu(bounds.size.height.emu(), 885_825);

    assert_eq!(bounds.row_source, GeometrySource::SheetDefault);
    assert_eq!(bounds.column_source, GeometrySource::Stated);
    assert!(!bounds.is_fully_stated());
}

#[test]
fn the_three_anchor_modes_resolve_through_three_different_amounts_of_sheet() {
    let part = poi_geometry();
    let anchors = SheetAnchors::new(&part, ColumnMetrics::CALIBRI_11_AT_96_DPI);

    let drawing_markup = format!(
        r#"<xdr:wsDr xmlns:xdr="{XDR}">
  <xdr:twoCellAnchor><xdr:from>{MARKER_FROM}</xdr:from><xdr:to>{MARKER_TO}</xdr:to><xdr:clientData/></xdr:twoCellAnchor>
  <xdr:oneCellAnchor><xdr:from>{MARKER_FROM}</xdr:from><xdr:ext cx="914400" cy="457200"/><xdr:clientData/></xdr:oneCellAnchor>
  <xdr:absoluteAnchor><xdr:pos x="1905000" y="952500"/><xdr:ext cx="685800" cy="342900"/><xdr:clientData/></xdr:absoluteAnchor>
</xdr:wsDr>"#
    );
    let document = fidelity::parse(drawing_markup.as_bytes()).expect("it parses");
    let drawing = WorksheetDrawing::read_part(&document)
        .expect("it reads")
        .expect("an xdr:wsDr");
    let list: Vec<Anchor> = drawing.anchors(&document.interner).collect();

    let two = anchors
        .resolve(&list[0], &document.interner)
        .expect("placed");
    let one = anchors
        .resolve(&list[1], &document.interner)
        .expect("placed");
    let absolute = anchors
        .resolve(&list[2], &document.interner)
        .expect("placed");

    // The two-cell and the one-cell anchors start at the same point — they share a `from` marker —
    // and have different sizes, because only one of them states its own.
    assert_eq!(two.position, one.position);
    assert_eq!(one.size, Size::from_emu(914_400, 457_200));
    assert_ne!(two.size, one.size);

    // The absolute anchor consulted no row and no column at all: its rectangle is what it says.
    assert_eq!(absolute.position, Position::from_emu(1_905_000, 952_500));
    assert_eq!(absolute.size, Size::from_emu(685_800, 342_900));
    assert!(absolute.is_fully_stated());
    assert_ne!(absolute.position, two.position);
}

#[test]
fn a_sheet_that_states_no_row_height_at_all_answers_none_rather_than_guessing() {
    // `@defaultRowHeight` is `use="required"`, so a worksheet with no `x:sheetFormatPr` states no
    // default row height. Row 3 states 42 points, so rows 1 and 2 have no height this library can
    // report — and answering `0`, or Excel's own 15, would be presenting a guess as a measurement.
    let markup = format!(
        r#"<worksheet xmlns="{SML}">
  <sheetData>
    <row r="1"><c r="A1"/></row>
    <row r="3" ht="42" customHeight="1"><c r="A3"/></row>
  </sheetData>
</worksheet>"#
    );
    let part = WorksheetPart::read_part(markup.as_bytes())
        .expect("it parses")
        .expect("an x:worksheet");
    let anchors = SheetAnchors::new(&part, ColumnMetrics::CALIBRI_11_AT_96_DPI);

    assert_eq!(anchors.row_height(0), None, "row 1 states nothing");
    assert!(
        anchors.row_height(2).is_some(),
        "row 3 states 42 points, so it is answerable"
    );

    let (anchor, interner) = two_cell_anchor(MARKER_FROM, MARKER_TO);
    assert_eq!(
        anchors.resolve(&anchor, &interner),
        None,
        "an anchor whose rows nothing states cannot be placed, and `None` is the honest report"
    );

    // …and the *column* half of the same sheet is still answerable, so the `None` above is about the
    // rows rather than about the sheet being empty.
    let (width, source) = anchors.column_width(0);
    assert!(width > 0.0);
    assert_eq!(source, GeometrySource::BaseColumnWidth);
}

#[test]
fn a_hidden_row_and_a_hidden_column_occupy_no_space() {
    let cols = r#"<cols><col min="1" max="1" width="3.5" customWidth="1"/><col min="2" max="2" width="20.75" hidden="1" customWidth="1"/><col min="3" max="3" width="12" customWidth="1"/></cols>"#;
    let markup = format!(
        r#"<worksheet xmlns="{SML}">
  <sheetFormatPr defaultRowHeight="15"/>
  {cols}
  <sheetData>
    <row r="1"><c r="A1"/></row>
    <row r="2" hidden="1"><c r="A2"/></row>
    <row r="3" ht="42" customHeight="1"><c r="A3"/></row>
  </sheetData>
</worksheet>"#
    );
    let part = WorksheetPart::read_part(markup.as_bytes())
        .expect("it parses")
        .expect("an x:worksheet");
    let anchors = SheetAnchors::new(&part, ColumnMetrics::CALIBRI_11_AT_96_DPI);

    assert_emu(anchors.column_width(1).0.round() as i64, 0);
    assert_emu(
        anchors.row_height(1).expect("answerable").0.round() as i64,
        0,
    );

    // Column C now starts where column B used to: A's width, and nothing for the hidden B.
    assert_emu(anchors.column_offset(2).0.round() as i64, 24 * 9525);
    // Row 3 starts one default row down, because row 2 is hidden.
    assert_emu(
        anchors.row_offset(2).expect("answerable").0.round() as i64,
        15 * 12_700,
    );
}

#[test]
fn zero_height_hides_every_row_that_states_nothing_of_its_own() {
    let cols = r#"<cols><col min="1" max="1" width="3.5" customWidth="1"/></cols>"#;
    let part = WorksheetPart::read_part(sheet_markup(r#" zeroHeight="1""#, cols).as_bytes())
        .expect("it parses")
        .expect("an x:worksheet");
    let anchors = SheetAnchors::new(&part, ColumnMetrics::CALIBRI_11_AT_96_DPI);

    assert_emu(
        anchors.row_height(0).expect("answerable").0.round() as i64,
        0,
    );
    // …but row 3 states 42 points of its own, which `zeroHeight` does not override.
    assert_emu(
        anchors.row_height(2).expect("answerable").0.round() as i64,
        42 * 12_700,
    );
    assert_emu(
        anchors.row_offset(3).expect("answerable").0.round() as i64,
        42 * 12_700,
    );
}

#[test]
fn the_metrics_are_carried_into_the_answer_and_a_different_font_gives_a_different_rectangle() {
    let part = poi_geometry();
    let (anchor, interner) = two_cell_anchor(MARKER_FROM, MARKER_TO);

    let calibri = SheetAnchors::new(&part, ColumnMetrics::CALIBRI_11_AT_96_DPI)
        .resolve(&anchor, &interner)
        .expect("placed");
    let wider = SheetAnchors::new(
        &part,
        ColumnMetrics {
            maximum_digit_width_pixels: 10.0,
            pixels_per_inch: 96.0,
        },
    )
    .resolve(&anchor, &interner)
    .expect("placed");

    assert_ne!(
        calibri.size.width, wider.size.width,
        "a column width is a character count; the font decides what it is worth"
    );
    // The row half is a point measurement, so it does not move with the font.
    assert_eq!(calibri.size.height, wider.size.height);
    assert_eq!(
        calibri.column_metrics,
        ColumnMetrics::CALIBRI_11_AT_96_DPI,
        "the answer says which metrics produced it"
    );
    assert_eq!(wider.column_metrics.maximum_digit_width_pixels, 10.0);
}

#[test]
fn a_point_resolves_back_to_the_marker_it_came_from() {
    let part = poi_geometry();
    let anchors = SheetAnchors::new(&part, ColumnMetrics::CALIBRI_11_AT_96_DPI);
    let marker = CellMarker::new(1, 190_500, 2, 47_625);

    let (position, _, _) = anchors.marker_position(marker).expect("placed");
    let back = anchors
        .marker_at(position, 1_048_576, 16_384)
        .expect("the sheet places the point");
    assert_eq!(back, marker, "the round trip must land in the same cell");

    // …and a point one EMU short of column B's left edge lands in column A, so the search is not
    // simply answering the column it was handed.
    let edge = Position::from_emu(24 * 9525 - 1, 0);
    let back = anchors.marker_at(edge, 1_048_576, 16_384).expect("placed");
    assert_eq!(back.column, 0);
    assert_eq!(back.column_offset.emu(), 24 * 9525 - 1);
}
