//! What resolving a chart's `c:f` against a big sheet costs — **measured**, not described
//! (MJXOFF-111, E4).
//!
//! ```sh
//! cargo run -p mjx-xlsx --example chart_range_cost
//! ```
//!
//! # The claim this exists to hold
//!
//! MJXOFF-135 measured this crate's read path on a 300,000-cell worksheet and MJXOFF-137 acted on
//! the measurement: no per-cell accessor ships on the facade or either binding, because reaching
//! one cell costs a whole-worksheet parse. A range resolver that then **materialised the sheet to
//! answer one series** would give that measurement straight back — the ticket says so in as many
//! words, and `mjx-allocation-counter` is the instrument that can prove it does not, because a
//! byte count is deterministic where a stopwatch is not.
//!
//! Two things are asserted here, and neither could be asserted by inspection:
//!
//! 1. **The resolution is bounded by the range, not by the sheet.** Resolving a three-cell range
//!    and resolving a three-thousand-cell range on the *same* worksheet differ by what the larger
//!    one carries, not by anything to do with the sheet's size. Both are measured against the same
//!    baseline — one `worksheet_markup` read of that sheet, which is MJXOFF-153's cost and not this
//!    child's — so what is compared is the resolver's own contribution.
//! 2. **A sheet is parsed once per call, however many series name it.** Resolving four references
//!    into the same sheet costs one parse, not four. The resolver caches the worksheet part and the
//!    shared-string table for the length of one call, which is what keeps a four-series chart from
//!    paying the read path four times over.
//!
//! **Why `total_allocated` and not `peak`.** Peak is the high-water mark of *live* bytes, so a
//! transient buffer that is freed before the next one is allocated does not raise it — which is
//! exactly the shape a per-cell loop has. Total allocated counts every request the program made, so
//! a resolver that touched a hundred thousand cells to answer three cannot hide in it. MJXOFF-137
//! used the same figure for the same reason.

use anyhow::{Context, Result};
use mjx_chart::ChartKind;
use mjx_dml::spreadsheet_drawing::CellMarker;
use mjx_ooxml_types::spreadsheetdrawing::ResizingBehavior;
use mjx_sml::{CellReference, CellValue};
use mjx_xlsx::{SheetChartSeries, SheetChartSource, Workbook};

mod support;

/// The counting allocator, from the crate `xtask`'s fuzz campaign and `mjx-sml`'s cell-store gate
/// both install. One `unsafe impl GlobalAlloc` in the workspace outside the two binding crates.
#[global_allocator]
static ALLOCATOR: mjx_allocation_counter::Counting = mjx_allocation_counter::Counting;

/// Rows of the sheet the ranges are resolved against. Thirty thousand cells is enough for the
/// sheet's cost to dwarf any range's, and small enough that an unoptimised build — which is what CI
/// runs every example in — finishes quickly.
const ROWS: u32 = 5_000;
/// Columns of that sheet.
const COLUMNS: u16 = 6;

/// How many cells the small range names — a chart series of the size a real one has.
const SMALL_RANGE: u32 = 3;
/// How many cells the large range names, a thousand times the small one.
const LARGE_RANGE: u32 = 3_000;

/// What the small resolution may allocate **beyond the sheet parse it shares with the baseline**.
///
/// Generous by three orders of magnitude against what the sheet itself costs, and still far under
/// what materialising 30,000 cells would take: the point is the *shape* of the answer, not a tight
/// figure that would rot on the next allocator change.
const SMALL_RANGE_BOUND: usize = 64 * 1024;

fn main() -> Result<()> {
    let workbook = a_sheet_with_a_chart_over_part_of_it()?;
    let (baseline, small, large) = what_each_resolution_costs(&workbook)?;
    a_sheet_is_parsed_once_however_many_series_name_it(&workbook, baseline)?;
    println!("\nevery case passed");
    anyhow::ensure!(baseline > 0 && small > 0 && large > 0);
    Ok(())
}

/// A workbook with a `ROWS` × `COLUMNS` sheet and a live-range chart over three of its cells.
fn a_sheet_with_a_chart_over_part_of_it() -> Result<Vec<u8>> {
    let mut workbook = Workbook::blank().context("building a blank workbook")?;
    workbook.rename_sheet(0, "Data")?;
    {
        // The bulk path, for the reason `large_sparse_sheet.rs` states: `set_cell_value` in a loop
        // is quadratic, which is a fact about MJXOFF-153's read path and not about this example.
        let mut markup = workbook
            .worksheet_markup(0)?
            .context("the tab reaches a worksheet part")?;
        let sheet = markup.sheet_data_or_insert();
        for row in 0..ROWS {
            for column in 0..COLUMNS {
                sheet.set_cell_value(
                    CellReference::relative(column, row)?,
                    CellValue::Number(f64::from(row) * 10.0 + f64::from(column)),
                )?;
            }
        }
        workbook.write_worksheet_markup(0, &markup)?;
    }

    let source = SheetChartSource {
        categories: Some(format!("Data!$A$1:$A${SMALL_RANGE}")),
        series: vec![SheetChartSeries {
            name_cell: None,
            name: "Series".to_owned(),
            values: format!("Data!$B$1:$B${SMALL_RANGE}"),
        }],
    };
    workbook
        .add_range_chart(
            0,
            ChartKind::Line,
            &source,
            CellMarker::new(7, 0, 1, 0),
            CellMarker::new(13, 0, 15, 0),
            "Cost",
            ResizingBehavior::MoveWithCellsButDoNotResize,
        )
        .context("anchoring a live-range chart")?;

    let saved = workbook.save().context("saving")?;
    let out = support::output_path("chart_range_cost.xlsx");
    std::fs::write(&out, &saved).with_context(|| format!("writing {}", out.display()))?;
    println!(
        "wrote {} ({} bytes, {} cells)",
        out.display(),
        saved.len(),
        u64::from(ROWS) * u64::from(COLUMNS)
    );
    Ok(saved)
}

