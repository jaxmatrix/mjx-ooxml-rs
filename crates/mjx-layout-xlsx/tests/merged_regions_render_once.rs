//! Merged regions — **the fixture a naive per-cell walk draws wrong.**
//!
//! A per-cell walk draws the anchor's text and border inside the *anchor's own* small rectangle,
//! leaves the covered cells drawing their own fills and gridlines through the middle of the region,
//! and produces a picture nobody would call a merge. Every assertion here is chosen so that such a
//! walk fails it:
//!
//! * the union renders **once**, from one fragment covering the whole rectangle;
//! * the covered positions produce **no fragment at all**, not even a box;
//! * the four borders resolve around the union rather than around the anchor;
//! * a merge whose anchor is off-screen is still drawn.

mod support;

use support::{grid_from, model, styles, viewport};

/// `A1:C1` merged, with a label in the anchor and a border stated on the *last* column of the run —
/// which is the authoring style Excel's own **Format Cells** produces and which an implementation
/// that only reads the anchor's `xf` loses.
const MERGED: &str = r#"<sheetFormatPr defaultRowHeight="15" defaultColWidth="9"/>
<sheetData>
<row r="1">
<c r="A1" s="1" t="inlineStr"><is><t>Merged heading</t></is></c>
<c r="C1" s="2"/>
</row>
<row r="2"><c r="A2" t="inlineStr"><is><t>below</t></is></c></row>
</sheetData>
<mergeCells count="1"><mergeCell ref="A1:C1"/></mergeCells>"#;

/// `xf` 1 states a left border, `xf` 2 a right one — so the union's left comes from the anchor and
/// its right from the far column, and only a perimeter scan finds both.
fn merged_styles() -> Vec<u8> {
    styles(
        &[
            r#"<xf numFmtId="0" fontId="0" fillId="0" borderId="1" applyBorder="1"/>"#,
            r#"<xf numFmtId="0" fontId="0" fillId="0" borderId="2" applyBorder="1"/>"#,
        ],
        r#"<borders count="3"><border><left/><right/><top/><bottom/><diagonal/></border><border><left style="thin"><color rgb="FF000000"/></left><right/><top/><bottom/><diagonal/></border><border><left/><right style="thick"><color rgb="FFFF0000"/></right><top/><bottom/><diagonal/></border></borders>"#,
    )
}

#[test]
fn the_union_is_one_fragment_and_the_covered_cells_are_none() {
    let styles_markup = merged_styles();
    let grid = grid_from(MERGED, &styles_markup);
    let constraints = viewport(10.0, 5.0);
    let mut model = model();
    let tree = support::lay_out(&mut model, &grid, &constraints, 0);

    let cells = support::cells(&tree);
    let inside: Vec<_> = cells
        .iter()
        .filter(|(row, column, _)| *row == 0 && *column < 3)
        .collect();
    assert_eq!(
        inside.len(),
        1,
        "the three merged positions render exactly one cell — the anchor: {inside:?}"
    );
    let (row, column, rect) = inside[0];
    assert_eq!((*row, *column), (0, 0));

    let geometry = model.geometry(&grid).expect("the geometry");
    let union = geometry.block_rect(0, 0, 0, 2);
    assert_eq!(
        rect.width(),
        union.width(),
        "the fragment covers the whole union, not the anchor's own column"
    );
    assert!(
        rect.width() > geometry.cell_rect(0, 0).width(),
        "which a per-cell walk would get wrong by drawing at the anchor's own width"
    );

    // The covered positions are absent entirely: no box, so no background and no gridline.
    assert!(
        !cells
            .iter()
            .any(|(row, column, _)| *row == 0 && (*column == 1 || *column == 2)),
        "B1 and C1 render nothing at all: {cells:?}"
    );
    assert!(
        cells
            .iter()
            .any(|(row, column, _)| *row == 0 && *column == 3),
        "and D1, which the merge does not cover, is still drawn"
    );
}

