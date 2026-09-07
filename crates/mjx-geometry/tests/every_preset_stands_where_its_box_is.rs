//! Where each of the 186 presets actually lands, measured — and the two lists that make the
//! measurement falsifiable.
//!
//! # Why this is not "every shape fits in its box"
//!
//! It would be a pleasing sentence and it is **false**, twice over.
//!
//! * A callout's tail is *supposed* to leave the box. `callout1`'s second adjustment defaults to
//!   `-8333`, a negative fraction of the width, and the file's own adjust handles give these shapes
//!   a domain of ±2 147 483 647 — the schema's way of saying *unbounded*. Eighteen presets reach
//!   outside their box at their default adjustments and every one of them is a callout, a wedge, a
//!   `cloud` or a `heart`.
//! * Not every shape *reaches* its box either. `arc` at its defaults sweeps a quarter of an
//!   ellipse; `chord`, `blockArc`, the three circular arrows and the six `math*` symbols draw
//!   inside a box they never touch. Forty presets are in that position.
//!
//! So the strong statement available is a **census**: the shapes that fill their box do so within
//! [`BOX_TOLERANCE_PIXELS`], the shapes that do not are exactly the forty named below, and the
//! shapes that reach outside are exactly the eighteen. A transcription slip in any shape moves it
//! between the lists, or moves it further out than the bound, and fails here — which is what
//! MJXOFF-201 §6 asks a structural check to do without a reference.
//!
//! # The measurement is of the curve, not of the hull
//!
//! MJXOFF-202 §7: a Bézier lies inside its control polygon and not on it, so an arc split at angles
//! that are not quadrant boundaries has control points a few per cent past the ellipse while every
//! point it draws is inside. The seeded six were all quadrant-aligned; **these 186 are not**, so
//! everything here measures [`common::flattened`] points and never control points. `heart` is the
//! shape that shows why: its control points sit 83 device pixels outside a 160-pixel box and its
//! curve reaches 0.57.

mod common;

use std::collections::BTreeSet;

use common::{box_on_the_page, curve_bounds, extents_of_the_box};
use mjx_geometry::{preset_outline, seeded_shapes, PresetShapeType};
use mjx_scene::SceneRect;

/// How far a resolved outline's bounding box may sit from the shape's own box before the shape
/// counts as *not* filling it.
///
/// The same number, for the same reasons, as `a_preset_renders_as_itself.rs`'s own
/// `BOX_TOLERANCE_PIXELS`: half an EMU of coordinate rounding plus one `f32` narrowing, a hundred
/// times over. Restated here rather than shared because the two suites measure different things —
/// that one measures *whether a shape is its box*, this one *which shapes are*.
const BOX_TOLERANCE_PIXELS: f32 = 0.01;

/// How far outside its own box any preset may reach at its default adjustments.
///
/// The bound is set by `callout2`, whose tail reaches 74.667 device pixels past a 160 × 120 box —
/// which is `adj4`'s default of `-38333`, or 38.3 % of the width, plus the tail's own length. It is
/// stated to two decimal places above the measured worst case rather than rounded up to a
/// comfortable number, so a shape that grew a tail it should not have would fail rather than fit.
const FURTHEST_ANY_SHAPE_REACHES_OUTSIDE: f32 = 74.7;

