//! Building a table from nothing, merging cells, and reading the grid back — the runnable version
//! of [the tables page](mjx_docx::guide::tables_sections_and_headers).
//!
//! ```sh
//! cargo run -p mjx-docx --example build_table -- out.docx
//! ```
//!
//! The interesting assertion is the merge one. `(row, column)` addresses a *grid* position, and a
//! position covered by a merge is not the cell that holds the text — `merged_cell_anchor` says which
//! cell it is. Checking that on the reopened file is the only way to know the merge was written and
//! not merely requested.

use anyhow::{Context, Result};
use mjx_docx::{Document, MergedCellType, PageSize};

mod support;

fn main() -> Result<()> {
    let out = support::output_path("build_table.docx");
    let mut document = Document::blank(PageSize::us_letter()).context("blank document")?;
    document.insert_run(0, 0, "Regional results")?;

    // ---- A 4x3 table -----------------------------------------------------------------------------
    // Every cell starts with one empty paragraph, so `set_cell_text` has somewhere to write.
    let table = document.append_table(4, 3).context("appending a table")?;
    let rows = [
        ["Region", "Q3", "Q4"],
        ["North America", "1,200", "1,340"],
        ["EMEA", "980", "1,020"],
        ["Total", "2,180", "2,360"],
    ];
    for (row, cells) in rows.iter().enumerate() {
        for (column, text) in cells.iter().enumerate() {
            document.set_cell_text(table, row, column, text)?;
        }
    }
    let (row_count, column_count) = document.table_dimensions(table)?;
    println!("table {table}: {row_count} x {column_count}");
    anyhow::ensure!((row_count, column_count) == (4, 3), "unexpected dimensions");

    // ---- A horizontal merge ------------------------------------------------------------------------
    // `w:gridSpan` on the last row's first cell: "Total" now spans two grid columns, so grid position
    // (3, 1) is covered by the cell anchored at (3, 0).
    document.set_cell_span(table, 3, 0, Some(2))?;
    let span = document.cell_span(table, 3, 0)?;
    let anchor = document.merged_cell_anchor(table, 3, 1)?;
    println!("cell (3,0) spans {span:?}; grid position (3,1) is anchored at {anchor:?}");
    anyhow::ensure!(span == (1, 2), "the horizontal span was not written");
    anyhow::ensure!(
        anchor == (3, 0),
        "the covered position resolves to the wrong anchor"
    );

    // ---- A vertical merge --------------------------------------------------------------------------
    // `w:vMerge` is a two-part construct: the anchor restarts it, every continuation cell continues
    // it. Rows 1 and 2 of column 0 become one cell.
    document.set_cell_vertical_merge(table, 1, 0, Some(MergedCellType::Restart))?;
    document.set_cell_vertical_merge(table, 2, 0, Some(MergedCellType::Continue))?;
    anyhow::ensure!(
        document.merged_cell_anchor(table, 2, 0)? == (1, 0),
        "the vertical merge continuation does not resolve to its anchor"
    );

    // ---- Structural edits ---------------------------------------------------------------------------
    // Inserting a row inside a vertical merge grows the merge to include it, rather than splitting it.
    document.insert_row(table, 2)?;
    document.set_cell_text(table, 2, 1, "inserted")?;
    anyhow::ensure!(
        document.table_dimensions(table)? == (5, 3),
        "the row insert did not land"
    );
    anyhow::ensure!(
        document.merged_cell_anchor(table, 3, 0)? == (1, 0),
        "the vertical merge should still reach the row that moved down"
    );

    // ---- The table's own properties ------------------------------------------------------------------
    // `edit_table` is the escape hatch for `w:tblPr` — the style reference, `w:tblLook`, band sizes —
    // none of which has a narrower method, because there are dozens of them.
    document.edit_table(table, |table, interner| {
        if let Some(properties) = table.properties_mut() {
            properties.set_style_id(interner, Some("TableGrid"));
        }
    })?;

    // ---- Does the grid agree with the rows? ------------------------------------------------------------
    // `w:tblGrid` declares the column count; a row's cells, their `w:gridSpan`s summed, may not reach
    // it — or, as here, may overshoot it. Widening the "Total" cell above did **not** delete the cell
    // it now covers, because this library never removes content a caller did not ask it to remove, so
    // that row's three cells now span four grid columns against a grid that declares three.
    // `table_grid_discrepancies` reports exactly that, by name and by number, instead of silently
    // normalising the table — which is what a real-world `.docx` needs, since Word writes tables that
    // disagree with their own grids and a reader that "fixes" them loses the file's own intent.
    let discrepancies = document.table_grid_discrepancies(table)?;
    println!("grid discrepancies: {}", discrepancies.len());
    for discrepancy in &discrepancies {
        println!("  {discrepancy:?}");
    }
    anyhow::ensure!(
        discrepancies
            == [mjx_docx::GridDiscrepancy::RowWidthMismatch {
                row: 4,
                declared_columns: 3,
                spanned_columns: 4,
            }],
        "the widened row should be reported as a width mismatch, and nothing else should be"
    );

    // ---- Save, reopen, and check the merges came back ----------------------------------------------------
    let bytes = document.save().context("saving")?;
    std::fs::write(&out, &bytes).with_context(|| format!("writing {}", out.display()))?;
    println!("wrote {} ({} bytes)", out.display(), bytes.len());

    let mut reopened = Document::open(&bytes).context("reopening")?;
    anyhow::ensure!(
        reopened.table_count()? == 1,
        "the table did not survive the round trip"
    );
    anyhow::ensure!(
        reopened.cell_text(0, 0, 0)? == "Region",
        "the header cell did not survive the round trip"
    );
    anyhow::ensure!(
        reopened.cell_span(0, 4, 0)? == (1, 2),
        "the horizontal merge did not survive the round trip"
    );
    anyhow::ensure!(
        reopened.merged_cell_anchor(0, 3, 0)? == (1, 0),
        "the vertical merge did not survive the round trip"
    );
    let style = reopened.edit_table(0, |table, interner| {
        table
            .properties_mut()
            .and_then(|properties| properties.style_id(interner).ok().flatten())
    })?;
    anyhow::ensure!(
        style.as_deref() == Some("TableGrid"),
        "the table style reference did not survive the round trip"
    );
    println!("reopened: merges and style reference intact");

    Ok(())
}
