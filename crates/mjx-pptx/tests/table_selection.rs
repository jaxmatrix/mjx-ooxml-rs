//! Integration tests for formatting a *selection* of cells — [`Cells`] and [`CellFormat`].
//!
//! The claim under test is that one call over a region does exactly what the same edits done cell
//! by cell would have done, and touches nothing else: **the cells outside the selection**, and **the
//! properties the format does not name**. A `format_cells` that rebuilt each `a:tcPr`, or that
//! resolved a row as a rectangle one cell too wide, would pass a single-cell test and fail these.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_dml::{
    CellBorder, CharacterPropertiesSpec, ColorSpec, Emu, FillSpec, LineSpec,
    ParagraphPropertiesSpec, TextAlignment, TextAnchoring,
};
use mjx_opc::Package;
use mjx_pptx::{CellFormat, CellMargins, Cells, PptxError, Presentation, ShapeBounds};

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

/// A deck with a 3x3 table on slide 0, every cell carrying text.
fn deck_with_table() -> (Presentation, usize) {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let table = pres
        .add_table(0, 3, 3, ShapeBounds::from_inches(0.5, 1.0, 8.0, 3.0))
        .expect("add table");
    for row in 0..3 {
        for column in 0..3 {
            pres.set_cell_text(0, table, row, column, 0, &format!("{row}{column}"))
                .expect("text");
        }
    }
    (pres, table)
}

fn navy() -> FillSpec {
    FillSpec::Solid(ColorSpec::Srgb("1F3864".to_owned()))
}

// ---------------------------------------------------------------------------------------------
// What a selection covers
// ---------------------------------------------------------------------------------------------

#[test]
fn a_row_selection_fills_that_row_and_no_other() {
    let (mut pres, table) = deck_with_table();
    pres.format_cells(
        0,
        table,
        Cells::row(1),
        &CellFormat::new().with_fill(navy()),
    )
    .expect("fill a row");

    for row in 0..3 {
        for column in 0..3 {
            let filled = pres
                .cell_fill(0, table, row, column)
                .expect("read")
                .is_some();
            assert_eq!(filled, row == 1, "cell {row},{column}");
        }
    }
}

#[test]
fn a_column_selection_covers_the_column() {
    let (mut pres, table) = deck_with_table();
    pres.format_cells(
        0,
        table,
        Cells::column(2),
        &CellFormat::new().with_anchor(TextAnchoring::Bottom),
    )
    .expect("anchor a column");

    for row in 0..3 {
        for column in 0..3 {
            let anchored = pres
                .cell_anchor(0, table, row, column)
                .expect("read")
                .is_some();
            assert_eq!(anchored, column == 2, "cell {row},{column}");
        }
    }
}

#[test]
fn a_rectangle_covers_exactly_its_corners() {
    let (mut pres, table) = deck_with_table();
    pres.format_cells(
        0,
        table,
        Cells::rectangle(1..3, 0..2),
        &CellFormat::new().with_fill(navy()),
    )
    .expect("fill a block");

    for row in 0..3 {
        for column in 0..3 {
            let filled = pres
                .cell_fill(0, table, row, column)
                .expect("read")
                .is_some();
            let inside = (1..3).contains(&row) && (0..2).contains(&column);
            assert_eq!(filled, inside, "cell {row},{column}");
        }
    }
}

#[test]
fn all_covers_every_cell() {
    let (mut pres, table) = deck_with_table();
    pres.format_cells(
        0,
        table,
        Cells::all(),
        &CellFormat::new().with_margins(CellMargins::uniform(Emu::from_emu(1_000))),
    )
    .expect("inset everything");

    for row in 0..3 {
        for column in 0..3 {
            assert_eq!(
                pres.cell_margins(0, table, row, column).expect("read").left,
                Some(Emu::from_emu(1_000)),
                "cell {row},{column}"
            );
        }
    }
}

#[test]
fn selecting_nothing_is_allowed_and_changes_nothing() {
    let (mut pres, table) = deck_with_table();
    pres.format_cells(
        0,
        table,
        Cells::rectangle(1..1, 0..3),
        &CellFormat::new().with_fill(navy()),
    )
    .expect("an empty selection is well-formed");

    for row in 0..3 {
        assert!(pres.cell_fill(0, table, row, 0).expect("read").is_none());
    }
}

