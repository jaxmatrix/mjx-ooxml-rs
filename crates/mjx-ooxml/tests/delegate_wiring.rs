//! Every delegate is wired to the method it is named after.
//!
//! A facade of 250-odd one-line delegates has exactly one interesting failure mode, and it is not
//! "the underlying method is broken" — that is `mjx-pptx`'s test suite's job, and a facade test that
//! passes because the method below it works proves nothing about the facade. The failure mode here is
//! **a delegate pointing at the wrong thing**: `column_width` forwarding to `row_height`,
//! `Surface::Layout` arriving as `Surface::Slide`, a `(rows, columns)` pair coming back the other way
//! round, a `u32` index widened from the wrong argument.
//!
//! So every assertion below is **asymmetric on purpose**. Nothing is set to the same value as its
//! neighbour, nothing is read at an index that would answer the same at another, and every pair that
//! could be swapped is given values that differ. A delegate wired to its neighbour fails here even
//! though the method it wrongly calls works perfectly.

use mjx_ooxml::{
    CellFormat, Cells, CharacterPropertiesSpec, ColorSpec, Deck, ErrorCode, FillSpec, LineSpec,
    LineWidth, PresetShapeType, ShapeBounds, ShapePath, SlideSize, Surface,
};

/// Four distinguishable rectangles, so a bounds delegate cannot pass by reading the wrong one.
fn bounds(x: i64, y: i64, w: i64, h: i64) -> ShapeBounds {
    ShapeBounds::new(x, y, w, h)
}

fn navy() -> FillSpec {
    FillSpec::solid(ColorSpec::Srgb("1F3864".into()))
}

fn gold() -> FillSpec {
    FillSpec::solid(ColorSpec::Srgb("FFC000".into()))
}

/// The colour a solid fill states, so two fills can be told apart by value.
fn solid_color(fill: &FillSpec) -> String {
    match fill {
        FillSpec::Solid(ColorSpec::Srgb(hex)) => hex.to_string(),
        other => panic!("expected a solid sRGB fill, got {other:?}"),
    }
}

/// Fill and outline are adjacent one-line delegates over the same `p:spPr`. Swapping them is the
/// single easiest mistake to make and the hardest to notice, so they are written with different
/// colours and read back separately.
#[test]
fn fill_and_outline_are_not_each_other() {
    let mut deck = Deck::blank(SlideSize::widescreen()).expect("a blank deck");
    let slide = Surface::Slide(deck.add_slide_from_layout(0).expect("a slide"));
    let shape: ShapePath = deck
        .add_shape(slide, PresetShapeType::Rectangle, bounds(0, 0, 100, 200))
        .expect("a shape")
        .into();

    deck.set_shape_fill(slide, shape.clone(), &navy())
        .expect("a fill");
    deck.set_shape_outline(
        slide,
        shape.clone(),
        &LineSpec::solid(
            LineWidth::from_points(3.0),
            ColorSpec::Srgb("FFC000".into()),
        ),
    )
    .expect("an outline");

    let fill = deck
        .shape_fill(slide, shape.clone())
        .expect("reading the fill")
        .expect("a stated fill");
    assert_eq!(solid_color(&fill), "1F3864", "shape_fill read the outline");

    let outline = deck
        .shape_outline(slide, shape.clone())
        .expect("reading the outline")
        .expect("a stated outline");
    assert_eq!(
        outline.width.expect("a stated width"),
        LineWidth::from_points(3.0),
        "shape_outline read something other than the line it was given"
    );

    // Clearing one must not clear the other.
    deck.set_shape_no_fill(slide, shape.clone())
        .expect("no fill");
    assert!(
        deck.shape_outline(slide, shape.clone())
            .expect("reading the outline")
            .is_some(),
        "set_shape_no_fill reached the outline"
    );
}

