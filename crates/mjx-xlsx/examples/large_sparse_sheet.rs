//! A large, sparse sheet — and the memory figure that makes it possible, **measured** rather than
//! described.
//!
//! ```sh
//! cargo run --release -p mjx-xlsx --example large_sparse_sheet -- out.xlsx
//! ```
//!
//! The runnable version of [the large-workbooks page](mjx_xlsx::guide::large_workbooks). Two
//! questions, both answered with a counting global allocator rather than with an inspection of the
//! types involved:
//!
//! 1. **Does a sheet with one cell at `XFD1048576` allocate for one cell, or for the grid it could
//!    address?** A dense index over the 1,048,576 addressable rows costs four megabytes at four
//!    bytes a slot; a dense grid over all 17,179,869,184 addressable cells cannot be allocated at
//!    all. The bound below is three orders of magnitude under the former.
//! 2. **What does a populated cell cost to hold?** `docs/BENCHMARKS.md` records **913 bytes of peak
//!    resident set per cell** for the same worksheet held as a `RawElement` tree. The cell store's
//!    bound is 48 B/cell, and `crates/mjx-sml/tests/cell_store_allocation.rs` is the gate that
//!    holds it; this example asserts the same bound end to end through a real `.xlsx` that was
//!    saved and reopened.
//!
//! **Why `size_of_val` would not do.** A `Vec` that reserved a million slots reports the same
//! twenty-four bytes as an empty one, so an assertion on it passes against precisely the defect this
//! is looking for. The allocator sees what the program actually asked for, which is the question.
//!
//! This is a debug build by default and the dense case is deliberately small enough to stay quick in
//! one; `--release` and a larger `DENSE_ROWS` is the way to reproduce the benchmark's shape.
//!
//! It also demonstrates the **bulk authoring path**, because building a sheet cell by cell through
//! [`Workbook::set_cell_value`] is quadratic — see `a_dense_block_costs_what_its_cells_cost` below.

use anyhow::{Context, Result};
use mjx_sml::{CellReference, CellValue};
use mjx_xlsx::Workbook;

mod support;

/// The counting allocator, from the crate both `xtask`'s fuzz campaign and `mjx-sml`'s cell-store
/// gate install. One `unsafe impl GlobalAlloc` in the workspace outside the two binding crates, and
/// every method forwards its arguments unchanged to `std::alloc::System`.
#[global_allocator]
static ALLOCATOR: mjx_allocation_counter::Counting = mjx_allocation_counter::Counting;

/// The far corner of the SpreadsheetML grid: column `XFD` (16,384) and row 1,048,576.
const FAR_CORNER: &str = "XFD1048576";

/// What reading a whole worksheet part whose one cell is at [`FAR_CORNER`] may allocate at peak.
///
/// Generous on purpose, and matched to `crates/mjx-sml/tests/cell_store_allocation.rs`'s
/// `SPARSE_PART_BOUND`: the part pays for an interner and for the frame around the cell. It is still
/// a hundred and twenty-eight times under the four megabytes a dense row index would cost.
const SPARSE_PART_BOUND: usize = 32 * 1024;

/// The store's per-cell bound for a dense sheet, the same figure `mjx-sml`'s gate holds.
const BYTES_PER_CELL_BOUND: usize = 48;

/// Rows and columns of the dense case — 30,000 cells, which a debug binary builds in a second or two
/// through the bulk path below. `docs/BENCHMARKS.md`'s corpus is 5,000 × 60 (300,000 cells) and is
/// measured with `cargo bench` rather than here, because CI runs every example unoptimised.
const DENSE_ROWS: u32 = 1_000;
const DENSE_COLUMNS: u16 = 30;

