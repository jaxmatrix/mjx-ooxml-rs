//! **The memory gate (MJXOFF-95).** One binary, one `main`, one thread, one counting global
//! allocator — and a hard byte bound on a sheet whose only populated cell is `XFD1048576`.
//!
//! # Why this is a target of its own with no test harness
//!
//! A `#[global_allocator]` is installed for a whole process, and `cargo test` runs a harness's cases
//! on several threads inside one process. A peak measured under those conditions is the peak of
//! whatever else happened to be running, so it would pass or fail for reasons that have nothing to do
//! with the cell store. `harness = false` in `Cargo.toml` gives this file a plain `main`: the cases
//! below run in sequence, on one thread, with nothing else in the process.
//!
//! # Why an allocation counter and not the two easier instruments
//!
//! * **`size_of_val` and its relatives are not merely weaker — they are blind to the exact defect
//!   this gate exists to catch.** A `Vec` that reserved 1,048,576 slots reports the same twenty-four
//!   bytes as an empty one, so a `size_of_val` assertion passes against a store that allocates for
//!   the whole grid. That is the failure mode; an instrument that cannot see it is not an
//!   instrument. (`SheetData::reserved_bytes` exists, says so in its own documentation, and is used
//!   below only to *explain* a figure this file has already measured.)
//! * **Peak resident set is the wrong instrument here too**, for the mirror-image reason.
//!   `xtask`'s corpus harness reads `VmHWM` from `/proc/self/status`, which is right for "what does
//!   opening this real file cost" — but the kernel never backs pages nobody wrote, so a
//!   `Vec::with_capacity(1_048_576)` that is never touched can be nearly free in resident set and is
//!   fully visible to an allocation counter. It is also whole-process and shared with whatever else
//!   the process did.
//!
//! The allocator itself is `mjx-allocation-counter`, which is `xtask`'s fuzz campaign's — one
//! `unsafe impl GlobalAlloc` in the workspace, two consumers, no second implementation.
//!
//! # Proving the gate can fail
//!
//! Case one is written against a specific wrong design: an index over grid *positions* rather than
//! over the rows a file actually has. Replacing the row lookup in `crates/mjx-sml/src/cells/store.rs`
//! with a dense `Vec` indexed by row number turns the measured figure from hundreds of bytes into
//! megabytes, and the assertion below names both.
//!
//! # Cases three and four: the same gate, through the real part (MJXOFF-102)
//!
//! Cases one and two measure a bare `sheetData`. A worksheet in a workbook is not one: it is a
//! thirty-nine slot `CT_Worksheet` with `sheetPr`, `dimension`, `sheetViews`, `cols`, `pageSetup`
//! and the rest around it, and MJXOFF-102's [`WorksheetPart`] is what holds all of that. So the same
//! two claims are re-asserted **driven through the part**, because a frame that materialised its
//! `sheetData` as a `RawElement` tree in order to serialize it — or that simply kept the parsed
//! document alive beside the store — would pass every structural assertion in
//! `crates/mjx-sml/tests/worksheet_spine.rs` and show up only here.

use std::sync::Arc;

use mjx_sml::{CellReference, CellValue, SheetData, WorksheetPart};

#[global_allocator]
static ALLOCATOR: mjx_allocation_counter::Counting = mjx_allocation_counter::Counting;

/// The bound for a one-cell sheet, in bytes.
///
/// Generous on purpose — it is not a regression bound on the exact figure, it is the line between
/// "this store is sparse" and "this store is not". A dense index over the 1,048,576 addressable rows
/// costs four megabytes at four bytes a slot, a thousand times this; a dense grid over all
/// 17,179,869,184 addressable cells cannot be allocated at all.
const SPARSE_BOUND: usize = 8 * 1024;

/// The bound for a whole worksheet **part** whose one cell is at `XFD1048576`, in bytes.
///
/// Larger than [`SPARSE_BOUND`] and for a stated reason: this case reads a real part, so it pays for
/// an interner, for the thirty-nine slot frame's own `Vec`, and for the twelve children the markup
/// below carries — none of which the bare-store case has. It is still three orders of magnitude
/// below the four megabytes a dense index over the 1,048,576 addressable rows would cost, which is
/// the line this bound draws.
const SPARSE_PART_BOUND: usize = 32 * 1024;