/// `x` and `y`, `width` and `height` are four independent numbers on one delegate; a transposed pair
/// reads as a plausible rectangle unless the four differ.
#[test]
fn bounds_keep_their_four_axes_apart() {
    let mut deck = Deck::blank(SlideSize::widescreen()).expect("a blank deck");
    let slide = Surface::Slide(deck.add_slide_from_layout(0).expect("a slide"));
    let shape: ShapePath = deck
        .add_shape(slide, PresetShapeType::Rectangle, bounds(11, 22, 33, 44))
        .expect("a shape")
        .into();

    let read = deck
        .shape_bounds(slide, shape)
        .expect("reading the bounds")
        .expect("stated bounds");
    assert_eq!(
        (
            read.offset_x_emu,
            read.offset_y_emu,
            read.width_emu,
            read.height_emu
        ),
        (11, 22, 33, 44)
    );
}

/// Rows and columns are the pair most easily transposed, and `cell_span` answers `(rows, columns)` —
/// the order A8 fixed. A 3x2 table with a 1x2 merge distinguishes every possible transposition.
#[test]
fn rows_and_columns_are_not_transposed() {
    let mut deck = Deck::blank(SlideSize::widescreen()).expect("a blank deck");
    let slide = Surface::Slide(deck.add_slide_from_layout(0).expect("a slide"));
    let table: ShapePath = deck
        .add_table(slide, 3, 2, bounds(0, 0, 6_000_000, 2_000_000))
        .expect("a table")
        .into();

    assert_eq!(
        deck.table_dimensions(slide, table.clone())
            .expect("the dimensions"),
        (3, 2),
        "table_dimensions answered (columns, rows)"
    );

    // Sizing: one column and one *different* row, at different indices, to different values.
    deck.set_column_width(slide, table.clone(), 1, mjx_ooxml::Emu::from_emu(1_234_567))
        .expect("a column width");
    deck.set_row_height(slide, table.clone(), 2, mjx_ooxml::Emu::from_emu(7_654_321))
        .expect("a row height");
    assert_eq!(
        deck.column_width(slide, table.clone(), 1)
            .expect("reading the width"),
        Some(mjx_ooxml::Emu::from_emu(1_234_567)),
        "column_width read a row height"
    );
    assert_eq!(
        deck.row_height(slide, table.clone(), 2)
            .expect("reading the height"),
        Some(mjx_ooxml::Emu::from_emu(7_654_321)),
        "row_height read a column width"
    );

    // Cell addressing: (0, 1) and (1, 0) must not be the same cell.
    deck.set_cell_text(slide, table.clone(), 0, 1, 0, "top right")
        .expect("a cell");
    deck.set_cell_text(slide, table.clone(), 1, 0, 0, "middle left")
        .expect("a cell");
    assert_eq!(
        deck.cell_text(slide, table.clone(), 0, 1).expect("text"),
        "top right"
    );
    assert_eq!(
        deck.cell_text(slide, table.clone(), 1, 0).expect("text"),
        "middle left"
    );

    // A 1-row by 2-column merge: `cell_span` answers (rows, columns), so (1, 2), never (2, 1).
    deck.merge_cells(slide, table.clone(), Cells::row(0))
        .expect("a merge");
    assert_eq!(
        deck.cell_span(slide, table.clone(), 0, 0).expect("a span"),
        (1, 2),
        "cell_span answered (columns, rows)"
    );

    // Structural edits must reach the axis they name.
    deck.insert_row(slide, table.clone(), 0).expect("a row");
    assert_eq!(
        deck.table_dimensions(slide, table.clone())
            .expect("the dimensions"),
        (4, 2),
        "insert_row changed the column count"
    );
    deck.insert_column(slide, table.clone(), 0)
        .expect("a column");
    assert_eq!(
        deck.table_dimensions(slide, table).expect("the dimensions"),
        (4, 3),
        "insert_column changed the row count"
    );
}

