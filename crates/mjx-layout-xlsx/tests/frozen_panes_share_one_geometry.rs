//! Frozen and split panes: up to four independently scrolled regions over **one** grid geometry.
//!
//! The property worth asserting is not that four rectangles appear — it is that the frozen half and
//! the scrolling half measure the *same* rows and columns. A renderer holding two geometries would
//! draw a frozen header one pixel taller than the row beneath it, at every zoom, and nothing about
//! the count of regions would notice.

mod support;

use mjx_layout_xlsx::PaneSplit;
use mjx_ooxml_types::spreadsheetml::{Pane, PaneState};

use support::{grid_from, model, styles, viewport};

/// A sheet frozen at `B2` — one row and one column pinned — with sixty rows of content below.
fn frozen_sheet() -> String {
    let mut rows = String::new();
    for row in 1..=60 {
        rows.push_str(&format!(
            r#"<row r="{row}" ht="{height}" customHeight="true"><c r="A{row}" t="inlineStr"><is><t>r{row}</t></is></c><c r="C{row}" t="inlineStr"><is><t>c{row}</t></is></c></row>"#,
            height = if row == 1 { 30 } else { 15 }
        ));
    }
    format!(
        r#"<sheetFormatPr defaultRowHeight="15" defaultColWidth="9"/>
<sheetViews><sheetView workbookViewId="0"><pane xSplit="1" ySplit="1" topLeftCell="B2" activePane="bottomRight" state="frozen"/></sheetView></sheetViews>
<sheetData>{rows}</sheetData>"#
    )
}

#[test]
fn a_frozen_pane_is_read_as_a_count_of_rows_and_columns() {
    let grid = grid_from(&frozen_sheet(), &styles(&[], ""));
    let split = grid.split();
    assert_eq!(split.state, PaneState::Frozen);
    assert_eq!(split.frozen_columns, 1, "column A is pinned");
    assert_eq!(split.frozen_rows, 1, "row 1 is pinned");
    assert_eq!(split.active, Pane::BottomRight);
    assert_eq!(split.top_left, Some((1, 1)), "B2, zero-based");
    assert!(split.is_divided());
    assert!(split.divides_rows() && split.divides_columns());
}

#[test]
fn the_window_becomes_four_regions_and_each_shows_a_different_part_of_the_sheet() {
    let grid = grid_from(&frozen_sheet(), &styles(&[], ""));
    let constraints = viewport(6.0, 2.0);
    let mut model = model();
    let _ = support::lay_out(&mut model, &grid, &constraints, 0);

    let regions = model.catalogue().regions();
    assert_eq!(
        regions.len(),
        4,
        "a sheet frozen on both axes shows four panes: {regions:?}"
    );
    let kinds: std::collections::BTreeSet<&str> = regions
        .iter()
        .map(|region| match region.pane {
            Pane::TopLeft => "topLeft",
            Pane::TopRight => "topRight",
            Pane::BottomLeft => "bottomLeft",
            Pane::BottomRight => "bottomRight",
        })
        .collect();
    assert_eq!(kinds.len(), 4, "all four, once each: {kinds:?}");

    // The frozen corner shows row 1 and column A alone.
    let corner = regions
        .iter()
        .find(|region| region.pane == Pane::TopLeft)
        .expect("a frozen corner");
    assert_eq!(corner.rows, 0..1);
    assert_eq!(corner.columns, 0..1);
    assert!(corner.rows_are_frozen && corner.columns_are_frozen);

    // The scrolling region starts past both.
    let scrolling = regions
        .iter()
        .find(|region| region.pane == Pane::BottomRight)
        .expect("a scrolling region");
    assert_eq!(scrolling.rows.start, 0, "band zero starts at row 1");
    assert!(!scrolling.rows_are_frozen && !scrolling.columns_are_frozen);
    assert!(
        scrolling.view.left > corner.view.left,
        "and it sits to the right of the frozen column"
    );
    assert!(
        scrolling.view.top > corner.view.top,
        "and below the frozen row"
    );
}

