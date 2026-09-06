//! Styling a range: one `xf` appended once, then pointed at from every cell that wears it — and
//! proved by resolving the format back through the reopened file's own stylesheet.
//!
//! ```sh
//! cargo run -p mjx-xlsx --example style_a_range -- out.xlsx
//! ```
//!
//! There is no "style this range" call, and that is the shape of the format model rather than a gap:
//! a cell's format is one attribute, `c@s`, holding an **index** into `cellXfs`. So styling a range
//! is one append and N cheap pointers, and the interesting assertion is that all N resolve to the
//! same font, fill and border while a cell just outside the range resolves to none of them.

use anyhow::{Context, Result};
use mjx_ooxml_types::spreadsheetml::BorderStyle;
use mjx_sml::write::{
    BorderEdgeSpec, BorderSpec, CellFormatSpec, CellFormatTarget, PatternFillSpec,
};
use mjx_sml::{CellRange, CellReference, CellValue, FontProperties};
use mjx_xlsx::Workbook;

mod support;

/// The styled block: `B2:D4`, nine cells.
const RANGE: &str = "B2:D4";

fn main() -> Result<()> {
    let out = support::output_path("style_a_range.xlsx");
    let reference = |text: &str| CellReference::parse(text).expect("a literal reference parses");

    let mut workbook = Workbook::blank().context("building a blank workbook")?;
    workbook.rename_sheet(0, "Grid")?;

    let range: CellRange = CellRange::parse(RANGE).context("parsing the range")?;
    let bounds = range.normalized_bounds();

    // Values first, so the styling below lands on cells that hold something. `set_cell_style` would
    // create a blank cell if it did not — a blank cell is not nothing: it carries a format and no
    // value, which is exactly what a shaded but empty cell is.
    let mut cells = Vec::new();
    for row in bounds.first_row()..=bounds.last_row() {
        for column in bounds.first_column()..=bounds.last_column() {
            let cell = CellReference::relative(column, row)?;
            let value = f64::from(row + 1) * 10.0 + f64::from(column + 1);
            workbook.set_cell_value(0, cell, CellValue::Number(value))?;
            cells.push((cell, value));
        }
    }
    // One cell outside the block, to prove the styling is a range and not the sheet.
    workbook.set_cell_value(0, reference("A1"), CellValue::Number(1.0))?;

    // ---- One format, appended once -----------------------------------------------------------------
    let font = workbook.append_font(&FontProperties {
        font_name: Some("Calibri".to_owned()),
        size_in_points: Some(12.0),
        bold: Some(true),
        italic: Some(true),
        ..FontProperties::default()
    })?;
    let fill = workbook.append_pattern_fill(&PatternFillSpec::solid("D9E1F2"))?;
    let border = workbook.append_border(&BorderSpec {
        top: Some(BorderEdgeSpec::styled(BorderStyle::Thin)),
        bottom: Some(BorderEdgeSpec::styled(BorderStyle::Thin)),
        left: Some(BorderEdgeSpec::styled(BorderStyle::Thin)),
        right: Some(BorderEdgeSpec::styled(BorderStyle::Thin)),
        ..BorderSpec::skeleton_border()
    })?;
    let style = workbook.append_cell_format(
        CellFormatTarget::CellFormats,
        &CellFormatSpec {
            font_index: Some(font),
            fill_index: Some(fill),
            border_index: Some(border),
            applies_font: Some(true),
            applies_fill: Some(true),
            applies_border: Some(true),
            ..CellFormatSpec::skeleton_cell_format()
        },
    )?;
    println!("appended font {font}, fill {fill}, border {border} -> cellXfs index {style}");

    for (cell, _) in &cells {
        workbook.set_cell_style(0, *cell, Some(style))?;
    }

    let saved = workbook.save().context("saving")?;
    std::fs::write(&out, &saved).with_context(|| format!("writing {}", out.display()))?;
    println!("wrote {} ({} bytes)", out.display(), saved.len());

    // ---- Assert, on the reopened file ----------------------------------------------------------------
    let reopened = Workbook::open(&saved).context("reopening")?;
    anyhow::ensure!(cells.len() == 9, "the range should cover nine cells");
    for (cell, value) in &cells {
        let text = reopened
            .cell_text(0, *cell)?
            .with_context(|| format!("{} holds a value", cell.text().as_str()))?;
        anyhow::ensure!(
            text.parse::<f64>().ok() == Some(*value),
            "{} came back as {text}, not {value}",
            cell.text().as_str()
        );
        let format = reopened
            .effective_cell_format(0, *cell)?
            .with_context(|| format!("{} has an effective format", cell.text().as_str()))?;
        anyhow::ensure!(
            format.font().resource_index == Some(font)
                && format.fill().resource_index == Some(fill)
                && format.border().resource_index == Some(border),
            "{} did not resolve to the appended font/fill/border",
            cell.text().as_str()
        );
    }
    println!("all {} cells of {RANGE} resolve to xf {style}", cells.len());

    // A1 is outside the block: it wears whatever the workbook's own default `xf` says, which is not
    // the one appended above. This is the assertion that would catch a "style the whole sheet" bug.
    let outside = reopened
        .effective_cell_format(0, reference("A1"))?
        .context("A1 has an effective format")?;
    anyhow::ensure!(
        outside.font().resource_index != Some(font),
        "A1 is outside {RANGE} and must not wear the range's font"
    );
    anyhow::ensure!(
        outside.fill().resource_index != Some(fill),
        "A1 is outside {RANGE} and must not wear the range's fill"
    );
    println!(
        "A1, outside the range, resolves to font {:?} — untouched",
        outside.font().resource_index
    );

    Ok(())
}
