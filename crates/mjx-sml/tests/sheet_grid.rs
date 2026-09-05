//! **MJXOFF-117's markup gate.** The sheet grid: merged ranges, row and column geometry, outline
//! levels, page breaks, sheet protection and scenarios.
//!
//! `crates/mjx-sml/tests/worksheet_spine.rs` (MJXOFF-102) pins the *frame* — that thirty-nine slots
//! come back in schema order whether or not they are modelled. This file pins what six of those
//! slots now **mean**, and one thing that is not a slot at all: the run-length column geometry a
//! `col` describes.
//!
//! # The fixture, and the one thing it is authored to make possible
//!
//! `tests/fixtures/sheet_grid.xlsx` carries a **wide** column run — `<col min="2" max="6" …/>`, five
//! columns sharing one width. That is deliberate and it is the whole point of the fixture: a split
//! that quietly edited the run rather than breaking it apart would change the width of B, C, D, E
//! *and* F, and a fixture whose runs were one column wide could not tell the difference. Every
//! splitting case below asserts on the columns it was **not** asked about.
//!
//! It also carries two non-overlapping merges, hidden rows and a hidden column, an outline two
//! levels deep in both axes, a manual page break beside an automatic one, a protected sheet with a
//! legacy `password` *and* the modern hash family, a protected range, and a scenario. And, as
//! `worksheet_spine.xlsx` does, one thing no rebuild reproduces: **two spaces** inside
//! `<sheetProtection …>`'s start tag, which is what makes
//! [`a_protected_sheets_hash_survives_an_unrelated_edit`] a comparison against the file rather than
//! against a second run of this crate's writer.
//!
//! # Protection is never treated as security here
//!
//! No test in this file computes a hash, verifies one, or asks whether a password is right, because
//! no call in this workspace can. What is asserted is that the five hash attributes come back byte
//! for byte — which is the entire contract.

use mjx_opc::{Package, PartName};
use mjx_sml::{
    CellRange, CellReference, CellSpan, CellValue, ColumnWidth, GridAnomaly, RowHeight, SmlError,
    WorksheetPart,
};

/// The bytes of one part of one committed fixture.
fn part_bytes(fixture: &str, part: &str) -> Vec<u8> {
    let bytes = mjx_fixtures::fixture(fixture);
    let package = Package::open(&bytes).expect("the fixture opens");
    let name = PartName::new(part).expect("a part name");
    package
        .part_bytes(&name)
        .expect("the part is there")
        .to_vec()
}

/// The grid fixture's own worksheet part.
fn grid_bytes() -> Vec<u8> {
    part_bytes("sheet_grid.xlsx", "/xl/worksheets/sheet1.xml")
}

/// Reads a worksheet part, insisting that it is one.
fn read(bytes: &[u8]) -> WorksheetPart {
    WorksheetPart::read_part(bytes)
        .expect("the worksheet reads")
        .expect("the root is an x:worksheet")
}

/// The grid fixture, read.
fn grid() -> WorksheetPart {
    read(&grid_bytes())
}

/// A worksheet part around `body`, in the SpreadsheetML namespace under the `x` prefix.
fn worksheet(body: &str) -> WorksheetPart {
    let markup = format!(
        "<x:worksheet xmlns:x=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">\
         {body}</x:worksheet>"
    );
    read(markup.as_bytes())
}

/// A cell reference, or a panic naming it.
fn cell(text: &str) -> CellReference {
    CellReference::parse(text).unwrap_or_else(|error| panic!("{text} is a cell reference: {error}"))
}

/// A range, or a panic naming it.
fn range(text: &str) -> CellRange {
    CellRange::parse(text).unwrap_or_else(|error| panic!("{text} is a range: {error}"))
}

/// A zero-based column span over `first..=last`, spelled in the letters a reader recognises.
fn columns(first: u16, last: u16) -> CellSpan {
    CellSpan::new(first, last).expect("a column span inside the grid")
}

/// Every `col` run of `sheet`, as `(min, max, width, custom)` — one-based, as the wire writes them.
fn runs(sheet: &WorksheetPart) -> Vec<(u32, u32, Option<f64>, bool)> {
    let mut found = Vec::new();
    for block in sheet.column_blocks() {
        for run in block.runs() {
            found.push((
                run.first_column(sheet.interner()).expect("@min"),
                run.last_column(sheet.interner()).expect("@max"),
                run.width(sheet.interner()).expect("@width"),
                run.custom_width(sheet.interner()).expect("@customWidth"),
            ));
        }
    }
    found
}

/// The width in force on `column` (zero-based), or `None` where no run covers it.
fn width_of(sheet: &WorksheetPart, column: u16) -> Option<f64> {
    sheet
        .column_run_covering(column)
        .expect("the runs have bounds")
        .and_then(|run| run.width(sheet.interner()).expect("@width"))
}

// -------------------------------------------------------------------------------------------
// Reading the six promoted slots
// -------------------------------------------------------------------------------------------

