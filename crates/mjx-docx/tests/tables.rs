//! Tables: the grid, rows, cells, spans and structural edits (MJXOFF-116).
//!
//! `ragged_table.docx` is authored, not templated — hand-written `<w:tbl>` markup spliced into a
//! blank document's own `word/document.xml` (`regenerate_fixtures`, `#[ignore]`, below), the same
//! technique MJXOFF-113's `header_watermark.docx` uses, for the same reason: this crate's own public
//! API changes an *existing* cell's `w:gridSpan`/`w:vMerge` in place (`Document::set_cell_span`/
//! `set_cell_vertical_merge`), it does not remove the now-redundant covered cells growing a span over
//! them would leave behind — that removal is exactly what `Table::insert_column`/`remove_column`
//! already do correctly for a *structural* edit, but "declare cell (1, 0) to already span two grid
//! columns" when authoring a table from nothing is a different operation this ticket was not asked to
//! build a convenience for. No committed Word fixture in this workspace contains a `w:tbl` at all
//! before this file.
//!
//! # The ragged 4×4 fixture, by construction
//!
//! Four grid columns, four rows. Row 0 is the one row where physical cell index equals grid column
//! for every cell — the fixture's own control. Each of the other three rows carries a `w:gridSpan="2"`
//! in a different place, so a resolver that reads "the physical cell at index N" as "the cell at
//! column N" disagrees with the true grid column for at least one cell in three of the four rows —
//! this ticket's own trap, named exactly ("a rectangular fixture with no merges passes … Seed a
//! fixture that disagrees"). Column 3 additionally carries a `w:vMerge` spanning rows 0–2 (`restart`
//! at row 0, bare `continue` at rows 1–2), independent of which row anchors which `gridSpan`:
//!
//! ```text
//!         col0      col1      col2      col3
//! row0  [ R0C0   ][ R0C1   ][ R0C2   ][ R0C3 (vMerge restart)  ]
//! row1  [ R1C01 (gridSpan=2)         ][ R1C2 ][ (vMerge cont.) ]
//! row2  [ R2C0   ][ R2C12 (gridSpan=2)       ][ (vMerge cont.) ]
//! row3  [ R3C0   ][ R3C1   ][ R3C23 (gridSpan=2)                ]
//! ```
//!
//! Row 3's `gridSpan=2` covers columns 2–3, and column 3 there carries **no** `w:vMerge` — the
//! merge closes after row 2 (ECMA-376 Part 1 §17.4.84's own "shall be closed" rule), matching
//! "a three-row `w:vMerge`", not four.

use mjx_docx::{Cell, Document, GridDiscrepancy, Package, PageSize, Table};
use mjx_fixtures::fixture;
use mjx_ooxml_core::FromXml;

fn ragged_fixture() -> Document {
    Document::open(&fixture("ragged_table.docx")).expect("open ragged_table.docx")
}

// -------------------------------------------------------------------------------------------
// The (row, column) -> cell mapping.
// -------------------------------------------------------------------------------------------

#[test]
fn row_zero_is_the_aligned_control_row() {
    let mut document = ragged_fixture();
    for column in 0..4 {
        let (rows, cols) = document.table_dimensions(0).expect("dimensions");
        assert_eq!((rows, cols), (4, 4));
        let text = document.cell_text(0, 0, column).expect("cell text");
        assert_eq!(text, format!("R0C{column}"));
    }
}

#[test]
fn the_row_to_column_map_disagrees_with_cell_index_in_three_of_four_rows() {
    let mut document = ragged_fixture();

    // Row 1: gridSpan=2 at columns 0-1 (one physical cell, index 0), so column 1's cell is the
    // *same* physical cell as column 0's, not "physical index 1".
    assert_eq!(document.cell_text(0, 1, 0).unwrap(), "R1C01");
    assert_eq!(document.cell_text(0, 1, 1).unwrap(), "R1C01");
    assert_eq!(document.cell_text(0, 1, 2).unwrap(), "R1C2");

    // Row 2: gridSpan=2 at columns 1-2.
    assert_eq!(document.cell_text(0, 2, 0).unwrap(), "R2C0");
    assert_eq!(document.cell_text(0, 2, 1).unwrap(), "R2C12");
    assert_eq!(document.cell_text(0, 2, 2).unwrap(), "R2C12");

    // Row 3: gridSpan=2 at columns 2-3.
    assert_eq!(document.cell_text(0, 3, 0).unwrap(), "R3C0");
    assert_eq!(document.cell_text(0, 3, 1).unwrap(), "R3C1");
    assert_eq!(document.cell_text(0, 3, 2).unwrap(), "R3C23");
    assert_eq!(document.cell_text(0, 3, 3).unwrap(), "R3C23");
}