fn main() -> Result<()> {
    let out = support::output_path("large_sparse_sheet.xlsx");

    let sparse = one_cell_in_the_far_corner(&out)?;
    let dense = a_dense_block_costs_what_its_cells_cost()?;
    println!("\nboth cases passed");
    // Keep both figures alive to the end of `main`, so nothing above can be optimised away on the
    // strength of a value being dropped early.
    anyhow::ensure!(sparse > 0 && dense > 0);
    Ok(())
}

/// A workbook whose one populated cell is at `XFD1048576`, saved, reopened, and read back under the
/// allocator.
fn one_cell_in_the_far_corner(out: &std::path::Path) -> Result<usize> {
    let far_corner = CellReference::parse(FAR_CORNER).context("the last cell of the grid")?;

    let mut workbook = Workbook::blank().context("building a blank workbook")?;
    workbook.rename_sheet(0, "Sparse")?;
    workbook
        .set_cell_value(0, far_corner, CellValue::Number(1.0))
        .context("setting the far corner")?;

    let saved = workbook.save().context("saving")?;
    std::fs::write(out, &saved).with_context(|| format!("writing {}", out.display()))?;
    println!("wrote {} ({} bytes)", out.display(), saved.len());

    // The package is opened *before* the measurement, so what is measured is reading the worksheet
    // rather than inflating the ZIP.
    let reopened = Workbook::open(&saved).context("reopening")?;

    let before = mjx_allocation_counter::reset_peak();
    let markup = reopened
        .worksheet_markup(0)?
        .context("the tab reaches a worksheet part")?;
    let peak = mjx_allocation_counter::peak().saturating_sub(before);

    println!("\ncase 1 — one cell at {FAR_CORNER}, through a real .xlsx");
    println!("  peak allocated while reading   {peak:>12} bytes");
    println!("  bound                          {SPARSE_PART_BOUND:>12} bytes");
    println!(
        "  rows held                      {:>12}",
        markup.row_count()
    );
    println!(
        "  cells held                     {:>12}",
        markup.cell_count()
    );
    println!(
        "  a dense row index would cost   {:>12} bytes (1,048,576 rows x 4)",
        1_048_576 * 4
    );

    anyhow::ensure!(markup.row_count() == 1, "one populated row, not 1,048,576");
    anyhow::ensure!(markup.cell_count() == 1, "one populated cell, not the grid");
    anyhow::ensure!(
        peak <= SPARSE_PART_BOUND,
        "reading a worksheet whose one cell is at {FAR_CORNER} allocated {peak} bytes at peak, over \
         the {SPARSE_PART_BOUND}-byte bound. A store that costs memory proportional to the \
         addressable range is the defect this measurement exists to catch."
    );
    // The value is readable, so the bound was not met by failing to store anything.
    let cell = markup.cell(far_corner).context("the cell is there")?;
    anyhow::ensure!(cell.number() == Some(1.0), "the far corner lost its value");
    anyhow::ensure!(
        reopened.cell_text(0, far_corner)?.as_deref() == Some("1"),
        "the far corner does not read back through the Workbook surface"
    );
    Ok(peak)
}

