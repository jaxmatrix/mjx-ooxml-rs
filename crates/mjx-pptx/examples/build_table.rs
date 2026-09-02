//! Tables: creation, cell text, bulk formatting through a selection, merging, and a style.
//!
//! ```sh
//! cargo run -p mjx-pptx --example build_table -- out.pptx
//! ```

use anyhow::Result;
use mjx_dml::{
    CellBorder, CharacterPropertiesSpec, ColorSpec, FillSpec, LineSpec, LineWidth, OnOffStyle,
    TableStylePart,
};
use mjx_pptx::{
    CellFormat, Cells, Presentation, ShapeBounds, TableStyleDefinition, TableStyleFormat,
};

mod support;

const ROWS: [[&str; 3]; 4] = [
    ["Region", "Q1", "Q2"],
    ["North", "4.2", "4.9"],
    ["South", "3.1", "3.4"],
    ["Total", "7.3", "8.3"],
];

fn main() -> Result<()> {
    let out = support::output_path("build_table.pptx");
    let mut deck = Presentation::open(&support::template()?)?;
    let slide = deck.add_slide_from_layout(2)?;

    // ---- The grid --------------------------------------------------------------------------
    let table = deck.add_table(
        slide,
        ROWS.len(),
        ROWS[0].len(),
        ShapeBounds::from_inches(0.75, 1.25, 8.0, 3.0),
    )?;

    for (row, cells) in ROWS.iter().enumerate() {
        for (column, text) in cells.iter().enumerate() {
            deck.set_cell_text(slide, table, row, column, 0, text)?;
        }
    }

    // ---- Column widths ---------------------------------------------------------------------
    use mjx_dml::Emu;
    deck.set_column_width(slide, table, 0, Emu::from_emu(3_657_600))?; // 4"
    deck.set_column_width(slide, table, 1, Emu::from_emu(1_828_800))?; // 2"
    deck.set_column_width(slide, table, 2, Emu::from_emu(1_828_800))?;

    // ---- Bulk formatting through a selection -----------------------------------------------
    // `Cells` names the selection and `CellFormat` names the change, so a header row is one call
    // rather than a loop over cells.
    deck.format_cells(
        slide,
        table,
        Cells::row(0),
        &CellFormat::new()
            .with_fill(FillSpec::solid(ColorSpec::Srgb("1F3864".into())))
            .with_border(
                CellBorder::Bottom,
                LineSpec::solid(
                    LineWidth::from_points(1.5),
                    ColorSpec::Srgb("FFC000".into()),
                ),
            ),
    )?;
    deck.format_cell_text(
        slide,
        table,
        Cells::row(0),
        &CharacterPropertiesSpec::new()
            .with_bold(true)
            .with_color(ColorSpec::Srgb("FFFFFF".into())),
    )?;

    // The numeric columns, as a rectangle rather than two calls.
    deck.format_cell_text(
        slide,
        table,
        Cells::rectangle(1..ROWS.len(), 1..3),
        &CharacterPropertiesSpec::new().with_size_points(14.0),
    )?;

    // ---- A merge ---------------------------------------------------------------------------
    // Merging keeps the covered cells in the file — that is how OOXML models it, and it is what
    // lets `unmerge_cells` put the grid back.
    deck.merge_cells(slide, table, Cells::rectangle(3..4, 1..3))?;
    deck.set_cell_text(slide, table, 3, 1, 0, "15.6 combined")?;

    // `cell_span` answers `(rows, columns)`, the same order as `table_dimensions`.
    let (row_span, column_span) = deck.cell_span(slide, table, 3, 1)?;
    println!("the total row spans {column_span} columns × {row_span} rows");
    println!(
        "covered cell (3,2) still holds {:?}, but shows {:?}",
        deck.cell_text(slide, table, 3, 2)?,
        deck.visible_cell_text(slide, table, 3, 2)?
    );

    // ---- An inline style -------------------------------------------------------------------
    // Inline means the definition travels with the table rather than living in tableStyles.xml.
    deck.set_inline_table_style(
        slide,
        table,
        &TableStyleDefinition::new().with_name("Report").with_part(
            TableStylePart::FirstRow,
            TableStyleFormat::new().with_bold(OnOffStyle::On),
        ),
    )?;

    let bytes = deck.save()?;
    std::fs::write(&out, &bytes)?;

    let mut reopened = Presentation::open(&bytes)?;
    let (rows, columns) = reopened.table_dimensions(slide, table)?;
    anyhow::ensure!((rows, columns) == (ROWS.len(), ROWS[0].len()));
    anyhow::ensure!(reopened.cell_text(slide, table, 0, 0)? == "Region");
    anyhow::ensure!(reopened.cell_span(slide, table, 3, 1)? == (1, 2)); // (rows, columns)
    println!("wrote {} and verified", out.display());

    Ok(())
}
