//! **The gate this whole child was written against** (MJXOFF-171): a sheet whose only populated cell
//! is `XFD1048576`, laid out under a counting global allocator, with a hard byte bound and a hard
//! bound on the number of cells the layout touched.
//!
//! # Why a small fixture proves nothing here
//!
//! *"A sheet lays out"* is green on a ten-cell fixture for an implementation that walks all
//! **17,179,869,184** addressable coordinates — it will simply be slow, and a small fixture hides
//! that completely. So there is no small fixture in this file. Every case is a sheet whose
//! *addressable* range is enormous and whose *populated* range is one cell or one row, and the
//! assertions are on cost rather than on correctness.
//!
//! # Why this is a target of its own with no test harness
//!
//! A `#[global_allocator]` is installed for a whole process, and `cargo test` runs a harness's cases
//! on several threads inside one process — so a peak measured under those conditions is the peak of
//! whatever else happened to be running. `harness = false` in `Cargo.toml` gives this file a plain
//! `main`: the cases below run in sequence, on one thread, with nothing else in the process.
//! `mjx-sml`'s own cell-store gate learned this first and `mjx-session`'s learned it again.
//!
//! # Three instruments, and why each is here
//!
//! * **Bytes**, from `mjx-allocation-counter`. It is the one instrument that can see a
//!   `Vec::with_capacity(1_048_576)` that is never written to — peak resident set cannot, because the
//!   kernel never backs pages nobody touched, and `size_of_val` cannot, because a `Vec` that reserved
//!   a million slots reports the same twenty-four bytes as an empty one.
//! * **Cells visited**, from [`PageCatalogue::cells`] and [`PageCatalogue::rows`]. A byte bound alone
//!   would pass for an implementation that walked every coordinate and allocated nothing while doing
//!   it — which is the *slow* half of the defect, and the half a memory instrument is blind to.
//! * **Wall time**, with a deliberately generous ceiling. Not a performance assertion: a frame-time
//!   assertion on a shared host is a flaky gate, and R15 refused to write one. This ceiling is three
//!   orders of magnitude above what a windowed layout costs and many orders below what seventeen
//!   billion probes cost, so it can only fail for the reason it names.
//!
//! # Proving the gate can fail
//!
//! [`the_naive_walk_breaches_every_bound`] does the thing this crate refuses to do — visits a range
//! of *coordinates* rather than the populated cells — over a range a thousandth the size of the
//! sheet, and asserts that it already costs more than the windowed layout of the whole sheet. The
//! gate that passes above and the walk that fails below are the same measurement.

use std::time::Instant;

use mjx_layout::{BoxModel, Constraints, LayoutSize, PageIndex};
use mjx_layout_xlsx::{SheetBoxModel, SheetGrid};
use mjx_ooxml_core::measure::Emu;
use mjx_sml::CellReference;
use mjx_text::FontResolver;
use mjx_xlsx::{PartName, Workbook};

#[global_allocator]
static ALLOCATOR: mjx_allocation_counter::Counting = mjx_allocation_counter::Counting;

/// The bound on laying out one band of a sheet whose only populated cell is `XFD1048576`, in bytes.
///
/// Generous on purpose — it is not a regression bound on the exact figure, it is the line between
/// *this layout is windowed* and *this layout is not*. A dense index over the 1,048,576 addressable
/// rows costs four megabytes at four bytes a slot, thirty times this; a fragment per addressable
/// cell cannot be allocated at all.
const WINDOWED_BOUND: usize = 128 * 1024;

/// The most cells one band of the far-corner sheet may lay out.
///
/// A viewport six inches by four holds roughly forty rows of twelve columns — call it five hundred
/// positions — and the far-corner sheet has **one** populated cell in none of them. The bound is set
/// an order of magnitude above the visible window and eight orders below the grid.
const VISITED_BOUND: usize = 4_000;

/// The wall-clock ceiling for twenty bands of the far-corner sheet.
///
/// See the module documentation: this is not a performance assertion. Twenty windowed bands take
/// single-digit milliseconds; twenty bands of a coordinate walk would not finish today.
const TIME_CEILING: std::time::Duration = std::time::Duration::from_secs(20);

/// How many bands are laid out for the timing case.
const BANDS: u32 = 20;

