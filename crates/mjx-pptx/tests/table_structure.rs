//! Integration tests for growing and shrinking a table — inserting and removing rows and columns.
//!
//! Two invariants carry the weight here. **The grid and every row stay in step**: a column edit that
//! touches `a:tblGrid` but forgets a row (or the reverse) declares a width the rows disagree with,
//! and the file is broken. And **merges are adjusted, not left dangling**: a merge the new line falls
//! inside grows, a merge whose anchor is removed promotes the next cell of the region — which takes
//! over the anchor's text and formatting so the table looks unchanged.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_dml::Emu;
use mjx_opc::Package;
use mjx_pptx::{Cells, PptxError, Presentation, ShapeBounds};

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()))
}

fn byte_map(pkg: &Package) -> BTreeMap<String, Vec<u8>> {
    pkg.entries()
        .iter()
        .filter_map(|e| e.bytes().map(|b| (e.name.clone(), b.to_vec())))
        .collect()
}

/// A `rows`×`columns` table whose every cell holds its own `"rc"` coordinates, so a cell that moved,
/// vanished or lost its content is visible at a glance.
fn deck_with_table(rows: usize, columns: usize) -> (Presentation, usize) {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let table = pres
        .add_table(
            0,
            rows,
            columns,
            ShapeBounds::from_inches(0.5, 1.0, 8.0, 3.0),
        )
        .expect("add table");
    for row in 0..rows {
        for column in 0..columns {
            pres.set_cell_text(0, table, row, column, 0, &format!("{row}{column}"))
                .expect("text");
        }
    }
    (pres, table)
}

/// The table's text, cell by cell — asserts every position is addressable (no holes) as a side
/// effect, since a missing cell would panic the read.
fn text_grid(pres: &mut Presentation, table: usize) -> Vec<Vec<String>> {
    let (rows, columns) = pres.table_dimensions(0, table).expect("dimensions");
    (0..rows)
        .map(|row| {
            (0..columns)
                .map(|column| pres.cell_text(0, table, row, column).expect("text"))
                .collect()
        })
        .collect()
}