#[test]
fn the_span_is_reported_on_the_cell_fragment() {
    // Without this, a hit test on the middle of a merged heading would answer "some box" rather than
    // "row 1, column 1, spanning three" — which is what a spreadsheet's selection model needs.
    let grid = grid_from(MERGED, &merged_styles());
    let constraints = viewport(10.0, 5.0);
    let mut model = model();
    let tree = support::lay_out(&mut model, &grid, &constraints, 0);

    let span = tree
        .nodes()
        .find_map(|(_, node)| match node.fragment() {
            mjx_layout::Fragment::Box(box_fragment) => box_fragment
                .cell
                .filter(|cell| cell.row == 0 && cell.column == 0),
            _ => None,
        })
        .expect("the anchor");
    assert_eq!(span.column_span, 3);
    assert_eq!(span.row_span, 1);
}

#[test]
fn the_four_borders_resolve_around_the_union_rather_than_around_the_anchor() {
    let grid = grid_from(MERGED, &merged_styles());
    let constraints = viewport(10.0, 5.0);
    let mut model = model();
    let tree = support::lay_out(&mut model, &grid, &constraints, 0);

    let handle = tree
        .nodes()
        .find_map(|(_, node)| match node.fragment() {
            mjx_layout::Fragment::Box(box_fragment) => box_fragment
                .cell
                .filter(|cell| cell.row == 0 && cell.column == 0)
                .and(box_fragment.decoration),
            _ => None,
        })
        .expect("the anchor's decoration");
    let decoration = model
        .catalogue()
        .decoration(handle)
        .expect("the handle resolves");

    let left = decoration
        .borders
        .left
        .as_ref()
        .expect("the anchor states a left border");
    assert_eq!(
        left.style,
        mjx_ooxml_types::spreadsheetml::BorderStyle::Thin
    );
    let right = decoration.borders.right.as_ref().expect(
        "C1 states the right border, and the anchor does not — a walk that read only the \
                 anchor's `xf` would find nothing here",
    );
    assert_eq!(
        right.style,
        mjx_ooxml_types::spreadsheetml::BorderStyle::Thick
    );
    assert!(
        right.colour.is_some(),
        "and it carries the colour the file stated, unresolved"
    );
}

#[test]
fn a_merge_whose_anchor_is_above_the_band_is_still_drawn() {
    // A tall merge that starts on row 1 and reaches row 60. Scroll past row 1 and a walk over the
    // band's own cells finds nothing to draw — the region simply vanishes, which is the second way a
    // naive implementation loses a merge.
    let body = r#"<sheetFormatPr defaultRowHeight="15" defaultColWidth="9"/>
<sheetData>
<row r="1"><c r="A1" t="inlineStr"><is><t>tall</t></is></c></row>
<row r="60"><c r="D60" t="inlineStr"><is><t>foot</t></is></c></row>
</sheetData>
<mergeCells count="1"><mergeCell ref="A1:B60"/></mergeCells>"#;
    let grid = grid_from(body, &styles(&[], ""));
    // A band an inch tall holds about five rows, so band two starts well below row 1.
    let constraints = viewport(6.0, 1.0);
    let mut model = model();
    let tree = support::lay_out(&mut model, &grid, &constraints, 2);

    let anchors: Vec<_> = support::cells(&tree)
        .into_iter()
        .filter(|(row, column, _)| *row == 0 && *column == 0)
        .collect();
    assert_eq!(
        anchors.len(),
        1,
        "the merge is drawn on a band its anchor is not in: {anchors:?}"
    );
}

#[test]
fn the_committed_corpus_has_a_merge_and_it_lays_out() {
    // `sheet_grid.xlsx` states `A7:C7` and `E1:F2` — a horizontal merge and a rectangular one, in a
    // file this project did not author.
    let grid = support::grid_of("sheet_grid.xlsx", 0);
    assert_eq!(grid.merges().len(), 2);

    let constraints = viewport(10.0, 5.0);
    let mut model = model();
    let tree = support::lay_out(&mut model, &grid, &constraints, 0);
    let spans: Vec<_> = tree
        .nodes()
        .filter_map(|(_, node)| match node.fragment() {
            mjx_layout::Fragment::Box(box_fragment) => box_fragment
                .cell
                .filter(|cell| cell.column_span > 1 || cell.row_span > 1),
            _ => None,
        })
        .collect();
    assert_eq!(spans.len(), 2, "both merges render, and each renders once");
    assert!(
        spans.iter().any(|cell| cell.row_span > 1),
        "one of them spans rows as well as columns: {spans:?}"
    );
}
