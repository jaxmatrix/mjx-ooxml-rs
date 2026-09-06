//! **The range-crossing gate (MJXOFF-137).** The facade's cell door parses a worksheet **once** per
//! call, whatever the call was asked for — held to a measured allocation ratio rather than to a
//! stopwatch.
//!
//! # What is being defended, and why a test rather than a paragraph
//!
//! `mjx_xlsx::Workbook` holds no parsed worksheet: reading one never dirties the package, so there
//! is nothing to invalidate and nothing is cached. Every per-sheet accessor therefore parses that
//! sheet's part again — measured on the 300,000-cell corpus in a release build at **387 ms to read
//! one cell** and **485 ms to write one**, against 14.8 ms to open the whole file, with a
//! 4,000-cell write loop at **18.39 s** against **1.12 ms** for the batched shape (16,427×). All of
//! that is in [*Large workbooks*](mjx_xlsx::guide::large_workbooks).
//!
//! [`Workbook::read_range`] and [`Workbook::write_cells`] exist to make the batched shape the only
//! shape a caller can reach through this facade — including from Python and JavaScript, where the
//! Rust escape hatch does not exist. A later change that quietly made either of them loop
//! per-cell would still be *correct*: the same file, byte for byte. That is exactly why it needs a
//! gate that is not about correctness.
//!
//! # Why an allocation counter and not a timer
//!
//! A timing assertion has to be loose enough to survive a loaded CI runner and a debug build, and a
//! threshold that loose stops discriminating. [`mjx_allocation_counter::total_allocated`] counts
//! bytes handed out since the process started and is never decremented by a free — so a routine that
//! parses one worksheet and a routine that parses the same worksheet N times, freeing each one
//! before the next, have the **same live bytes, the same peak**, and differ N-fold here. That is the
//! quantity this file asserts, and it reads the same on any machine, in any profile.
//!
//! # Why `harness = false`
//!
//! A `#[global_allocator]` is process-wide and `cargo test` runs a harness's cases on several
//! threads in one process, so a figure measured under a harness is the figure of whatever else
//! happened to be running. This file has a plain `main`: the cases below run in sequence, on one
//! thread, with nothing else in the process — the same arrangement
//! `crates/mjx-sml/tests/cell_store_allocation.rs` makes, and for the same reason.
//!
//! # Proving the gate can fail
//!
//! Rewrite [`Workbook::write_cells`] to call `self.workbook.set_cell_value(…)` once per entry
//! instead of reading the part once, or [`Workbook::read_range`] to call
//! `self.worksheet_or_refuse(sheet)` inside its cell loop, and the ratios below collapse from three
//! digits to one. Both mutations produce **the same bytes**, which is the point.

use mjx_ooxml::{CellInput, CellWrite, Workbook};

#[global_allocator]
static ALLOCATOR: mjx_allocation_counter::Counting = mjx_allocation_counter::Counting;

/// How many cells the batched and per-cell shapes are compared over.
///
/// Big enough that the per-cell shape's repeated parses dominate its own fixed costs, small enough
/// that the per-cell shape still finishes quickly in a debug build — this file runs the slow shape
/// on purpose, so it must not be slow in wall-clock terms.
const CELLS: u32 = 200;

/// How many cells the sheet already holds before either shape runs.
///
/// The seed is what makes a parse expensive. With an empty sheet, parsing costs almost nothing and
/// the two shapes would be indistinguishable — a gate that passes because there is nothing to
/// measure. Two thousand cells is enough for one parse to dwarf one cell's own allocation, which is
/// the relationship the real 300,000-cell case has in the extreme.
const SEED_CELLS: u32 = 2_000;

/// The least improvement the batched write must show over the per-cell write.
///
/// **Not a regression bound on a measured figure** — it is the line between "one parse" and "one
/// parse per cell". With [`CELLS`] = 200 the honest ratio is two orders of magnitude; a fallback to
/// per-cell lands at roughly 1×. Twenty is far below the first and far above the second, so the
/// bound cannot be met by accident and cannot fail on noise: there is no noise in a deterministic
/// byte count.
const MINIMUM_WRITE_RATIO: usize = 20;

