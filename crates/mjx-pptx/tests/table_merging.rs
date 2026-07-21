//! Integration tests for merging and unmerging table cells.
//!
//! The invariant under everything here: **merging never removes a cell**. The grid stays
//! rectangular, every position stays addressable, and a covered cell keeps the text it was holding.
//! A merge implemented by deleting cells would pass a naive "does it look merged" test and fail
//! every round-trip, every unmerge, and every subsequent `(row, column)` read.

use std::collections::BTreeMap;
use std::path::PathBuf;

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

/// A 3x3 table whose every cell holds its own coordinates as text, so a cell that moved or lost its
/// content says so.
fn deck_with_table() -> (Presentation, usize) {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let table = pres
        .add_table(0, 3, 3, ShapeBounds::from_inches(0.5, 1.0, 8.0, 3.0))
        .expect("add table");
    for row in 0..3 {
        for column in 0..3 {
            pres.set_cell_text(0, table, row, column, 0, &format!("{row}{column}"))
                .expect("text");
        }
    }
    (pres, table)
}

/// Every cell's span and anchor, as a grid, for comparing a whole table at once.
fn merge_map(pres: &mut Presentation, table: usize) -> Vec<((usize, usize), (usize, usize))> {
    let (rows, columns) = pres.table_dimensions(0, table).expect("dimensions");
    let mut map = Vec::new();
    for row in 0..rows {
        for column in 0..columns {
            map.push((
                pres.cell_span(0, table, row, column).expect("span"),
                pres.merged_cell_anchor(0, table, row, column)
                    .expect("anchor"),
            ));
        }
    }
    map
}

// ---------------------------------------------------------------------------------------------
// Merging
// ---------------------------------------------------------------------------------------------

#[test]
fn a_header_spans_three_columns() {
    let (mut pres, table) = deck_with_table();
    pres.merge_cells(0, table, Cells::rectangle(0..1, 0..3))
        .expect("merge the header");

    assert_eq!(pres.cell_span(0, table, 0, 0).expect("span"), (3, 1));
    for column in 0..3 {
        assert_eq!(
            pres.merged_cell_anchor(0, table, 0, column)
                .expect("anchor"),
            (0, 0),
            "column {column} should point at the anchor"
        );
    }
    // The row below is untouched.
    assert_eq!(pres.cell_span(0, table, 1, 0).expect("span"), (1, 1));
    assert_eq!(
        pres.merged_cell_anchor(0, table, 1, 1).expect("anchor"),
        (1, 1)
    );
}

#[test]
fn a_merge_can_run_down_as_well_as_across() {
    let (mut pres, table) = deck_with_table();
    pres.merge_cells(0, table, Cells::rectangle(0..3, 1..2))
        .expect("merge a column");

    assert_eq!(pres.cell_span(0, table, 0, 1).expect("span"), (1, 3));
    for row in 0..3 {
        assert_eq!(
            pres.merged_cell_anchor(0, table, row, 1).expect("anchor"),
            (0, 1)
        );
    }
}

#[test]
fn a_rectangle_merges_in_both_directions_at_once() {
    // The cell at the bottom-right of the region is covered from the left *and* from above.
    let (mut pres, table) = deck_with_table();
    pres.merge_cells(0, table, Cells::rectangle(1..3, 1..3))
        .expect("merge a block");

    assert_eq!(pres.cell_span(0, table, 1, 1).expect("span"), (2, 2));
    for (row, column) in [(1, 2), (2, 1), (2, 2)] {
        assert_eq!(
            pres.merged_cell_anchor(0, table, row, column)
                .expect("anchor"),
            (1, 1),
            "cell {row},{column}"
        );
    }
}

#[test]
fn a_row_selection_merges_that_row() {
    // `Cells::row` describes a region as well as it describes a selection.
    let (mut pres, table) = deck_with_table();
    pres.merge_cells(0, table, Cells::row(2))
        .expect("merge a row");
    assert_eq!(pres.cell_span(0, table, 2, 0).expect("span"), (3, 1));
}