/// The mutation this ticket's own "Done when" asks be provable: an implementation that reads the
/// physical cell at index `column` as *the* cell for grid column `column` — exactly what
/// `Table::resolve_cell`'s accumulating-span walk exists to not do. Confirmed by hand: temporarily
/// replacing `Table::resolve_cell`'s span-accumulating loop with
/// `row_ref.cells().nth(column).map(|cell| (column, cell))` turns **10 of this file's 16 tests**
/// red, this one included (`left: (1, 1), right: (4, 1)` on `cell_span`'s vertical-run-length
/// assertion — the merge is no longer found at all once column resolution goes wrong) and, most
/// directly,
/// `the_row_to_column_map_disagrees_with_cell_index_in_three_of_four_rows` failing at its very
/// first assertion with `left: "R1C2", right: "R1C01"` (`cell_text(0, 1, 1)` now reads physical
/// index 1, `R1C2`, instead of the gridSpan-2 anchor `R1C01` index 0 actually covers) — restored
/// by re-editing back to the accumulating walk, not `git checkout --`.
#[test]
fn cell_index_as_column_index_is_the_bug_this_fixture_is_built_to_catch() {
    let mut document = ragged_fixture();
    // Row 1 has 3 physical cells (span-2, plain, vMerge-continue) covering 4 grid columns — a
    // naive `cells().nth(column)` would already panic-safe `None` past physical index 2, so this
    // assertion alone proves cell count and column count diverge on this fixture without touching
    // implementation internals from the test.
    let (rows, columns) = document.table_dimensions(0).expect("dimensions");
    assert_eq!((rows, columns), (4, 4));
    // Row 1's own physical cell count (verified indirectly): column 3 must still resolve for a
    // conforming reader even though row 1 has only 3 `<w:tc>` elements for 4 grid columns.
    assert!(
        document.cell_text(0, 1, 3).is_ok(),
        "row 1's 4th grid column must still resolve"
    );
}

// -------------------------------------------------------------------------------------------
// cell_span / merged_cell_anchor — mirroring mjx_pptx::Presentation's own names, argument order
// and return shape.
// -------------------------------------------------------------------------------------------

#[test]
fn cell_span_reports_the_vertical_anchors_true_row_span_and_a_covered_cells_span_as_one_one() {
    let mut document = ragged_fixture();

    // The vMerge anchor: 3 rows tall, 1 column wide.
    assert_eq!(document.cell_span(0, 0, 3).unwrap(), (3, 1));
    // A covered (continuation) cell always reports (1, 1) — ask merged_cell_anchor instead.
    assert_eq!(document.cell_span(0, 1, 3).unwrap(), (1, 1));
    assert_eq!(document.cell_span(0, 2, 3).unwrap(), (1, 1));

    // The gridSpan=2 anchors: 1 row tall (no vertical merge), 2 columns wide.
    assert_eq!(document.cell_span(0, 1, 0).unwrap(), (1, 2));
    assert_eq!(document.cell_span(0, 2, 1).unwrap(), (1, 2));
    assert_eq!(document.cell_span(0, 3, 2).unwrap(), (1, 2));

    // An ordinary cell: (1, 1).
    assert_eq!(document.cell_span(0, 0, 0).unwrap(), (1, 1));
}

#[test]
fn merged_cell_anchor_walks_every_continuation_up_to_the_restart() {
    let mut document = ragged_fixture();
    assert_eq!(document.merged_cell_anchor(0, 0, 3).unwrap(), (0, 3));
    assert_eq!(document.merged_cell_anchor(0, 1, 3).unwrap(), (0, 3));
    assert_eq!(document.merged_cell_anchor(0, 2, 3).unwrap(), (0, 3));
    // Row 3's column 3 is not part of the vertical merge (it closed at row 2) — it is its own,
    // horizontally-spanned cell, so its anchor is itself (at its span's own starting column).
    assert_eq!(document.merged_cell_anchor(0, 3, 3).unwrap(), (3, 2));
}