/// Every slot this child promoted from `Raw` to modelled reads back as the file wrote it.
///
/// One test over all six, because the claim is one claim: the frame that held them as markup now
/// hands back a type, and nothing was lost on the way.
#[test]
fn every_promoted_slot_reads_back_as_the_file_wrote_it() {
    let sheet = grid();
    let interner = sheet.interner();

    let merges = sheet.merged_ranges().expect("the merges parse");
    assert_eq!(
        merges
            .iter()
            .map(|range| range.text().as_str().to_owned())
            .collect::<Vec<_>>(),
        ["A7:C7", "E1:F2"]
    );
    assert_eq!(
        sheet
            .merged_cells()
            .expect("mergeCells")
            .declared_count(interner),
        Ok(Some(2))
    );

    let protection = sheet.protection().expect("sheetProtection");
    assert_eq!(
        protection
            .legacy_password_hash(interner)
            .unwrap()
            .as_deref(),
        Some("CC1A")
    );
    assert_eq!(
        protection.hash_algorithm_name(interner).unwrap().as_deref(),
        Some("SHA-512")
    );
    assert_eq!(
        protection.hash_value(interner).unwrap().as_deref(),
        Some("bm90LWEtcmVhbC1oYXNoIQ==")
    );
    assert_eq!(
        protection.salt_value(interner).unwrap().as_deref(),
        Some("c2FsdHNhbHQ=")
    );
    assert_eq!(protection.hash_iteration_count(interner), Ok(Some(100_000)));
    assert_eq!(protection.is_protected(interner), Ok(true));
    // Every flag is a *lock*: `formatCells="false"` **allows** formatting, and the eleven the file
    // omits stay locked by the schema's own defaults.
    assert_eq!(protection.locks_formatting_cells(interner), Ok(false));
    assert_eq!(protection.locks_inserting_rows(interner), Ok(true));
    assert_eq!(protection.locks_selecting_locked_cells(interner), Ok(true));
    assert_eq!(protection.locks_editing_objects(interner), Ok(false));

    let protected: Vec<_> = sheet.protected_range_list().collect();
    assert_eq!(protected.len(), 1);
    assert_eq!(protected[0].name(interner).as_deref(), Ok("Inputs"));
    assert_eq!(
        protected[0].ranges(interner).expect("@sqref").to_string(),
        "D10:E11"
    );
    assert_eq!(
        protected[0]
            .legacy_password_hash(interner)
            .unwrap()
            .as_deref(),
        Some("83AF")
    );
    assert_eq!(
        protected[0].extra().len(),
        1,
        "the `securityDescriptor` *child element* is not modelled and must survive in `extra`"
    );

    let scenarios = sheet.scenarios().expect("scenarios");
    assert_eq!(scenarios.len(), 1);
    assert_eq!(scenarios.current_index(interner), Ok(Some(0)));
    let scenario = scenarios.scenarios().next().expect("one scenario");
    assert_eq!(scenario.name(interner).as_deref(), Ok("Best case"));
    assert_eq!(scenario.declared_count(interner), Ok(Some(1)));
    let inputs: Vec<_> = scenario.input_cells().collect();
    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0].cell(interner), Ok(cell("B2")));
    assert_eq!(inputs[0].value(interner).as_deref(), Ok("1500"));

    let row_breaks = sheet.row_breaks().expect("rowBreaks");
    assert_eq!(row_breaks.len(), 2);
    assert_eq!(
        row_breaks.manual_count(interner),
        1,
        "one manual, one automatic"
    );
    assert_eq!(row_breaks.declared_count(interner), Ok(2));
    assert_eq!(row_breaks.declared_manual_count(interner), Ok(1));
    let column_breaks = sheet.column_breaks().expect("colBreaks");
    assert_eq!(column_breaks.len(), 1);
    let first = column_breaks.breaks().next().expect("one break");
    assert_eq!(first.at(interner), Ok(4));
    assert_eq!(first.last(interner), Ok(1_048_575));
    assert_eq!(first.is_manual(interner), Ok(true));
}

/// The fixture re-emits byte for byte, untouched and after an edit has forced the frame off its
/// whole-part shortcut.
///
/// `worksheet_spine.rs` makes this claim over the whole corpus; it is restated here because the six
/// slots it covers were **unmodelled** when that suite was written, and a model that dropped an
/// attribute of one of them would now be the way this file breaks.
#[test]
fn the_grid_fixture_re_emits_byte_for_byte_before_and_after_an_edit() {
    let bytes = grid_bytes();
    let sheet = grid();
    assert!(sheet.is_verbatim());
    assert_eq!(sheet.to_markup(), bytes, "an untouched part is one memcpy");

    let mut sheet = grid();
    sheet
        .set_cell_value(cell("B3"), CellValue::Number(401.0))
        .expect("one cell");
    assert!(!sheet.is_verbatim());
    let edited = sheet.to_markup();
    assert_ne!(edited, bytes, "the edited cell really did change");
    let before = String::from_utf8_lossy(&bytes);
    let after = String::from_utf8_lossy(&edited);
    for slot in [
        "<sheetProtection",
        "<protectedRanges>",
        "<scenarios ",
        "<mergeCells ",
        "<rowBreaks ",
        "<colBreaks ",
        "<cols>",
    ] {
        let start = before
            .find(slot)
            .unwrap_or_else(|| panic!("{slot} is in the fixture"));
        let end = before[start..]
            .find("\n")
            .expect("each slot is on its own line")
            + start;
        assert!(
            after.contains(&before[start..end]),
            "{slot} was re-flowed by an edit that had nothing to do with it"
        );
    }
}

// -------------------------------------------------------------------------------------------
// Column-run splitting: the four cases, and the one they exist to protect
// -------------------------------------------------------------------------------------------

/// **Three-way split.** A width set strictly inside a run breaks it into three, and only the middle
/// piece takes the new width.
#[test]
fn a_width_set_inside_a_run_splits_it_three_ways() {
    let mut sheet = grid();
    // `D` is column index 3, one-based 4 — strictly inside the wide run `2..=6`.
    sheet
        .set_column_width(columns(3, 3), Some(ColumnWidth::Custom(40.0)))
        .expect("the split succeeds");

    assert_eq!(
        runs(&sheet),
        [
            (2, 3, Some(12.5), true),
            (4, 4, Some(40.0), true),
            (5, 6, Some(12.5), true),
            (8, 8, Some(4.0), true),
        ],
        "one run in, three out — and the run outside the target untouched"
    );
}

/// **Left-edge split.** A width set on a run's first column breaks it into two.
#[test]
fn a_width_set_at_a_runs_left_edge_splits_it_in_two() {
    let mut sheet = grid();
    // `B` is index 1, one-based 2 — the wide run's own `@min`.
    sheet
        .set_column_width(columns(1, 1), Some(ColumnWidth::Custom(40.0)))
        .expect("the split succeeds");

    assert_eq!(
        runs(&sheet),
        [
            (2, 2, Some(40.0), true),
            (3, 6, Some(12.5), true),
            (8, 8, Some(4.0), true),
        ],
        "no empty left piece is written for a target sitting on `@min`"
    );
}

/// **Right-edge split.** A width set on a run's last column breaks it into two the other way round.
#[test]
fn a_width_set_at_a_runs_right_edge_splits_it_in_two() {
    let mut sheet = grid();
    // `F` is index 5, one-based 6 — the wide run's own `@max`.
    sheet
        .set_column_width(columns(5, 5), Some(ColumnWidth::Custom(40.0)))
        .expect("the split succeeds");

    assert_eq!(
        runs(&sheet),
        [
            (2, 5, Some(12.5), true),
            (6, 6, Some(40.0), true),
            (8, 8, Some(4.0), true),
        ],
        "no empty right piece is written for a target sitting on `@max`"
    );
}