/// The bound on what the store itself retains, per populated cell, for a realistic dense sheet.
///
/// Measured against `docs/BENCHMARKS.md`'s **913 bytes of peak resident set per cell** for the same
/// worksheet held as a `RawElement` tree. This counts the store's own records and byte arena and not
/// the part's buffer, which the store *shares* with the package rather than copying — see the note
/// printed beside the figure.
const BYTES_PER_CELL_BOUND: usize = 48;

/// The same bound for a sheet whose every cell carries a **formula** (MJXOFF-115's memory clause).
///
/// Higher than [`BYTES_PER_CELL_BOUND`] and for a stated reason rather than a fudged one: a cell with
/// an `<f>` has a side-table record, because that is where MJXOFF-95 keeps the formula's byte range,
/// so it pays 36 bytes for its `PackedCell` plus 40 for its `CellExtras`. MJXOFF-115 added **nothing**
/// to that — a formula is read out of the bytes the store already held. The line this bound draws is
/// against the design that would: a decoded formula struct or a `String` per cell, or a formula table
/// with a four-byte index on every cell in the sheet.
const FORMULA_BYTES_PER_CELL_BOUND: usize = 96;

/// The corpus shape from `xtask/src/corpus/xlsx.rs` (MJXOFF-147): 5,000 rows × 60 columns.
///
/// The same shape, so the figure below is comparable with the 913 B/cell that harness recorded — but
/// built here rather than read from `target/corpus/`, because a gate that skips when a generated file
/// is missing is a gate that passes when it is missing. The *real* corpus file is measured by
/// `cargo run --release -p xtask -- corpus --mem xlsx`, which extends MJXOFF-147's own harness rather
/// than duplicating it, and both figures are quoted in the pull request.
const ROW_COUNT: usize = 5_000;
/// Populated columns per row, as above.
const COLUMN_COUNT: usize = 60;

fn main() {
    println!("MJXOFF-95 — the cell store's memory gate, and MJXOFF-115's clause on it\n");
    let sparse = sparse_sheet_costs_what_its_one_cell_costs();
    let dense = a_realistic_sheet_costs_far_less_than_a_tree_of_it();
    let sparse_part = a_sparse_worksheet_part_costs_what_its_one_cell_costs();
    let dense_part = a_realistic_worksheet_part_keeps_the_per_cell_bound();
    let formulas = a_sheet_of_formula_cells_pays_for_no_formula_table();
    println!("\nall five cases passed");
    // Keep the figures alive to the end of `main`, so nothing above can be optimised away on the
    // strength of the store being dropped early.
    assert!(sparse > 0 && dense > 0 && sparse_part > 0 && dense_part > 0 && formulas > 0);
}

