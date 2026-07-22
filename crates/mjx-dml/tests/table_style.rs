//! Unit tests for the table-style model (`tableStyles.xml`), through the public API only.
//!
//! The model exists so a table's `a:tableStyleId` **resolves**, and so a later tier can read what a
//! part of a table is formatted as. Two things get the attention: that every accessor reaches the
//! DrawingML it reuses (fills, borders, colours, the tri-state bold/italic, theme references), and
//! that the whole part round-trips byte-for-byte — a table style carries an `extLst`, a `cell3D` and
//! theme references this tier reuses but does not rebuild.

use mjx_dml::{
    FontCollectionIndex, OnOffStyle, TableStyleBorder, TableStyleList, TableStylePart,
    ThemeableLineStyle,
};
use mjx_ooxml_core::{FromXml, RawDocument, ToXml};
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
