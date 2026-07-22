//! Unit tests for the table-style model (`tableStyles.xml`), through the public API only.
//!
//! The model exists so a table's `a:tableStyleId` **resolves**, and so a later tier can read what a
//! part of a table is formatted as. Two things get the attention: that every accessor reaches the
//! DrawingML it reuses (fills, borders, colours, the tri-state bold/italic, theme references), and
//! that the whole part round-trips byte-for-byte — a table style carries an `extLst`, a `cell3D` and
//! theme references this tier reuses but does not rebuild.

use mjx_dml::{
    ColorSpec, FillSpec, FontCollectionIndex, LineSpec, OnOffStyle, TablePartStyle, TableStyle,
    TableStyleBorder, TableStyleCellStyle, TableStyleList, TableStylePart, TableStyleTextStyle,
    ThemeableLineStyle,
};
use mjx_ooxml_core::{FromXml, Interner, RawDocument, ToXml};
use mjx_xml::fidelity;

const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";

fn parse(fragment: &str) -> (TableStyleList, RawDocument) {
    let doc = fidelity::parse(fragment.as_bytes()).expect("fragment parses");
    let list = TableStyleList::from_xml(&doc.root, &doc.interner).expect("from_xml");
    (list, doc)
}

/// A `tableStyles.xml` root with one style, shaped like what PowerPoint writes for a themed table:
/// a whole-table cell fill and border, a bold-and-filled header row, a banded row, a theme-referenced
/// border, a font reference, a 3-D corner cell, and a table background.
fn styles() -> String {
    format!(r#"<a:tblStyleLst xmlns:a="{A}" def="{{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}}">"#)
        + concat!(
            r#"<a:tblStyle styleId="{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}" styleName="Medium Style 2 - Accent 1">"#,
            r#"<a:tblBg><a:fillRef idx="3"><a:schemeClr val="accent1"/></a:fillRef></a:tblBg>"#,
            r#"<a:wholeTbl>"#,
            r#"<a:tcTxStyle><a:fontRef idx="minor"/><a:schemeClr val="dk1"/></a:tcTxStyle>"#,
            r#"<a:tcStyle><a:tcBdr>"#,
            r#"<a:left><a:ln w="12700"><a:solidFill><a:schemeClr val="lt1"/></a:solidFill></a:ln></a:left>"#,
            r#"<a:top><a:lnRef idx="1"><a:schemeClr val="accent1"/></a:lnRef></a:top>"#,
            r#"</a:tcBdr><a:fill><a:solidFill><a:schemeClr val="lt1"/></a:solidFill></a:fill></a:tcStyle>"#,
            r#"</a:wholeTbl>"#,
            r#"<a:band1H><a:tcStyle><a:fill><a:solidFill><a:schemeClr val="accent1"><a:alpha val="20000"/></a:schemeClr></a:solidFill></a:fill></a:tcStyle></a:band1H>"#,
            r#"<a:firstRow>"#,
            r#"<a:tcTxStyle b="on"><a:schemeClr val="lt1"/></a:tcTxStyle>"#,
            r#"<a:tcStyle><a:fill><a:solidFill><a:schemeClr val="accent1"/></a:solidFill></a:fill><a:cell3D prstMaterial="matte"><a:bevel w="38100" h="38100"/></a:cell3D></a:tcStyle>"#,
            r#"</a:firstRow>"#,
            r#"</a:tblStyle>"#,
            r#"</a:tblStyleLst>"#,
        )
}

#[track_caller]
fn assert_round_trips(list: &TableStyleList, mut doc: RawDocument, expected: &str) {
    doc.root = list.to_xml(&mut doc.interner);
    let out = fidelity::serialize_to_vec(&doc);
    assert_eq!(
        String::from_utf8_lossy(&out),
        expected,
        "round-trip mismatch"
    );
}

#[test]
fn resolves_the_default_and_a_style_by_id() {
    let (list, doc) = parse(&styles());
    let interner = &doc.interner;

    let guid = "{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}";
    assert_eq!(list.default_style_id(interner), Some(guid));
    assert_eq!(list.styles(interner).len(), 1);

    let style = list.style(interner, guid).expect("the style resolves");
    assert_eq!(
        style.style_name(interner),
        Some("Medium Style 2 - Accent 1")
    );
    assert!(
        list.style(interner, "{00000000-0000-0000-0000-000000000000}")
            .is_none(),
        "a dangling id resolves to nothing"
    );
}

#[test]
fn a_part_reaches_its_cell_fill_and_borders() {
    let (list, doc) = parse(&styles());
    let interner = &doc.interner;
    let style = list.styles(interner).into_iter().next().expect("a style");

    let whole = style
        .part(interner, TableStylePart::WholeTable)
        .expect("wholeTbl");
    let cell = whole.cell_style(interner).expect("tcStyle");
    assert!(cell.fill(interner).is_some(), "the whole-table cell fill");

    let borders = cell.borders(interner).expect("tcBdr");
    // An explicit line on the left; a theme reference on top — the two ways a style names a border.
    assert!(matches!(
        borders.border(interner, TableStyleBorder::Left),
        Some(ThemeableLineStyle::Line(_))
    ));
    assert!(matches!(
        borders.border(interner, TableStyleBorder::Top),
        Some(ThemeableLineStyle::Reference(_))
    ));
    assert!(
        borders.border(interner, TableStyleBorder::Bottom).is_none(),
        "an edge the style leaves alone"
    );
}

#[test]
fn the_header_row_states_bold_and_a_colour_and_the_whole_table_defaults() {
    let (list, doc) = parse(&styles());
    let interner = &doc.interner;
    let style = list.styles(interner).into_iter().next().expect("a style");

    let header = style
        .part(interner, TableStylePart::FirstRow)
        .expect("firstRow");
    let text = header.text_style(interner).expect("tcTxStyle");
    assert_eq!(
        text.bold(interner),
        OnOffStyle::On,
        "the header forces bold"
    );
    assert_eq!(
        text.italic(interner),
        OnOffStyle::Default,
        "unstated italic follows the parent"
    );
    assert!(
        text.color(interner).is_some(),
        "the header states a text colour"
    );

    // The whole-table text style states neither, so both are Default (follow the inheritance chain).
    let whole_text = style
        .part(interner, TableStylePart::WholeTable)
        .and_then(|p| p.text_style(interner))
        .expect("wholeTbl text");
    assert_eq!(whole_text.bold(interner), OnOffStyle::Default);
    assert_eq!(
        whole_text
            .font_reference(interner)
            .and_then(|f| f.index(interner)),
        Some(FontCollectionIndex::Minor),
        "the whole-table text names the minor theme font"
    );
}

#[test]
fn a_cell_3d_reports_its_material_and_keeps_its_bevel() {
    let (list, doc) = parse(&styles());
    let interner = &doc.interner;
    let style = list.styles(interner).into_iter().next().expect("a style");
    let header_cell = style
        .part(interner, TableStylePart::FirstRow)
        .and_then(|p| p.cell_style(interner))
        .expect("firstRow cell");

    let cell_3d = header_cell.cell_3d(interner).expect("cell3D");
    assert_eq!(cell_3d.preset_material(interner), Some("matte"));
    // The bevel is preserved though this tier does not decompose it (that is the 3-D workstream).
    assert!(!cell_3d.children().is_empty(), "the a:bevel round-trips");
}

#[test]
fn the_background_reaches_its_fill_reference() {
    let (list, doc) = parse(&styles());
    let interner = &doc.interner;
    let style = list.styles(interner).into_iter().next().expect("a style");
    let background = style.background(interner).expect("tblBg");
    assert!(
        background.fill_reference(interner).is_some(),
        "the background names a theme fill"
    );
    assert!(background.fill(interner).is_none(), "not an explicit fill");
}

#[test]
fn a_table_style_list_round_trips_byte_for_byte() {
    let (list, doc) = parse(&styles());
    let expected = styles();
    assert_round_trips(&list, doc, &expected);
}

// ---------------------------------------------------------------------------------------------
// Authoring — building a style up from parts
// ---------------------------------------------------------------------------------------------

const GUID: &str = "{11111111-2222-3333-4444-555555555555}";

#[test]
fn a_style_can_be_authored_from_parts_and_read_back() {
    let mut interner = Interner::new();

    let mut text = TableStyleTextStyle::new(&mut interner);
    text.set_bold(&mut interner, OnOffStyle::On);
    text.set_color(&mut interner, &ColorSpec::Srgb("FFFFFF".to_owned()));

    let mut cell = TableStyleCellStyle::new(&mut interner);
    cell.set_fill(
        &mut interner,
        &FillSpec::solid(ColorSpec::Srgb("1F3864".to_owned())),
    );
    cell.set_border(
        &mut interner,
        TableStyleBorder::Bottom,
        &LineSpec::default(),
    );

    let mut part = TablePartStyle::new(&mut interner);
    part.set_text_style(&mut interner, &text);
    part.set_cell_style(&mut interner, &cell);

    let mut style = TableStyle::new(&mut interner, GUID, "Authored");
    style.set_part(&mut interner, TableStylePart::FirstRow, &part);

    let mut list = TableStyleList::new(&mut interner, GUID);
    list.upsert_style(&mut interner, &style);

    // Round-trip the built element through the reader, then read it back with the accessors.
    let element = list.to_xml(&mut interner);
    let reparsed = TableStyleList::from_xml(&element, &interner).expect("re-parse");

    assert_eq!(reparsed.default_style_id(&interner), Some(GUID));
    let read = reparsed.style(&interner, GUID).expect("the authored style");
    assert_eq!(read.style_name(&interner), Some("Authored"));

    let header = read
        .part(&interner, TableStylePart::FirstRow)
        .expect("firstRow");
    let text = header.text_style(&interner).expect("tcTxStyle");
    assert_eq!(text.bold(&interner), OnOffStyle::On);
    assert_eq!(
        text.italic(&interner),
        OnOffStyle::Default,
        "unset stays default"
    );
    assert!(text.color(&interner).is_some());

    let cell = header.cell_style(&interner).expect("tcStyle");
    assert!(
        cell.fill(&interner).is_some(),
        "the authored fill reads back"
    );
    assert!(matches!(
        cell.borders(&interner)
            .and_then(|b| b.border(&interner, TableStyleBorder::Bottom)),
        Some(ThemeableLineStyle::Line(_))
    ));
}

#[test]
fn authoring_the_same_style_id_twice_updates_it() {
    let mut interner = Interner::new();
    let mut list = TableStyleList::new(&mut interner, GUID);

    let first = TableStyle::new(&mut interner, GUID, "First");
    list.upsert_style(&mut interner, &first);
    let second = TableStyle::new(&mut interner, GUID, "Second");
    list.upsert_style(&mut interner, &second);

    assert_eq!(list.styles(&interner).len(), 1, "not duplicated");
    assert_eq!(
        list.style(&interner, GUID)
            .and_then(|s| s.style_name(&interner).map(str::to_owned)),
        Some("Second".to_owned()),
        "the later authoring won"
    );
}

#[test]
fn a_default_bold_take_writes_no_attribute() {
    // OnOffStyle::Default is the schema default (def = follow the parent), so it must not be written.
    let mut interner = Interner::new();
    let mut text = TableStyleTextStyle::new(&mut interner);
    text.set_bold(&mut interner, OnOffStyle::On);
    text.set_bold(&mut interner, OnOffStyle::Default); // back to default → the attribute goes away

    let element = text.to_xml(&mut interner);
    let reparsed = TableStyleTextStyle::from_xml(&element, &interner).expect("re-parse");
    assert_eq!(reparsed.bold(&interner), OnOffStyle::Default);
    // No @b survived.
    let has_b = element
        .attributes
        .iter()
        .any(|a| interner.resolve(a.name.local) == "b");
    assert!(!has_b, "a default take leaves no @b");
}

// ---------------------------------------------------------------------------------------------
// Position → applicable style parts (the resolution substance)
// ---------------------------------------------------------------------------------------------

use mjx_dml::{applicable_parts, TableStyleFlags};

/// A 4×4 table with a header row and column, banding on both axes — every kind of cell is present.
fn all_flags() -> TableStyleFlags {
    TableStyleFlags {
        first_row: true,
        last_row: false,
        first_column: true,
        last_column: false,
        banded_rows: true,
        banded_columns: true,
    }
}

#[test]
fn the_top_left_cell_is_the_nw_corner_over_its_edges() {
    // (0,0) is the header row and header column at once: NW corner, then column, then row, then whole.
    let parts = applicable_parts(0, 0, 4, 4, all_flags());
    assert_eq!(
        parts,
        [
            TableStylePart::NorthWestCell,
            TableStylePart::FirstColumn,
            TableStylePart::FirstRow,
            TableStylePart::WholeTable,
        ]
    );
}

#[test]
fn a_header_cell_takes_first_row_not_banding() {
    // (0,2): header row, a data column — firstRow wins, and the header row is not banded.
    let parts = applicable_parts(0, 2, 4, 4, all_flags());
    // Column 2 is the second data column (data index 1 → band2V), stacking beneath the row edge.
    assert_eq!(
        parts,
        [
            TableStylePart::FirstRow,
            TableStylePart::Band2Vertical,
            TableStylePart::WholeTable,
        ],
        "a header cell is firstRow, with column banding beneath, over wholeTbl"
    );
}

#[test]
fn columns_override_rows_but_both_stack() {
    // (0,0) already covered; check a non-corner header-column data-row cell keeps firstColumn top.
    let parts = applicable_parts(2, 0, 4, 4, all_flags());
    // Row 2 is the second data row (data index 1 → band2H), stacking beneath the column edge.
    assert_eq!(
        parts,
        [
            TableStylePart::FirstColumn,
            TableStylePart::Band2Horizontal,
            TableStylePart::WholeTable,
        ]
    );
}

#[test]
fn banding_parity_counts_data_cells_from_the_first() {
    let flags = all_flags();
    // Row 0 is the header; data rows 1,2,3 map to band1H, band2H, band1H. Column 2 is a data column
    // (data index 1 → band2V), and row banding stacks over column banding.
    assert_eq!(
        applicable_parts(1, 2, 4, 4, flags),
        [
            TableStylePart::Band1Horizontal,
            TableStylePart::Band2Vertical,
            TableStylePart::WholeTable,
        ]
    );
    assert_eq!(
        applicable_parts(2, 2, 4, 4, flags),
        [
            TableStylePart::Band2Horizontal,
            TableStylePart::Band2Vertical,
            TableStylePart::WholeTable,
        ]
    );
}

#[test]
fn a_plain_cell_with_no_flags_is_just_the_whole_table() {
    let parts = applicable_parts(1, 1, 4, 4, TableStyleFlags::default());
    assert_eq!(parts, [TableStylePart::WholeTable]);
}

#[test]
fn the_bottom_right_cell_is_the_se_corner() {
    let flags = TableStyleFlags {
        last_row: true,
        last_column: true,
        ..TableStyleFlags::default()
    };
    let parts = applicable_parts(3, 3, 4, 4, flags);
    assert_eq!(
        parts,
        [
            TableStylePart::SouthEastCell,
            TableStylePart::LastColumn,
            TableStylePart::LastRow,
            TableStylePart::WholeTable,
        ]
    );
}