/// The same, for the batched read. The read shape has no serialize half, so its per-cell
/// alternative is cheaper than the write's and the honest ratio is smaller — still far above this.
const MINIMUM_READ_RATIO: usize = 20;

fn main() {
    let seeded = seeded_workbook();
    let filled = filled_workbook(&seeded);
    let batched = batched_read_allocates(&filled);
    let per_cell = per_cell_read_allocates(&filled);
    report("read", batched, per_cell, MINIMUM_READ_RATIO);

    let batched = batched_write_allocates(&seeded);
    let per_cell = per_cell_write_allocates(&seeded);
    report("write", batched, per_cell, MINIMUM_WRITE_RATIO);

    the_two_write_shapes_produce_the_same_bytes(&seeded);
    println!("workbook_boundary: all cases passed");
}

/// The container bytes of a workbook whose first sheet already holds [`SEED_CELLS`] cells.
///
/// Built with the batched call, because building it with the per-cell one would take a hundred times
/// longer for a fixture neither case measures.
fn seeded_workbook() -> Vec<u8> {
    let mut workbook = Workbook::blank().expect("a blank workbook");
    let seed: Vec<CellWrite> = (0..SEED_CELLS)
        .map(|n| CellWrite::new(address(n), CellInput::Number(f64::from(n))))
        .collect();
    workbook.write_cells(0, &seed).expect("the seed");
    workbook.save().expect("saving the seed")
}

/// The seeded workbook with [`CELLS`] more cells written into a second region, so the read cases
/// have a block that is worth reading.
fn filled_workbook(seeded: &[u8]) -> Vec<u8> {
    let mut workbook = Workbook::open(seeded).expect("the seeded workbook");
    workbook
        .write_cells(0, &writes())
        .expect("the measured region");
    workbook.save().expect("saving")
}

/// `"A1"`, `"A2"`, … — one column, so a block read of the region is a rectangle.
fn address(row: u32) -> String {
    format!("A{}", row + 1)
}

/// The same [`CELLS`] writes both write shapes apply, in the same order.
fn writes() -> Vec<CellWrite> {
    (0..CELLS)
        .map(|n| CellWrite::new(format!("C{}", n + 1), CellInput::Number(f64::from(n) + 0.5)))
        .collect()
}

/// Bytes the allocator handed out while one [`Workbook::write_cells`] applied every write.
fn batched_write_allocates(seeded: &[u8]) -> usize {
    let mut workbook = Workbook::open(seeded).expect("the seeded workbook");
    let writes = writes();
    let before = mjx_allocation_counter::total_allocated();
    workbook.write_cells(0, &writes).expect("the batched write");
    let after = mjx_allocation_counter::total_allocated();
    // Returned rather than dropped inside the measured region, so the workbook's own teardown is
    // outside it — `docs/BENCHMARKS.md` records the criterion version of this same trap.
    let _ = workbook;
    after - before
}

/// Bytes the allocator handed out while the **same** writes were applied one call at a time, through
/// the per-cell surface one layer down.
///
/// This is the shape the facade deliberately does not offer, reached through
/// [`Workbook::workbook_mut`] precisely so it can be measured against the one it does.
fn per_cell_write_allocates(seeded: &[u8]) -> usize {
    let mut workbook = Workbook::open(seeded).expect("the seeded workbook");
    let writes = writes();
    let references: Vec<mjx_ooxml::CellReference> = writes
        .iter()
        .map(|write| mjx_ooxml::CellReference::parse(&write.reference).expect("an address"))
        .collect();
    let before = mjx_allocation_counter::total_allocated();
    for (write, reference) in writes.iter().zip(&references) {
        let CellInput::Number(number) = write.value else {
            panic!("the measured writes are numbers");
        };
        workbook
            .workbook_mut()
            .set_cell_value(0, *reference, mjx_sml::CellValue::Number(number))
            .expect("one cell");
    }
    let after = mjx_allocation_counter::total_allocated();
    let _ = workbook;
    after - before
}