/// **Exact match.** A width set on a run that is already exactly the target edits it in place and
/// splits nothing.
#[test]
fn a_width_set_on_a_run_that_is_exactly_the_target_edits_it_in_place() {
    let mut sheet = grid();
    // `H` is index 7, one-based 8 — the second run, which is one column wide.
    sheet
        .set_column_width(columns(7, 7), Some(ColumnWidth::Custom(40.0)))
        .expect("the edit succeeds");

    assert_eq!(
        runs(&sheet),
        [(2, 6, Some(12.5), true), (8, 8, Some(40.0), true)],
        "an exact match is an edit, not a split"
    );
    assert_eq!(
        sheet
            .column_run_covering(7)
            .expect("the runs have bounds")
            .expect("H is covered")
            .hidden(sheet.interner()),
        Ok(true),
        "the run's other attributes survive being edited"
    );
}

/// **The failure the whole thing exists to prevent.** Setting one column's width must leave every
/// other column of the wide run at the width the file gave it.
///
/// This is the assertion the ticket names — *"get this wrong and every column in the sheet changes
/// width"* — and it is separate from the three-way-split case on purpose: a split that produced the
/// right *number* of runs but edited the wrong one would pass a shape assertion and fail this.
#[test]
fn every_column_outside_the_target_keeps_the_width_the_file_gave_it() {
    let mut sheet = grid();
    sheet
        .set_column_width(columns(3, 3), Some(ColumnWidth::Custom(40.0)))
        .expect("the split succeeds");

    assert_eq!(width_of(&sheet, 3), Some(40.0), "D took the new width");
    for column in [1_u16, 2, 4, 5] {
        assert_eq!(
            width_of(&sheet, column),
            Some(12.5),
            "column index {column} was inside the same run and must be unchanged"
        );
    }
    assert_eq!(width_of(&sheet, 7), Some(4.0), "the other run is untouched");
    assert_eq!(
        width_of(&sheet, 0),
        None,
        "A was covered by no run and still is"
    );
}

/// A split that produced two runs which happen to agree must **not** coalesce them.
///
/// `CT_Cols` declares `col` `maxOccurs="unbounded"`, so the number of elements is part of the file:
/// merging `2..=3` and `4..=6` back into `2..=6` describes the same widths in different bytes. This
/// is the second of the two mutations this child is proved by — make the split merge adjacent runs
/// and this goes red while the shape assertions above stay green.
#[test]
fn a_split_never_merges_the_runs_it_leaves_behind() {
    let mut sheet = grid();
    // Set D to the width it already has. Every piece of the split then carries identical
    // attributes, which is exactly when a coalescing writer would fold them back together.
    sheet
        .set_column_width(columns(3, 3), Some(ColumnWidth::Custom(12.5)))
        .expect("the split succeeds");

    assert_eq!(
        runs(&sheet),
        [
            (2, 3, Some(12.5), true),
            (4, 4, Some(12.5), true),
            (5, 6, Some(12.5), true),
            (8, 8, Some(4.0), true),
        ],
        "three runs describing one width are three runs, not one"
    );
}

/// A run split across a *span* takes the whole span, and the pieces outside it keep every attribute
/// of the run they came from — including the ones this crate does not model.
#[test]
fn a_split_keeps_the_original_runs_other_attributes_on_every_piece() {
    let mut sheet = worksheet(
        "<x:cols><x:col min=\"2\" max=\"6\" width=\"12.5\" style=\"3\" phonetic=\"true\" \
         customWidth=\"true\"/></x:cols><x:sheetData/>",
    );
    sheet
        .set_column_width(columns(2, 3), Some(ColumnWidth::Custom(40.0)))
        .expect("the split succeeds");

    let interner = sheet.interner();
    let mut seen = Vec::new();
    for block in sheet.column_blocks() {
        for run in block.runs() {
            seen.push((
                run.first_column(interner).expect("@min"),
                run.last_column(interner).expect("@max"),
                run.style_index(interner).expect("@style"),
                run.shows_phonetic(interner).expect("@phonetic"),
            ));
        }
    }
    assert_eq!(seen, [(2, 2, 3, true), (3, 4, 3, true), (5, 6, 3, true)]);
}

/// A width set on columns no run covers appends one run per contiguous stretch, in ascending order.
#[test]
fn a_width_set_on_bare_columns_writes_one_run_per_contiguous_stretch() {
    let mut sheet = grid();
    // A (index 0) is bare; G (index 6) is bare; H (index 7) is covered. Asking for A..H therefore
    // needs two new runs, not eight, and the covered stretches are split rather than duplicated.
    sheet
        .set_column_width(columns(0, 7), Some(ColumnWidth::Fitted(9.0)))
        .expect("the edit succeeds");

    assert_eq!(
        runs(&sheet),
        [
            (1, 1, Some(9.0), false),
            (2, 6, Some(9.0), false),
            (7, 7, Some(9.0), false),
            (8, 8, Some(9.0), false),
        ],
        "the two covered runs were edited whole, the two bare stretches became one run each, and \
         each new run went in at the position that keeps the block ascending by `@min`"
    );
}

/// A worksheet with no `cols` block at all gains one, at rank 4 of `CT_Worksheet`'s sequence.
#[test]
fn a_sheet_with_no_cols_block_gains_one_in_schema_position() {
    let mut sheet = worksheet("<x:sheetData/><x:pageMargins left=\"0.7\"/>");
    sheet
        .set_column_width(columns(2, 4), Some(ColumnWidth::Custom(11.0)))
        .expect("the edit succeeds");

    assert_eq!(
        sheet.child_element_locals().collect::<Vec<_>>(),
        ["cols", "sheetData", "pageMargins"],
        "rank 4 is before `sheetData`, and placement goes through the generated table"
    );
    assert_eq!(runs(&sheet), [(3, 5, Some(11.0), true)]);
}

/// Hiding a column is the same split, so it gets the same guarantee.
#[test]
fn hiding_one_column_inside_a_run_hides_only_that_column() {
    let mut sheet = grid();
    sheet
        .set_column_hidden(columns(3, 3), true)
        .expect("the split succeeds");

    let interner = sheet.interner();
    for (column, expected) in [
        (1_u16, false),
        (2, false),
        (3, true),
        (4, false),
        (5, false),
    ] {
        assert_eq!(
            sheet
                .column_run_covering(column)
                .expect("bounds")
                .expect("covered")
                .hidden(interner),
            Ok(expected),
            "column index {column}"
        );
    }
}

// -------------------------------------------------------------------------------------------
// Merging
// -------------------------------------------------------------------------------------------