/// The presets whose resolved outline does **not** fill the box the seam gave them, at their
/// default adjustments.
///
/// Every one is explicable from what the shape is, and they group into five kinds:
///
/// * **Partial sweeps** — `arc`, `chord`, `blockArc`, `circularArrow`, `leftCircularArrow`,
///   `leftRightCircularArrow` draw part of an ellipse and stop.
/// * **Callouts** — the twelve `*Callout*` shapes and the three `wedge*Callout`s put their body
///   inside an inset rectangle and their tail outside it, so neither edge is the box's.
/// * **Symbols** — `mathPlus`, `mathMinus`, `mathMultiply`, `mathDivide`, `mathEqual`,
///   `mathNotEqual` are glyph-like marks centred in a box they do not fill.
/// * **Shapes with an inset envelope** — `gear6`, `gear9`, the four `curved*Arrow`s, `cloud`,
///   `wave`, `doubleWave`, `heart`, `flowChartDocument`, `flowChartMultidocument`.
///
/// The list is sorted by wire token, which is the order a failure prints in.
const DOES_NOT_FILL_ITS_BOX: &[PresetShapeType] = &[
    PresetShapeType::AccentBorderCallout1,
    PresetShapeType::AccentBorderCallout2,
    PresetShapeType::AccentBorderCallout3,
    PresetShapeType::AccentCallout1,
    PresetShapeType::AccentCallout2,
    PresetShapeType::AccentCallout3,
    PresetShapeType::Arc,
    PresetShapeType::BlockArc,
    PresetShapeType::BorderCallout1,
    PresetShapeType::BorderCallout2,
    PresetShapeType::BorderCallout3,
    PresetShapeType::Callout1,
    PresetShapeType::Callout2,
    PresetShapeType::Callout3,
    PresetShapeType::Chord,
    PresetShapeType::CircularArrow,
    PresetShapeType::Cloud,
    PresetShapeType::CloudCallout,
    PresetShapeType::CurvedDownArrow,
    PresetShapeType::CurvedLeftArrow,
    PresetShapeType::CurvedRightArrow,
    PresetShapeType::CurvedUpArrow,
    PresetShapeType::DoubleWave,
    PresetShapeType::FlowChartDocument,
    PresetShapeType::FlowChartMultidocument,
    PresetShapeType::Gear6,
    PresetShapeType::Gear9,
    PresetShapeType::Heart,
    PresetShapeType::LeftCircularArrow,
    PresetShapeType::LeftRightCircularArrow,
    PresetShapeType::MathDivide,
    PresetShapeType::MathEqual,
    PresetShapeType::MathMinus,
    PresetShapeType::MathMultiply,
    PresetShapeType::MathNotEqual,
    PresetShapeType::MathPlus,
    PresetShapeType::Wave,
    PresetShapeType::WedgeEllipseCallout,
    PresetShapeType::WedgeRectangleCallout,
    PresetShapeType::WedgeRoundedRectangleCallout,
];

/// The presets that draw **outside** the box the seam gave them, at their default adjustments.
///
/// A subset of [`DOES_NOT_FILL_ITS_BOX`], and every one of them is a shape whose picture is
/// deliberately larger than its frame: a callout's tail, a `cloud`'s puffs, a `heart`'s lobes.
/// Nothing else in the 186 does this, which is worth asserting in both directions: a shape that
/// started spilling would be a scale or a sign error, and it would show up here rather than in a
/// render nobody reads.
const REACHES_OUTSIDE_ITS_BOX: &[PresetShapeType] = &[
    PresetShapeType::AccentBorderCallout1,
    PresetShapeType::AccentBorderCallout2,
    PresetShapeType::AccentBorderCallout3,
    PresetShapeType::AccentCallout1,
    PresetShapeType::AccentCallout2,
    PresetShapeType::AccentCallout3,
    PresetShapeType::BorderCallout1,
    PresetShapeType::BorderCallout2,
    PresetShapeType::BorderCallout3,
    PresetShapeType::Callout1,
    PresetShapeType::Callout2,
    PresetShapeType::Callout3,
    PresetShapeType::Cloud,
    PresetShapeType::CloudCallout,
    PresetShapeType::Heart,
    PresetShapeType::WedgeEllipseCallout,
    PresetShapeType::WedgeRectangleCallout,
    PresetShapeType::WedgeRoundedRectangleCallout,
];

/// How far the two rectangles' corresponding edges are apart at worst.
fn edge_distance(left: SceneRect, right: SceneRect) -> f32 {
    (left.left - right.left)
        .abs()
        .max((left.top - right.top).abs())
        .max((left.right - right.right).abs())
        .max((left.bottom - right.bottom).abs())
}

/// How far outside `box_` the rectangle reaches; zero when it is inside.
fn outside(reached: SceneRect, box_: SceneRect) -> f32 {
    (box_.left - reached.left)
        .max(reached.right - box_.right)
        .max(box_.top - reached.top)
        .max(reached.bottom - box_.bottom)
        .max(0.0)
}

