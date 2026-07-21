//! Integration tests for the table surface: create a table, read its shape, size its columns and
//! rows, and read and write the text in its cells — with fidelity (only the edited part changes,
//! and reading changes nothing).
//!
//! `tests/fixtures/layouts.pptx` slide 2 carries a real one-cell table inside a `p:graphicFrame`,
//! left there by the transform workstream; it is the "read a table someone else wrote" case.
//! Everything else is built through `add_table`, because a caller's first act is to make one.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_dml::{CharacterPropertiesSpec, Emu, ParagraphPropertiesSpec, TextAlignment};
use mjx_opc::Package;
use mjx_pptx::{PptxError, Presentation, ShapeBounds, ShapeKind, Surface};

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

fn sample() -> Presentation {
    Presentation::open(&fixture("sample.pptx")).expect("open sample.pptx")
}

fn bounds() -> ShapeBounds {
    ShapeBounds::from_inches(1.0, 1.0, 6.0, 2.0)
}

/// The index of the first graphic frame on slide `slide` of `layouts.pptx`.
fn fixture_table(pres: &mut Presentation, slide: usize) -> usize {
    let count = pres.shape_count(slide).expect("count");
    (0..count)
        .find(|&idx| pres.shape_kind(slide, idx).expect("kind") == ShapeKind::GraphicFrame)
        .expect("a graphic frame")
}

// ---------------------------------------------------------------------------------------------
// Reading a table someone else wrote
// ---------------------------------------------------------------------------------------------

#[test]
fn reads_the_fixtures_table() {
    let mut pres = Presentation::open(&fixture("layouts.pptx")).expect("open");
    let table = fixture_table(&mut pres, 1);

    assert_eq!(pres.table_dimensions(1, table).expect("dimensions"), (1, 1));
    assert_eq!(
        pres.column_width(1, table, 0).expect("width"),
        Some(Emu::from_emu(3_048_000))
    );
    assert_eq!(
        pres.row_height(1, table, 0).expect("height"),
        Some(Emu::from_emu(370_840))
    );
    assert_eq!(pres.cell_text(1, table, 0, 0).expect("text"), "Cell");
}

#[test]
fn reading_a_table_dirties_nothing() {
    let bytes = fixture("layouts.pptx");
    let original = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    let table = fixture_table(&mut pres, 1);
    let _ = pres.table_dimensions(1, table).expect("dimensions");
    let _ = pres.cell_text(1, table, 0, 0).expect("text");
    let _ = pres.column_width(1, table, 0).expect("width");
    let _ = pres.cell_span(1, table, 0, 0).expect("span");

    assert_eq!(
        byte_map(&Package::open(&pres.save().expect("save")).expect("reopen")),
        original
    );
}

#[test]
fn a_table_is_a_shape_like_any_other() {
    // It is on the one shape index space, so it is positioned and counted like anything else.
    let mut pres = Presentation::open(&fixture("layouts.pptx")).expect("open");
    let table = fixture_table(&mut pres, 1);

    assert!(pres.shape_bounds(1, table).expect("bounds").is_some());
    let moved = ShapeBounds::from_inches(1.0, 4.0, 4.0, 1.0);
    pres.set_shape_bounds(1, table, moved).expect("move");
    assert_eq!(pres.shape_bounds(1, table).expect("bounds"), Some(moved));
}

// ---------------------------------------------------------------------------------------------
// Creating one
// ---------------------------------------------------------------------------------------------

#[test]
fn creates_a_table_of_the_requested_shape() {
    let mut pres = sample();
    let table = pres.add_table(0, 3, 4, bounds()).expect("add table");

    assert_eq!(
        pres.shape_kind(0, table).expect("kind"),
        ShapeKind::GraphicFrame
    );
    assert_eq!(pres.table_dimensions(0, table).expect("dims"), (3, 4));
    assert_eq!(pres.shape_bounds(0, table).expect("bounds"), Some(bounds()));
}

#[test]
fn the_columns_of_a_new_table_sum_to_its_width() {
    // Three columns cannot divide most widths evenly; the last absorbs the rounding so the table
    // is exactly as wide as its frame rather than a few EMU short.
    let mut pres = sample();
    let frame = ShapeBounds::new(0, 0, 1_000_000, 500_000);
    let table = pres.add_table(0, 2, 3, frame).expect("add table");

    let total: i64 = (0..3)
        .map(|column| {
            pres.column_width(0, table, column)
                .expect("width")
                .expect("stated")
                .emu()
        })
        .sum();
    assert_eq!(total, 1_000_000);
}

#[test]
fn a_table_with_no_cells_is_refused() {
    let mut pres = sample();
    assert!(matches!(
        pres.add_table(0, 0, 3, bounds()),
        Err(PptxError::InvalidTableSize {
            rows: 0,
            columns: 3
        })
    ));
    assert!(matches!(
        pres.add_table(0, 2, 0, bounds()),
        Err(PptxError::InvalidTableSize {
            rows: 2,
            columns: 0
        })
    ));
}