/// A merge is readable from **every** cell in it, and the anchor is the top-left.
#[test]
fn a_merge_is_readable_from_every_cell_in_it() {
    let sheet = grid();
    for name in ["A7", "B7", "C7"] {
        assert_eq!(
            sheet
                .merged_range_containing(cell(name))
                .expect("the merges parse")
                .map(|range| range.text().as_str().to_owned()),
            Some("A7:C7".to_owned()),
            "{name} is inside the merge"
        );
        assert_eq!(sheet.merge_anchor(cell(name)).expect("anchor"), cell("A7"));
    }
    assert!(!sheet.is_covered_by_merge(cell("A7")).expect("covered"));
    assert!(sheet.is_covered_by_merge(cell("B7")).expect("covered"));

    // The anchor is a position this call derived, so it is relative whatever the `@ref` spelled —
    // and it answers for a `$`-anchored lookup too.
    let absolute =
        worksheet("<x:sheetData/><x:mergeCells><x:mergeCell ref=\"$A$7:$C$7\"/></x:mergeCells>");
    let anchor = absolute.merge_anchor(cell("$B$7")).expect("anchor");
    assert_eq!(anchor, cell("A7"));
    assert_eq!(
        anchor.text().as_str(),
        "A7",
        "no `$` is attached to a derived position"
    );
    assert_eq!(
        absolute
            .merged_range_containing(cell("B7"))
            .expect("the merges parse")
            .expect("covered")
            .text()
            .as_str(),
        "$A$7:$C$7",
        "the range itself still comes back as the file spelled it"
    );

    // The span belongs to the anchor; a covered cell reports the one cell it occupies.
    assert_eq!(sheet.cell_span(cell("A7")).expect("span"), (1, 3));
    assert_eq!(sheet.cell_span(cell("B7")).expect("span"), (1, 1));
    assert_eq!(sheet.cell_span(cell("E1")).expect("span"), (2, 2));
    assert_eq!(sheet.cell_span(cell("D10")).expect("span"), (1, 1));
    assert_eq!(
        sheet.merge_anchor(cell("D10")).expect("anchor"),
        cell("D10")
    );
}

/// Merging over a range that already overlaps a merge is a **typed error**, not a silent overwrite.
#[test]
fn merging_over_an_existing_merge_is_a_typed_error() {
    for overlapping in ["B7:D7", "A7:C7", "C6:C8", "F2:G3"] {
        let mut sheet = grid();
        let error = sheet
            .merge_cells(range(overlapping))
            .expect_err("the overlap is refused");
        assert!(
            matches!(error, SmlError::MergeOverlapsExistingMerge { .. }),
            "{overlapping} gave {error:?}"
        );
        assert_eq!(
            sheet.merged_ranges().expect("the merges parse").len(),
            2,
            "a refused merge writes nothing"
        );
        assert!(
            sheet.is_verbatim(),
            "a refused merge does not dirty the part"
        );
    }
}

/// A one-cell merge merges nothing, and is refused the same way.
#[test]
fn merging_a_single_cell_is_a_typed_error() {
    let mut sheet = grid();
    for degenerate in ["B4", "B4:B4", "$B$4"] {
        let error = sheet
            .merge_cells(range(degenerate))
            .expect_err("a one-cell merge is refused");
        assert!(
            matches!(error, SmlError::DegenerateMerge { .. }),
            "{degenerate} gave {error:?}"
        );
    }
}

/// Merging records a range and **touches no cell** — it neither creates the covered cells nor
/// clears the values already in them.
#[test]
fn merging_touches_no_cell() {
    let mut sheet = grid();
    let cells_before: Vec<_> = sheet
        .cells()
        .map(|cell| (cell.reference(), cell.markup()))
        .collect();
    sheet
        .merge_cells(range("A2:B4"))
        .expect("the merge succeeds");

    let cells_after: Vec<_> = sheet
        .cells()
        .map(|cell| (cell.reference(), cell.markup()))
        .collect();
    assert_eq!(cells_before, cells_after, "a merge is not a cell edit");
    assert_eq!(
        sheet
            .merged_ranges()
            .expect("the merges parse")
            .iter()
            .map(|range| range.text().as_str().to_owned())
            .collect::<Vec<_>>(),
        ["A7:C7", "E1:F2", "A2:B4"]
    );
    assert_eq!(
        sheet
            .merged_cells()
            .expect("mergeCells")
            .declared_count(sheet.interner()),
        Ok(Some(3)),
        "@count follows a collection that was edited"
    );
}

/// A worksheet with no `mergeCells` gains one at rank 14, and loses it again with its last range.
///
/// The schema declares `mergeCell` `minOccurs="1"`, so an empty `<mergeCells/>` is markup the gate
/// rejects — leaving one behind would be this library authoring an invalid file.
#[test]
fn the_merge_element_appears_at_rank_fourteen_and_leaves_with_its_last_range() {
    let mut sheet = worksheet("<x:sheetData/><x:pageMargins left=\"0.7\"/>");
    sheet
        .merge_cells(range("A1:B2"))
        .expect("the merge succeeds");
    assert_eq!(
        sheet.child_element_locals().collect::<Vec<_>>(),
        ["sheetData", "mergeCells", "pageMargins"],
        "rank 14 sits between `sheetData` (5) and `pageMargins` (21)"
    );

    assert!(sheet
        .unmerge_cells(range("A1:B2"))
        .expect("the merges parse"));
    assert_eq!(
        sheet.child_element_locals().collect::<Vec<_>>(),
        ["sheetData", "pageMargins"],
        "the element goes with its last range"
    );
    assert!(!sheet
        .unmerge_cells(range("A1:B2"))
        .expect("the merges parse"));
}

/// Unmerging matches on the rectangle, not on the spelling: `C7:A7` names the same cells as
/// `A7:C7`, and the file's own choice of spelling is not a reason to refuse.
#[test]
fn unmerging_matches_the_rectangle_rather_than_the_spelling() {
    let mut sheet = grid();
    assert!(sheet
        .unmerge_cells(range("C7:A7"))
        .expect("the merges parse"));
    assert_eq!(
        sheet
            .merged_ranges()
            .expect("the merges parse")
            .iter()
            .map(|range| range.text().as_str().to_owned())
            .collect::<Vec<_>>(),
        ["E1:F2"]
    );
    assert_eq!(
        sheet
            .merged_cells()
            .expect("mergeCells")
            .declared_count(sheet.interner()),
        Ok(Some(1))
    );
}

