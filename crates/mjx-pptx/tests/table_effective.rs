//! Integration tests for effective cell formatting — what a table cell actually renders as.
//!
//! The resolution order under everything here: the cell's own `a:tcPr` wins; then the table style's
//! parts, chosen by the cell's position and the `a:tblPr` flags; then the theme. Colours bake to
//! concrete `RRGGBB`. The committed `tables.pptx` (a "Report Style" with a white bold header on navy,
//! a light-blue banded row, and whole-table inside borders) is the input that matters — resolution
//! reads it and dirties nothing.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_dml::{CellBorder, ColorSpec, FillSpec};
use mjx_opc::Package;
use mjx_pptx::{CellFormat, Cells, Presentation, ShapeBounds};

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()))
}

fn byte_map(pkg: &Package) -> BTreeMap<String, Vec<u8>> {
    pkg.entries()
        .iter()
        .filter_map(|e| e.bytes().map(|b| (e.name.clone(), b.to_vec())))
        .collect()
}

/// The `tables.pptx` deck and the index of its table on slide 0.
fn styled_deck() -> (Presentation, usize) {
    let mut pres = Presentation::open(&fixture("tables.pptx")).expect("open");
    let count = pres.shape_count(0).expect("count");
    let table = (0..count)
        .find(|&idx| pres.table_dimensions(0, idx).is_ok())
        .expect("a table on slide 0");
    (pres, table)
}

fn solid_hex(fill: Option<FillSpec>) -> Option<String> {
    match fill {
        Some(FillSpec::Solid(ColorSpec::Srgb(hex))) => Some(hex),
        _ => None,
    }
}

// ---------------------------------------------------------------------------------------------
// Fill resolution against the real style
// ---------------------------------------------------------------------------------------------

#[test]
fn the_header_row_resolves_the_first_row_fill() {
    let (mut pres, table) = styled_deck();
    // Row 0 is the header (firstRow="1"); the style fills it navy 1F3864.
    assert_eq!(
        solid_hex(pres.effective_cell_fill(0, table, 0, 1).expect("fill")).as_deref(),
        Some("1F3864"),
        "the header takes the firstRow fill"
    );
}

#[test]
fn a_banded_data_row_resolves_the_band_fill() {
    let (mut pres, table) = styled_deck();
    // Row 1 is the first data row (band1H), which the style fills light blue D9E1F2.
    assert_eq!(
        solid_hex(pres.effective_cell_fill(0, table, 1, 1).expect("fill")).as_deref(),
        Some("D9E1F2"),
        "the first data row takes the band1H fill"
    );
    // Row 2 is band2H, which this style leaves unfilled — so the cell resolves no fill.
    assert_eq!(
        pres.effective_cell_fill(0, table, 2, 1).expect("fill"),
        None,
        "an even band with no fill resolves to nothing"
    );
}

#[test]
fn an_explicit_cell_fill_wins_over_the_style() {
    let (mut pres, table) = styled_deck();
    // Override the header cell's own tcPr — an explicit fill must beat the firstRow style fill.
    pres.format_cells(
        0,
        table,
        Cells::rectangle(0..1, 0..1),
        &CellFormat::new().with_fill(FillSpec::solid(ColorSpec::Srgb("00FF00".to_owned()))),
    )
    .expect("override cell fill");
    assert_eq!(
        solid_hex(pres.effective_cell_fill(0, table, 0, 0).expect("fill")).as_deref(),
        Some("00FF00"),
        "the cell's own fill wins"
    );
    // Its neighbour still resolves the style.
    assert_eq!(
        solid_hex(pres.effective_cell_fill(0, table, 0, 1).expect("fill")).as_deref(),
        Some("1F3864")
    );
}

// ---------------------------------------------------------------------------------------------
// Border resolution — outer edge vs interior edge
// ---------------------------------------------------------------------------------------------

#[test]
fn an_interior_top_border_resolves_the_inside_horizontal_style() {
    let (mut pres, table) = styled_deck();
    // The wholeTbl style states an `insideH` border. A cell within the table (row 1) resolves its
    // top border from insideH; the top row's cells have no such interior line above them.
    assert!(
        pres.effective_cell_border(0, table, 1, 1, CellBorder::Top)
            .expect("border")
            .is_some(),
        "an interior cell's top border comes from insideH"
    );
    assert!(
        pres.effective_cell_border(0, table, 0, 1, CellBorder::Top)
            .expect("border")
            .is_none(),
        "the top row has no interior line above it, and the style sets no outer top"
    );
}

// ---------------------------------------------------------------------------------------------
// Text resolution — the style's tcTxStyle
// ---------------------------------------------------------------------------------------------

