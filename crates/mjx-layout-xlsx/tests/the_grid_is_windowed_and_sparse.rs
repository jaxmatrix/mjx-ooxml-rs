//! The two sparse indices: row heights, column widths, hidden rows and columns, and outline levels.
//!
//! # The identity-value trap this suite is written against
//!
//! A grid where every row is the default height and every column the default width exercises **one
//! value of two parameters**, and an implementation that ignored `@ht` and `@width` entirely would
//! pass every assertion made against such a sheet. So every case here states a value that differs
//! from the default, and the suite's last test counts how many distinct values the fixtures actually
//! carried.

mod support;

use mjx_layout_xlsx::geometry::{
    column_characters_to_pixels, column_pixels_to_characters, ColumnGeometry, MaximumDigitWidth,
    EMU_PER_PIXEL,
};
use mjx_ooxml_core::measure::Emu;

use support::{grid_from, styles};

/// The committed fixture that carries custom heights, custom widths, hidden rows, a hidden column,
/// outline levels on both axes and merges — the one sheet in the corpus with every knob turned.
const GRID_FIXTURE: &str = "sheet_grid.xlsx";

#[test]
fn a_row_that_states_a_height_gets_it_and_one_that_does_not_gets_the_default() {
    let grid = support::grid_of(GRID_FIXTURE, 0);
    let rows = grid.rows();

    // `sheet_grid.xlsx` writes `sheetFormatPr@defaultRowHeight="15"` and `row r="2" ht="24"`.
    assert_eq!(rows.default_row_height(), Emu::from_points(15.0));
    assert_eq!(rows.height(1), Emu::from_points(24.0), "row 2 states ht=24");
    assert_eq!(
        rows.height(0),
        Emu::from_points(15.0),
        "row 1 states no height"
    );
    assert_eq!(
        rows.height(500),
        Emu::from_points(15.0),
        "a row the file never mentions is the default and costs nothing"
    );
}

#[test]
fn a_hidden_row_occupies_nothing_and_the_row_below_moves_up() {
    let grid = support::grid_of(GRID_FIXTURE, 0);
    let rows = grid.rows();

    // Rows 3 and 4 are `hidden="true"`, and rows 1, 2 and 5 are not.
    assert!(rows.is_hidden(2), "row 3");
    assert!(rows.is_hidden(3), "row 4");
    assert!(!rows.is_hidden(4), "row 5");
    assert_eq!(rows.height(2), Emu::ZERO);

    // Row 5's top is row 1 (15) + row 2 (24) + two hidden rows (0).
    assert_eq!(rows.top(4), Emu::from_points(15.0 + 24.0));
    assert_eq!(
        rows.next_visible_row(2),
        Some(4),
        "scrolling into a hidden block lands on the first row below it"
    );
}

#[test]
fn outline_levels_are_read_on_both_axes() {
    let grid = support::grid_of(GRID_FIXTURE, 0);
    assert_eq!(
        grid.rows().outline_level(2),
        1,
        "row 3 states outlineLevel=1"
    );
    assert_eq!(
        grid.rows().outline_level(3),
        2,
        "row 4 states outlineLevel=2"
    );
    assert_eq!(grid.rows().outline_level(0), 0, "row 1 states none");
}

#[test]
fn a_column_run_covers_every_column_it_names_and_nothing_else() {
    let grid = support::grid_of(GRID_FIXTURE, 0);
    let digit = MaximumDigitWidth::ASSUMED;
    let columns = ColumnGeometry::read(grid.worksheet(), grid.format_properties(), digit)
        .expect("the col runs read");

    // `<col min="2" max="6" width="12.5" .../>` — one element, five columns.
    let wide = Emu::from_emu(column_characters_to_pixels(12.5, digit) * EMU_PER_PIXEL);
    for column in 1_u16..=5 {
        assert_eq!(columns.width(column), wide, "column {column} is in the run");
    }
    assert_ne!(
        columns.width(0),
        wide,
        "column A is outside the run and takes the sheet default"
    );
    assert_ne!(
        columns.width(6),
        wide,
        "column G is outside the run and takes the sheet default"
    );

    assert_eq!(
        columns.spans().len(),
        2,
        "two `col` elements, and the index holds two records rather than 16,384"
    );
    assert_eq!(
        columns.outline_level(7),
        2,
        "column H states outlineLevel=2"
    );
    assert!(columns.is_hidden(7), "column H is hidden");
    assert_eq!(columns.width(7), Emu::ZERO);
}