// -------------------------------------------------------------------------------------------
// Row geometry, and the flag that makes a height mean anything
// -------------------------------------------------------------------------------------------

/// A height and its `customHeight` flag are written **together**, because the value alone is a
/// height Excel may recompute.
///
/// This is the first of the two mutations this child is proved by: write `ht` without
/// `customHeight` and this goes red.
#[test]
fn setting_a_custom_row_height_writes_the_flag_beside_the_value() {
    let mut sheet = grid();
    sheet
        .set_row_height(5, Some(RowHeight::Custom(33.0)))
        .expect("the height is set");

    let markup = String::from_utf8(
        sheet
            .sheet_data()
            .expect("sheetData")
            .row(5)
            .expect("row 5")
            .markup(),
    )
    .expect("utf-8");
    assert!(markup.contains("ht=\"33\""), "{markup}");
    assert!(
        markup.contains("customHeight=\"true\""),
        "a height without its flag is a height Excel recomputes: {markup}"
    );

    let row = sheet
        .sheet_data()
        .expect("sheetData")
        .row(5)
        .expect("row 5");
    assert_eq!(row.height(), Some(33.0));
    assert!(row.uses_custom_height());
}

/// A *fitted* height is the other claim, and it is expressible: `ht` alone, no flag.
///
/// Both spellings exist in files Excel wrote — an auto-fitted row carries `ht` with no
/// `customHeight` — so the API's job is not to forbid one but to stop a caller writing it by
/// accident.
#[test]
fn a_fitted_row_height_writes_no_flag_at_all() {
    let mut sheet = grid();
    sheet
        .set_row_height(5, Some(RowHeight::Fitted(33.0)))
        .expect("the height is set");

    let markup = String::from_utf8(
        sheet
            .sheet_data()
            .expect("sheetData")
            .row(5)
            .expect("row 5")
            .markup(),
    )
    .expect("utf-8");
    assert!(markup.contains("ht=\"33\""), "{markup}");
    assert!(!markup.contains("customHeight"), "{markup}");
}

/// A row that already claims a custom height keeps the file's own spelling of the flag when the
/// claim does not change.
///
/// `customHeight="true"` is not rewritten to `customHeight="1"` by a call that leaves the flag's
/// *value* alone — the same rule the cell store applies to every attribute it does not change.
#[test]
fn a_height_change_does_not_re_spell_a_flag_it_does_not_change() {
    let mut sheet = grid();
    sheet
        .set_row_height(2, Some(RowHeight::Custom(30.0)))
        .expect("the height is set");
    let markup = String::from_utf8(
        sheet
            .sheet_data()
            .expect("sheetData")
            .row(2)
            .expect("row 2")
            .markup(),
    )
    .expect("utf-8");
    assert!(markup.contains("ht=\"30\""), "{markup}");
    assert!(
        markup.contains("customHeight=\"true\""),
        "the fixture's own spelling survives: {markup}"
    );
    assert!(!markup.contains("customHeight=\"1\""), "{markup}");
}

/// Removing a height removes the flag with it: `customHeight` with no `ht` claims a height that is
/// not there.
#[test]
fn removing_a_row_height_removes_its_flag_too() {
    let mut sheet = grid();
    sheet
        .set_row_height(2, None)
        .expect("the height is removed");
    let markup = String::from_utf8(
        sheet
            .sheet_data()
            .expect("sheetData")
            .row(2)
            .expect("row 2")
            .markup(),
    )
    .expect("utf-8");
    assert!(!markup.contains("ht="), "{markup}");
    assert!(!markup.contains("customHeight"), "{markup}");
}

/// Hiding one row leaves every other row and every other worksheet child byte-identical.
#[test]
fn hiding_one_row_leaves_every_other_child_byte_identical() {
    let bytes = grid_bytes();
    let mut sheet = grid();
    sheet.set_row_hidden(5, true).expect("the row is hidden");
    let edited = sheet.to_markup();

    let before = String::from_utf8_lossy(&bytes);
    let after = String::from_utf8_lossy(&edited);
    for line in before.lines() {
        if line.starts_with("<row r=\"5\"") {
            assert!(!after.contains(line), "row 5 really did change");
            continue;
        }
        assert!(
            after.contains(line),
            "an unrelated line was re-flowed by hiding one row:\n{line}"
        );
    }
    assert!(
        after.contains("<row r=\"5\" collapsed=\"true\" hidden=\"true\">"),
        "the hidden flag is appended and the row's own attributes stay put"
    );
}

// -------------------------------------------------------------------------------------------
// Outline levels and the two maxima
// -------------------------------------------------------------------------------------------

/// Setting a level deeper than the sheet declares raises the declared maximum, in both axes.
#[test]
fn setting_a_deeper_outline_level_raises_the_sheets_maximum() {
    let mut sheet = grid();
    assert_eq!(
        sheet
            .format_properties()
            .expect("sheetFormatPr")
            .deepest_row_outline_level(sheet.interner()),
        Ok(2)
    );

    sheet.set_row_outline_level(5, 4).expect("the level is set");
    assert_eq!(
        sheet
            .format_properties()
            .expect("sheetFormatPr")
            .deepest_row_outline_level(sheet.interner()),
        Ok(4),
        "the maximum was raised to match the row"
    );

    sheet
        .set_column_outline_level(columns(0, 0), 5)
        .expect("the level is set");
    assert_eq!(
        sheet
            .format_properties()
            .expect("sheetFormatPr")
            .deepest_column_outline_level(sheet.interner()),
        Ok(5)
    );
}

/// Nothing lowers a maximum on its own; a caller who wants that asks for it.
#[test]
fn a_maximum_is_only_lowered_when_the_caller_asks() {
    let mut sheet = grid();
    sheet
        .set_row_outline_level(3, 0)
        .expect("the level is cleared");
    sheet
        .set_row_outline_level(4, 0)
        .expect("the level is cleared");
    assert_eq!(
        sheet
            .format_properties()
            .expect("sheetFormatPr")
            .deepest_row_outline_level(sheet.interner()),
        Ok(2),
        "flattening the rows does not silently rewrite the sheet's claim"
    );

    assert_eq!(
        sheet
            .recompute_outline_levels()
            .expect("the runs have bounds"),
        Some((0, 2))
    );
    assert_eq!(
        sheet
            .format_properties()
            .expect("sheetFormatPr")
            .deepest_row_outline_level(sheet.interner()),
        Ok(0)
    );
}