/// The wire tokens of a set of presets, sorted — what a failure message prints.
fn tokens(presets: impl IntoIterator<Item = PresetShapeType>) -> BTreeSet<&'static str> {
    presets.into_iter().map(PresetShapeType::to_wire).collect()
}

#[test]
fn the_shapes_that_fill_their_box_are_exactly_the_ones_that_should() {
    let (within, extents) = (box_on_the_page(), extents_of_the_box());
    let mut measured: BTreeSet<&'static str> = BTreeSet::new();
    let mut worst_filling = 0.0f32;

    for definition in seeded_shapes() {
        let token = definition.preset.to_wire();
        let outline = preset_outline(definition.preset, extents, &[], within)
            .unwrap_or_else(|error| panic!("`{token}` did not resolve: {error}"));
        let distance = edge_distance(curve_bounds(&outline.commands), within);
        if distance > BOX_TOLERANCE_PIXELS {
            measured.insert(token);
        } else {
            worst_filling = worst_filling.max(distance);
        }
    }

    assert_eq!(
        measured,
        tokens(DOES_NOT_FILL_ITS_BOX.iter().copied()),
        "the presets that do not fill their box have changed"
    );
    assert_eq!(
        seeded_shapes().len() - DOES_NOT_FILL_ITS_BOX.len(),
        146,
        "the census no longer adds up to 186"
    );

    // The headroom, asserted rather than hoped for: every shape that *does* fill its box is inside
    // half the tolerance, so the number is not knife-edge.
    println!("worst box error among the shapes that fill their box: {worst_filling} px");
    assert!(
        worst_filling <= BOX_TOLERANCE_PIXELS / 2.0,
        "the worst box-filling shape is {worst_filling} px off, more than half the tolerance — the \
         tolerance is doing no work and should be tightened or the geometry fixed"
    );
}

#[test]
fn the_shapes_that_reach_outside_their_box_are_exactly_the_ones_that_should() {
    let (within, extents) = (box_on_the_page(), extents_of_the_box());
    let mut measured: BTreeSet<&'static str> = BTreeSet::new();
    let mut worst = 0.0f32;

    for definition in seeded_shapes() {
        let token = definition.preset.to_wire();
        let outline = preset_outline(definition.preset, extents, &[], within)
            .unwrap_or_else(|error| panic!("`{token}` did not resolve: {error}"));
        let over = outside(curve_bounds(&outline.commands), within);
        if over > BOX_TOLERANCE_PIXELS {
            measured.insert(token);
            worst = worst.max(over);
        }
        assert!(
            over <= FURTHEST_ANY_SHAPE_REACHES_OUTSIDE,
            "`{token}` reaches {over} device pixels outside a 160 × 120 box, past the \
             {FURTHEST_ANY_SHAPE_REACHES_OUTSIDE} the furthest callout tail reaches"
        );
    }

    assert_eq!(
        measured,
        tokens(REACHES_OUTSIDE_ITS_BOX.iter().copied()),
        "the presets that draw outside their box have changed"
    );
    println!("furthest any shape reaches outside its box: {worst} px");
    // The bound bites: the worst case is within a pixel of it, so it is a measurement and not a
    // comfortable round number a wrong shape could hide inside.
    assert!(
        worst > FURTHEST_ANY_SHAPE_REACHES_OUTSIDE - 1.0,
        "the furthest overhang is {worst}, well inside the {FURTHEST_ANY_SHAPE_REACHES_OUTSIDE} \
         bound — the bound has stopped measuring anything"
    );
}

#[test]
fn the_shapes_that_reach_outside_are_a_subset_of_the_ones_that_do_not_fill() {
    // Not a tautology about the lists — a statement about the geometry, and the one that would
    // catch a list edited to make a failure go away. A shape that reaches outside its box cannot
    // also have its bounding box equal to that box.
    for preset in REACHES_OUTSIDE_ITS_BOX {
        assert!(
            DOES_NOT_FILL_ITS_BOX.contains(preset),
            "`{}` is listed as reaching outside its box but also as filling it exactly",
            preset.to_wire()
        );
    }
}