#[test]
fn the_fixtures_own_grid_has_no_discrepancies() {
    let mut document = ragged_fixture();
    let discrepancies = document
        .table_grid_discrepancies(0)
        .expect("read discrepancies");
    assert_eq!(
        discrepancies,
        Vec::new(),
        "the authored fixture's grid must itself be well-formed"
    );
}

// -------------------------------------------------------------------------------------------
// Structural edits — insert/remove row/column, asserting the grid invariant after each.
// -------------------------------------------------------------------------------------------

#[test]
fn inserting_a_row_inside_the_vertical_merge_grows_it_and_keeps_the_grid_coherent() {
    let mut document = ragged_fixture();
    // Insert between row 0 (restart) and row 1 (continue) — strictly inside the 3-row merge, which
    // must grow to 4.
    document.insert_row(0, 1).expect("insert row");

    assert_eq!(document.table_dimensions(0).unwrap(), (5, 4));
    assert_eq!(
        document.cell_span(0, 0, 3).unwrap(),
        (4, 1),
        "the vertical merge must have grown by the inserted row"
    );
    assert_eq!(document.merged_cell_anchor(0, 1, 3).unwrap(), (0, 3));
    assert_eq!(document.merged_cell_anchor(0, 2, 3).unwrap(), (0, 3));
    assert_eq!(document.merged_cell_anchor(0, 3, 3).unwrap(), (0, 3));
    // The original row 1 (gridSpan=2 at cols 0-1), pushed down one place by the insert, is now row 2.
    assert_eq!(document.cell_text(0, 2, 0).unwrap(), "R1C01");
    assert_eq!(document.cell_text(0, 2, 1).unwrap(), "R1C01");

    let discrepancies = document
        .table_grid_discrepancies(0)
        .expect("read discrepancies");
    assert_eq!(
        discrepancies,
        Vec::new(),
        "grid invariant must hold after the insert"
    );
}

#[test]
fn removing_the_anchor_row_promotes_the_row_below_and_keeps_the_grid_coherent() {
    let mut document = ragged_fixture();
    // Row 0 anchors the vertical merge at column 3, and there are two more rows below it.
    document.remove_row(0, 0).expect("remove row");

    assert_eq!(document.table_dimensions(0).unwrap(), (3, 4));
    // The promoted row (old row 1) must now anchor a 2-row merge, and (per promotion) took over the
    // old row 0's column-3 content — an empty-anchor cell, distinguishable from row 1's own former
    // (empty, continuation) column-3 text only by the merge state, which is what actually matters.
    assert_eq!(document.cell_span(0, 0, 3).unwrap(), (2, 1));
    assert_eq!(document.merged_cell_anchor(0, 0, 3).unwrap(), (0, 3));
    assert_eq!(document.merged_cell_anchor(0, 1, 3).unwrap(), (0, 3));
    // The promoted row's own gridSpan=2 content (columns 0-1) is untouched — promotion only
    // replaces the *anchor cell's* content, not the whole row.
    assert_eq!(document.cell_text(0, 0, 0).unwrap(), "R1C01");
    assert_eq!(document.cell_text(0, 0, 1).unwrap(), "R1C01");

    let discrepancies = document
        .table_grid_discrepancies(0)
        .expect("read discrepancies");
    assert_eq!(
        discrepancies,
        Vec::new(),
        "grid invariant must hold after the removal"
    );
}

