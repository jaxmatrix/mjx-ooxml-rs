//! A worksheet table with an autofilter over it — four things written together, and a filter that
//! hides nothing.
//!
//! ```sh
//! cargo run -p mjx-xlsx --example table_and_autofilter -- out.xlsx
//! ```
//!
//! The runnable version of [the tables page](mjx_xlsx::guide::worksheet_tables) and
//! [the filters page](mjx_xlsx::guide::filters_and_data_validation). Two things this example exists
//! to make concrete:
//!
//! * **A table is a part of its own.** `add_table` writes `/xl/tables/tableN.xml`, its content-type
//!   override, a relationship from the *sheet*, and a `tablePart` entry — and **touches no cell**.
//!   The headings in the spec go into the table part; writing them into row 1 is the caller's, which
//!   is why this example does it explicitly and then asserts both halves agree.
//! * **A filter records, it does not apply.** `set_auto_filter` writes an `x:autoFilter`; no row
//!   gains a `@hidden`, because hiding rows is a consumer's business and doing it here would edit
//!   cells nobody named.

use anyhow::{Context, Result};
use mjx_sml::{
    AutoFilterSpec, CellRange, CellReference, CellValue, FilterColumnSpec, FilterSpecKind,
    WorksheetTableSpec,
};
use mjx_xlsx::{PartKind, Workbook};

mod support;

/// The table's headings, and its `@displayName` — what a structured reference (`Sales[Region]`)
/// names. §18.5.1.2: it "shall not have any spaces in it".
const HEADINGS: [&str; 3] = ["Region", "Quantity", "Price"];
const ROWS: [(&str, f64, f64); 4] = [
    ("North", 12.0, 9.99),
    ("South", 7.0, 4.5),
    ("East", 30.0, 1.25),
    ("West", 4.0, 12.0),
];
/// Header row plus four data rows: `A1:C5`.
const RANGE: &str = "A1:C5";