fn main() {
    println!("MJXOFF-171 — the grid's sparsity gate\n");
    let corner = one_populated_cell_in_the_far_corner_costs_what_one_cell_costs();
    let scrolled = a_band_far_down_the_sheet_costs_the_same_as_the_first();
    let wide = a_row_of_sixteen_thousand_columns_lays_out_only_what_is_on_screen();
    let naive = the_naive_walk_breaches_every_bound();
    println!("\nall four cases passed");
    assert!(corner > 0 && scrolled > 0 && wide > 0 && naive > 0);
}

/// A worksheet part whose only populated cell is `XFD1048576`, wrapped in the slots a real sheet
/// carries so that nothing here is measuring a stripped-down file.
fn far_corner_worksheet() -> Vec<u8> {
    br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
<dimension ref="XFD1048576"/>
<sheetViews><sheetView tabSelected="true" workbookViewId="0"/></sheetViews>
<sheetFormatPr defaultRowHeight="15" defaultColWidth="9.140625"/>
<cols><col min="1" max="16384" width="9.140625"/></cols>
<sheetData><row r="1048576"><c r="XFD1048576" t="inlineStr"><is><t>the far corner</t></is></c></row></sheetData>
<pageMargins left="0.7" right="0.7" top="0.75" bottom="0.75" header="0.3" footer="0.3"/>
</worksheet>"#
        .to_vec()
}

/// A worksheet with one row of 16,384 populated cells — the other shape of enormous.
fn full_width_worksheet() -> Vec<u8> {
    let mut cells = String::with_capacity(16_384 * 40);
    for column in 0..16_384_u16 {
        let reference = CellReference::relative(column, 0).expect("inside the grid");
        cells.push_str(&format!(
            "<c r=\"{}\" t=\"inlineStr\"><is><t>{column}</t></is></c>",
            reference.text()
        ));
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
<sheetFormatPr defaultRowHeight="15" defaultColWidth="9.140625"/>
<sheetData><row r="1">{cells}</row></sheetData>
</worksheet>"#
    )
    .into_bytes()
}

/// A workbook whose first sheet is `markup`.
fn workbook(markup: Vec<u8>) -> Workbook {
    let bytes = Workbook::blank()
        .expect("a blank workbook")
        .save_unchecked()
        .expect("blank saves");
    let mut package = mjx_xlsx::Package::open(&bytes).expect("the blank package opens");
    package
        .replace_part_bytes(
            &PartName::new("/xl/worksheets/sheet1.xml").expect("a valid part name"),
            markup,
        )
        .expect("the worksheet is replaceable");
    Workbook::from_package(package).expect("the authored package resolves")
}

/// A box model over the faces `mjx-text` commits, so this gate measures layout and not a font
/// database.
fn model() -> SheetBoxModel {
    let fonts = FontResolver::builder()
        .with_bundled_font_directory(
            &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../mjx-text/assets/fonts"),
        )
        .expect("the committed faces index")
        .build();
    SheetBoxModel::new(fonts)
}

/// A viewport six inches by four, which is a realistic window onto a sheet.
fn constraints() -> Constraints {
    mjx_layout_xlsx::constraints_for(LayoutSize {
        width: Emu::from_inches(6.0),
        height: Emu::from_inches(4.0),
    })
}