#[test]
fn removing_a_middle_continuation_row_needs_no_markup_rewrite_elsewhere() {
    let mut document = ragged_fixture();
    // Row 1 is a continuation (neither anchor nor last) of the column-3 merge.
    document.remove_row(0, 1).expect("remove row");

    assert_eq!(document.table_dimensions(0).unwrap(), (3, 4));
    // The merge is now 2 rows: row 0 (restart) + the old row 2 (still `continue`), promoted to row
    // 1 purely by the row shifting down — no promotion/content-copy needed.
    assert_eq!(document.cell_span(0, 0, 3).unwrap(), (2, 1));
    assert_eq!(document.merged_cell_anchor(0, 1, 3).unwrap(), (0, 3));
    // The old row 2's own gridSpan=2 content (columns 1-2) survives unchanged, now at row 1.
    assert_eq!(document.cell_text(0, 1, 1).unwrap(), "R2C12");
    assert_eq!(document.cell_text(0, 1, 2).unwrap(), "R2C12");

    let discrepancies = document
        .table_grid_discrepancies(0)
        .expect("read discrepancies");
    assert_eq!(
        discrepancies,
        Vec::new(),
        "grid invariant must hold after the removal"
    );
}

#[test]
fn inserting_a_column_inside_a_gridspan_grows_it_and_keeps_the_grid_coherent() {
    let mut document = ragged_fixture();
    // Column 1 falls strictly inside row 1's gridSpan=2 (columns 0-1).
    document.insert_column(0, 1).expect("insert column");

    assert_eq!(document.table_dimensions(0).unwrap(), (4, 5));
    assert_eq!(
        document.cell_span(0, 1, 0).unwrap(),
        (1, 3),
        "row 1's span must have grown by the inserted column"
    );
    // Row 0 is unaffected in kind (still every column its own cell) but now has 5 columns.
    assert_eq!(document.cell_text(0, 0, 0).unwrap(), "R0C0");
    assert_eq!(document.cell_text(0, 0, 4).unwrap(), "R0C3");
    // Column 3's own vertical merge, now at grid column 4, must be untouched.
    assert_eq!(document.cell_span(0, 0, 4).unwrap(), (3, 1));

    let discrepancies = document
        .table_grid_discrepancies(0)
        .expect("read discrepancies");
    assert_eq!(
        discrepancies,
        Vec::new(),
        "grid invariant must hold after the insert"
    );
}

#[test]
fn removing_a_column_inside_a_gridspan_shrinks_it_and_keeps_the_grid_coherent() {
    let mut document = ragged_fixture();
    // Column 1 falls strictly inside row 2's gridSpan=2 (columns 1-2).
    document.remove_column(0, 1).expect("remove column");

    assert_eq!(document.table_dimensions(0).unwrap(), (4, 3));
    assert_eq!(
        document.cell_span(0, 2, 1).unwrap(),
        (1, 1),
        "row 2's span must have shrunk to one"
    );
    assert_eq!(document.cell_text(0, 2, 1).unwrap(), "R2C12");
    // Row 1's own gridSpan=2 (originally columns 0-1) loses its second column entirely.
    assert_eq!(document.cell_span(0, 1, 0).unwrap(), (1, 1));
    assert_eq!(document.cell_text(0, 1, 0).unwrap(), "R1C01");

    let discrepancies = document
        .table_grid_discrepancies(0)
        .expect("read discrepancies");
    assert_eq!(
        discrepancies,
        Vec::new(),
        "grid invariant must hold after the removal"
    );
}

#[test]
fn removing_the_tables_only_row_or_column_is_refused() {
    let mut document = Document::blank(PageSize::a4()).expect("blank document");
    document.append_table(1, 1).expect("append 1x1 table");
    assert!(document.remove_row(0, 0).is_err());
    assert!(document.remove_column(0, 0).is_err());
}

// -------------------------------------------------------------------------------------------
// Nested tables — three deep, reading and round-tripping.
// -------------------------------------------------------------------------------------------