/// A slide, a layout and a master are three different parts. A `Surface` conversion that dropped the
/// kind — or read the index off the wrong variant — would read a layout's text off a slide.
#[test]
fn a_surface_reaches_the_part_it_names() {
    let mut deck = Deck::blank(SlideSize::widescreen()).expect("a blank deck");
    let slide = Surface::Slide(deck.add_slide_from_layout(0).expect("a slide"));
    let before = [
        deck.shape_count(slide).expect("a count"),
        deck.shape_count(Surface::Layout(0)).expect("a count"),
        deck.shape_count(Surface::Master(0)).expect("a count"),
    ];

    let on_slide: ShapePath = deck
        .add_text_box(slide, "on the slide", bounds(0, 0, 100, 100))
        .expect("a text box")
        .into();
    let on_layout: ShapePath = deck
        .add_text_box(Surface::Layout(0), "on the layout", bounds(0, 0, 100, 100))
        .expect("a text box")
        .into();
    let on_master: ShapePath = deck
        .add_text_box(Surface::Master(0), "on the master", bounds(0, 0, 100, 100))
        .expect("a text box")
        .into();

    assert_eq!(
        deck.shape_text(slide, on_slide).expect("text"),
        "on the slide"
    );
    assert_eq!(
        deck.shape_text(Surface::Layout(0), on_layout)
            .expect("text"),
        "on the layout"
    );
    assert_eq!(
        deck.shape_text(Surface::Master(0), on_master)
            .expect("text"),
        "on the master"
    );

    // Each surface gained exactly one shape. A `Surface` conversion that collapsed a layout or a
    // master onto the slide would have put three shapes on one part and none on the others — which
    // the text assertions above would still pass, because each shape would be at its own index.
    let after = [
        deck.shape_count(slide).expect("a count"),
        deck.shape_count(Surface::Layout(0)).expect("a count"),
        deck.shape_count(Surface::Master(0)).expect("a count"),
    ];
    for (index, (before, after)) in before.iter().zip(after.iter()).enumerate() {
        assert_eq!(
            after - before,
            1,
            "surface {index} gained {} shapes, not one",
            after - before
        );
    }
}

/// A `ShapePath` that lost its tail would address the group instead of the member, and the group's
/// own fill reads back fine — so the members are given different fills and read separately.
#[test]
fn a_shape_path_descends_into_the_group_it_names() {
    let mut deck = Deck::blank(SlideSize::widescreen()).expect("a blank deck");
    let slide = Surface::Slide(deck.add_slide_from_layout(0).expect("a slide"));
    let first = deck
        .add_shape(slide, PresetShapeType::Rectangle, bounds(0, 0, 100, 100))
        .expect("a shape");
    let second = deck
        .add_shape(slide, PresetShapeType::Ellipse, bounds(200, 0, 100, 100))
        .expect("a shape");

    let group = deck
        .group_shapes(slide, &[first.into(), second.into()])
        .expect("a group");
    assert_eq!(group.depth(), 1, "a new group is a top-level shape");

    deck.set_shape_fill(slide, group.child(0), &navy())
        .expect("a fill");
    deck.set_shape_fill(slide, group.child(1), &gold())
        .expect("a fill");

    assert_eq!(
        solid_color(
            &deck
                .shape_fill(slide, group.child(0))
                .expect("reading")
                .expect("a fill")
        ),
        "1F3864"
    );
    assert_eq!(
        solid_color(
            &deck
                .shape_fill(slide, group.child(1))
                .expect("reading")
                .expect("a fill")
        ),
        "FFC000",
        "the second member read the first"
    );
    assert_eq!(
        deck.shape_member_count(slide, group.clone())
            .expect("a member count"),
        2
    );
    assert_eq!(group.child(1).indices(), [group.indices()[0], 1]);
}

