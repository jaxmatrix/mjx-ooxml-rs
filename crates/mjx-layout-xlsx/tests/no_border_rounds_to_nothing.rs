//! **A hairline that rounds to nothing is the defect this file exists to refuse.**
//!
//! R15 found `mjx-layout-pptx` flooring a DrawingML hairline at *one EMU* and then halving it to
//! zero: a band whose two edges are the same coordinate, covering no pixels, at every zoom and in
//! every export, with nothing anywhere to notice. The path was unreached, so no test saw it.
//!
//! **Excel is made of hairline borders**, so the same defect here would be worth more than it was
//! there — `thin` is the most common border in every spreadsheet ever written, and `hair` exists
//! precisely to be thinner than that. So this suite walks **every** variant of `ST_BorderStyle`,
//! asserts a strictly positive width, and asserts that the rectangle each one actually produces has
//! strictly positive area on the axis it is thin in.
//!
//! The list is written out rather than iterated over a `Vec` the crate exports, so that a style
//! added to the schema fails to compile here as well as in [`mjx_layout_xlsx::band_width`].
//!
//! The other half of the same question — *does a hairline put ink on a page* — cannot be asked from
//! a ranked crate, because answering it needs a painter and a painter links Vulkan.
//! `crates/mjx-reference-pack/tests/a_real_worksheet_reaches_pixels.rs` asks it there.

use mjx_layout::LayoutRect;
use mjx_layout_xlsx::{band_width, bands, BorderEdge, CellBorders, RegionEdge};
use mjx_ooxml_core::measure::Emu;
use mjx_ooxml_types::spreadsheetml::BorderStyle;

/// Every style a file can state, `none` excepted — which is the one that is *not* drawn.
const DRAWN: &[BorderStyle] = &[
    BorderStyle::Thin,
    BorderStyle::Medium,
    BorderStyle::Dashed,
    BorderStyle::Dotted,
    BorderStyle::Thick,
    BorderStyle::Double,
    BorderStyle::Hair,
    BorderStyle::MediumDashed,
    BorderStyle::DashDot,
    BorderStyle::MediumDashDot,
    BorderStyle::DashDotDot,
    BorderStyle::MediumDashDotDot,
    BorderStyle::SlantDashDot,
];

/// A cell an inch square, which is the shape a band is measured against.
fn cell() -> LayoutRect {
    LayoutRect::from_edges(
        Emu::ZERO,
        Emu::ZERO,
        Emu::from_inches(1.0),
        Emu::from_inches(1.0),
    )
}

/// The exhaustive match in `band_width` is what keeps this list honest: a fourteenth style would
/// fail that function to compile. This asserts the count so that a style added to the schema and
/// given a width still has to be added here.
#[test]
fn every_style_the_schema_declares_is_covered() {
    assert_eq!(
        DRAWN.len(),
        13,
        "`ST_BorderStyle` declares fourteen values and one of them is `none`; if that changed, \
         `band_width`'s match changed too and this list has to follow"
    );
    assert_eq!(
        band_width(BorderStyle::None),
        Emu::ZERO,
        "`none` is the one style that is not drawn, and its zero is a decision rather than an \
         accident"
    );
}

#[test]
fn no_drawn_style_is_zero_wide() {
    for &style in DRAWN {
        let width = band_width(style);
        assert!(
            width > Emu::ZERO,
            "{style:?} is drawn at {} EMU. A band of zero width covers no pixels, which is a \
             border that is not there.",
            width.emu()
        );
        // One EMU is the number R15's defect produced, and it is the number that halves to zero.
        assert!(
            width.emu() > 2,
            "{style:?} is drawn at {} EMU, which is thin enough that halving it rounds to nothing \
             — the exact shape of the defect this suite exists to refuse.",
            width.emu()
        );
    }
}

#[test]
fn every_style_produces_a_band_with_area_on_all_four_edges() {
    for &style in DRAWN {
        for edge in RegionEdge::ALL {
            let mut borders = CellBorders::default();
            borders.set(
                edge,
                Some(BorderEdge {
                    style,
                    colour: None,
                }),
            );
            let drawn = bands(cell(), &borders);
            assert!(
                !drawn.is_empty(),
                "{style:?} on the {edge:?} edge produced no band at all"
            );
            for band in &drawn {
                assert_eq!(band.edge, edge);
                assert!(
                    band.rect.width() > Emu::ZERO && band.rect.height() > Emu::ZERO,
                    "{style:?} on the {edge:?} edge produced [{},{},{},{}], which covers no area",
                    band.rect.left.emu(),
                    band.rect.top.emu(),
                    band.rect.right.emu(),
                    band.rect.bottom.emu()
                );
            }
        }
    }
}

/// A `double` is two lines and a gap, and the gap is what makes it read as double.
#[test]
fn a_double_edge_is_two_separated_lines() {
    let mut borders = CellBorders::default();
    borders.set(
        RegionEdge::Bottom,
        Some(BorderEdge {
            style: BorderStyle::Double,
            colour: None,
        }),
    );
    let drawn = bands(cell(), &borders);
    assert_eq!(drawn.len(), 2, "a `double` edge draws two lines");
    let (first, second) = (&drawn[0].rect, &drawn[1].rect);
    assert!(
        first.bottom < second.top,
        "the two lines of a `double` overlap: [{},{}] and [{},{}]. Two lines with no gap are one \
         thick line.",
        first.top.emu(),
        first.bottom.emu(),
        second.top.emu(),
        second.bottom.emu()
    );
    // Both share the cell's own edge between them, which is what keeps a double border centred
    // where a single one would have been.
    let centre = (first.top.emu() + second.bottom.emu()) / 2;
    assert_eq!(
        centre,
        cell().bottom.emu(),
        "a `double` edge is not centred on the cell's own edge"
    );
}

/// The weights are a ladder rather than a set of unrelated numbers: a reader who states `thick`
/// expects something heavier than `medium`, which is heavier than `thin`, which is heavier than
/// `hair`. Every one of the four is a GUESS (see [`mjx_layout_xlsx::border`]); their *order* is not.
#[test]
fn the_weights_are_ordered_hair_thin_medium_thick() {
    assert!(band_width(BorderStyle::Hair) < band_width(BorderStyle::Thin));
    assert!(band_width(BorderStyle::Thin) < band_width(BorderStyle::Medium));
    assert!(band_width(BorderStyle::Medium) < band_width(BorderStyle::Thick));
    assert_eq!(
        band_width(BorderStyle::Dashed),
        band_width(BorderStyle::Thin),
        "the dashed family is drawn at its own family's weight; only the dash is lost"
    );
    assert_eq!(
        band_width(BorderStyle::MediumDashDotDot),
        band_width(BorderStyle::Medium)
    );
}

/// A cell with no stated border allocates nothing, which matters because most cells have none.
#[test]
fn an_unbordered_cell_produces_no_bands_at_all() {
    assert!(bands(cell(), &CellBorders::default()).is_empty());
}
