//! Unit tests for the DrawingML table model (`a:tbl`), through the public API only.
//!
//! Two things get the most attention, because they are the two a table gets wrong: **the grid stays
//! rectangular** (merging covers cells, it never removes them, so `(row, column)` addressing has no
//! holes), and **round-trip fidelity** — a table carries `extLst`, `cell3D`, a style reference and a
//! whole text body this tier does not interpret, and all of it has to come back out unchanged.

use mjx_dml::{CellBorder, LineSpec, Table, TableCellProperties, TablePart};
use mjx_ooxml_core::{FromXml, RawDocument, ToXml};
use mjx_xml::fidelity;

const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";

fn parse(fragment: &str) -> (Table, RawDocument) {
    let doc = fidelity::parse(fragment.as_bytes()).expect("fragment parses");
    let table = Table::from_xml(&doc.root, &doc.interner).expect("from_xml");
    (table, doc)
}

/// Wraps a table body with the namespace declaration the fidelity reader needs.
fn tbl(body: &str) -> String {
    format!(r#"<a:tbl xmlns:a="{A}">{body}</a:tbl>"#)
}

/// A 2×2 table with a header row, distinct cell text, and column widths.
fn simple() -> String {
    tbl(concat!(
        r#"<a:tblPr firstRow="1" bandRow="1"/>"#,
        r#"<a:tblGrid><a:gridCol w="3048000"/><a:gridCol w="1524000"/></a:tblGrid>"#,
        r#"<a:tr h="370840">"#,
        r#"<a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>Region</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc>"#,
        r#"<a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>Revenue</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc>"#,
        r#"</a:tr>"#,
        r#"<a:tr h="370840">"#,
        r#"<a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>North</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc>"#,
        r#"<a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>42</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc>"#,
        r#"</a:tr>"#
    ))
}

#[track_caller]
fn assert_round_trips(table: &Table, mut doc: RawDocument, expected: &str) {
    doc.root = table.to_xml(&mut doc.interner);
    let out = fidelity::serialize_to_vec(&doc);
    assert_eq!(
        String::from_utf8_lossy(&out),
        expected,
        "round-trip mismatch"
    );
}

// ---------------------------------------------------------------------------------------------
// Shape of the grid
// ---------------------------------------------------------------------------------------------

#[test]
fn reads_its_dimensions_from_the_grid_and_the_rows() {
    let (table, doc) = parse(&simple());

    assert_eq!(table.row_count(), 2);
    assert_eq!(table.column_count(), 2, "the grid declares the width");
    assert_eq!(table.row(0).expect("row 0").cell_count(), 2);

    let widths: Vec<i64> = table
        .grid()
        .expect("a grid")
        .columns()
        .map(|column| column.width(&doc.interner).expect("a width").emu())
        .collect();
    assert_eq!(widths, [3_048_000, 1_524_000]);
    assert_eq!(
        table
            .row(0)
            .expect("row 0")
            .height(&doc.interner)
            .expect("a height")
            .emu(),
        370_840
    );
}

#[test]
fn addresses_a_cell_by_row_and_column() {
    let (table, _doc) = parse(&simple());

    assert_eq!(table.cell(0, 0).expect("0,0").text(), "Region");
    assert_eq!(table.cell(0, 1).expect("0,1").text(), "Revenue");
    assert_eq!(table.cell(1, 0).expect("1,0").text(), "North");
    assert_eq!(table.cell(1, 1).expect("1,1").text(), "42");
    assert!(table.cell(2, 0).is_none(), "past the last row");
    assert!(table.cell(0, 2).is_none(), "past the last column");
}

#[test]
fn a_cells_text_is_the_text_tree_unchanged() {
    // The point of reusing CT_TextBody: paragraphs and runs work inside a cell exactly as in a shape.
    let (table, _doc) = parse(&simple());
    let body = table
        .cell(0, 0)
        .expect("0,0")
        .text_body()
        .expect("a text body");

    assert_eq!(body.paragraphs().count(), 1);
    let paragraph = body.paragraphs().next().expect("one paragraph");
    assert_eq!(paragraph.runs().count(), 1);
    assert_eq!(paragraph.text(), "Region");
}

#[test]
fn a_table_with_no_grid_reports_no_columns_rather_than_guessing() {
    // `a:tblGrid` is required by the schema; a file missing it is read as it is, not repaired.
    let (table, _doc) = parse(&tbl(r#"<a:tr h="1"><a:tc/></a:tr>"#));
    assert_eq!(table.column_count(), 0);
    assert_eq!(table.row_count(), 1);
}

// ---------------------------------------------------------------------------------------------
// Merging — the grid stays rectangular
// ---------------------------------------------------------------------------------------------

/// A 2×2 table whose top row is one horizontally merged cell, and whose first column is vertically
/// merged down — the two merge directions at once.
fn merged() -> String {
    tbl(concat!(
        r#"<a:tblGrid><a:gridCol w="100"/><a:gridCol w="100"/></a:tblGrid>"#,
        r#"<a:tr h="10">"#,
        r#"<a:tc gridSpan="2" rowSpan="2"><a:txBody><a:bodyPr/><a:p><a:r><a:t>Anchor</a:t></a:r></a:p></a:txBody></a:tc>"#,
        r#"<a:tc hMerge="1"/>"#,
        r#"</a:tr>"#,
        r#"<a:tr h="10">"#,
        r#"<a:tc vMerge="1"/>"#,
        r#"<a:tc hMerge="1" vMerge="1"/>"#,
        r#"</a:tr>"#
    ))
}

#[test]
fn a_merge_covers_cells_without_removing_them() {
    let (table, _doc) = parse(&merged());

    // Every position is still addressable — this is what keeps (row, column) honest.
    assert_eq!(table.row(0).expect("row 0").cell_count(), 2);
    assert_eq!(table.row(1).expect("row 1").cell_count(), 2);
    for row in 0..2 {
        for column in 0..2 {
            assert!(table.cell(row, column).is_some(), "{row},{column} missing");
        }
    }
}

#[test]
fn the_anchor_carries_the_spans_and_the_covered_cells_say_so() {
    let (table, doc) = parse(&merged());
    let interner = &doc.interner;

    let anchor = table.cell(0, 0).expect("anchor");
    assert_eq!(anchor.column_span(interner), 2);
    assert_eq!(anchor.row_span(interner), 2);
    assert!(
        !anchor.is_covered_by_merge(interner),
        "the anchor renders, however far it spans"
    );
    assert_eq!(anchor.text(), "Anchor");

    let right = table.cell(0, 1).expect("0,1");
    assert!(right.merged_horizontally(interner));
    assert!(right.is_covered_by_merge(interner));

    let below = table.cell(1, 0).expect("1,0");
    assert!(below.merged_vertically(interner));
}

#[test]
fn a_covered_cell_names_the_anchor_that_owns_it() {
    let (table, doc) = parse(&merged());
    let interner = &doc.interner;

    for (row, column) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
        assert_eq!(
            table.merge_anchor(interner, row, column),
            Some((0, 0)),
            "{row},{column} should resolve to the anchor"
        );
    }
}

#[test]
fn an_unmerged_cell_is_its_own_anchor() {
    let (table, doc) = parse(&simple());
    assert_eq!(table.merge_anchor(&doc.interner, 1, 1), Some((1, 1)));
}

#[test]
fn a_span_below_one_is_not_a_span() {
    // A covered cell states `hMerge`, never `gridSpan="0"`; a nonsense value reads as the default.
    let (table, doc) = parse(&tbl(concat!(
        r#"<a:tblGrid><a:gridCol w="1"/></a:tblGrid>"#,
        r#"<a:tr h="1"><a:tc gridSpan="0" rowSpan="-2"/></a:tr>"#
    )));
    let cell = table.cell(0, 0).expect("0,0");
    assert_eq!(cell.column_span(&doc.interner), 1);
    assert_eq!(cell.row_span(&doc.interner), 1);
}

// ---------------------------------------------------------------------------------------------
// Properties
// ---------------------------------------------------------------------------------------------

#[test]
fn table_parts_are_reported_stated_and_in_effect() {
    let (table, doc) = parse(&simple());
    let properties = table.properties().expect("a:tblPr");
    let interner = &doc.interner;

    assert_eq!(properties.part(interner, TablePart::FirstRow), Some(true));
    assert_eq!(properties.part(interner, TablePart::BandedRows), Some(true));
    // Unstated is distinguishable from off, though both render the same.
    assert_eq!(properties.part(interner, TablePart::LastRow), None);
    assert!(!properties.has_part(interner, TablePart::LastRow));
    assert!(properties.has_part(interner, TablePart::FirstRow));
}

#[test]
fn a_cells_borders_are_line_properties_and_read_per_edge() {
    let (table, doc) = parse(&tbl(concat!(
        r#"<a:tblGrid><a:gridCol w="1"/></a:tblGrid>"#,
        r#"<a:tr h="1"><a:tc><a:tcPr marL="10" marR="20" marT="30" marB="40" anchorCtr="1">"#,
        r#"<a:lnL w="12700"><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></a:lnL>"#,
        r#"<a:lnB w="25400"/>"#,
        r#"</a:tcPr></a:tc></a:tr>"#
    )));
    let properties = table.cell(0, 0).expect("0,0").properties().expect("a:tcPr");
    let interner = &doc.interner;

    assert_eq!(properties.left_margin(interner).expect("marL").emu(), 10);
    assert_eq!(properties.right_margin(interner).expect("marR").emu(), 20);
    assert_eq!(properties.top_margin(interner).expect("marT").emu(), 30);
    assert_eq!(properties.bottom_margin(interner).expect("marB").emu(), 40);
    assert_eq!(properties.anchor_centered(interner), Some(true));

    let left = properties
        .border(interner, CellBorder::Left)
        .expect("a left border");
    assert_eq!(left.width(interner).expect("w").emu(), 12_700);
    assert!(properties.border(interner, CellBorder::Bottom).is_some());
    assert!(
        properties.border(interner, CellBorder::Right).is_none(),
        "an edge with no line declares no border"
    );
    assert!(properties
        .border(interner, CellBorder::TopLeftToBottomRight)
        .is_none());
}

#[test]
fn margins_have_non_zero_schema_defaults() {
    // An unstated margin is not a zero one — a renderer substitutes 0.1"/0.05".
    let (table, doc) = parse(&tbl(concat!(
        r#"<a:tblGrid><a:gridCol w="1"/></a:tblGrid>"#,
        r#"<a:tr h="1"><a:tc><a:tcPr/></a:tc></a:tr>"#
    )));
    let properties = table.cell(0, 0).expect("0,0").properties().expect("a:tcPr");
    assert_eq!(properties.left_margin(&doc.interner), None);
    assert_eq!(TableCellProperties::DEFAULT_MARGIN_HORIZONTAL.emu(), 91_440);
    assert_eq!(TableCellProperties::DEFAULT_MARGIN_VERTICAL.emu(), 45_720);
}

#[test]
fn a_style_reference_is_reported_but_not_resolved() {
    let (table, doc) = parse(&tbl(concat!(
        r#"<a:tblPr><a:tableStyleId>{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}</a:tableStyleId></a:tblPr>"#,
        r#"<a:tblGrid><a:gridCol w="1"/></a:tblGrid>"#
    )));
    assert_eq!(
        table
            .properties()
            .expect("a:tblPr")
            .table_style_id(&doc.interner),
        Some("{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}")
    );
}

#[test]
fn a_cell_and_table_fill_are_the_fill_model() {
    let (table, doc) = parse(&tbl(concat!(
        r#"<a:tblPr><a:solidFill><a:srgbClr val="00FF00"/></a:solidFill></a:tblPr>"#,
        r#"<a:tblGrid><a:gridCol w="1"/></a:tblGrid>"#,
        r#"<a:tr h="1"><a:tc><a:tcPr><a:solidFill><a:srgbClr val="0000FF"/></a:solidFill></a:tcPr></a:tc></a:tr>"#
    )));
    assert!(table
        .properties()
        .expect("a:tblPr")
        .fill(&doc.interner)
        .is_some());
    assert!(table
        .cell(0, 0)
        .expect("0,0")
        .properties()
        .expect("a:tcPr")
        .fill(&doc.interner)
        .is_some());
}

// ---------------------------------------------------------------------------------------------
// Fidelity
// ---------------------------------------------------------------------------------------------

#[test]
fn a_table_round_trips_byte_for_byte() {
    let source = simple();
    let (table, doc) = parse(&source);
    assert_round_trips(&table, doc, &source);
}

#[test]
fn what_this_tier_does_not_model_survives() {
    // A style reference, a cell3D, an extLst, an unknown attribute and an MCE-ish child: none of
    // these are interpreted here, and all of them must come back out.
    let source = tbl(concat!(
        r#"<a:tblPr firstRow="1" unknownAttr="kept">"#,
        r#"<a:tableStyleId>{GUID}</a:tableStyleId><a:extLst><a:ext uri="x"/></a:extLst>"#,
        r#"</a:tblPr>"#,
        r#"<a:tblGrid><a:gridCol w="100"><a:extLst/></a:gridCol></a:tblGrid>"#,
        r#"<a:tr h="10" custom="1">"#,
        r#"<a:tc id="c1"><a:txBody><a:bodyPr/><a:p/></a:txBody>"#,
        r#"<a:tcPr><a:cell3D prstMaterial="matte"/><a:headers><a:header>h</a:header></a:headers></a:tcPr>"#,
        r#"<a:extLst/></a:tc>"#,
        r#"</a:tr>"#
    ));
    let (table, doc) = parse(&source);

    // Read something through the model first, so the round-trip is not passing by never looking.
    assert_eq!(table.column_count(), 1);
    assert_eq!(table.cell(0, 0).expect("0,0").column_span(&doc.interner), 1);
    assert_round_trips(&table, doc, &source);
}

#[test]
fn an_empty_table_round_trips() {
    let source = tbl("");
    let (table, doc) = parse(&source);
    assert_eq!(table.row_count(), 0);
    assert_eq!(table.column_count(), 0);
    assert_round_trips(&table, doc, &source);
}

#[test]
fn editing_a_width_leaves_the_rest_of_the_element_alone() {
    let source = tbl(concat!(
        r#"<a:tblGrid><a:gridCol w="100" unknown="kept"><a:extLst/></a:gridCol></a:tblGrid>"#,
        r#"<a:tr h="10"><a:tc/></a:tr>"#
    ));
    let (mut table, mut doc) = parse(&source);

    table
        .grid_mut()
        .expect("a grid")
        .column_mut(0)
        .expect("column 0")
        .set_width(&mut doc.interner, mjx_dml::Emu::from_emu(999));
    table
        .row_mut(0)
        .expect("row 0")
        .set_height(&mut doc.interner, mjx_dml::Emu::from_emu(888));

    doc.root = table.to_xml(&mut doc.interner);
    let out = String::from_utf8(fidelity::serialize_to_vec(&doc)).expect("utf-8");

    assert!(out.contains(r#"w="999""#), "{out}");
    assert!(out.contains(r#"h="888""#), "{out}");
    assert!(out.contains(r#"unknown="kept""#), "{out}");
    assert!(out.contains("<a:extLst/>"), "{out}");
}

#[test]
fn a_cells_text_can_be_edited_through_the_text_tree() {
    let source = simple();
    let (mut table, mut doc) = parse(&source);

    let body = table
        .cell_mut(1, 1)
        .expect("1,1")
        .text_body_mut()
        .expect("a text body");
    let run = body
        .paragraphs_mut()
        .next()
        .expect("a paragraph")
        .runs_mut()
        .next()
        .expect("a run");
    run.set_text("99");

    assert_eq!(table.cell(1, 1).expect("1,1").text(), "99");
    doc.root = table.to_xml(&mut doc.interner);
    let out = String::from_utf8(fidelity::serialize_to_vec(&doc)).expect("utf-8");
    assert!(out.contains("<a:t>99</a:t>"), "{out}");
    assert!(
        out.contains("<a:t>North</a:t>"),
        "other cells untouched: {out}"
    );
}

// ---------------------------------------------------------------------------------------------
// Writing cell properties
// ---------------------------------------------------------------------------------------------

#[test]
fn setting_a_border_keeps_what_this_tier_does_not_model() {
    // `a:tcPr` carries a `cell3D`, a `headers` and an unknown attribute, none of which this tier
    // interprets. A writer that rebuilt the element instead of merging into it would drop all three.
    let source = tbl(concat!(
        r#"<a:tblGrid><a:gridCol w="1"/></a:tblGrid>"#,
        r#"<a:tr h="1"><a:tc><a:tcPr anchor="ctr" unknown="kept">"#,
        r#"<a:cell3D prstMaterial="matte"/>"#,
        r#"<a:headers><a:header>h</a:header></a:headers>"#,
        r#"</a:tcPr></a:tc></a:tr>"#
    ));
    let (mut table, mut doc) = parse(&source);

    table
        .cell_mut(0, 0)
        .expect("0,0")
        .properties_mut()
        .expect("a:tcPr")
        .set_border(
            &mut doc.interner,
            CellBorder::Left,
            Some(&LineSpec::default()),
        );

    doc.root = table.to_xml(&mut doc.interner);
    let out = String::from_utf8(fidelity::serialize_to_vec(&doc)).expect("utf-8");

    assert!(out.contains("<a:lnL"), "the border was written: {out}");
    assert!(out.contains(r#"unknown="kept""#), "{out}");
    assert!(out.contains("<a:cell3D"), "{out}");
    assert!(out.contains("<a:header>h</a:header>"), "{out}");
    assert!(out.contains(r#"anchor="ctr""#), "{out}");
}

#[test]
fn a_new_border_lands_before_the_children_it_must_precede() {
    // The sequence is the six borders, then `cell3D`, the fill, `headers`, `extLst`. A border added
    // to a cell that already has the later children must go in front of them, not at the end.
    let source = tbl(concat!(
        r#"<a:tblGrid><a:gridCol w="1"/></a:tblGrid>"#,
        r#"<a:tr h="1"><a:tc><a:tcPr>"#,
        r#"<a:cell3D prstMaterial="matte"/><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill>"#,
        r#"</a:tcPr></a:tc></a:tr>"#
    ));
    let (mut table, mut doc) = parse(&source);

    table
        .cell_mut(0, 0)
        .expect("0,0")
        .properties_mut()
        .expect("a:tcPr")
        .set_border(
            &mut doc.interner,
            CellBorder::Bottom,
            Some(&LineSpec::default()),
        );

    doc.root = table.to_xml(&mut doc.interner);
    let out = String::from_utf8(fidelity::serialize_to_vec(&doc)).expect("utf-8");
    let at = |needle: &str| {
        out.find(needle)
            .unwrap_or_else(|| panic!("{needle}: {out}"))
    };
    assert!(at("<a:lnB") < at("<a:cell3D"), "{out}");
    assert!(at("<a:cell3D") < at("<a:solidFill"), "{out}");
}

#[test]
fn removing_a_border_leaves_the_others() {
    let source = tbl(concat!(
        r#"<a:tblGrid><a:gridCol w="1"/></a:tblGrid>"#,
        r#"<a:tr h="1"><a:tc><a:tcPr><a:lnL/><a:lnR/></a:tcPr></a:tc></a:tr>"#
    ));
    let (mut table, mut doc) = parse(&source);
    let properties = table
        .cell_mut(0, 0)
        .expect("0,0")
        .properties_mut()
        .expect("a:tcPr");

    properties.set_border(&mut doc.interner, CellBorder::Left, None);
    assert!(properties.border(&doc.interner, CellBorder::Left).is_none());
    assert!(properties
        .border(&doc.interner, CellBorder::Right)
        .is_some());
}

#[test]
fn a_default_span_is_removed_rather_than_written() {
    // `gridSpan="1"` and `hMerge="0"` are what the schema already assumes. Writing them would add
    // noise to every table this library touches and make a plain cell look like a decision.
    let source = tbl(concat!(
        r#"<a:tblGrid><a:gridCol w="1"/></a:tblGrid>"#,
        r#"<a:tr h="1"><a:tc gridSpan="2" rowSpan="3" hMerge="1" vMerge="1"/></a:tr>"#
    ));
    let (mut table, mut doc) = parse(&source);

    table
        .cell_mut(0, 0)
        .expect("0,0")
        .clear_merge(&mut doc.interner);

    doc.root = table.to_xml(&mut doc.interner);
    let out = String::from_utf8(fidelity::serialize_to_vec(&doc)).expect("utf-8");
    for attribute in ["gridSpan", "rowSpan", "hMerge", "vMerge"] {
        assert!(!out.contains(attribute), "{attribute} survived: {out}");
    }
}

#[test]
fn setting_a_merge_leaves_the_cells_own_content_alone() {
    let source = tbl(concat!(
        r#"<a:tblGrid><a:gridCol w="1"/></a:tblGrid>"#,
        r#"<a:tr h="1"><a:tc id="c1"><a:txBody><a:bodyPr/><a:p><a:r><a:t>kept</a:t></a:r></a:p>"#,
        r#"</a:txBody><a:tcPr anchor="ctr"/></a:tc></a:tr>"#
    ));
    let (mut table, mut doc) = parse(&source);

    let cell = table.cell_mut(0, 0).expect("0,0");
    cell.set_spans(&mut doc.interner, 2, 1);
    cell.set_merged(&mut doc.interner, false, true);

    doc.root = table.to_xml(&mut doc.interner);
    let out = String::from_utf8(fidelity::serialize_to_vec(&doc)).expect("utf-8");
    assert!(out.contains(r#"gridSpan="2""#), "{out}");
    assert!(out.contains(r#"vMerge="1""#), "{out}");
    assert!(!out.contains("hMerge"), "an unset flag is absent: {out}");
    // The merge attributes are the only change — the text, the properties and the id are untouched.
    assert!(out.contains("<a:t>kept</a:t>"), "{out}");
    assert!(out.contains(r#"anchor="ctr""#), "{out}");
    assert!(out.contains(r#"id="c1""#), "{out}");
}