#[test]
fn a_hidden_column_run_is_skipped_in_one_step_and_not_one_per_column() {
    // The property that matters is not that the answer is right — it is that finding it does not
    // walk the columns. A run of five thousand hidden columns is one `col` element and therefore one
    // record, so `next_visible_column` steps over the whole block at once.
    let body = r#"<sheetFormatPr defaultRowHeight="15" defaultColWidth="9"/>
<cols><col min="2" max="5001" width="9" hidden="true"/></cols>
<sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>a</t></is></c></row></sheetData>"#;
    let grid = grid_from(body, &styles(&[], ""));
    let columns = ColumnGeometry::read(
        grid.worksheet(),
        grid.format_properties(),
        MaximumDigitWidth::ASSUMED,
    )
    .expect("the col run reads");

    assert_eq!(columns.spans().len(), 1, "one element, one record");
    assert_eq!(
        columns.next_visible_column(1),
        Some(5001),
        "the whole hidden block is skipped"
    );
    assert_eq!(
        columns.left(5001),
        columns.width(0),
        "five thousand hidden columns occupy no width at all"
    );
}

#[test]
fn a_row_far_down_an_empty_sheet_is_found_without_touching_the_rows_above_it() {
    // The far-corner shape, at the level of the index. `tests/sparsity_is_measured.rs` asserts the
    // same property in bytes; this one asserts the arithmetic.
    let body = r#"<sheetFormatPr defaultRowHeight="15" defaultColWidth="9"/>
<sheetData><row r="1048576"><c r="XFD1048576" t="inlineStr"><is><t>corner</t></is></c></row></sheetData>"#;
    let grid = grid_from(body, &styles(&[], ""));
    let rows = grid.rows();

    assert_eq!(
        rows.spans().len(),
        0,
        "the one row states no height, no hidden flag and no outline level, so it is not an override"
    );
    assert_eq!(rows.top(1_048_575), Emu::from_points(15.0).times(1_048_575));
    assert_eq!(
        rows.row_at(Emu::from_points(15.0).times(1_048_575)),
        Some(1_048_575),
        "the far corner is found by arithmetic, not by counting"
    );
    assert_eq!(grid.last_populated_row(), Some(1_048_575));
}

#[test]
fn the_character_to_pixel_round_trip_is_the_specification_s_own() {
    // ECMA-376 Part 1 §18.3.1.13, with Excel's own 11 pt Calibri figure of MDW = 7: the familiar
    // default column of 8.43 characters is 64 pixels wide. That number is checkable against any
    // spreadsheet's own column-width dialogue, which is what makes it worth asserting rather than
    // asserting a round trip of this code with itself.
    let digit = MaximumDigitWidth::ASSUMED;
    assert_eq!(digit.pixels(), 7);
    assert_eq!(column_characters_to_pixels(8.43, digit), 64);
    assert_eq!(column_characters_to_pixels(0.0, digit), 0);

    // And back: 64 pixels of column, gridline and margins included, is 8.43 characters.
    assert!((column_pixels_to_characters(64, digit) - 8.43).abs() < 0.01);
}