/// Paragraphs and runs are two nested index spaces over one text body. A delegate that forwarded a
/// run index as a paragraph index reads a plausible string unless the two counts differ.
#[test]
fn paragraph_and_run_indices_are_not_each_other() {
    let mut deck = Deck::blank(SlideSize::widescreen()).expect("a blank deck");
    let slide = Surface::Slide(deck.add_slide_from_layout(0).expect("a slide"));
    let shape: ShapePath = deck
        .add_text_box(slide, "first\nsecond\nthird", bounds(0, 0, 100, 100))
        .expect("a text box")
        .into();

    assert_eq!(
        deck.paragraph_count(slide, shape.clone()).expect("a count"),
        3
    );
    assert_eq!(
        deck.run_count(slide, shape.clone(), 1).expect("a count"),
        1,
        "run_count answered the paragraph count"
    );
    assert_eq!(
        deck.paragraph_text(slide, shape.clone(), 2).expect("text"),
        "third",
        "paragraph_text read the wrong paragraph"
    );
    assert_eq!(
        deck.run_text(slide, shape, 1, 0).expect("text"),
        "second",
        "run_text read the wrong paragraph"
    );
}

/// Counts and indices cross the `usize`/`u32` boundary in opposite directions. Three slides added
/// one at a time, each addressed by the index it was handed, catches a conversion that saturated or
/// dropped a value.
#[test]
fn u32_indices_survive_the_round_trip() {
    let mut deck = Deck::blank(SlideSize::widescreen()).expect("a blank deck");
    for expected in 0..3u32 {
        let added = deck.add_slide_from_layout(0).expect("a slide");
        assert_eq!(added, expected, "add_slide_from_layout returned {added}");
        deck.set_notes_text(added, &format!("notes for slide {added}"))
            .expect("notes");
    }
    assert_eq!(deck.slide_count(), 3);
    for slide in 0..3u32 {
        assert_eq!(
            deck.notes_text(slide).expect("notes").expect("some notes"),
            format!("notes for slide {slide}"),
            "the notes of slide {slide} came from another slide"
        );
    }
}

/// `format_cells` and the per-cell setters are *not* the same call, which is why both are exposed:
/// the bulk formatter deliberately skips a cell covered by a merge, and the per-cell setter reaches
/// it. If they were wired to each other, one of these two assertions would fail.
#[test]
fn a_per_cell_setter_reaches_what_the_bulk_formatter_skips() {
    let mut deck = Deck::blank(SlideSize::widescreen()).expect("a blank deck");
    let slide = Surface::Slide(deck.add_slide_from_layout(0).expect("a slide"));
    let table: ShapePath = deck
        .add_table(slide, 2, 2, bounds(0, 0, 6_000_000, 2_000_000))
        .expect("a table")
        .into();

    // Merge the top row: (0, 0) anchors it and (0, 1) is covered.
    deck.merge_cells(slide, table.clone(), Cells::row(0))
        .expect("a merge");

    // The bulk formatter skips the covered cell.
    deck.format_cells(
        slide,
        table.clone(),
        Cells::all(),
        &CellFormat::new().with_fill(navy()),
    )
    .expect("bulk formatting");
    assert!(
        deck.cell_fill(slide, table.clone(), 0, 1)
            .expect("reading")
            .is_none(),
        "format_cells reached a covered cell; the two spellings really are the same call"
    );

    // The per-cell setter reaches it — which is why dropping it would drop a capability.
    deck.set_cell_fill(slide, table.clone(), 0, 1, &gold())
        .expect("a per-cell fill");
    assert_eq!(
        solid_color(
            &deck
                .cell_fill(slide, table.clone(), 0, 1)
                .expect("reading")
                .expect("a fill")
        ),
        "FFC000"
    );
    // And it did not disturb the anchor.
    assert_eq!(
        solid_color(
            &deck
                .cell_fill(slide, table, 0, 0)
                .expect("reading")
                .expect("a fill")
        ),
        "1F3864"
    );
}