#[test]
fn a_selection_past_an_edge_reports_the_tables_shape() {
    let (mut pres, table) = deck_with_table();
    assert!(matches!(
        pres.format_cells(
            0,
            table,
            Cells::row(7),
            &CellFormat::new().with_fill(navy())
        ),
        Err(PptxError::TableCellOutOfRange {
            row: 7,
            rows: 3,
            columns: 3,
            ..
        })
    ));
    assert!(matches!(
        pres.format_cells(
            0,
            table,
            Cells::rectangle(0..2, 0..9),
            &CellFormat::new().with_fill(navy())
        ),
        Err(PptxError::TableCellOutOfRange { column: 8, .. })
    ));
}

// ---------------------------------------------------------------------------------------------
// What a format names, and what it leaves alone
// ---------------------------------------------------------------------------------------------

#[test]
fn a_format_leaves_properties_it_does_not_name_alone() {
    // The reason `CellFormat` names properties rather than describing a whole cell: a caller can
    // recolour a region whose cells carry different borders without flattening them.
    let (mut pres, table) = deck_with_table();
    pres.set_cell_border(0, table, 1, 1, CellBorder::Top, &LineSpec::default())
        .expect("a border on one cell");

    pres.format_cells(0, table, Cells::all(), &CellFormat::new().with_fill(navy()))
        .expect("fill everything");

    assert!(
        pres.cell_border(0, table, 1, 1, CellBorder::Top)
            .expect("read")
            .is_some(),
        "the fill flattened a border it never mentioned"
    );
    assert!(pres.cell_fill(0, table, 1, 1).expect("read").is_some());
}

#[test]
fn a_format_that_names_nothing_writes_nothing() {
    // Not even an empty `a:tcPr` — an untouched cell should stay byte-identical.
    let (pres, table) = deck_with_table();
    let before = pres.save().expect("save");

    let mut again = Presentation::open(&before).expect("reopen");
    again
        .format_cells(0, table, Cells::all(), &CellFormat::new())
        .expect("a format naming nothing");

    assert_eq!(
        byte_map(&Package::open(&again.save().expect("save")).expect("reopen")),
        byte_map(&Package::open(&before).expect("reopen"))
    );
}

#[test]
fn one_call_does_what_the_loop_would_have_done() {
    // The equivalence the whole surface rests on.
    let (mut by_hand, table) = deck_with_table();
    let rule = LineSpec::default();
    for column in 0..3 {
        by_hand
            .set_cell_fill(0, table, 0, column, &navy())
            .expect("fill");
        by_hand
            .set_cell_border(0, table, 0, column, CellBorder::Bottom, &rule)
            .expect("border");
        by_hand
            .set_cell_anchor(0, table, 0, column, TextAnchoring::Center)
            .expect("anchor");
    }

    let (mut by_selection, _) = deck_with_table();
    by_selection
        .format_cells(
            0,
            table,
            Cells::row(0),
            &CellFormat::new()
                .with_fill(navy())
                .with_border(CellBorder::Bottom, rule)
                .with_anchor(TextAnchoring::Center),
        )
        .expect("format the row");

    assert_eq!(
        byte_map(&Package::open(&by_selection.save().expect("save")).expect("reopen")),
        byte_map(&Package::open(&by_hand.save().expect("save")).expect("reopen")),
        "the selection form must produce the same bytes as the per-cell form"
    );
}

#[test]
fn an_outline_draws_the_four_outer_edges() {
    let (mut pres, table) = deck_with_table();
    pres.format_cells(
        0,
        table,
        Cells::one(0, 0),
        &CellFormat::new().with_outline(LineSpec::default()),
    )
    .expect("box a cell");

    for edge in [
        CellBorder::Left,
        CellBorder::Right,
        CellBorder::Top,
        CellBorder::Bottom,
    ] {
        assert!(
            pres.cell_border(0, table, 0, 0, edge)
                .expect("read")
                .is_some(),
            "{edge:?}"
        );
    }
    assert!(
        pres.cell_border(0, table, 0, 0, CellBorder::TopLeftToBottomRight)
            .expect("read")
            .is_none(),
        "an outline is not a diagonal"
    );
}