/// A sheet with no `sheetFormatPr` never gains one: `@defaultRowHeight` is `use="required"`, so
/// authoring the element to record a maximum would mean inventing a default row height.
#[test]
fn a_maximum_is_never_authored_onto_a_sheet_that_declares_none() {
    let mut sheet = worksheet("<x:cols><x:col min=\"1\" max=\"1\"/></x:cols><x:sheetData/>");
    sheet
        .set_column_outline_level(columns(0, 0), 3)
        .expect("the level is set");
    assert!(sheet.format_properties().is_none());
    assert_eq!(
        sheet.child_element_locals().collect::<Vec<_>>(),
        ["cols", "sheetData"]
    );
    assert_eq!(sheet.recompute_outline_levels().expect("bounds"), None);
}

// -------------------------------------------------------------------------------------------
// Protection: preserved, never computed
// -------------------------------------------------------------------------------------------

/// A protected sheet's hash bytes are **byte-identical** after an edit that had nothing to do with
/// protection.
///
/// The comparison is against the committed file's own bytes, and the fixture writes two spaces
/// inside `<sheetProtection …>` for exactly that reason: a writer that rebuilt the element from the
/// model would normalise the whitespace and fail here even with every attribute value intact.
#[test]
fn a_protected_sheets_hash_survives_an_unrelated_edit() {
    let bytes = grid_bytes();
    let source = String::from_utf8_lossy(&bytes);
    let start = source
        .find("<sheetProtection")
        .expect("the fixture is protected");
    let end = source[start..].find("/>").expect("a self-closing element") + start + 2;
    let original = &source[start..end];
    assert!(
        original.contains("password=\"CC1A\"  algorithmName"),
        "the fixture must carry a doubled space no rebuild reproduces: {original}"
    );

    let mut sheet = grid();
    sheet
        .set_cell_value(cell("B2"), CellValue::Number(1_300.0))
        .expect("an unrelated cell");
    sheet.set_row_hidden(5, true).expect("an unrelated row");
    sheet
        .set_column_width(columns(3, 3), Some(ColumnWidth::Custom(40.0)))
        .expect("an unrelated column");
    sheet
        .merge_cells(range("A2:A4"))
        .expect("an unrelated merge");

    let edited = String::from_utf8(sheet.to_markup()).expect("utf-8");
    assert!(
        edited.contains(original),
        "the protection element was rewritten by an unrelated edit:\nwanted {original}"
    );
    assert!(
        edited.contains("<protectedRange password=\"83AF\" sqref=\"D10:E11\" name=\"Inputs\">"),
        "the protected range's hash was rewritten too"
    );
}

/// Reading protection reports the file's own text and decodes none of it.
///
/// There is deliberately no accessor that returns a *derived* value here — no "is this password
/// correct", no decoded salt, no algorithm object. The five attributes are text in and text out.
#[test]
fn the_hash_family_is_reported_as_text_and_never_interpreted() {
    let mut sheet = worksheet(
        "<x:sheetData/><x:sheetProtection algorithmName=\"NOT-AN-ALGORITHM\" \
         hashValue=\"////\" saltValue=\"\" spinCount=\"0\" sheet=\"true\"/>",
    );
    let interner = sheet.interner();
    let protection = sheet.protection().expect("sheetProtection");
    assert_eq!(
        protection.hash_algorithm_name(interner).unwrap().as_deref(),
        Some("NOT-AN-ALGORITHM"),
        "an algorithm nothing implements is still read back as written"
    );
    assert_eq!(
        protection.hash_value(interner).unwrap().as_deref(),
        Some("////")
    );
    assert_eq!(
        protection.salt_value(interner).unwrap().as_deref(),
        Some("")
    );
    assert_eq!(protection.hash_iteration_count(interner), Ok(Some(0)));

    // And an edit elsewhere leaves all four exactly as they were.
    sheet
        .set_cell_value(cell("A1"), CellValue::Number(1.0))
        .expect("a cell");
    assert!(String::from_utf8(sheet.to_markup())
        .expect("utf-8")
        .contains("algorithmName=\"NOT-AN-ALGORITHM\" hashValue=\"////\" saltValue=\"\""));
}

// -------------------------------------------------------------------------------------------
// Page breaks
// -------------------------------------------------------------------------------------------

/// Pushing a break keeps `@count` and `@manualBreakCount` in step — each only where the file
/// declared it.
#[test]
fn a_pushed_break_updates_only_the_counts_the_file_declared() {
    let mut sheet = worksheet(
        "<x:sheetData/><x:rowBreaks count=\"1\"><x:brk id=\"3\" man=\"true\"/></x:rowBreaks>\
         <x:colBreaks><x:brk id=\"2\"/></x:colBreaks>",
    );
    let prefix = "x".to_owned();
    sheet.with_page_break_pushed(true, &prefix, 9, true);
    sheet.with_page_break_pushed(false, &prefix, 5, false);

    let markup = String::from_utf8(sheet.to_markup()).expect("utf-8");
    assert!(
        markup.contains("<x:rowBreaks count=\"2\">"),
        "a declared count follows the collection: {markup}"
    );
    assert!(
        !markup.contains("manualBreakCount"),
        "a count the file never wrote is never authored: {markup}"
    );
    assert!(markup.contains("<x:colBreaks>"), "{markup}");
    assert_eq!(sheet.column_breaks().expect("colBreaks").len(), 2);
}

/// Manual and automatic breaks are told apart by `@man`, and `manual_count` counts only the first
/// kind.
#[test]
fn manual_and_automatic_breaks_are_told_apart() {
    let sheet = grid();
    let breaks = sheet.row_breaks().expect("rowBreaks");
    let flags: Vec<_> = breaks
        .breaks()
        .map(|entry| {
            (
                entry.at(sheet.interner()).expect("@id"),
                entry.is_manual(sheet.interner()).expect("@man"),
            )
        })
        .collect();
    assert_eq!(flags, [(5, true), (9, false)]);
    assert_eq!(breaks.manual_count(sheet.interner()), 1);
}

/// A helper for [`a_pushed_break_updates_only_the_counts_the_file_declared`]: appends one break to
/// one of the two axes.
trait PushBreak {
    fn with_page_break_pushed(&mut self, rows: bool, prefix: &str, at: u32, manual: bool);
}