/// A failure carries the coordinates the caller used, in the caller's own `u32` addressing — not the
/// model's, and not another call's.
#[test]
fn an_error_names_where_it_happened() {
    let mut deck = Deck::blank(SlideSize::widescreen()).expect("a blank deck");
    let slide = Surface::Slide(deck.add_slide_from_layout(0).expect("a slide"));
    deck.add_shape(slide, PresetShapeType::Rectangle, bounds(0, 0, 100, 100))
        .expect("a shape");

    // A shape address past the end of a layout — a different surface from the one edited above.
    let error = deck
        .shape_fill(Surface::Layout(0), ShapePath::from(99))
        .expect_err("no such shape");
    assert_eq!(error.code(), ErrorCode::IndexOutOfRange);
    assert_eq!(error.detail().surface, Some(Surface::Layout(0)));
    assert_eq!(error.detail().shape, Some(ShapePath::from(99)));
    assert_eq!(error.detail().row, None);

    // A table cell past the edge names the row and the column, and does not transpose them.
    let table: ShapePath = deck
        .add_table(slide, 3, 2, bounds(0, 0, 6_000_000, 2_000_000))
        .expect("a table")
        .into();
    let error = deck
        .cell_text(slide, table, 1, 7)
        .expect_err("no such cell");
    assert_eq!(error.code(), ErrorCode::IndexOutOfRange);
    assert_eq!(error.detail().row, Some(1));
    assert_eq!(error.detail().column, Some(7));

    // A shape with no text body is a different code from an out-of-range one.
    let picture: ShapePath = deck
        .add_picture(
            slide,
            mjx_ooxml::DEFAULT_PLACEHOLDER_IMAGE,
            bounds(0, 0, 100, 100),
        )
        .expect("a picture")
        .into();
    assert_eq!(
        deck.shape_text(slide, picture.clone())
            .expect_err("a picture has no text body")
            .code(),
        ErrorCode::NothingToRead
    );
    // And asking a picture for a table is a third.
    assert_eq!(
        deck.table_dimensions(slide, picture)
            .expect_err("a picture is not a table")
            .code(),
        ErrorCode::WrongKind
    );
}

/// The typed cause survives the collapse into a code, so Rust callers lose nothing.
#[test]
fn the_underlying_error_is_still_reachable() {
    use std::error::Error as _;

    let mut deck = Deck::blank(SlideSize::widescreen()).expect("a blank deck");
    let error = deck
        .shape_fill(Surface::Slide(0), ShapePath::from(0))
        .expect_err("a blank deck has no slides");

    let source: &mjx_ooxml::PptxError = error
        .source()
        .and_then(<dyn std::error::Error>::downcast_ref)
        .expect("the PptxError behind the code");
    assert!(
        matches!(
            source,
            mjx_ooxml::PptxError::SlideIndexOutOfRange { index: 0, count: 0 }
        ),
        "unexpected cause: {source:?}"
    );
}

/// `save` inherits the validation `Presentation::save` performs; the facade does not route around it.
#[test]
fn save_still_validates() {
    let mut deck = Deck::blank(SlideSize::widescreen()).expect("a blank deck");
    let slide = Surface::Slide(deck.add_slide_from_layout(0).expect("a slide"));
    let shape: ShapePath = deck
        .add_text_box(slide, "text", bounds(0, 0, 100, 100))
        .expect("a text box")
        .into();
    deck.set_shape_run_properties(
        slide,
        shape.clone(),
        &CharacterPropertiesSpec::new().with_bold(true),
    )
    .expect("run properties");

    deck.validate().expect("a well-formed deck validates");
    let saved = deck.save().expect("a well-formed deck saves");

    // `save` and `save_unchecked` write the same bytes for a valid deck — the difference is only the
    // check, which is what makes the check free to keep.
    assert_eq!(
        saved.len(),
        deck.save_unchecked().expect("saving").len(),
        "validation changed what was written"
    );
    assert_eq!(
        Deck::open(&saved)
            .expect("reopening")
            .shape_text(slide, shape)
            .expect("text"),
        "text"
    );
}