#[test]
fn a_format_can_remove_as_well_as_write() {
    let (mut pres, table) = deck_with_table();
    pres.format_cells(
        0,
        table,
        Cells::all(),
        &CellFormat::new()
            .with_fill(navy())
            .with_outline(LineSpec::default()),
    )
    .expect("fill and box");

    pres.format_cells(
        0,
        table,
        Cells::row(2),
        &CellFormat::new().without_fill().without_borders(),
    )
    .expect("strip the last row");

    assert!(pres.cell_fill(0, table, 2, 0).expect("read").is_none());
    assert!(pres
        .cell_border(0, table, 2, 0, CellBorder::Top)
        .expect("read")
        .is_none());
    // The rows above kept theirs.
    assert!(pres.cell_fill(0, table, 1, 0).expect("read").is_some());
}

// ---------------------------------------------------------------------------------------------
// Text over a selection
// ---------------------------------------------------------------------------------------------

#[test]
fn a_header_row_is_bolded_in_one_call() {
    let (mut pres, table) = deck_with_table();
    pres.format_cell_text(
        0,
        table,
        Cells::row(0),
        &CharacterPropertiesSpec::new().with_bold(true),
    )
    .expect("bold the header");

    for column in 0..3 {
        assert_eq!(
            pres.cell_run_properties(0, table, 0, column, 0, 0)
                .expect("read")
                .and_then(|spec| spec.is_bold()),
            Some(true),
            "header cell {column}"
        );
    }
    assert!(pres
        .cell_run_properties(0, table, 1, 0, 0, 0)
        .expect("read")
        .and_then(|spec| spec.is_bold())
        .is_none());
}

#[test]
fn a_block_of_numbers_is_aligned_in_one_call() {
    let (mut pres, table) = deck_with_table();
    pres.format_cell_paragraphs(
        0,
        table,
        Cells::rectangle(1..3, 1..3),
        &ParagraphPropertiesSpec::new().with_alignment(TextAlignment::Right),
    )
    .expect("align");

    assert_eq!(
        pres.cell_paragraph_properties(0, table, 2, 2, 0)
            .expect("read")
            .and_then(|spec| spec.alignment()),
        Some(TextAlignment::Right)
    );
    assert!(pres
        .cell_paragraph_properties(0, table, 0, 0, 0)
        .expect("read")
        .and_then(|spec| spec.alignment())
        .is_none());
}

#[test]
fn formatting_text_over_a_selection_leaves_the_text_itself_alone() {
    let (mut pres, table) = deck_with_table();
    pres.format_cell_text(
        0,
        table,
        Cells::all(),
        &CharacterPropertiesSpec::new().with_bold(true),
    )
    .expect("bold everything");

    for row in 0..3 {
        for column in 0..3 {
            assert_eq!(
                pres.cell_text(0, table, row, column).expect("text"),
                format!("{row}{column}")
            );
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Fidelity
// ---------------------------------------------------------------------------------------------

#[test]
fn formatting_a_selection_dirties_only_its_slide() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let before = byte_map(&Package::open(&fixture("sample.pptx")).expect("baseline"));
    let table = pres
        .add_table(0, 2, 2, ShapeBounds::from_inches(1.0, 1.0, 4.0, 2.0))
        .expect("add table");
    pres.format_cells(0, table, Cells::all(), &CellFormat::new().with_fill(navy()))
        .expect("fill");

    let after = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));
    for (name, original) in &before {
        if name.ends_with("slide1.xml") {
            continue;
        }
        assert_eq!(after.get(name), Some(original), "dirtied {name}");
    }
}

#[test]
fn a_selection_on_a_shape_that_frames_no_table_says_so() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    assert!(matches!(
        pres.format_cells(0, 0, Cells::all(), &CellFormat::new().with_fill(navy())),
        Err(PptxError::ShapeIsNotATable)
    ));
}