#[test]
fn a_table_nested_three_deep_reads_and_round_trips() {
    let mut document = Document::blank(PageSize::a4()).expect("blank document");
    document.append_table(1, 1).expect("outer table");
    document
        .edit_cell(0, 0, 0, |cell: &mut Cell, interner| {
            let mut middle = Table::new(interner, 1, 1);
            if let Some(inner_cell) = middle.cell_mut(interner, 0, 0) {
                inner_cell.append_table(Table::new(interner, 1, 1));
            }
            cell.append_table(middle);
        })
        .expect("nest two more tables into the outer table's own cell");
    document
        .set_cell_text(0, 0, 0, "outer")
        .expect("outer table's own top-level text still resolves through the same address");

    let saved = document.save().expect("save");
    let mut reopened = Document::open(&saved).expect("reopen");

    assert_eq!(reopened.cell_text(0, 0, 0).unwrap(), "outer");
    let outer_tables_in_cell = {
        let doc = &mut reopened;
        let (rows, columns) = doc.table_dimensions(0).unwrap();
        assert_eq!((rows, columns), (1, 1));
        doc.edit_cell(0, 0, 0, |cell, _interner| cell.tables().count())
            .expect("read nested table count")
    };
    assert_eq!(
        outer_tables_in_cell, 1,
        "exactly one middle table nested in the outer cell"
    );

    // Round-trips byte-for-byte on a second, unrelated save (nothing here touches the table).
    let saved_again = reopened.save().expect("save again");
    assert_eq!(
        saved, saved_again,
        "an untouched nested-table document must re-save identically"
    );
}

// -------------------------------------------------------------------------------------------
// Malformed grids — real files violate the invariant; this crate must expose it, never panic.
// -------------------------------------------------------------------------------------------

/// A row whose cells' spans do not sum to the grid's declared column count.
#[test]
fn a_short_row_is_exposed_as_a_discrepancy_not_a_panic() {
    let table = parse_table_fragment(
        r#"<w:tbl xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:tblPr/>
<w:tblGrid><w:gridCol/><w:gridCol/><w:gridCol/></w:tblGrid>
<w:tr><w:tc><w:p/></w:tc><w:tc><w:p/></w:tc></w:tr>
</w:tbl>"#,
    );
    let interner = mjx_ooxml_core::Interner::new();
    // Column 2 is beyond what row 0's two cells (spans summing to 2) actually cover.
    assert_eq!(
        table.cell(&interner, 0, 2),
        None,
        "must answer None, not panic, past a short row"
    );
    assert_eq!(table.merge_anchor(&interner, 0, 2), None);

    let discrepancies = table.grid_discrepancies(&interner);
    assert_eq!(
        discrepancies,
        vec![GridDiscrepancy::RowWidthMismatch {
            row: 0,
            declared_columns: 3,
            spanned_columns: 2
        }]
    );
}

/// A `w:vMerge` continuation with no `restart` reachable above it.
#[test]
fn an_orphaned_vertical_merge_is_exposed_as_a_discrepancy_not_a_panic() {
    let table = parse_table_fragment(
        r#"<w:tbl xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:tblPr/>
<w:tblGrid><w:gridCol/></w:tblGrid>
<w:tr><w:tc><w:tcPr><w:vMerge/></w:tcPr><w:p/></w:tc></w:tr>
</w:tbl>"#,
    );
    let interner = mjx_ooxml_core::Interner::new();
    assert_eq!(
        table.merge_anchor(&interner, 0, 0),
        None,
        "a bottomless continuation chain must answer None, not panic"
    );
    assert_eq!(
        table.grid_discrepancies(&interner),
        vec![GridDiscrepancy::OrphanedVerticalMerge { row: 0, column: 0 }]
    );
}

/// A row with zero cells.
#[test]
fn an_empty_row_is_exposed_as_a_discrepancy_not_a_panic() {
    let table = parse_table_fragment(
        r#"<w:tbl xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:tblPr/>
<w:tblGrid><w:gridCol/></w:tblGrid>
<w:tr></w:tr>
</w:tbl>"#,
    );
    let interner = mjx_ooxml_core::Interner::new();
    assert_eq!(table.cell(&interner, 0, 0), None);
    assert_eq!(
        table.grid_discrepancies(&interner),
        vec![GridDiscrepancy::EmptyRow { row: 0 }]
    );
}

fn parse_table_fragment(xml: &str) -> Table {
    let fragment_doc =
        mjx_xml::fidelity::parse(xml.as_bytes()).expect("fragment parses standalone");
    Table::from_xml(&fragment_doc.root, &fragment_doc.interner).expect("parses as a Table")
}

// -------------------------------------------------------------------------------------------
// Fixture generation — not run by `cargo test`; the record of how `ragged_table.docx` was built.
// -------------------------------------------------------------------------------------------