/// **MJXOFF-102's re-assertion, driven through the real part.**
///
/// The two cases above build or read a bare `sheetData`. This one reads a whole
/// `xl/worksheets/sheetN.xml` — a thirty-nine slot `CT_Worksheet` with a `sheetPr`, a `dimension`,
/// a `sheetViews`, two `cols` blocks, a `pageSetup`, a `headerFooter` and a `tableParts` around it —
/// whose only populated cell is at `XFD1048576`. Reading that part must still cost what its one cell
/// costs.
///
/// It is measured rather than inspected for the same reason case one is: a frame that materialised
/// its `sheetData` as a `RawElement` tree to serialize it, or that kept the parsed document alive
/// beside the store, would pass every structural assertion and show up only here.
fn a_sparse_worksheet_part_costs_what_its_one_cell_costs() -> usize {
    let far_corner = CellReference::parse("XFD1048576").expect("the last cell of the grid");
    let markup: Arc<[u8]> = Arc::from(sparse_worksheet_markup().into_bytes());

    // The buffer is allocated *before* the measurement and handed over shared, exactly as case two
    // does: the package already holds these bytes for its own copy-on-write, and the part points
    // into them rather than duplicating them.
    let before = mjx_allocation_counter::reset_peak();
    let part = WorksheetPart::read_shared(Arc::clone(&markup))
        .expect("the part reads")
        .expect("the root is an x:worksheet");
    let peak = mjx_allocation_counter::peak() - before;
    let live = mjx_allocation_counter::live() - before;

    println!("\ncase 3 — a whole worksheet part whose one cell is at XFD1048576");
    println!(
        "  part markup                    {:>12} bytes",
        markup.len()
    );
    println!("  peak allocated while reading   {peak:>12} bytes");
    println!("  live after reading             {live:>12} bytes");
    println!("  bound                          {SPARSE_PART_BOUND:>12} bytes");
    println!("  rows held                      {:>12}", part.row_count());
    println!("  cells held                     {:>12}", part.cell_count());
    println!(
        "  worksheet children held        {:>12}",
        part.child_element_locals().count()
    );
    println!(
        "  a dense row index would cost   {:>12} bytes (1,048,576 rows x 4)",
        1_048_576 * 4
    );

    assert_eq!(part.row_count(), 1, "one populated row, not 1,048,576");
    assert_eq!(part.cell_count(), 1);
    assert!(
        part.child_element_locals().count() >= 8,
        "the part must really carry the frame around the one cell, or this measures a bare \
         sheetData under another name"
    );
    assert!(
        peak <= SPARSE_PART_BOUND,
        "reading a worksheet part with one cell at XFD1048576 allocated {peak} bytes at peak, over \
         the {SPARSE_PART_BOUND}-byte bound. A frame that costs memory proportional to the \
         addressable range — or that keeps the parsed tree alive beside the store — is the defect \
         this gate exists to catch."
    );

    // The value is readable, so the bound was not met by failing to store anything.
    let cell = part.cell(far_corner).expect("the cell is there");
    assert_eq!(cell.number(), Some(1.0));
    assert_eq!(cell.reference().text().as_str(), "XFD1048576");
    // …and the part re-emits verbatim, so it was not met by dropping the frame either.
    assert_eq!(part.to_markup(), markup.as_ref());
    peak
}

/// The same 5,000 x 60 sheet as case two, read as a **whole part** rather than as a bare
/// `sheetData`, so MJXOFF-95's per-cell bound is asserted end to end through MJXOFF-102's frame.
fn a_realistic_worksheet_part_keeps_the_per_cell_bound() -> usize {
    let markup: Arc<[u8]> = Arc::from(worksheet_markup().into_bytes());
    let cells = ROW_COUNT * COLUMN_COUNT;

    let before = mjx_allocation_counter::reset_peak();
    let part = WorksheetPart::read_shared(Arc::clone(&markup))
        .expect("the part reads")
        .expect("the root is an x:worksheet");
    let live = mjx_allocation_counter::live() - before;
    let peak = mjx_allocation_counter::peak() - before;

    println!("\ncase 4 — the same {cells} cells, read as a whole worksheet part");
    println!(
        "  part markup                    {:>12} bytes",
        markup.len()
    );
    println!(
        "  part + store, live             {live:>12} bytes  ({:.1} B/cell)",
        live as f64 / cells as f64
    );
    println!("  peak across parse + read       {peak:>12} bytes");
    println!("  bound                          {BYTES_PER_CELL_BOUND:>12} B/cell");
    println!("  MJXOFF-147 recorded                     913 B/cell of peak RSS for this shape");

    assert_eq!(part.cell_count(), cells);
    assert_eq!(part.row_count(), ROW_COUNT);
    let per_cell = live / cells;
    assert!(
        per_cell <= BYTES_PER_CELL_BOUND,
        "the part retains {live} bytes for {cells} cells — {per_cell} B/cell, over the \
         {BYTES_PER_CELL_BOUND} B/cell bound"
    );
    live
}