#[test]
fn a_created_table_survives_a_save_and_reopen() {
    let mut pres = sample();
    let before = byte_map(&Package::open(&fixture("sample.pptx")).expect("open"));
    let table = pres.add_table(0, 2, 2, bounds()).expect("add table");
    pres.set_cell_text(0, table, 0, 0, 0, "Region")
        .expect("text");
    pres.set_cell_text(0, table, 1, 1, 0, "42").expect("text");

    let saved = pres.save().expect("save");
    // Only the slide changed — creating a table adds no parts and no relationships.
    let after = byte_map(&Package::open(&saved).expect("reopen"));
    for (name, original) in &before {
        if name.ends_with("slide1.xml") {
            continue;
        }
        assert_eq!(after.get(name), Some(original), "dirtied {name}");
    }

    let mut reopened = Presentation::open(&saved).expect("reopen");
    assert_eq!(reopened.table_dimensions(0, table).expect("dims"), (2, 2));
    assert_eq!(reopened.cell_text(0, table, 0, 0).expect("text"), "Region");
    assert_eq!(reopened.cell_text(0, table, 1, 1).expect("text"), "42");
    assert_eq!(reopened.cell_text(0, table, 0, 1).expect("text"), "");
}

// ---------------------------------------------------------------------------------------------
// Sizing
// ---------------------------------------------------------------------------------------------

#[test]
fn columns_and_rows_are_resizable() {
    let mut pres = sample();
    let table = pres.add_table(0, 2, 2, bounds()).expect("add table");

    pres.set_column_width(0, table, 1, Emu::from_emu(999_000))
        .expect("set width");
    pres.set_row_height(0, table, 0, Emu::from_emu(555_000))
        .expect("set height");

    let saved = pres.save().expect("save");
    let mut reopened = Presentation::open(&saved).expect("reopen");
    assert_eq!(
        reopened.column_width(0, table, 1).expect("width"),
        Some(Emu::from_emu(999_000))
    );
    assert_eq!(
        reopened.row_height(0, table, 0).expect("height"),
        Some(Emu::from_emu(555_000))
    );
    // The neighbours are untouched.
    assert_ne!(
        reopened.column_width(0, table, 0).expect("width"),
        Some(Emu::from_emu(999_000))
    );
}

#[test]
fn sizing_past_the_edge_reports_the_tables_real_shape() {
    let mut pres = sample();
    let table = pres.add_table(0, 2, 3, bounds()).expect("add table");

    assert!(matches!(
        pres.column_width(0, table, 9),
        Err(PptxError::TableCellOutOfRange {
            column: 9,
            rows: 2,
            columns: 3,
            ..
        })
    ));
    assert!(matches!(
        pres.set_row_height(0, table, 7, Emu::from_emu(1)),
        Err(PptxError::TableCellOutOfRange {
            row: 7,
            rows: 2,
            columns: 3,
            ..
        })
    ));
}

// ---------------------------------------------------------------------------------------------
// Cell text — the same operations as a shape, addressed at a cell
// ---------------------------------------------------------------------------------------------

#[test]
fn writing_one_cell_leaves_its_neighbours_alone() {
    let mut pres = sample();
    let table = pres.add_table(0, 2, 2, bounds()).expect("add table");
    for (row, column, text) in [(0, 0, "A"), (0, 1, "B"), (1, 0, "C"), (1, 1, "D")] {
        pres.set_cell_text(0, table, row, column, 0, text)
            .expect("set text");
    }

    pres.set_cell_text(0, table, 1, 0, 0, "changed")
        .expect("set text");

    assert_eq!(pres.cell_text(0, table, 0, 0).expect("text"), "A");
    assert_eq!(pres.cell_text(0, table, 0, 1).expect("text"), "B");
    assert_eq!(pres.cell_text(0, table, 1, 0).expect("text"), "changed");
    assert_eq!(pres.cell_text(0, table, 1, 1).expect("text"), "D");
}

#[test]
fn a_header_cell_can_be_made_bold() {
    let mut pres = sample();
    let table = pres.add_table(0, 2, 2, bounds()).expect("add table");
    pres.set_cell_text(0, table, 0, 0, 0, "Region")
        .expect("text");

    let bold = CharacterPropertiesSpec::new().with_bold(true);
    pres.set_cell_run_properties_all(0, table, 0, 0, &bold)
        .expect("bold the header");

    let saved = pres.save().expect("save");
    let mut reopened = Presentation::open(&saved).expect("reopen");
    let properties = reopened
        .cell_run_properties(0, table, 0, 0, 0, 0)
        .expect("run properties")
        .expect("declared");
    assert_eq!(properties.is_bold(), Some(true));
    // The cell beside it took nothing.
    assert!(reopened
        .cell_run_properties(0, table, 0, 1, 0, 0)
        .expect("run properties")
        .and_then(|spec| spec.is_bold())
        .is_none());
}

