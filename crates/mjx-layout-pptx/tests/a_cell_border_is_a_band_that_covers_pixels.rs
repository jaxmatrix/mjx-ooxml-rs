//! A cell's borders become fragments, and the thinnest one still covers area.
//!
//! # Why a border is a band, and what that costs
//!
//! A [`mjx_scene::Decoration`] carries **one** stroke and a cell has four edges that can differ in
//! colour, weight and dash — so each edge is its own fragment: a box the width of the line, filled
//! with what the line is filled with. That is the only shape expressible in `mjx-layout`'s closed
//! six-kind vocabulary that gets colour, position and weight right for all four at once. What it
//! loses is the dash pattern, which is stated at the site rather than hidden.
//!
//! # The defect this suite exists for
//!
//! A band's thickness is the line's width, and DrawingML states a **hairline** as `@w="0"`. A band
//! of zero width covers no pixels, so a table whose style states hairline borders would draw none of
//! them — at every zoom level, in every export, with nothing anywhere to notice it. The first
//! version of `border_band` floored the width at one EMU, and half of one EMU is zero, so it had
//! exactly that defect and no test would have found it: the committed `tables.pptx` fixture's style
//! states no borders at all, so the whole code path was unreached.
//!
//! Hence this file. Every assertion below is on a **non-zero extent**, never on a fragment existing.

mod support;

use mjx_dml::{CellBorder, ColorSpec, FillSpec, LineSpec, LineWidth};
use mjx_layout::{Fragment, FragmentTree, LayoutRect};
use mjx_ooxml_core::measure::Emu;
use mjx_pptx::{Presentation, ShapeBounds};

/// A one-cell table whose bottom edge is a line of `width`.
fn table_with_a_bottom_border(width: LineWidth) -> (Presentation, usize) {
    let (mut deck, slide) = support::blank_deck();
    let table = deck
        .add_table(slide, 2, 2, ShapeBounds::from_inches(1.0, 1.0, 4.0, 2.0))
        .expect("a table");
    deck.set_cell_text(slide, table, 0, 0, 0, "edge")
        .expect("cell text");
    deck.set_cell_border(
        slide,
        table,
        0,
        0,
        CellBorder::Bottom,
        &LineSpec {
            width: Some(width),
            fill: Some(FillSpec::Solid(ColorSpec::Srgb("C00000".to_owned()))),
            ..LineSpec::new()
        },
    )
    .expect("a bottom border");
    (deck, slide)
}

/// Every box fragment of `tree` that is neither a cell nor a page — which is what a border band is.
fn bands(tree: &FragmentTree) -> Vec<LayoutRect> {
    tree.nodes()
        .filter_map(|(_, node)| match node.fragment() {
            Fragment::Box(box_fragment)
                if box_fragment.cell.is_none()
                    && box_fragment.decoration.is_some()
                    && node.parent().is_some() =>
            {
                Some(node.rect())
            }
            _ => None,
        })
        .collect()
}

#[test]
fn a_stated_border_becomes_a_band_of_its_own_width() {
    let (mut deck, slide) = table_with_a_bottom_border(LineWidth::from_points(2.0));
    let mut model = support::model();
    let tree = support::lay_out(&mut model, &mut deck, slide);

    let bands = bands(&tree);
    assert_eq!(
        bands.len(),
        1,
        "one edge was drawn on one cell, so exactly one band is expected; {} means either the \
         border was lost or every edge drew one",
        bands.len()
    );

    let band = bands[0];
    assert_eq!(
        band.height(),
        Emu::from_emu(LineWidth::from_points(2.0).emu()),
        "a two-point border is a band two points tall — the width the document states, not a \
         default"
    );
    assert!(
        band.width() > Emu::from_emu(1_000_000),
        "the band spans its cell's whole width; {} EMU is not a cell",
        band.width().emu()
    );
}

#[test]
fn a_hairline_border_still_covers_area() {
    // `@w="0"` is DrawingML's hairline. Half of a one-EMU band rounds to zero, which is the defect
    // this file's own documentation describes — and it is invisible in every other suite, because a
    // fragment with a zero-height rectangle is still a fragment.
    let (mut deck, slide) = table_with_a_bottom_border(LineWidth::from_emu(0));
    let mut model = support::model();
    let tree = support::lay_out(&mut model, &mut deck, slide);

    let bands = bands(&tree);
    assert_eq!(bands.len(), 1, "the hairline border is still a border");
    assert!(
        bands[0].height() > Emu::ZERO,
        "a hairline border came back {} EMU tall. A band of zero height covers no pixels, so the \
         border would be absent from every render and every export with nothing to report it.",
        bands[0].height().emu()
    );
}

#[test]
fn four_different_edges_draw_four_different_bands() {
    // How many distinct values did the gate see: one edge proves one arm of `border_band`'s match,
    // and the four are computed from different corners of the rectangle. A translation that used
    // `rect.top` for the bottom edge would pass a suite that only ever drew one.
    let (mut deck, slide) = support::blank_deck();
    let table = deck
        .add_table(slide, 2, 2, ShapeBounds::from_inches(1.0, 1.0, 4.0, 2.0))
        .expect("a table");
    let rule = LineSpec {
        width: Some(LineWidth::from_points(1.0)),
        fill: Some(FillSpec::Solid(ColorSpec::Srgb("0070C0".to_owned()))),
        ..LineSpec::new()
    };
    for edge in [
        CellBorder::Left,
        CellBorder::Right,
        CellBorder::Top,
        CellBorder::Bottom,
    ] {
        deck.set_cell_border(slide, table, 0, 0, edge, &rule)
            .expect("a border");
    }

    let mut model = support::model();
    let tree = support::lay_out(&mut model, &mut deck, slide);
    let bands = bands(&tree);
    assert_eq!(bands.len(), 4, "four edges, four bands");

    let distinct: std::collections::BTreeSet<(i64, i64, i64, i64)> = bands
        .iter()
        .map(|band| {
            (
                band.left.emu(),
                band.top.emu(),
                band.right.emu(),
                band.bottom.emu(),
            )
        })
        .collect();
    assert_eq!(
        distinct.len(),
        4,
        "the four bands are at four different places; {} distinct rectangles means two edges were \
         computed from the same corner",
        distinct.len()
    );

    // Two horizontal and two vertical, which is what says left/right and top/bottom were not
    // swapped wholesale.
    let horizontal = bands
        .iter()
        .filter(|band| band.width() > band.height())
        .count();
    assert_eq!(
        horizontal, 2,
        "the top and bottom edges are wide and short; {horizontal} of the four are"
    );
}

#[test]
fn a_border_with_no_fill_draws_no_band() {
    // A `a:lnL` that states a width and no fill outlines nothing, and a band of no colour would be
    // a draw call covering pixels with nothing in them.
    let (mut deck, slide) = support::blank_deck();
    let table = deck
        .add_table(slide, 2, 2, ShapeBounds::from_inches(1.0, 1.0, 4.0, 2.0))
        .expect("a table");
    deck.set_cell_border(
        slide,
        table,
        0,
        0,
        CellBorder::Top,
        &LineSpec {
            width: Some(LineWidth::from_points(2.0)),
            ..LineSpec::new()
        },
    )
    .expect("a border with no fill");

    let mut model = support::model();
    let tree = support::lay_out(&mut model, &mut deck, slide);
    assert!(
        bands(&tree).is_empty(),
        "a border that fills with nothing must not become a band"
    );
}