/// A whole `CT_Worksheet` around one cell at the far corner of the grid: several modelled slots and
/// several held ones, so the frame is really exercised.
fn sparse_worksheet_markup() -> String {
    concat!(
        r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" "#,
        r#"xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">"#,
        r#"<sheetPr codeName="Far"><tabColor rgb="FF00B050"/><pageSetUpPr fitToPage="true"/></sheetPr>"#,
        r#"<dimension ref="XFD1048576"/>"#,
        r#"<sheetViews><sheetView tabSelected="true" workbookViewId="0">"#,
        r#"<selection activeCell="XFD1048576" sqref="XFD1048576"/></sheetView></sheetViews>"#,
        r#"<sheetFormatPr defaultRowHeight="15"/>"#,
        r#"<cols><col min="16384" max="16384" width="30" customWidth="true"/></cols>"#,
        r#"<sheetData><row r="1048576"><c r="XFD1048576"><v>1</v></c></row></sheetData>"#,
        r#"<sheetCalcPr fullCalcOnLoad="true"/>"#,
        r#"<mergeCells count="1"><mergeCell ref="A5:B5"/></mergeCells>"#,
        r#"<pageMargins left="0.7" right="0.7" top="0.75" bottom="0.75" header="0.3" footer="0.3"/>"#,
        r#"<pageSetup paperSize="9" orientation="landscape"/>"#,
        r#"<headerFooter><oddHeader>&amp;CFar</oddHeader></headerFooter>"#,
        r#"<tableParts count="1"><tablePart r:id="rId1"/></tableParts>"#,
        r#"</worksheet>"#,
    )
    .to_owned()
}

/// **The clause.** A sheet with one cell at `XFD1048576` allocates for one cell, not for the grid.
fn sparse_sheet_costs_what_its_one_cell_costs() -> usize {
    let far_corner = CellReference::parse("XFD1048576").expect("the last cell of the grid");
    assert_eq!(
        far_corner.row(),
        1_048_575,
        "zero-based, so one below the count"
    );
    assert_eq!(far_corner.column(), 16_383);

    let before = mjx_allocation_counter::reset_peak();
    let mut sheet = SheetData::authored(None);
    sheet
        .set_cell_value(far_corner, CellValue::Number(1.0))
        .expect("the far corner is inside the grid");
    let peak = mjx_allocation_counter::peak() - before;
    let live = mjx_allocation_counter::live() - before;

    println!("case 1 — one cell at XFD1048576");
    println!("  peak allocated while building  {peak:>12} bytes");
    println!("  live after building            {live:>12} bytes");
    println!("  bound                          {SPARSE_BOUND:>12} bytes");
    println!("  rows held                      {:>12}", sheet.row_count());
    println!(
        "  cells held                     {:>12}",
        sheet.cell_count()
    );
    println!(
        "  a dense row index would cost   {:>12} bytes (1,048,576 rows x 4)",
        1_048_576 * 4
    );

    assert_eq!(sheet.row_count(), 1, "one populated row, not 1,048,576");
    assert_eq!(sheet.cell_count(), 1);
    assert!(
        peak <= SPARSE_BOUND,
        "building a sheet with one cell at XFD1048576 allocated {peak} bytes at peak, over the \
         {SPARSE_BOUND}-byte bound. A store that costs memory proportional to the addressable range \
         rather than to the populated cells is the defect this gate exists to catch."
    );

    // The value is readable, so the bound was not met by failing to store anything.
    let cell = sheet.cell(far_corner).expect("the cell is there");
    assert_eq!(cell.number(), Some(1.0));
    assert_eq!(cell.reference().text().as_str(), "XFD1048576");
    peak
}