/// **Case one.** One band of the far-corner sheet, measured in bytes and in cells visited.
fn one_populated_cell_in_the_far_corner_costs_what_one_cell_costs() -> usize {
    let book = workbook(far_corner_worksheet());
    let grid = SheetGrid::read(&book, 0).expect("the sheet reads");
    let constraints = constraints();
    let mut model = model();
    // The first layout builds the geometry and resolves a face, which is a one-off; the measurement
    // is of the second, which is what a frame actually costs.
    let _ = model
        .layout_page(&grid, PageIndex::FIRST, &constraints, None)
        .expect("the first band lays out");

    let before = mjx_allocation_counter::reset_peak();
    let started = Instant::now();
    let page = model
        .layout_page(&grid, PageIndex::FIRST, &constraints, None)
        .expect("the first band lays out again");
    let peak = mjx_allocation_counter::peak() - before;
    let elapsed = started.elapsed();
    let visited = model.catalogue().cells().len();
    let rows = model.catalogue().rows().len();
    let columns = model.catalogue().columns().len();

    println!("case 1 — one band of a sheet whose only cell is XFD1048576");
    println!(
        "  addressable cells              {:>12}",
        17_179_869_184_u64
    );
    println!("  populated cells                {:>12}", 1);
    println!("  peak allocated laying out      {peak:>12} bytes  (bound {WINDOWED_BOUND})");
    println!("  cells with text laid out       {visited:>12}         (bound {VISITED_BOUND})");
    println!("  rows visited                   {rows:>12}");
    println!("  columns visited                {columns:>12}");
    println!(
        "  fragments                      {:>12}",
        page.fragments().len()
    );
    println!("  elapsed                        {elapsed:>12?}");

    assert!(
        peak < WINDOWED_BOUND,
        "laying out one band of a one-cell sheet allocated {peak} bytes, past the {WINDOWED_BOUND} \
         this gate allows. A layout that walked the coordinate range, or that built an index over \
         the 1,048,576 addressable rows, is what this figure looks like."
    );
    assert!(
        rows * columns < VISITED_BOUND,
        "the band visited {rows} rows by {columns} columns, past the {VISITED_BOUND} positions \
         this gate allows. A window six inches by four holds a few hundred."
    );
    assert!(
        visited == 0,
        "no populated cell is on screen, so no cell text was laid out; got {visited}"
    );
    peak.max(1)
}

/// **Case two.** The band *containing* the far corner costs the same as the first — which is the
/// property `RowGeometry`'s binary search exists to give, and the one a prefix sum over the
/// addressable range would fail.
fn a_band_far_down_the_sheet_costs_the_same_as_the_first() -> usize {
    let book = workbook(far_corner_worksheet());
    let grid = SheetGrid::read(&book, 0).expect("the sheet reads");
    let constraints = constraints();
    let mut model = model();
    // The far corner is at column XFD as well as at row 1,048,576, so reaching it means scrolling in
    // **both** axes — the vertical one through the page index, the horizontal one through the box
    // model's own window. `mjx-view` pages in one dimension and a grid moves in two; this is where
    // that shows.
    model.scroll_to_column(16_380);
    let extent = model.estimate_extent(&grid, &constraints);
    let last = PageIndex::new(extent.pages.saturating_sub(1));
    let _ = model
        .layout_page(&grid, last, &constraints, None)
        .expect("the last band lays out");

    let before = mjx_allocation_counter::reset_peak();
    let started = Instant::now();
    let page = model
        .layout_page(&grid, last, &constraints, None)
        .expect("the last band lays out again");
    let peak = mjx_allocation_counter::peak() - before;
    let elapsed = started.elapsed();
    let visited = model.catalogue().cells().len();

    println!(
        "\ncase 2 — the band containing XFD1048576, {} bands down and scrolled to column {}",
        last.number(),
        model.first_column()
    );
    println!("  bands in the sheet             {:>12}", extent.pages);
    println!("  peak allocated laying out      {peak:>12} bytes  (bound {WINDOWED_BOUND})");
    println!("  cells with text laid out       {visited:>12}");
    println!(
        "  fragments                      {:>12}",
        page.fragments().len()
    );
    println!("  elapsed                        {elapsed:>12?}");

    assert!(
        extent.pages > 10_000,
        "a sheet a million rows tall is many bands deep; got {}",
        extent.pages
    );
    assert!(
        peak < WINDOWED_BOUND,
        "reaching the last band of a million-row sheet allocated {peak} bytes, past \
         {WINDOWED_BOUND}. Scrolling to the end must cost what scrolling to the start costs."
    );
    assert!(
        visited >= 1,
        "and the far corner's own cell really is on this band, so the case is not vacuous"
    );

    // Twenty bands, timed together. Not a performance assertion — see the module documentation.
    let started = Instant::now();
    for band in 0..BANDS {
        let page = PageIndex::new(last.number().saturating_sub(band));
        let _ = model
            .layout_page(&grid, page, &constraints, None)
            .expect("a band lays out");
    }
    let elapsed = started.elapsed();
    println!("  {BANDS} bands took             {elapsed:>12?}  (ceiling {TIME_CEILING:?})");
    assert!(
        elapsed < TIME_CEILING,
        "{BANDS} windowed bands took {elapsed:?}, past the {TIME_CEILING:?} ceiling. A layout that \
         walked the coordinate range would not have finished."
    );
    peak.max(1)
}