impl PushBreak for WorksheetPart {
    fn with_page_break_pushed(&mut self, rows: bool, prefix: &str, at: u32, manual: bool) {
        let mut interner = mjx_ooxml_core::Interner::default();
        core::mem::swap(&mut interner, self.interner_mut());
        let mut entry = mjx_sml::PageBreak::new(&mut interner, Some(prefix));
        entry.set_at(&mut interner, Some(at));
        entry.set_is_manual(&mut interner, manual.then_some(true));
        let block = if rows {
            self.row_breaks_mut()
        } else {
            self.column_breaks_mut()
        };
        if let Some(block) = block {
            block.push(&mut interner, entry);
        }
        core::mem::swap(&mut interner, self.interner_mut());
    }
}

// -------------------------------------------------------------------------------------------
// Anomalies: reported, never repaired
// -------------------------------------------------------------------------------------------

/// The committed fixture is a clean sheet, so the report is empty.
///
/// Without this, every assertion below could pass against a report that always fires.
#[test]
fn a_well_formed_grid_reports_nothing() {
    assert_eq!(grid().grid_anomalies().expect("the runs have bounds"), []);
}

/// Two merges that overlap are **preserved** and reported. Nothing is dropped, nothing is trimmed.
#[test]
fn overlapping_merges_in_a_file_are_reported_and_never_repaired() {
    let markup = "<x:sheetData/><x:mergeCells count=\"9\">\
        <x:mergeCell ref=\"A1:C3\"/><x:mergeCell ref=\"B2:D4\"/><x:mergeCell ref=\"F1\"/>\
        </x:mergeCells>";
    let sheet = worksheet(markup);
    let found = sheet.grid_anomalies().expect("the runs have bounds");

    assert!(found
        .iter()
        .any(|anomaly| matches!(anomaly, GridAnomaly::MergesOverlap { .. })));
    assert!(found
        .iter()
        .any(|anomaly| matches!(anomaly, GridAnomaly::DegenerateMerge { .. })));
    assert!(found.iter().any(|anomaly| matches!(
        anomaly,
        GridAnomaly::MergeCountDisagrees {
            declared: 9,
            actual: 3
        }
    )));
    assert_eq!(
        sheet.merged_ranges().expect("the merges parse").len(),
        3,
        "all three are still there"
    );
    assert!(sheet.is_verbatim(), "asking does not change the part");
}

/// A merge whose interior cells hold values is preserved and reported — Excel hides them, and this
/// library neither hides nor deletes them.
#[test]
fn a_merge_whose_interior_holds_a_value_is_reported() {
    let sheet = worksheet(
        "<x:sheetData><x:row r=\"1\"><x:c r=\"A1\"><x:v>1</x:v></x:c>\
         <x:c r=\"B1\"><x:v>2</x:v></x:c></x:row></x:sheetData>\
         <x:mergeCells><x:mergeCell ref=\"A1:C1\"/></x:mergeCells>",
    );
    let found = sheet.grid_anomalies().expect("the runs have bounds");
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(matches!(
        found[0],
        GridAnomaly::MergeInteriorCellHasValue { .. }
    ));
    assert_eq!(
        sheet.cell(cell("B1")).expect("B1 is still there").number(),
        Some(2.0),
        "the value is readable, not swallowed"
    );
}

/// A `mergeCell@ref` that will not parse is reported by name, and makes the merge queries refuse
/// rather than answer from a shortened list.
#[test]
fn an_unreadable_merge_reference_is_reported_and_refuses_the_queries() {
    let sheet = worksheet(
        "<x:sheetData/><x:mergeCells><x:mergeCell ref=\"A1:B2\"/>\
         <x:mergeCell ref=\"not a range\"/></x:mergeCells>",
    );
    assert_eq!(
        sheet.grid_anomalies().expect("the runs have bounds"),
        [GridAnomaly::MergeReferenceUnreadable { index: 1 }]
    );
    assert!(
        sheet.merged_ranges().is_err(),
        "answering `A1:B2 and nothing else` would report a covered cell as free"
    );
}

/// An `outlineLevelRow` that no row reaches, and one the rows reach past, are two different things:
/// only the second is an anomaly, and neither is repaired.
#[test]
fn an_outline_maximum_the_rows_reach_past_is_reported() {
    let deeper = worksheet(
        "<x:sheetFormatPr defaultRowHeight=\"15\" outlineLevelRow=\"1\"/>\
         <x:sheetData><x:row r=\"1\" outlineLevel=\"3\"/></x:sheetData>",
    );
    assert_eq!(
        deeper.grid_anomalies().expect("the runs have bounds"),
        [GridAnomaly::RowOutlineLevelPastDeclaredMaximum {
            deepest: 3,
            declared: 1
        }]
    );

    let shallower = worksheet(
        "<x:sheetFormatPr defaultRowHeight=\"15\" outlineLevelRow=\"7\"/>\
         <x:sheetData><x:row r=\"1\" outlineLevel=\"1\"/></x:sheetData>",
    );
    assert_eq!(
        shallower.grid_anomalies().expect("the runs have bounds"),
        [],
        "a maximum nothing reaches is a stale cache, not a contradiction"
    );

    let silent = worksheet("<x:sheetData><x:row r=\"1\" outlineLevel=\"3\"/></x:sheetData>");
    assert_eq!(
        silent.grid_anomalies().expect("the runs have bounds"),
        [],
        "a sheet with no `sheetFormatPr` has declared nothing to disagree with"
    );
}

/// Two `col` runs claiming the same column, and a run whose `@min` exceeds its `@max`, are reported
/// and left alone.
#[test]
fn column_runs_that_overlap_or_invert_are_reported() {
    let sheet = worksheet(
        "<x:cols><x:col min=\"1\" max=\"5\" width=\"9\"/></x:cols>\
         <x:cols><x:col min=\"4\" max=\"6\" width=\"11\"/><x:col min=\"9\" max=\"7\"/></x:cols>\
         <x:sheetData/>",
    );
    let found = sheet.grid_anomalies().expect("the runs have bounds");
    assert!(found.iter().any(|anomaly| matches!(
        anomaly,
        GridAnomaly::ColumnRunsOverlap {
            first: (1, 5),
            second: (4, 6)
        }
    )));
    assert!(found.iter().any(|anomaly| matches!(
        anomaly,
        GridAnomaly::ColumnRunBoundsInverted {
            first_column: 9,
            last_column: 7
        }
    )));
    assert_eq!(
        width_of(&sheet, 3),
        Some(9.0),
        "the first run in document order answers, and nothing is deduplicated"
    );
}

