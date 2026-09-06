//! A workbook from nothing: two tabs, shared strings, numbers, a format — and every claim checked
//! against the reopened file rather than against the model that built it.
//!
//! ```sh
//! cargo run -p mjx-xlsx --example build_a_workbook -- out.xlsx
//! ```
//!
//! The runnable version of [the authoring page](mjx_xlsx::guide::authoring_a_workbook).
//! [`Workbook::blank`] is not a shell — `CT_Workbook` requires a `sheets` list, `CT_Sheets` requires
//! a `sheet`, and `CT_Worksheet` requires a `sheetData` — so a blank workbook is already a complete,
//! schema-valid package with `xl/styles.xml` and `xl/sharedStrings.xml` in it. Everything below adds
//! content to that.

use anyhow::{Context, Result};
use mjx_sml::write::{CellFormatSpec, CellFormatTarget, PatternFillSpec};
use mjx_sml::{CellReference, CellValue, FontProperties};
use mjx_xlsx::{PartClassification, PartKind, Workbook};

mod support;

/// The rows this example writes: a region, a quantity and a unit price.
const ROWS: [(&str, f64, f64); 3] = [
    ("North", 12.0, 9.99),
    ("South", 7.0, 4.5),
    ("East", 30.0, 1.25),
];

fn main() -> Result<()> {
    let out = support::output_path("build_a_workbook.xlsx");
    let reference = |text: &str| CellReference::parse(text).expect("a literal reference parses");

    let mut workbook = Workbook::blank().context("building a blank workbook")?;
    workbook
        .rename_sheet(0, "Sales")
        .context("renaming Sheet1")?;
    let notes = workbook.add_sheet("Notes").context("adding a second tab")?;
    println!(
        "tabs: {:?}",
        workbook
            .sheets()
            .iter()
            .map(|s| &s.name)
            .collect::<Vec<_>>()
    );

    // ---- A header row, through the shared-string table -------------------------------------------
    // `intern_shared_string` appends in *first-use* order and answers the index a `t="s"` cell's
    // `<v>` holds. Nothing in a workbook names a string; the index is its identity.
    let mut heading_indices = Vec::new();
    for (column, heading) in ["Region", "Quantity", "Price"].iter().enumerate() {
        let index = workbook
            .intern_shared_string(heading)
            .with_context(|| format!("interning {heading}"))?;
        heading_indices.push(index);
        let cell = CellReference::relative(u16::try_from(column)?, 0)?;
        workbook.set_cell_value(0, cell, CellValue::SharedString(index))?;
    }
    // First-use order, and no duplicate: interning "Region" again answers the index it already has.
    anyhow::ensure!(
        workbook.intern_shared_string("Region")? == heading_indices[0],
        "interning the same text twice must answer the same index"
    );

    // ---- The data rows ---------------------------------------------------------------------------
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

    // ---- A format for the header row ---------------------------------------------------------------
    // Four appends, four indices. `append_*` is the only mutation any style table offers: an entry's
    // *position* is its identity, so reordering or deduplicating one would silently repaint every
    // cell that referred to anything after the entry that moved.
    let bold = workbook.append_font(&FontProperties {
        font_name: Some("Calibri".to_owned()),
        size_in_points: Some(11.0),
        bold: Some(true),
        ..FontProperties::default()
    })?;
    let yellow = workbook.append_pattern_fill(&PatternFillSpec::solid("FFFF00"))?;
    let heading = workbook.append_cell_format(
        CellFormatTarget::CellFormats,
        &CellFormatSpec {
            font_index: Some(bold),
            fill_index: Some(yellow),
            applies_font: Some(true),
            applies_fill: Some(true),
            ..CellFormatSpec::skeleton_cell_format()
        },
    )?;
    for column in 0..3u16 {
        workbook.set_cell_style(0, CellReference::relative(column, 0)?, Some(heading))?;
    }

    // ---- A note on the second tab -------------------------------------------------------------------
    // An inline string lives in the cell itself rather than in the shared-string table — the right
    // shape for a one-off value nothing else will repeat.
    workbook.set_cell_value(
        notes,
        reference("A1"),
        CellValue::InlineString("Prices are per unit."),
    )?;

    let saved = workbook.save().context("saving")?;
    std::fs::write(&out, &saved).with_context(|| format!("writing {}", out.display()))?;
    println!("wrote {} ({} bytes)", out.display(), saved.len());

    // ---- Assert, against the reopened file -----------------------------------------------------------
    // Never against the model just built: a writer that dropped a namespace declaration would
    // satisfy every assertion made on the value it had in hand, and none of these.
    let reopened = Workbook::open(&saved).context("reopening what we wrote")?;
    anyhow::ensure!(
        reopened
            .sheets()
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>()
            == ["Sales", "Notes"],
        "the tabs did not come back as written"
    );
    anyhow::ensure!(
        reopened.cell_text(0, reference("A1"))?.as_deref() == Some("Region"),
        "A1's shared string did not resolve"
    );
    for (row, (region, quantity, price)) in ROWS.iter().enumerate() {
        let row_number = u32::try_from(row + 1)?;
        anyhow::ensure!(
            reopened
                .cell_text(0, CellReference::relative(0, row_number)?)?
                .as_deref()
                == Some(*region),
            "row {row_number}'s region did not come back"
        );
        for (column, value) in [(1u16, quantity), (2, price)] {
            let cell = CellReference::relative(column, row_number)?;
            let text = reopened
                .cell_text(0, cell)?
                .with_context(|| format!("{} has a value", cell.text().as_str()))?;
            let parsed: f64 = text.parse().context("the cell holds a number")?;
            anyhow::ensure!(
                (parsed - *value).abs() < f64::EPSILON,
                "{} came back as {text}, not {value}",
                cell.text().as_str()
            );
        }
    }
    anyhow::ensure!(
        reopened.cell_text(notes, reference("A1"))?.as_deref() == Some("Prices are per unit."),
        "the inline string on the second tab did not come back"
    );

    // The format resolved through the `xf` indirection, on the reopened file's own stylesheet.
    let format = reopened
        .effective_cell_format(0, reference("B1"))?
        .context("B1 has an effective format")?;
    anyhow::ensure!(
        format.font().resource_index == Some(bold),
        "B1 did not come back wearing the bold font that was appended for it"
    );
    anyhow::ensure!(
        format.fill().resource_index == Some(yellow),
        "B1 did not come back wearing the yellow fill"
    );
    println!("header row resolves to font {bold} / fill {yellow} on the reopened file");

    // Two tabs means two worksheet parts, and this crate classified both.
    let worksheets = reopened
        .part_inventory()
        .into_iter()
        .filter(|entry| entry.classification == PartClassification::Classified(PartKind::Worksheet))
        .count();
    anyhow::ensure!(
        worksheets == 2,
        "expected two worksheet parts, found {worksheets}"
    );
    println!(
        "part inventory: {worksheets} worksheet part(s), and every part classified or carried"
    );

    Ok(())
}