/// A dense block, measured for what the store **retains** rather than for what reading it peaked at.
///
/// Note how the cells are written: **one `worksheet_markup` read, N edits on the model, one
/// `write_worksheet_markup`.** [`Workbook::set_cell_value`] is a whole-part read-modify-write — it
/// parses the worksheet, sets one cell and serializes it again — so filling a sheet through it is
/// quadratic in the number of cells. That is the right shape for the one-cell edit it is named for
/// and the wrong one for bulk authoring; see
/// [the large-workbooks page](mjx_xlsx::guide::large_workbooks) for the measured difference.
fn a_dense_block_costs_what_its_cells_cost() -> Result<usize> {
    let cells = usize::try_from(DENSE_ROWS)? * usize::from(DENSE_COLUMNS);

    let mut workbook = Workbook::blank().context("building a blank workbook")?;
    workbook.rename_sheet(0, "Dense")?;
    {
        let mut markup = workbook
            .worksheet_markup(0)?
            .context("the tab reaches a worksheet part")?;
        let sheet = markup.sheet_data_or_insert();
        for row in 0..DENSE_ROWS {
            for column in 0..DENSE_COLUMNS {
                sheet.set_cell_value(
                    CellReference::relative(column, row)?,
                    CellValue::Number(f64::from(row) * 100.0 + f64::from(column)),
                )?;
            }
        }
        workbook.write_worksheet_markup(0, &markup)?;
    }
    let saved = workbook.save().context("saving")?;
    let reopened = Workbook::open(&saved).context("reopening")?;

    // The raw XML of the part, which the reader copies into an `Arc<[u8]>` the store then points
    // into. That copy is charged below and named, rather than folded into the per-cell figure: the
    // store's records are what the 48 B/cell bound is about, and a sheet's own markup is not a
    // per-cell cost the store's design controls.
    let part_name = reopened.sheets()[0]
        .part
        .clone()
        .context("the first tab reaches a part")?;
    let raw_bytes = reopened
        .package()
        .part_bytes(&part_name)
        .context("the package holds the worksheet part")?
        .len();

    let before_live = mjx_allocation_counter::live();
    let before_peak = mjx_allocation_counter::reset_peak();
    let markup = reopened
        .worksheet_markup(0)?
        .context("the tab reaches a worksheet part")?;
    let live = mjx_allocation_counter::live().saturating_sub(before_live);
    let peak = mjx_allocation_counter::peak().saturating_sub(before_peak);
    let store_only = live.saturating_sub(raw_bytes);

    println!("\ncase 2 — {cells} cells ({DENSE_ROWS} rows x {DENSE_COLUMNS} columns)");
    println!("  saved package                  {:>12} bytes", saved.len());
    println!("  worksheet part, raw XML        {raw_bytes:>12} bytes");
    println!(
        "  part + store, live             {live:>12} bytes  ({:.1} B/cell)",
        live as f64 / cells as f64
    );
    println!(
        "  store records alone            {store_only:>12} bytes  ({:.1} B/cell)",
        store_only as f64 / cells as f64
    );
    println!("  peak across parse + read       {peak:>12} bytes");
    println!("  bound                          {BYTES_PER_CELL_BOUND:>12} B/cell");
    println!(
        "  docs/BENCHMARKS.md records              913 B/cell of peak RSS for a RawElement tree"
    );

    anyhow::ensure!(markup.cell_count() == cells, "the sheet lost cells");
    anyhow::ensure!(
        usize::try_from(DENSE_ROWS)? == markup.row_count(),
        "the sheet lost rows"
    );
    let per_cell = store_only / cells;
    anyhow::ensure!(
        per_cell <= BYTES_PER_CELL_BOUND,
        "the store retains {store_only} bytes for {cells} cells — {per_cell} B/cell, over the \
         {BYTES_PER_CELL_BOUND} B/cell bound"
    );
    // The copy is real and is worth seeing: `Workbook::worksheet_markup` reads through
    // `WorksheetPart::read_part`, which takes `&[u8]` and does `Arc::from`. A package that could
    // hand out its bytes already shared would not pay it — `read_shared` exists for exactly that
    // caller — but `mjx_opc::Package::part_bytes` answers `&[u8]`, so today it does.
    anyhow::ensure!(
        live >= raw_bytes,
        "reading the part cost less than the part's own bytes, so this measurement is not \
         measuring what it claims to"
    );
    // And the values are all still there, so the figure was not met by dropping the contents.
    let probe = CellReference::relative(DENSE_COLUMNS - 1, DENSE_ROWS - 1)?;
    anyhow::ensure!(
        markup.cell(probe).and_then(|cell| cell.number())
            == Some(f64::from(DENSE_ROWS - 1) * 100.0 + f64::from(DENSE_COLUMNS - 1)),
        "the last cell of the block lost its value"
    );
    Ok(live)
}