/// The same store against a realistic sheet, next to the figure a `RawElement` tree cost.
fn a_realistic_sheet_costs_far_less_than_a_tree_of_it() -> usize {
    let markup: Arc<[u8]> = Arc::from(worksheet_markup().into_bytes());
    let cells = ROW_COUNT * COLUMN_COUNT;

    // The buffer is allocated *before* the measurement and handed over shared, so what is measured
    // below is the store's own cost and not a second copy of the part. That is the honest comparison:
    // the package already holds these bytes for its own copy-on-write, and the store points into
    // them rather than duplicating them.
    let before = mjx_allocation_counter::reset_peak();
    let document = mjx_xml::fidelity::parse_shared(Arc::clone(&markup)).expect("the sheet parses");
    let tree_peak = mjx_allocation_counter::peak() - before;
    let tree_live = mjx_allocation_counter::live() - before;

    let sheet = SheetData::read_worksheet(&document)
        .expect("the sheet reads")
        .expect("the worksheet has a sheetData");
    drop(document);
    let live = mjx_allocation_counter::live() - before;
    let peak = mjx_allocation_counter::peak() - before;

    println!("\ncase 2 — {cells} populated cells ({ROW_COUNT} rows x {COLUMN_COUNT} columns)");
    println!(
        "  raw XML                        {:>12} bytes",
        markup.len()
    );
    println!(
        "  RawElement tree, live          {tree_live:>12} bytes  ({:.0} B/cell)",
        tree_live as f64 / cells as f64
    );
    println!("  RawElement tree, peak          {tree_peak:>12} bytes");
    println!(
        "  cell store, live               {live:>12} bytes  ({:.1} B/cell)",
        live as f64 / cells as f64
    );
    println!("  peak across parse + read       {peak:>12} bytes");
    println!(
        "  the store's own accounting     {:>12} bytes  (SheetData::reserved_bytes, documentation only)",
        sheet.reserved_bytes()
    );
    println!("  bound                          {BYTES_PER_CELL_BOUND:>12} B/cell");
    println!("  MJXOFF-147 recorded                     913 B/cell of peak RSS for this shape");

    assert_eq!(sheet.cell_count(), cells);
    assert_eq!(sheet.row_count(), ROW_COUNT);
    assert_eq!(
        sheet.edited_bytes(),
        0,
        "a worksheet nobody has edited must own no bytes of its own"
    );
    let per_cell = live / cells;
    assert!(
        per_cell <= BYTES_PER_CELL_BOUND,
        "the store retains {live} bytes for {cells} cells — {per_cell} B/cell, over the \
         {BYTES_PER_CELL_BOUND} B/cell bound"
    );
    assert!(
        live < tree_live,
        "the store ({live} bytes) must cost less than the tree it was read from ({tree_live} bytes)"
    );

    // The sheet is readable, so the figure was not met by dropping the contents.
    let cell = sheet
        .cell(CellReference::parse("B2").expect("a reference"))
        .expect("row 2 column B is populated");
    // Read back through the generator's own formula, so the expectation is the input rather than a
    // number copied out of a passing run.
    assert_eq!(cell.number(), Some(numeric_value(2, 1) as f64));
    live
}

/// A worksheet in the shape `xtask/src/corpus/xlsx.rs` builds: column A an inline string, the rest
/// numeric, every row carrying the `spans` Excel writes.
fn worksheet_markup() -> String {
    let mut xml = String::with_capacity(ROW_COUNT * COLUMN_COUNT * 28 + 256);
    xml.push_str(
        "<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">\
         <sheetData>\r\n",
    );
    for row in 1..=ROW_COUNT {
        xml.push_str(&format!("<row r=\"{row}\" spans=\"1:{COLUMN_COUNT}\">"));
        for column in 0..COLUMN_COUNT {
            let letters = column_letters(column);
            if column == 0 {
                xml.push_str(&format!(
                    "<c r=\"{letters}{row}\" t=\"inlineStr\"><is><t>Row {row}</t></is></c>"
                ));
            } else {
                let value = numeric_value(row, column);
                xml.push_str(&format!("<c r=\"{letters}{row}\"><v>{value}</v></c>"));
            }
        }
        xml.push_str("</row>\r\n");
    }
    xml.push_str("</sheetData></worksheet>");
    xml
}

/// The value the generated sheet puts at `(row, column)` — a small deterministic spread rather than
/// a counter, the same shape `xtask/src/corpus/xlsx.rs` writes.
fn numeric_value(row: usize, column: usize) -> usize {
    (row * 7 + column * 13) % 100_000
}

fn column_letters(index: usize) -> String {
    mjx_sml::address::column_letters(u16::try_from(index).expect("inside the grid"))
        .expect("inside the grid")
        .as_str()
        .to_owned()
}