/// Asserts the grid's column count and every row's cell count agree — the invariant a column edit is
/// likeliest to break. `cell_text` reaching every position already proves it indirectly; this states
/// it outright.
#[track_caller]
fn assert_rectangular(pres: &mut Presentation, table: usize) {
    let (rows, columns) = pres.table_dimensions(0, table).expect("dimensions");
    for row in 0..rows {
        for column in 0..columns {
            pres.cell_span(0, table, row, column)
                .unwrap_or_else(|e| panic!("({row},{column}) not addressable: {e:?}"));
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Inserting and removing, no merges
// ---------------------------------------------------------------------------------------------

#[test]
fn a_row_can_be_inserted_in_the_middle() {
    let (mut pres, table) = deck_with_table(3, 3);
    pres.insert_row(0, table, 1).expect("insert");

    assert_eq!(pres.table_dimensions(0, table).expect("dims"), (4, 3));
    assert_rectangular(&mut pres, table);
    let grid = text_grid(&mut pres, table);
    assert_eq!(grid[0], ["00", "01", "02"], "row above unchanged");
    assert_eq!(grid[1], ["", "", ""], "the new row is empty");
    assert_eq!(grid[2], ["10", "11", "12"], "the old row 1 moved down");
}

#[test]
fn a_row_can_be_appended_at_the_end() {
    let (mut pres, table) = deck_with_table(2, 2);
    pres.insert_row(0, table, 2).expect("append");

    assert_eq!(pres.table_dimensions(0, table).expect("dims"), (3, 2));
    assert_rectangular(&mut pres, table);
    assert_eq!(text_grid(&mut pres, table)[2], ["", ""]);
}

#[test]
fn a_new_row_copies_its_neighbours_height() {
    let (mut pres, table) = deck_with_table(2, 2);
    pres.set_row_height(0, table, 1, Emu::from_points(50.0))
        .expect("set height");
    pres.insert_row(0, table, 2).expect("append beside row 1");

    assert_eq!(
        pres.row_height(0, table, 2).expect("height"),
        Some(Emu::from_points(50.0)),
        "the appended row copied the last row's height"
    );
}

#[test]
fn a_column_can_be_inserted_and_grows_the_grid_with_the_rows() {
    let (mut pres, table) = deck_with_table(3, 3);
    pres.insert_column(0, table, 1).expect("insert");

    assert_eq!(pres.table_dimensions(0, table).expect("dims"), (3, 4));
    assert_rectangular(&mut pres, table);
    let grid = text_grid(&mut pres, table);
    assert_eq!(
        grid[0],
        ["00", "", "01", "02"],
        "the new column split the row"
    );
}

#[test]
fn a_new_column_copies_its_neighbours_width() {
    let (mut pres, table) = deck_with_table(2, 2);
    pres.set_column_width(0, table, 1, Emu::from_points(120.0))
        .expect("set width");
    pres.insert_column(0, table, 2)
        .expect("append beside column 1");

    assert_eq!(
        pres.column_width(0, table, 2).expect("width"),
        Some(Emu::from_points(120.0)),
        "the appended column copied the last column's width"
    );
}

#[test]
fn a_row_can_be_removed() {
    let (mut pres, table) = deck_with_table(3, 3);
    pres.remove_row(0, table, 1).expect("remove");

    assert_eq!(pres.table_dimensions(0, table).expect("dims"), (2, 3));
    assert_rectangular(&mut pres, table);
    let grid = text_grid(&mut pres, table);
    assert_eq!(grid[0], ["00", "01", "02"]);
    assert_eq!(grid[1], ["20", "21", "22"], "row 2 closed the gap");
}

#[test]
fn a_column_can_be_removed_from_the_grid_and_every_row() {
    let (mut pres, table) = deck_with_table(3, 3);
    pres.remove_column(0, table, 0).expect("remove");

    assert_eq!(pres.table_dimensions(0, table).expect("dims"), (3, 2));
    assert_rectangular(&mut pres, table);
    assert_eq!(text_grid(&mut pres, table)[0], ["01", "02"]);
}

// ---------------------------------------------------------------------------------------------
// Refusals
// ---------------------------------------------------------------------------------------------

#[test]
fn the_last_row_and_column_cannot_be_removed() {
    let (mut pres, table) = deck_with_table(1, 1);
    assert!(matches!(
        pres.remove_row(0, table, 0),
        Err(PptxError::InvalidTableSize {
            rows: 0,
            columns: 1
        })
    ));
    assert!(matches!(
        pres.remove_column(0, table, 0),
        Err(PptxError::InvalidTableSize {
            rows: 1,
            columns: 0
        })
    ));
}

#[test]
fn inserting_or_removing_past_the_end_is_out_of_range() {
    let (mut pres, table) = deck_with_table(2, 2);
    // Inserting *at* the count appends; beyond it is an error.
    assert!(matches!(
        pres.insert_row(0, table, 3),
        Err(PptxError::TableCellOutOfRange {
            row: 3,
            rows: 2,
            columns: 2,
            ..
        })
    ));
    assert!(matches!(
        pres.insert_column(0, table, 3),
        Err(PptxError::TableCellOutOfRange {
            column: 3,
            rows: 2,
            columns: 2,
            ..
        })
    ));
    assert!(matches!(
        pres.remove_row(0, table, 2),
        Err(PptxError::TableCellOutOfRange { row: 2, .. })
    ));
    assert!(matches!(
        pres.remove_column(0, table, 2),
        Err(PptxError::TableCellOutOfRange { column: 2, .. })
    ));
}

#[test]
fn a_structural_edit_on_a_non_table_says_so() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    assert!(matches!(
        pres.insert_row(0, 0, 0),
        Err(PptxError::ShapeIsNotATable)
    ));
}

// ---------------------------------------------------------------------------------------------
// Span adjustment through the public surface
// ---------------------------------------------------------------------------------------------

#[test]
fn inserting_a_column_inside_a_merge_widens_it() {
    let (mut pres, table) = deck_with_table(3, 3);
    pres.merge_cells(0, table, Cells::rectangle(0..1, 0..3))
        .expect("merge the header row");
    pres.insert_column(0, table, 1)
        .expect("insert inside the merge");

    assert_eq!(pres.table_dimensions(0, table).expect("dims"), (3, 4));
    assert_rectangular(&mut pres, table);
    // The header still spans the whole row, now four wide.
    assert_eq!(pres.cell_span(0, table, 0, 0).expect("span"), (4, 1));
    for column in 0..4 {
        assert_eq!(
            pres.merged_cell_anchor(0, table, 0, column)
                .expect("anchor"),
            (0, 0),
            "column {column} still belongs to the header merge"
        );
    }
}

#[test]
fn removing_the_anchor_column_promotes_the_next_cell() {
    let (mut pres, table) = deck_with_table(3, 3);
    // Merge the whole first row into one cell anchored at (0,0), then delete that anchor's column.
    pres.merge_cells(0, table, Cells::rectangle(0..1, 0..3))
        .expect("merge");
    pres.remove_column(0, table, 0)
        .expect("remove the anchor column");

    assert_eq!(pres.table_dimensions(0, table).expect("dims"), (3, 2));
    assert_rectangular(&mut pres, table);
    // The header is two wide now, still one region, and it kept the anchor's text.
    assert_eq!(pres.cell_span(0, table, 0, 0).expect("span"), (2, 1));
    assert_eq!(pres.cell_text(0, table, 0, 0).expect("text"), "00");
    assert_eq!(
        pres.merged_cell_anchor(0, table, 0, 1).expect("anchor"),
        (0, 0)
    );
}

#[test]
fn removing_the_anchor_row_of_a_block_merge_keeps_the_block_square() {
    let (mut pres, table) = deck_with_table(3, 3);
    // A 2×2 block anchored at (0,0). Removing its anchor row must leave a 1×2 block anchored at the
    // row below, keeping the horizontal span and losing exactly one row.
    pres.merge_cells(0, table, Cells::rectangle(0..2, 0..2))
        .expect("merge a 2x2 block");
    pres.remove_row(0, table, 0).expect("remove the anchor row");

    assert_eq!(pres.table_dimensions(0, table).expect("dims"), (2, 3));
    assert_rectangular(&mut pres, table);
    assert_eq!(pres.cell_span(0, table, 0, 0).expect("span"), (2, 1));
    assert_eq!(
        pres.cell_text(0, table, 0, 0).expect("text"),
        "00",
        "anchor text promoted"
    );
    assert_eq!(
        pres.merged_cell_anchor(0, table, 0, 1).expect("anchor"),
        (0, 0)
    );
}

// ---------------------------------------------------------------------------------------------
// Fidelity
// ---------------------------------------------------------------------------------------------

#[test]
fn insert_then_remove_a_row_leaves_the_file_byte_identical() {
    let (pres, table) = deck_with_table(3, 3);
    let before = pres.save().expect("save");

    let mut pres = Presentation::open(&before).expect("reopen");
    pres.insert_row(0, table, 1).expect("insert");
    pres.remove_row(0, table, 1).expect("remove");

    assert_eq!(
        byte_map(&Package::open(&pres.save().expect("save")).expect("reopen")),
        byte_map(&Package::open(&before).expect("reopen")),
        "inserting a row then removing it must leave no trace"
    );
}

#[test]
fn insert_then_remove_a_column_leaves_the_file_byte_identical() {
    let (pres, table) = deck_with_table(3, 3);
    let before = pres.save().expect("save");

    let mut pres = Presentation::open(&before).expect("reopen");
    pres.insert_column(0, table, 1).expect("insert");
    pres.remove_column(0, table, 1).expect("remove");

    assert_eq!(
        byte_map(&Package::open(&pres.save().expect("save")).expect("reopen")),
        byte_map(&Package::open(&before).expect("reopen")),
        "inserting a column then removing it must leave no trace"
    );
}

#[test]
fn a_structural_edit_survives_a_save_and_reopen() {
    let (mut pres, table) = deck_with_table(3, 3);
    pres.insert_row(0, table, 1).expect("insert");
    pres.remove_column(0, table, 2).expect("remove");
    let expected = text_grid(&mut pres, table);

    let saved = pres.save().expect("save");
    let mut reopened = Presentation::open(&saved).expect("reopen");
    assert_eq!(text_grid(&mut reopened, table), expected);
    assert_eq!(reopened.table_dimensions(0, table).expect("dims"), (4, 2));
}

#[test]
fn a_structural_edit_dirties_only_the_slide_it_is_on() {
    // The `layouts.pptx` fixture carries a 1×1 table on slide 2; growing it must not touch any other
    // part (layout, master, theme, the other slides).
    let mut pres = Presentation::open(&fixture("layouts.pptx")).expect("open");
    let before = byte_map(&Package::open(&fixture("layouts.pptx")).expect("baseline"));

    let count = pres.shape_count(1).expect("count");
    let table = (0..count)
        .find(|&idx| pres.table_dimensions(1, idx).is_ok())
        .expect("a table on slide 2");
    pres.insert_row(1, table, 1).expect("append a row");

    let after = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));
    let slide = "ppt/slides/slide2.xml";
    for (name, original) in &before {
        if name == slide {
            assert_ne!(
                after.get(name),
                Some(original),
                "the slide should have changed"
            );
        } else {
            assert_eq!(after.get(name), Some(original), "dirtied {name}");
        }
    }
}