#[test]
fn the_header_text_resolves_bold_and_white_from_the_style() {
    let (mut pres, table) = styled_deck();
    let header = pres
        .effective_cell_run_properties(0, table, 0, 0, 0, 0)
        .expect("run properties");
    assert_eq!(
        header.is_bold(),
        Some(true),
        "the firstRow style makes the header bold"
    );
    assert_eq!(
        solid_hex(header.fill().cloned()).as_deref(),
        Some("FFFFFF"),
        "the header text is white"
    );

    // A data cell takes none of that — the style says nothing about its text.
    let data = pres
        .effective_cell_run_properties(0, table, 1, 0, 0, 0)
        .expect("run properties");
    assert_eq!(
        data.is_bold(),
        None,
        "a data cell is not made bold by the style"
    );
}

// ---------------------------------------------------------------------------------------------
// Fidelity
// ---------------------------------------------------------------------------------------------

#[test]
fn resolving_a_cell_dirties_nothing() {
    let before = byte_map(&Package::open(&fixture("tables.pptx")).expect("baseline"));

    let (mut pres, table) = styled_deck();
    // Resolve fill, borders and text across several cells — a pure read.
    for row in 0..3 {
        for column in 0..3 {
            let _ = pres
                .effective_cell_fill(0, table, row, column)
                .expect("fill");
            let _ = pres
                .effective_cell_border(0, table, row, column, CellBorder::Top)
                .expect("border");
            let _ = pres
                .effective_cell_run_properties(0, table, row, column, 0, 0)
                .expect("text");
        }
    }

    let after = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));
    assert_eq!(after, before, "resolution must not dirty any part");
}

// ---------------------------------------------------------------------------------------------
// From-scratch, for out-of-range and no-style behaviour
// ---------------------------------------------------------------------------------------------

/// A table from `add_table` is **not** unstyled, and that is MJXOFF-232.
///
/// It is born with `firstRow` and `bandRow` on, and until MJXOFF-232 it named no style for them to
/// emphasise — so this case read `None` from the header row and called the table unstyled. It now
/// points at the default style `add_table` authors, whose header row is the deck's own `accent1`,
/// and the assertion is the interesting one either way: **the emphasis flags resolve to a colour
/// that came out of the theme.** `4472C4` is `sample.pptx`'s `accent1`, not a literal anything in
/// this library writes — open the same deck in a template branded green and this reads green.
///
/// The second half is unchanged and is what the case was originally for: a cell's *own* fill beats
/// the style's.
#[test]
fn a_table_from_add_table_resolves_its_style_and_a_cell_still_beats_it() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let table = pres
        .add_table(0, 2, 2, ShapeBounds::from_inches(1.0, 1.0, 4.0, 2.0))
        .expect("add table");
    let theme_accent = pres
        .theme(mjx_pptx::Surface::Master(0))
        .expect("reading the theme")
        .expect("a theme")
        .color(mjx_dml::ColorSchemeSlot::Accent1)
        .cloned();
    assert_eq!(
        solid_hex(pres.effective_cell_fill(0, table, 0, 0).expect("fill")).as_deref(),
        Some("4472C4"),
        "the header row resolves through the style `add_table` authored"
    );
    assert!(
        matches!(theme_accent, Some(mjx_dml::ColorSpec::Srgb(ref hex)) if hex == "4472C4"),
        "and that colour is the deck's own accent1, not one this library chose: {theme_accent:?}"
    );

    // And the banded row resolves the *derived* colour: `accent1` with its luminance modulated to
    // 20% and offset by 80% — Office's own "Lighter 80%". Nothing in this library writes `DAE3F3`;
    // it is what the resolver computes from the deck's accent, which is the strongest statement
    // available here that the banding follows the theme rather than a literal.
    assert_eq!(
        solid_hex(pres.effective_cell_fill(0, table, 1, 0).expect("fill")).as_deref(),
        Some("DAE3F3"),
        "the banded row resolves accent1 lightened, not a colour of ours"
    );

    // Its own fill still resolves.
    pres.format_cells(
        0,
        table,
        Cells::rectangle(1..2, 1..2),
        &CellFormat::new().with_fill(FillSpec::solid(ColorSpec::Srgb("ABCDEF".to_owned()))),
    )
    .expect("fill a cell");
    assert_eq!(
        solid_hex(pres.effective_cell_fill(0, table, 1, 1).expect("fill")).as_deref(),
        Some("ABCDEF")
    );
}

#[test]
fn resolving_past_the_edge_is_out_of_range() {
    let (mut pres, table) = styled_deck();
    assert!(matches!(
        pres.effective_cell_fill(0, table, 9, 0),
        Err(mjx_pptx::PptxError::TableCellOutOfRange { row: 9, .. })
    ));
}