#[test]
fn merging_never_removes_a_cell() {
    let (mut pres, table) = deck_with_table();
    pres.merge_cells(0, table, Cells::all()).expect("merge all");

    // The table is still 3x3 and every position still answers.
    assert_eq!(pres.table_dimensions(0, table).expect("dims"), (3, 3));
    for row in 0..3 {
        for column in 0..3 {
            assert_eq!(
                pres.cell_text(0, table, row, column).expect("text"),
                format!("{row}{column}"),
                "cell {row},{column} lost its text"
            );
        }
    }
}

#[test]
fn merging_one_cell_or_nothing_changes_nothing() {
    let (pres, table) = deck_with_table();
    let before = pres.save().expect("save");

    let mut single = Presentation::open(&before).expect("reopen");
    single
        .merge_cells(0, table, Cells::one(1, 1))
        .expect("a one-cell merge");
    let mut empty = Presentation::open(&before).expect("reopen");
    empty
        .merge_cells(0, table, Cells::rectangle(1..1, 0..3))
        .expect("an empty merge");

    let original = byte_map(&Package::open(&before).expect("reopen"));
    for deck in [single, empty] {
        assert_eq!(
            byte_map(&Package::open(&deck.save().expect("save")).expect("reopen")),
            original
        );
    }
}

#[test]
fn a_merge_survives_a_save_and_reopen() {
    let (mut pres, table) = deck_with_table();
    pres.merge_cells(0, table, Cells::rectangle(0..2, 0..2))
        .expect("merge");
    let expected = merge_map(&mut pres, table);

    let saved = pres.save().expect("save");
    let mut reopened = Presentation::open(&saved).expect("reopen");
    assert_eq!(merge_map(&mut reopened, table), expected);
}

// ---------------------------------------------------------------------------------------------
// Overlapping merges
// ---------------------------------------------------------------------------------------------

#[test]
fn a_merge_inside_the_selection_is_absorbed() {
    let (mut pres, table) = deck_with_table();
    pres.merge_cells(0, table, Cells::rectangle(0..1, 0..2))
        .expect("a small merge");

    pres.merge_cells(0, table, Cells::rectangle(0..2, 0..3))
        .expect("a bigger one over it");

    assert_eq!(pres.cell_span(0, table, 0, 0).expect("span"), (3, 2));
    // The old anchor's span is gone — it is a covered cell now, not a region of its own.
    assert_eq!(pres.cell_span(0, table, 0, 1).expect("span"), (1, 1));
    for row in 0..2 {
        for column in 0..3 {
            assert_eq!(
                pres.merged_cell_anchor(0, table, row, column)
                    .expect("anchor"),
                (0, 0)
            );
        }
    }
}

#[test]
fn a_merge_reaching_outside_the_selection_is_refused() {
    // Truncating it would leave the table claiming a span that no longer fits; growing the
    // selection would merge cells the caller never named. Neither is a defensible guess.
    let (mut pres, table) = deck_with_table();
    pres.merge_cells(0, table, Cells::rectangle(0..1, 0..3))
        .expect("merge the whole first row");

    assert!(matches!(
        pres.merge_cells(0, table, Cells::rectangle(0..2, 0..2)),
        Err(PptxError::TableMergeCrossesSelection { .. })
    ));
}

#[test]
fn a_refused_merge_changes_nothing() {
    let (mut pres, table) = deck_with_table();
    pres.merge_cells(0, table, Cells::rectangle(0..1, 0..3))
        .expect("merge a row");
    let before = pres.save().expect("save");

    let mut pres = Presentation::open(&before).expect("reopen");
    let _ = pres
        .merge_cells(0, table, Cells::rectangle(0..2, 0..2))
        .expect_err("refused");

    assert_eq!(
        byte_map(&Package::open(&pres.save().expect("save")).expect("reopen")),
        byte_map(&Package::open(&before).expect("reopen")),
        "a refused merge must not have half-written itself"
    );
}

// ---------------------------------------------------------------------------------------------
// Unmerging
// ---------------------------------------------------------------------------------------------