#[test]
fn the_two_halves_measure_the_same_rows_and_the_same_columns() {
    // **The assertion the whole module exists for.** Row 1 is 30 points tall and every other row is
    // 15, so a second geometry would show up immediately as a frozen header of the wrong height.
    let grid = grid_from(&frozen_sheet(), &styles(&[], ""));
    let constraints = viewport(6.0, 2.0);
    let mut model = model();
    let tree = support::lay_out(&mut model, &grid, &constraints, 0);

    let cells = support::cells(&tree);
    let frozen_header = cells
        .iter()
        .find(|(row, column, _)| *row == 0 && *column == 0)
        .expect("A1, in the frozen corner");
    let frozen_row_in_the_scrolling_half = cells
        .iter()
        .find(|(row, column, _)| *row == 0 && *column >= 2)
        .expect("C1, in the frozen top-right pane");
    assert_eq!(
        frozen_header.2.height(),
        frozen_row_in_the_scrolling_half.2.height(),
        "row 1 is the same height in both panes it appears in"
    );

    let frozen_column = cells
        .iter()
        .find(|(row, column, _)| *row >= 1 && *column == 0)
        .expect("a cell in the frozen left column");
    assert_eq!(
        frozen_column.2.width(),
        frozen_header.2.width(),
        "column A is the same width in both panes it appears in"
    );
}

#[test]
fn a_frozen_row_appears_on_every_band_and_a_scrolled_one_does_not() {
    // This is what freezing *means*, and it is the assertion "fragment positions across the split"
    // is really about: scroll two bands down and row 1 is still there.
    let grid = grid_from(&frozen_sheet(), &styles(&[], ""));
    let constraints = viewport(6.0, 1.0);
    let mut model = model();

    let first = support::lay_out(&mut model, &grid, &constraints, 0);
    let rows_on_first: std::collections::BTreeSet<u32> = support::cells(&first)
        .into_iter()
        .map(|(row, ..)| row)
        .collect();

    let later = support::lay_out(&mut model, &grid, &constraints, 3);
    let rows_on_later: std::collections::BTreeSet<u32> = support::cells(&later)
        .into_iter()
        .map(|(row, ..)| row)
        .collect();

    assert!(rows_on_first.contains(&0), "row 1 is on band zero");
    assert!(
        rows_on_later.contains(&0),
        "and still on band three, because it is frozen: {rows_on_later:?}"
    );
    assert!(
        rows_on_later.iter().any(|row| *row > 5),
        "while the scrolling half has moved on: {rows_on_later:?}"
    );
    assert_ne!(
        rows_on_first, rows_on_later,
        "a band that showed the same rows as the first would mean nothing scrolled"
    );
}

#[test]
fn a_sheet_with_no_pane_is_one_region() {
    // The other value of the parameter. A gate that only ever saw a frozen sheet could not tell a
    // correct implementation from one that always produces four regions.
    let grid = grid_from(
        r#"<sheetFormatPr defaultRowHeight="15" defaultColWidth="9"/>
<sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>plain</t></is></c></row></sheetData>"#,
        &styles(&[], ""),
    );
    assert_eq!(*grid.split(), PaneSplit::default());
    assert!(!grid.split().is_divided());

    let constraints = viewport(6.0, 2.0);
    let mut model = model();
    let _ = support::lay_out(&mut model, &grid, &constraints, 0);
    assert_eq!(model.catalogue().regions().len(), 1);
}

#[test]
fn a_split_pane_is_read_in_twentieths_of_a_point_rather_than_as_a_row_count() {
    // `@ySplit` means two different things depending on `@state`, and reading a split's as a row
    // count would freeze eleven hundred rows instead of dividing the window an inch down.
    let grid = grid_from(
        r#"<sheetFormatPr defaultRowHeight="15" defaultColWidth="9"/>
<sheetViews><sheetView workbookViewId="0"><pane ySplit="1440" topLeftCell="A5" activePane="bottomLeft" state="split"/></sheetView></sheetViews>
<sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>x</t></is></c></row></sheetData>"#,
        &styles(&[], ""),
    );
    let split = grid.split();
    assert_eq!(split.state, PaneState::Split);
    assert_eq!(
        split.frozen_rows, 0,
        "a split freezes nothing; it divides the window"
    );
    assert_eq!(
        split.split_y,
        mjx_ooxml_core::measure::Emu::from_points(72.0),
        "1440 twentieths of a point is one inch"
    );
    assert!(split.divides_rows() && !split.divides_columns());
}