#[test]
#[ignore = "one-shot generator for the committed fixture; run manually with --ignored"]
fn regenerate_fixtures() {
    std::fs::write(
        mjx_fixtures::fixtures_dir().join("ragged_table.docx"),
        build_ragged_fixture(),
    )
    .expect("write ragged_table.docx");
}

fn build_ragged_fixture() -> Vec<u8> {
    let document = Document::blank(PageSize::a4()).expect("blank a4 document");
    let bytes = document.save().expect("intermediate save");

    // This crate's own public API changes an *existing* cell's gridSpan/vMerge in place, but
    // authoring the ragged geometry itself needs a table shaped exactly right from the start (see
    // this file's own module doc comment for why `set_cell_span` is the wrong tool for that).
    let document_part = mjx_opc::PartName::new("/word/document.xml")
        .expect("word/document.xml is a valid part name");
    let mut package = Package::open(&bytes).expect("reopen the intermediate package");
    let original = package
        .part_bytes(&document_part)
        .expect("word/document.xml exists")
        .to_vec();
    let original =
        String::from_utf8(original).expect("this crate's own writer only ever emits UTF-8");
    let with_table = original.replacen("<w:body>", &format!("<w:body>{}", ragged_table_xml()), 1);
    package
        .replace_part_bytes(&document_part, with_table.into_bytes())
        .expect("splice in the ragged table");
    package.save().expect("serialize the fixture package")
}

/// The literal `<w:tbl>` markup for the ragged 4×4 fixture — see this file's own module doc comment
/// for the geometry.
fn ragged_table_xml() -> &'static str {
    concat!(
        "<w:tbl>",
        "<w:tblPr/>",
        "<w:tblGrid><w:gridCol w:w=\"2000\"/><w:gridCol w:w=\"2000\"/><w:gridCol w:w=\"2000\"/>",
        "<w:gridCol w:w=\"2000\"/></w:tblGrid>",
        // Row 0 — the aligned control row; column 3 anchors the vertical merge.
        "<w:tr>",
        "<w:tc><w:p><w:r><w:t>R0C0</w:t></w:r></w:p></w:tc>",
        "<w:tc><w:p><w:r><w:t>R0C1</w:t></w:r></w:p></w:tc>",
        "<w:tc><w:p><w:r><w:t>R0C2</w:t></w:r></w:p></w:tc>",
        "<w:tc><w:tcPr><w:vMerge w:val=\"restart\"/></w:tcPr>",
        "<w:p><w:r><w:t>R0C3</w:t></w:r></w:p></w:tc>",
        "</w:tr>",
        // Row 1 — gridSpan=2 at columns 0-1; column 3 continues the vertical merge.
        "<w:tr>",
        "<w:tc><w:tcPr><w:gridSpan w:val=\"2\"/></w:tcPr>",
        "<w:p><w:r><w:t>R1C01</w:t></w:r></w:p></w:tc>",
        "<w:tc><w:p><w:r><w:t>R1C2</w:t></w:r></w:p></w:tc>",
        "<w:tc><w:tcPr><w:vMerge/></w:tcPr><w:p/></w:tc>",
        "</w:tr>",
        // Row 2 — gridSpan=2 at columns 1-2; column 3 continues the vertical merge.
        "<w:tr>",
        "<w:tc><w:p><w:r><w:t>R2C0</w:t></w:r></w:p></w:tc>",
        "<w:tc><w:tcPr><w:gridSpan w:val=\"2\"/></w:tcPr>",
        "<w:p><w:r><w:t>R2C12</w:t></w:r></w:p></w:tc>",
        "<w:tc><w:tcPr><w:vMerge/></w:tcPr><w:p/></w:tc>",
        "</w:tr>",
        // Row 3 — gridSpan=2 at columns 2-3; the vertical merge has closed (no w:vMerge here).
        "<w:tr>",
        "<w:tc><w:p><w:r><w:t>R3C0</w:t></w:r></w:p></w:tc>",
        "<w:tc><w:p><w:r><w:t>R3C1</w:t></w:r></w:p></w:tc>",
        "<w:tc><w:tcPr><w:gridSpan w:val=\"2\"/></w:tcPr>",
        "<w:p><w:r><w:t>R3C23</w:t></w:r></w:p></w:tc>",
        "</w:tr>",
        "</w:tbl>",
    )
}