#[test]
fn the_default_column_width_comes_from_whichever_spelling_the_sheet_used() {
    let digit = MaximumDigitWidth::ASSUMED;

    // `@defaultColWidth` is a character *width* and goes through §18.3.1.13.
    let stated = grid_from(
        r#"<sheetFormatPr defaultRowHeight="15" defaultColWidth="20"/><sheetData/>"#,
        &styles(&[], ""),
    );
    let stated =
        ColumnGeometry::read(stated.worksheet(), stated.format_properties(), digit).expect("reads");
    assert_eq!(
        stated.default_column_width(),
        Emu::from_emu(column_characters_to_pixels(20.0, digit) * EMU_PER_PIXEL)
    );

    // `@baseColWidth` is a character *count*, and §18.3.1.81's `count * MDW + 5` is its pixel width.
    let base = grid_from(
        r#"<sheetFormatPr defaultRowHeight="15" baseColWidth="10"/><sheetData/>"#,
        &styles(&[], ""),
    );
    let base =
        ColumnGeometry::read(base.worksheet(), base.format_properties(), digit).expect("reads");
    assert_eq!(
        base.default_column_width(),
        Emu::from_emu((10 * 7 + 5) * EMU_PER_PIXEL),
        "ten characters of a seven-pixel digit, plus the five-pixel margin"
    );
    assert_ne!(
        base.default_column_width(),
        stated.default_column_width(),
        "the two spellings are different quantities and must not collapse into one"
    );
}

#[test]
fn zero_height_makes_every_unstated_row_hidden_without_looping() {
    // `sheetFormatPr@zeroHeight` is the one setting that makes the *default* row height zero, and a
    // `row_at` written as "divide the offset by the default" would divide by zero or loop forever.
    let body = r#"<sheetFormatPr defaultRowHeight="15" zeroHeight="true"/>
<sheetData><row r="3" ht="20" customHeight="true"><c r="A3" t="inlineStr"><is><t>x</t></is></c></row></sheetData>"#;
    let grid = grid_from(body, &styles(&[], ""));
    let rows = grid.rows();

    assert!(rows.rows_hidden_by_default());
    assert_eq!(rows.default_row_height(), Emu::ZERO);
    assert!(
        rows.is_hidden(0),
        "row 1 states nothing and is therefore hidden"
    );
    assert!(!rows.is_hidden(2), "row 3 states a height and is not");
    assert_eq!(
        rows.top(2),
        Emu::ZERO,
        "everything above it occupies nothing"
    );
    assert_eq!(rows.row_at(Emu::ZERO), Some(2));
    assert_eq!(rows.row_at(Emu::from_points(10.0)), Some(2));
}

#[test]
fn the_fixtures_carry_more_than_one_value_of_every_parameter() {
    // The identity-value instrument. A grid tested only against the sheet's defaults exercises one
    // value of two parameters, and this is the assertion that says the corpus did better.
    let grid = support::grid_of(GRID_FIXTURE, 0);
    let rows = grid.rows();
    let columns = ColumnGeometry::read(
        grid.worksheet(),
        grid.format_properties(),
        MaximumDigitWidth::ASSUMED,
    )
    .expect("reads");

    let heights: std::collections::BTreeSet<i64> =
        (0..10).map(|row| rows.height(row).emu()).collect();
    assert!(
        heights.len() >= 3,
        "a default height, a custom one and a hidden one: {heights:?}"
    );

    let widths: std::collections::BTreeSet<i64> =
        (0..10).map(|column| columns.width(column).emu()).collect();
    assert!(
        widths.len() >= 3,
        "a default width, a run width and a hidden one: {widths:?}"
    );

    let outlines: std::collections::BTreeSet<u8> =
        (0..10).map(|row| rows.outline_level(row)).collect();
    assert!(outlines.len() >= 3, "levels 0, 1 and 2: {outlines:?}");
    assert!(
        (0..10).any(|row| rows.is_hidden(row)),
        "at least one hidden row"
    );
    assert!(
        (0..10).any(|column| columns.is_hidden(column)),
        "at least one hidden column"
    );
}