/// **MJXOFF-115's memory clause.** The same 5,000 x 60 shape as case two, but with a formula in
/// *every* cell — 5,000 shared-group hosts carrying the text and 295,000 members carrying none.
///
/// The question this answers is the one MJXOFF-115's constraints ask: *what does a shared-group
/// member cost?* The answer is that it costs what any formula cell costs and **nothing at all for
/// being in a group** — there is no group table, no per-cell index into one, and above all no copy of
/// the host's text per member. `SheetData::shared_formula_groups` builds a report on demand and
/// allocates one entry per *group*, so it is not part of this measurement, which is what the store
/// retains.
///
/// It also says what D11 itself added to the per-cell cost, which is **nothing**: a formula cell paid
/// for a `CellExtras` record under MJXOFF-95 already, because that is where the `<f>` byte range
/// lives, and this child gave those bytes a type rather than a second home. The figure below is
/// `PackedCell` + `CellExtras` and no third thing.
fn a_sheet_of_formula_cells_pays_for_no_formula_table() -> usize {
    let markup: Arc<[u8]> = Arc::from(formula_worksheet_markup().into_bytes());
    let cells = ROW_COUNT * COLUMN_COUNT;

    let before = mjx_allocation_counter::reset_peak();
    let part = WorksheetPart::read_shared(Arc::clone(&markup))
        .expect("the part reads")
        .expect("the root is an x:worksheet");
    let live = mjx_allocation_counter::live() - before;
    let peak = mjx_allocation_counter::peak() - before;

    println!("\ncase 5 — the same {cells} cells, every one of them a formula cell");
    println!(
        "  part markup                    {:>12} bytes",
        markup.len()
    );
    println!(
        "  part + store, live             {live:>12} bytes  ({:.1} B/cell)",
        live as f64 / cells as f64
    );
    println!("  peak across parse + read       {peak:>12} bytes");
    println!("  bound                          {FORMULA_BYTES_PER_CELL_BOUND:>12} B/cell");
    println!("  of which, per formula cell:              36 B PackedCell + 40 B CellExtras = 76 B");
    println!(
        "  MJXOFF-147 recorded                     913 B/cell of peak RSS for the value shape"
    );
    println!("  case 2 measured a value cell at          36.8 B/cell");

    assert_eq!(part.cell_count(), cells);
    assert_eq!(part.row_count(), ROW_COUNT);

    // Every cell really carries a formula, and the members really carry no text — or this measures
    // something other than what it claims to.
    let hosts = part.cells().filter(|cell| {
        cell.formula()
            .is_some_and(|formula| formula.is_shared_group_host() == Ok(true))
    });
    assert_eq!(hosts.count(), ROW_COUNT, "one host a row");
    let text_less = part.cells().filter(|cell| {
        cell.formula().is_some_and(|formula| {
            !formula.has_text() && formula.is_shared_group_member() == Ok(true)
        })
    });
    assert_eq!(
        text_less.count(),
        cells - ROW_COUNT,
        "every other cell is a member carrying no text at all"
    );

    let per_cell = live / cells;
    assert!(
        per_cell <= FORMULA_BYTES_PER_CELL_BOUND,
        "the part retains {live} bytes for {cells} formula cells — {per_cell} B/cell, over the \
         {FORMULA_BYTES_PER_CELL_BOUND} B/cell bound. A formula table with a per-cell index, or a \
         decoded formula struct per cell, is the defect this bound exists to catch."
    );

    // The formulas are readable, so the figure was not met by dropping them.
    let host = part
        .cell(CellReference::parse("A2").expect("a reference"))
        .expect("row 2 column A is a host")
        .formula()
        .expect("the host carries an f element");
    assert_eq!(host.raw_text(), b"SUM(B2:BH2)");
    live
}

/// The same sheet as [`worksheet_markup`], with every cell a formula cell: column A hosts a shared
/// group covering the row, and every other column is a text-less member of it.
fn formula_worksheet_markup() -> String {
    let mut xml = String::with_capacity(ROW_COUNT * COLUMN_COUNT * 40 + 256);
    xml.push_str(
        "<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">\
         <sheetData>\r\n",
    );
    let last = column_letters(COLUMN_COUNT - 1);
    for row in 1..=ROW_COUNT {
        let group = row - 1;
        xml.push_str(&format!("<row r=\"{row}\" spans=\"1:{COLUMN_COUNT}\">"));
        for column in 0..COLUMN_COUNT {
            let letters = column_letters(column);
            let value = numeric_value(row, column);
            if column == 0 {
                xml.push_str(&format!(
                    "<c r=\"{letters}{row}\"><f t=\"shared\" ref=\"A{row}:{last}{row}\" \
                     si=\"{group}\">SUM(B{row}:{last}{row})</f><v>{value}</v></c>"
                ));
            } else {
                xml.push_str(&format!(
                    "<c r=\"{letters}{row}\"><f t=\"shared\" si=\"{group}\"/><v>{value}</v></c>"
                ));
            }
        }
        xml.push_str("</row>\r\n");
    }
    xml.push_str("</sheetData></worksheet>");
    xml
}