/// Measures the sheet parse alone, a three-cell resolution and a three-thousand-cell one.
fn what_each_resolution_costs(saved: &[u8]) -> Result<(usize, usize, usize)> {
    // The baseline: what reading this worksheet costs, with no range resolved at all. Everything
    // below is reported net of it, because that cost is MJXOFF-153's and not this child's.
    let baseline = {
        let workbook = Workbook::open(saved).context("reopening")?;
        let before = mjx_allocation_counter::total_allocated();
        let markup = workbook
            .worksheet_markup(0)?
            .context("the tab reaches a worksheet part")?;
        let total = mjx_allocation_counter::total_allocated().saturating_sub(before);
        anyhow::ensure!(markup.cell_count() > 0, "the sheet really was populated");
        total
    };

    let measure = |cells: u32| -> Result<usize> {
        let mut workbook = Workbook::open(saved).context("reopening")?;
        let reference = format!("Data!$B$1:$B${cells}");
        let before = mjx_allocation_counter::total_allocated();
        let resolved = workbook.resolve_range_reference(0, &reference)?;
        let total = mjx_allocation_counter::total_allocated().saturating_sub(before);
        anyhow::ensure!(
            resolved.cells.len() == cells as usize,
            "{reference} resolved {} cells, not {cells}",
            resolved.cells.len()
        );
        Ok(total)
    };
    let small = measure(SMALL_RANGE)?;
    let large = measure(LARGE_RANGE)?;
    let small_net = small.saturating_sub(baseline);
    let large_net = large.saturating_sub(baseline);

    println!("\ncase 1 — the resolution is bounded by the range, not by the sheet");
    println!("  one worksheet_markup read      {baseline:>12} bytes  (MJXOFF-153's cost)");
    println!("  resolving {SMALL_RANGE:>4} cells             {small:>12} bytes  ({small_net} net of the read)");
    println!("  resolving {LARGE_RANGE:>4} cells             {large:>12} bytes  ({large_net} net of the read)");
    println!("  small-range bound, net         {SMALL_RANGE_BOUND:>12} bytes");

    anyhow::ensure!(
        small_net <= SMALL_RANGE_BOUND,
        "resolving a {SMALL_RANGE}-cell range on a {}-cell sheet allocated {small_net} bytes beyond \
         the sheet's own read, over the {SMALL_RANGE_BOUND}-byte bound. A resolver that \
         materialises the sheet to answer one series defeats the design it stands on.",
        u64::from(ROWS) * u64::from(COLUMNS)
    );
    anyhow::ensure!(
        large_net > small_net,
        "a range a thousand times larger allocated no more than the small one ({large_net} vs \
         {small_net}) — which would mean the figure is measuring the sheet rather than the range, \
         and the bound above proves nothing"
    );
    Ok((baseline, small, large))
}

/// Four references into the same sheet cost one parse of it, not four.
fn a_sheet_is_parsed_once_however_many_series_name_it(saved: &[u8], baseline: usize) -> Result<()> {
    let mut workbook = Workbook::open(saved).context("reopening")?;

    // Four separate calls: four parses, because each call opens its own resolver.
    let separate = {
        let before = mjx_allocation_counter::total_allocated();
        for column in ["B", "C", "D", "E"] {
            workbook.resolve_range_reference(0, &format!("Data!${column}$1:${column}$3"))?;
        }
        mjx_allocation_counter::total_allocated().saturating_sub(before)
    };

    // One call naming the same four ranges as four areas: one parse, because the resolver caches
    // the worksheet part for the length of the call. This is the property a chart with four series
    // depends on — `chart_series_freshness` resolves every reference through one resolver.
    let together = {
        let before = mjx_allocation_counter::total_allocated();
        workbook.resolve_range_reference(
            0,
            "Data!$B$1:$B$3,Data!$C$1:$C$3,Data!$D$1:$D$3,Data!$E$1:$E$3",
        )?;
        mjx_allocation_counter::total_allocated().saturating_sub(before)
    };

    println!("\ncase 2 — a sheet is parsed once per call, however many areas name it");
    println!("  four calls, one area each      {separate:>12} bytes");
    println!("  one call, four areas           {together:>12} bytes");
    println!("  one worksheet_markup read      {baseline:>12} bytes");

    anyhow::ensure!(
        together < separate,
        "four areas in one call ({together} bytes) cost no less than four calls ({separate}), so \
         the resolver is not caching the worksheet part it was written to cache"
    );
    anyhow::ensure!(
        together < baseline * 2,
        "one call over four areas of one sheet allocated {together} bytes against a single \
         worksheet read's {baseline} — more than one parse's worth, which is the caching this case \
         exists to hold"
    );
    Ok(())
}