fn main() -> Result<()> {
    let out = support::output_path("table_and_autofilter.xlsx");
    let mut workbook = Workbook::blank().context("building a blank workbook")?;
    workbook.rename_sheet(0, "Sales")?;

    // ---- The cells, which `add_table` will not write for us ----------------------------------------
    for (column, heading) in HEADINGS.iter().enumerate() {
        let index = workbook.intern_shared_string(heading)?;
        workbook.set_cell_value(
            0,
            CellReference::relative(u16::try_from(column)?, 0)?,
            CellValue::SharedString(index),
        )?;
    }
    for (row, (region, quantity, price)) in ROWS.iter().enumerate() {
        let row_number = u32::try_from(row + 1)?;
        let region_index = workbook.intern_shared_string(region)?;
        workbook.set_cell_value(
            0,
            CellReference::relative(0, row_number)?,
            CellValue::SharedString(region_index),
        )?;
        workbook.set_cell_value(
            0,
            CellReference::relative(1, row_number)?,
            CellValue::Number(*quantity),
        )?;
        workbook.set_cell_value(
            0,
            CellReference::relative(2, row_number)?,
            CellValue::Number(*price),
        )?;
    }

    // ---- The table -----------------------------------------------------------------------------------
    // `spec.id` is ignored: only something holding the package knows which `@id`s are taken, so
    // `add_table` allocates it from `next_table_id`.
    let range = CellRange::parse(RANGE).context("parsing the table range")?;
    let created = workbook
        .add_table(0, &WorksheetTableSpec::new("SalesTable", range, &HEADINGS))
        .context("adding the table")?;
    println!(
        "table {} -> {} (@id {}), columns {:?}",
        created.display_name,
        created.part.as_str(),
        created.id,
        created
            .columns
            .iter()
            .map(|column| &column.name)
            .collect::<Vec<_>>(),
    );

    // ---- The sheet's own autofilter ---------------------------------------------------------------
    // Distinct from the table's own `x:autoFilter`, which `WorksheetTableSpec::auto_filter` carries.
    // The column index is table-relative: 0 is the range's first column.
    workbook
        .set_auto_filter(
            0,
            &AutoFilterSpec::over(range).with_column(FilterColumnSpec::new(
                0,
                FilterSpecKind::values(["North", "South"]),
            )),
        )
        .context("setting the autofilter")?;

    let saved = workbook.save().context("saving")?;
    std::fs::write(&out, &saved).with_context(|| format!("writing {}", out.display()))?;
    println!("wrote {} ({} bytes)", out.display(), saved.len());

    // ---- Assert, on the reopened file ------------------------------------------------------------------
    let reopened = Workbook::open(&saved).context("reopening")?;

    let tables = reopened
        .sheet_tables(0)
        .context("reading the sheet's tables")?;
    anyhow::ensure!(
        tables.len() == 1,
        "expected one table, found {}",
        tables.len()
    );
    let table = &tables[0];
    anyhow::ensure!(
        table.display_name == "SalesTable",
        "the display name did not survive"
    );
    anyhow::ensure!(
        table.range.text().as_str() == RANGE,
        "the table's @ref came back as {}, not {RANGE}",
        table.range.text().as_str()
    );
    anyhow::ensure!(table.header_row_count == 1 && table.totals_row_count == 0);
    anyhow::ensure!(
        table.data_row_count() == Some(u32::try_from(ROWS.len())?),
        "the table should have {} data rows",
        ROWS.len()
    );
    anyhow::ensure!(
        table
            .columns
            .iter()
            .map(|column| column.name.as_str())
            .collect::<Vec<_>>()
            == HEADINGS,
        "the table's column names do not match its headings"
    );

    // The table part is really a part: related from the *sheet*, and content-typed as a table.
    let worksheet = reopened
        .worksheet(0)?
        .context("the first tab is a worksheet")?;
    anyhow::ensure!(
        worksheet.parts().tables.contains(&table.part),
        "the sheet does not relate to the table part it names"
    );
    anyhow::ensure!(
        reopened
            .part_inventory()
            .iter()
            .any(|entry| entry.part == table.part
                && entry.classification
                    == mjx_xlsx::PartClassification::Classified(PartKind::Table)),
        "the table part was not classified as a table"
    );

    // The headings in row 1 and the headings in the table part agree — because this example wrote
    // both. `add_table` writes neither into the other.
    for (column, heading) in HEADINGS.iter().enumerate() {
        let cell = CellReference::relative(u16::try_from(column)?, 0)?;
        anyhow::ensure!(
            reopened.cell_text(0, cell)?.as_deref() == Some(*heading),
            "row 1 column {column} does not hold {heading}"
        );
    }

    // The filter came back, over the same range.
    let filter_range = reopened
        .auto_filter(0, |part, filter| {
            filter.and_then(|filter| filter.range(part.interner()).ok().flatten())
        })?
        .context("the tab reaches a worksheet part")?
        .context("the sheet has an autofilter with a @ref")?;
    anyhow::ensure!(
        filter_range.text().as_str() == RANGE,
        "the autofilter's @ref came back as {}, not {RANGE}",
        filter_range.text().as_str()
    );

    // …and it hid nothing. Every data row is still visible, because recording a filter is not
    // applying one.
    // "North" and "South" are among the four regions, so a library that *applied* this filter
    // would hide the other two rows. This one records it and hides nothing.
    let markup = reopened
        .worksheet_markup(0)?
        .context("the worksheet part reads")?;
    let rows: Vec<_> = markup.rows().collect();
    let hidden = rows.iter().filter(|row| row.is_hidden()).count();
    anyhow::ensure!(
        hidden == 0,
        "{hidden} row(s) were hidden — a filter applied itself"
    );
    println!(
        "autofilter over {RANGE} on {} row(s); {hidden} hidden — recorded, never applied",
        rows.len()
    );

    Ok(())
}