// -------------------------------------------------------------------------------------------
// Scenarios
// -------------------------------------------------------------------------------------------

/// A scenario is read, preserved and **never applied**: `B2` keeps the value `sheetData` gives it,
/// not the one the scenario would put there.
#[test]
fn a_scenario_is_reported_and_never_applied() {
    let sheet = grid();
    assert_eq!(
        sheet
            .scenarios()
            .expect("scenarios")
            .scenarios()
            .next()
            .expect("one scenario")
            .input_cells()
            .next()
            .expect("one input cell")
            .value(sheet.interner())
            .as_deref(),
        Ok("1500")
    );
    assert_eq!(
        sheet.cell(cell("B2")).expect("B2").number(),
        Some(1_200.0),
        "the cell keeps what `sheetData` says, whatever the scenario would put there"
    );
}

// -------------------------------------------------------------------------------------------
// Placement
// -------------------------------------------------------------------------------------------

/// **A new modelled slot lands on the right side of an *unmodelled* one.**
///
/// This is the defect the promoted ranks create and the reason `Slot::rank` ranks a held child too.
/// MJXOFF-102 (D07) modelled ranks 0–6 — a **prefix** — so an unmodelled child was always a later
/// one and putting a modelled child before it was always right. This child models 7, 8, 9, 14, 23
/// and 24 and leaves 10–13 and 15–22 held, so the two interleave: a `mergeCells` (14) inserted into a
/// sheet whose `autoFilter` (10) and `sortState` (11) are held markup has to land **after** them.
///
/// A placement that skipped what it could not rank would put it before both, and the whole worksheet
/// would then be in schema-invalid order — written by this library, into a file whose `autoFilter`
/// it never touched.
#[test]
fn a_new_slot_lands_after_the_unmodelled_slots_that_outrank_it() {
    let mut sheet = worksheet(
        "<x:sheetData/><x:autoFilter ref=\"A1:B2\"/><x:sortState ref=\"A1:B2\"/>\
         <x:colBreaks><x:brk id=\"2\"/></x:colBreaks>",
    );
    sheet
        .merge_cells(range("A1:B2"))
        .expect("the merge succeeds");
    assert_eq!(
        sheet.child_element_locals().collect::<Vec<_>>(),
        [
            "sheetData",
            "autoFilter",
            "sortState",
            "mergeCells",
            "colBreaks"
        ],
        "ranks 5, 10, 11, 14, 24 — the two held slots outrank nothing here by accident, they \
         outrank `mergeCells` in `CT_Worksheet`'s own sequence"
    );

    // And the other direction: a slot that must precede a held one still does.
    let mut sheet = worksheet("<x:sheetData/><x:autoFilter ref=\"A1:B2\"/>");
    let mut interner = mjx_ooxml_core::Interner::default();
    core::mem::swap(&mut interner, sheet.interner_mut());
    let protection = mjx_sml::SheetProtection::new(&mut interner, Some("x"));
    core::mem::swap(&mut interner, sheet.interner_mut());
    sheet.set_protection(Some(protection));
    assert_eq!(
        sheet.child_element_locals().collect::<Vec<_>>(),
        ["sheetData", "sheetProtection", "autoFilter"],
        "rank 7 precedes rank 10"
    );
}

/// What genuinely has no rank is still stepped over: a comment, and an element in a namespace
/// `CT_Worksheet` does not put here.
///
/// "Stepped over" means the unranked node is **not a boundary**: the new child lands beside its
/// ranked neighbours rather than being pushed past markup this model cannot order. The foreign
/// element still sits between `sheetData` and `colBreaks`, which is the only thing about its position
/// the file actually stated.
#[test]
fn a_child_with_no_rank_at_all_neither_moves_nor_displaces_what_is_inserted() {
    let mut sheet = worksheet(
        "<x:sheetData/><!-- a note --><foreign:thing xmlns:foreign=\"urn:elsewhere\"/>\
         <x:colBreaks><x:brk id=\"2\"/></x:colBreaks>",
    );
    sheet
        .merge_cells(range("A1:B2"))
        .expect("the merge succeeds");
    assert_eq!(
        sheet.child_element_locals().collect::<Vec<_>>(),
        ["sheetData", "mergeCells", "thing", "colBreaks"],
        "an element in a namespace `CT_Worksheet` does not name has no rank, so it is stepped over \
         rather than treated as the boundary a held `autoFilter` is"
    );
    assert!(
        String::from_utf8(sheet.to_markup())
            .expect("utf-8")
            .contains("<!-- a note -->"),
        "the comment survives"
    );
}

/// Every promoted slot is placed through the generated `ChildOrder` rank, not by the order a caller
/// happens to set them in.
///
/// The slots are set here in **reverse** schema order, which is the only way this can fail: a writer
/// appending in call order would emit them exactly backwards and a fixture written the right way
/// round could not tell.
#[test]
fn each_promoted_slot_is_placed_by_the_generated_rank() {
    let mut sheet = worksheet("<x:sheetData/>");
    let prefix = Some("x");

    let mut interner = mjx_ooxml_core::Interner::default();
    core::mem::swap(&mut interner, sheet.interner_mut());
    let column_breaks = mjx_sml::PageBreaks::new(&mut interner, prefix, mjx_sml::BreakAxis::Column);
    let row_breaks = mjx_sml::PageBreaks::new(&mut interner, prefix, mjx_sml::BreakAxis::Row);
    let scenarios = mjx_sml::Scenarios::new(&mut interner, prefix);
    let protected = mjx_sml::ProtectedRanges::new(&mut interner, prefix);
    let protection = mjx_sml::SheetProtection::new(&mut interner, prefix);
    core::mem::swap(&mut interner, sheet.interner_mut());

    sheet.set_column_breaks(Some(column_breaks));
    sheet.set_row_breaks(Some(row_breaks));
    sheet
        .merge_cells(range("A1:B2"))
        .expect("the merge succeeds");
    sheet.set_scenarios(Some(scenarios));
    sheet.set_protected_ranges(Some(protected));
    sheet.set_protection(Some(protection));

    assert_eq!(
        sheet.child_element_locals().collect::<Vec<_>>(),
        [
            "sheetData",
            "sheetProtection",
            "protectedRanges",
            "scenarios",
            "mergeCells",
            "rowBreaks",
            "colBreaks",
        ],
        "ranks 5, 7, 8, 9, 14, 23, 24 — from `mjx_ooxml_types::child_order::WORKSHEET`"
    );
}