/// Bytes the allocator handed out while one [`Workbook::read_range`] read the whole region.
fn batched_read_allocates(filled: &[u8]) -> usize {
    let workbook = Workbook::open(filled).expect("the filled workbook");
    let range = format!("C1:C{CELLS}");
    let before = mjx_allocation_counter::total_allocated();
    let block = workbook.read_range(0, &range).expect("the block");
    let after = mjx_allocation_counter::total_allocated();
    assert_eq!(block.row_count(), CELLS);
    let _ = (workbook, block);
    after - before
}

/// Bytes the allocator handed out while the same region was read one cell at a time — the shape a
/// facade with a `cell_value(sheet, "C7")` method would have made the obvious one.
fn per_cell_read_allocates(filled: &[u8]) -> usize {
    let workbook = Workbook::open(filled).expect("the filled workbook");
    let addresses: Vec<String> = (0..CELLS).map(|n| format!("C{}", n + 1)).collect();
    let before = mjx_allocation_counter::total_allocated();
    for address in &addresses {
        let block = workbook.read_range(0, address).expect("one cell");
        assert_eq!(block.row_count(), 1);
    }
    let after = mjx_allocation_counter::total_allocated();
    let _ = workbook;
    after - before
}

/// The two write shapes must be indistinguishable in the file they produce.
///
/// **This is what makes the ratio above the only thing standing between the facade and the slow
/// shape.** A byte-identity suite cannot tell them apart, because they are not different: the whole
/// difference is how many times the part was parsed on the way.
fn the_two_write_shapes_produce_the_same_bytes(seeded: &[u8]) {
    let mut batched = Workbook::open(seeded).expect("the seeded workbook");
    batched
        .write_cells(0, &writes())
        .expect("the batched write");

    let mut per_cell = Workbook::open(seeded).expect("the seeded workbook");
    for write in writes() {
        let reference = mjx_ooxml::CellReference::parse(&write.reference).expect("an address");
        let CellInput::Number(number) = write.value else {
            panic!("the measured writes are numbers");
        };
        per_cell
            .workbook_mut()
            .set_cell_value(0, reference, mjx_sml::CellValue::Number(number))
            .expect("one cell");
    }

    let batched = batched
        .read_sheet(0)
        .expect("the batched sheet")
        .into_rows();
    let per_cell = per_cell
        .read_sheet(0)
        .expect("the per-cell sheet")
        .into_rows();
    assert_eq!(
        batched, per_cell,
        "the batched and per-cell shapes must produce the same sheet — the difference between them \
         is cost, not content, which is why the ratio above is the only gate on it"
    );
    println!("  the two write shapes agree cell for cell");
}

/// Prints both figures and enforces the ratio, naming what a failure means.
fn report(what: &str, batched: usize, per_cell: usize, minimum: usize) {
    // Reported to one decimal, not as an integer: a fallback to per-cell lands at *almost exactly*
    // 1.0, and integer division would print that as `0x` — a number that reads like a measurement
    // failure rather than like the finding it is.
    #[expect(
        clippy::cast_precision_loss,
        reason = "a byte count of this size is exact in f64, and this figure is printed rather than \
                  compared — the assertion below uses the integer ratio"
    )]
    let ratio = per_cell as f64 / batched.max(1) as f64;
    let integer_ratio = per_cell / batched.max(1);
    println!(
        "  {what}: batched {batched} B, per-cell {per_cell} B over {CELLS} cells on a \
         {SEED_CELLS}-cell sheet — {ratio:.1}x"
    );
    assert!(
        integer_ratio >= minimum,
        "the batched {what} allocated {batched} B against the per-cell shape's {per_cell} B, a \
         ratio of {ratio:.1}x where at least {minimum}x is required. A ratio near 1 means the \
         batched \
         path is parsing the worksheet once per cell — which produces the same file and is four \
         orders of magnitude slower at real workbook sizes (see \
         `crates/mjx-ooxml/src/workbook/cells.rs`)."
    );
}