/// **Case three.** The other shape of enormous: one row of 16,384 populated cells, of which a window
/// shows a dozen.
///
/// The vertical axis is not the only one that can be walked. A layout that iterated a row's cells
/// rather than asking for the columns in the window would visit sixteen thousand of them per row and
/// pass every assertion above, because the *rows* are still few.
fn a_row_of_sixteen_thousand_columns_lays_out_only_what_is_on_screen() -> usize {
    let book = workbook(full_width_worksheet());
    let grid = SheetGrid::read(&book, 0).expect("the sheet reads");
    let constraints = constraints();
    let mut model = model();
    let _ = model
        .layout_page(&grid, PageIndex::FIRST, &constraints, None)
        .expect("the band lays out");

    let before = mjx_allocation_counter::reset_peak();
    let started = Instant::now();
    let _ = model
        .layout_page(&grid, PageIndex::FIRST, &constraints, None)
        .expect("the band lays out again");
    let peak = mjx_allocation_counter::peak() - before;
    let elapsed = started.elapsed();
    let columns = model.catalogue().columns().len();
    let visited = model.catalogue().cells().len();

    println!("\ncase 3 — one row of 16,384 populated cells");
    println!("  populated cells                {:>12}", 16_384);
    println!("  columns laid out               {columns:>12}");
    println!("  cells with text laid out       {visited:>12}");
    println!("  peak allocated laying out      {peak:>12} bytes");
    println!("  elapsed                        {elapsed:>12?}");

    assert!(
        columns < 100,
        "a six-inch window shows a dozen nine-character columns, not {columns}"
    );
    assert!(
        visited == columns,
        "every visible column of the populated row has text, and no invisible one was touched: \
         {visited} cells for {columns} columns"
    );
    peak.max(1)
}

/// **Proving the gate can fail.** The same sheet, walked the way this crate refuses to walk it.
///
/// The naive implementation asks about *coordinates* rather than about populated cells. Doing that
/// over the whole grid is not runnable, so this does it over one ten-thousandth of it — a single
/// column's worth of rows — and shows that even that already costs more than the windowed layout of
/// the entire sheet. The gate above and this walk are the same measurement, so a reader can see the
/// bound is a line between two designs rather than a number nobody has tested.
fn the_naive_walk_breaches_every_bound() -> usize {
    let book = workbook(far_corner_worksheet());
    let grid = SheetGrid::read(&book, 0).expect("the sheet reads");

    /// How many coordinates the naive walk visits — one column's worth of rows, out of 16,384
    /// columns and 1,048,576 rows.
    const NAIVE_COORDINATES: u32 = 1_048_576;

    let before = mjx_allocation_counter::reset_peak();
    let started = Instant::now();
    let mut found = 0_usize;
    let mut boxes: Vec<(u32, u16)> = Vec::new();
    for row in 0..NAIVE_COORDINATES {
        // Exactly the defect: a fragment per addressable position rather than per populated cell.
        boxes.push((row, 16_383));
        if grid.cell(row, 16_383).is_some() {
            found += 1;
        }
    }
    let peak = mjx_allocation_counter::peak() - before;
    let elapsed = started.elapsed();

    println!("\ncase 4 — the naive walk, over 1/16,384th of the grid");
    println!("  coordinates visited            {NAIVE_COORDINATES:>12}");
    println!("  populated cells found          {found:>12}");
    println!("  peak allocated                 {peak:>12} bytes  (the windowed bound is {WINDOWED_BOUND})");
    println!("  elapsed                        {elapsed:>12?}");

    assert_eq!(found, 1, "the walk is real and finds the one cell");
    assert!(
        peak > WINDOWED_BOUND,
        "the naive walk allocated only {peak} bytes over {NAIVE_COORDINATES} coordinates, which \
         means this gate's bound of {WINDOWED_BOUND} would not have caught it — and a gate that \
         cannot fail is not a gate."
    );
    println!(
        "  → one column of the grid already costs {}× the whole windowed layout's bound",
        peak / WINDOWED_BOUND.max(1)
    );
    assert!(!boxes.is_empty());
    peak.max(1)
}