#[test]
fn a_cells_paragraph_can_be_aligned() {
    let mut pres = sample();
    let table = pres.add_table(0, 1, 2, bounds()).expect("add table");
    pres.set_cell_text(0, table, 0, 1, 0, "42").expect("text");

    let centered = ParagraphPropertiesSpec::new().with_alignment(TextAlignment::Center);
    pres.set_cell_paragraph_properties(0, table, 0, 1, 0, &centered)
        .expect("align");

    let properties = pres
        .cell_paragraph_properties(0, table, 0, 1, 0)
        .expect("paragraph properties")
        .expect("declared");
    assert_eq!(properties.alignment(), Some(TextAlignment::Center));
}

#[test]
fn part_of_a_cells_text_can_be_formatted() {
    // The run-splitting range API reaches into a cell exactly as it reaches into a shape.
    let mut pres = sample();
    let table = pres.add_table(0, 1, 1, bounds()).expect("add table");
    pres.set_cell_text(0, table, 0, 0, 0, "hello world")
        .expect("text");
    assert_eq!(pres.cell_run_count(0, table, 0, 0, 0).expect("runs"), 1);

    let bold = CharacterPropertiesSpec::new().with_bold(true);
    pres.set_cell_text_range_properties(0, table, 0, 0, 0, 0..5, &bold)
        .expect("format a range");

    // The run split at the range's edge, and the text is unchanged.
    assert_eq!(pres.cell_run_count(0, table, 0, 0, 0).expect("runs"), 2);
    assert_eq!(
        pres.cell_paragraph_text(0, table, 0, 0, 0).expect("text"),
        "hello world"
    );
    assert_eq!(
        pres.cell_run_properties(0, table, 0, 0, 0, 0)
            .expect("properties")
            .and_then(|spec| spec.is_bold()),
        Some(true)
    );
    assert_eq!(
        pres.cell_run_properties(0, table, 0, 0, 0, 1)
            .expect("properties")
            .and_then(|spec| spec.is_bold()),
        None
    );
}

#[test]
fn an_empty_cell_can_be_given_the_format_text_will_take() {
    let mut pres = sample();
    let table = pres.add_table(0, 1, 1, bounds()).expect("add table");
    let spec = CharacterPropertiesSpec::new().with_size_points(24.0);
    pres.set_cell_end_run_properties(0, table, 0, 0, 0, &spec)
        .expect("set the paragraph mark");

    assert!(pres
        .cell_end_run_properties(0, table, 0, 0, 0)
        .expect("end properties")
        .expect("declared")
        .size()
        .is_some());
}

#[test]
fn a_table_on_a_layout_works_the_same() {
    let mut pres = Presentation::open(&fixture("layouts.pptx")).expect("open");
    let table = pres
        .add_table(Surface::Layout(0), 1, 2, bounds())
        .expect("add table to a layout");
    pres.set_cell_text(Surface::Layout(0), table, 0, 0, 0, "on a layout")
        .expect("text");

    assert_eq!(
        pres.cell_text(Surface::Layout(0), table, 0, 0)
            .expect("text"),
        "on a layout"
    );
}

// ---------------------------------------------------------------------------------------------
// Typed errors
// ---------------------------------------------------------------------------------------------

#[test]
fn a_cell_past_the_edge_says_what_the_table_is() {
    let mut pres = sample();
    let table = pres.add_table(0, 2, 3, bounds()).expect("add table");

    assert!(matches!(
        pres.cell_text(0, table, 5, 0),
        Err(PptxError::TableCellOutOfRange {
            row: 5,
            column: 0,
            rows: 2,
            columns: 3
        })
    ));
    assert!(matches!(
        pres.set_cell_text(0, table, 0, 9, 0, "x"),
        Err(PptxError::TableCellOutOfRange {
            row: 0,
            column: 9,
            rows: 2,
            columns: 3
        })
    ));
}

#[test]
fn a_shape_that_frames_no_table_says_so() {
    // sample.pptx's only shape is a `p:sp` — a text box is not a table, however much text it holds.
    let mut pres = sample();
    assert!(matches!(
        pres.cell_text(0, 0, 0, 0),
        Err(PptxError::ShapeIsNotATable)
    ));
    assert!(matches!(
        pres.table_dimensions(0, 0),
        Err(PptxError::ShapeIsNotATable)
    ));
}

#[test]
fn an_unmerged_cell_spans_one_and_anchors_itself() {
    let mut pres = sample();
    let table = pres.add_table(0, 2, 2, bounds()).expect("add table");

    assert_eq!(pres.cell_span(0, table, 1, 1).expect("span"), (1, 1));
    assert_eq!(
        pres.merged_cell_anchor(0, table, 1, 1).expect("anchor"),
        (1, 1)
    );
}