#[test]
fn unmerging_gives_back_every_cell_and_its_text() {
    let (mut pres, table) = deck_with_table();
    let before = merge_map(&mut pres, table);

    pres.merge_cells(0, table, Cells::rectangle(0..2, 0..3))
        .expect("merge");
    pres.unmerge_cells(0, table, 0, 0).expect("unmerge");

    assert_eq!(
        merge_map(&mut pres, table),
        before,
        "the table is as it was"
    );
    for row in 0..3 {
        for column in 0..3 {
            assert_eq!(
                pres.cell_text(0, table, row, column).expect("text"),
                format!("{row}{column}")
            );
        }
    }
}

#[test]
fn unmerging_can_be_asked_of_any_cell_of_the_region() {
    // A caller reading a table finds a covered cell as often as the anchor.
    let (mut pres, table) = deck_with_table();
    pres.merge_cells(0, table, Cells::rectangle(1..3, 1..3))
        .expect("merge");

    pres.unmerge_cells(0, table, 2, 2)
        .expect("unmerge via a covered cell");

    assert_eq!(pres.cell_span(0, table, 1, 1).expect("span"), (1, 1));
    assert_eq!(
        pres.merged_cell_anchor(0, table, 2, 2).expect("anchor"),
        (2, 2)
    );
}

#[test]
fn unmerging_a_cell_that_is_not_merged_does_nothing() {
    let (pres, table) = deck_with_table();
    let before = pres.save().expect("save");

    let mut pres = Presentation::open(&before).expect("reopen");
    pres.unmerge_cells(0, table, 1, 1).expect("a no-op unmerge");

    assert_eq!(
        byte_map(&Package::open(&pres.save().expect("save")).expect("reopen")),
        byte_map(&Package::open(&before).expect("reopen"))
    );
}

#[test]
fn a_merge_and_an_unmerge_leave_the_file_as_they_found_it() {
    // The strongest statement available: not merely equivalent, byte-identical.
    let (pres, table) = deck_with_table();
    let before = pres.save().expect("save");

    let mut pres = Presentation::open(&before).expect("reopen");
    pres.merge_cells(0, table, Cells::rectangle(0..2, 1..3))
        .expect("merge");
    pres.unmerge_cells(0, table, 1, 2).expect("unmerge");

    assert_eq!(
        byte_map(&Package::open(&pres.save().expect("save")).expect("reopen")),
        byte_map(&Package::open(&before).expect("reopen")),
        "unmerging must leave no trace, not even a gridSpan of one"
    );
}

// ---------------------------------------------------------------------------------------------
// Errors and fidelity
// ---------------------------------------------------------------------------------------------

#[test]
fn merging_past_the_edge_reports_the_tables_shape() {
    let (mut pres, table) = deck_with_table();
    assert!(matches!(
        pres.merge_cells(0, table, Cells::rectangle(0..9, 0..2)),
        Err(PptxError::TableCellOutOfRange {
            rows: 3,
            columns: 3,
            ..
        })
    ));
    assert!(matches!(
        pres.unmerge_cells(0, table, 7, 0),
        Err(PptxError::TableCellOutOfRange { row: 7, .. })
    ));
}

#[test]
fn merging_a_shape_that_frames_no_table_says_so() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    assert!(matches!(
        pres.merge_cells(0, 0, Cells::all()),
        Err(PptxError::ShapeIsNotATable)
    ));
}

#[test]
fn merging_dirties_only_the_slide_it_is_on() {
    let mut pres = Presentation::open(&fixture("layouts.pptx")).expect("open");
    let before = byte_map(&Package::open(&fixture("layouts.pptx")).expect("baseline"));
    let count = pres.shape_count(1).expect("count");
    let table = (0..count)
        .find(|&idx| pres.table_dimensions(1, idx).is_ok())
        .expect("the fixture table");

    // The fixture table is 1x1, so this is a no-op merge — but it must still not touch other parts.
    pres.merge_cells(1, table, Cells::all()).expect("merge");

    let after = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));
    for (name, original) in &before {
        if name.ends_with("slide2.xml") {
            continue;
        }
        assert_eq!(after.get(name), Some(original), "dirtied {name}");
    }
}
