//! The text rectangle of all 186 presets, measured — and the four answers that must never look
//! alike.
//!
//! # The defect this suite exists for
//!
//! MJXOFF-201 §6, in this child's terms: **a text rectangle that silently falls back to the
//! bounding box is invisible.** Text still renders, the page still looks like a page, and every
//! test that asks *"did text appear"* passes — while a rounded rectangle's first line sits on its
//! corner arc, a chevron's on its notch and a callout's in its tail. There is no reference render to
//! catch it and no exception to see.
//!
//! So this suite never asks whether there *is* a rectangle. It asks four questions that a
//! bounding-box fallback would answer wrongly:
//!
//! 1. **Is it different from the box?** [`the_shapes_whose_text_rectangle_is_the_whole_box_are_named`]
//!    pins the 45 presets whose `a:rect` genuinely *is* `l t r b` — and therefore the 136 whose is
//!    not. A fallback would move all 181 into the first list, and a shape that quietly joined it
//!    would fail here.
//! 2. **Is it inside the shape?** [`every_text_rectangle_is_inside_the_shapes_own_box`] over all
//!    181, in both orientations and with no exceptions at all, and
//!    [`the_text_rectangles_that_leave_the_outline_are_named`] for the stronger version against the
//!    drawn outline, which has seven or eight explicable exceptions depending on the box.
//! 3. **Is "absent" distinguishable from "unresolvable"?**
//!    [`the_four_answers_are_four_and_not_one`] takes all four arms of
//!    [`TextRectangle`] against real data.
//! 4. **Does the fallback have to be asked for?** [`or_bounding_box`](TextRectangle::or_bounding_box)
//!    is a named call; [`the_fallback_is_a_call_and_not_a_default`] shows the two disagreeing.
//!
//! # Every census is taken in two orientations, and two of them differ
//!
//! `ss` is `min(w, h)`, so in a landscape box the shorter side is always the height — and every box
//! and every non-degenerate extent this crate had was landscape or square. A census taken there
//! alone cannot tell `ss` from `h`, and it turns out it cannot tell two other things either. So
//! everything below is measured in [`common::box_on_the_page`] (160 × 120) **and** in
//! [`common::portrait_box_on_the_page`] (120 × 160), which have the same `ss` and differ only in
//! which side it is.
//!
//! | | |
//! |---|---|
//! | presets with an `a:rect` | **181** of 186 |
//! | presets without one | **5** — `chartPlus`, `chartStar`, `chartX`, `line`, `lineInv` |
//! | edges, and how many are literals | **724**, and **none** |
//! | whose rectangle is the whole box | **45** landscape, **46** portrait — `chevron` joins |
//! | whose rectangle leaves the drawn outline | **8** landscape, **7** portrait — `chord` leaves |
//! | how far it leaves by | at most **37.2** px landscape, **51.3** px portrait |
//! | singular somewhere in their adjustment domain | **3**, all in `il`, in both |
//! | inverted somewhere in their adjustment domain | **2**, both `ellipseRibbon*`, in both |
//!
//! The two differences are ECMA-376's arithmetic and not an inconsistency:
//! [`ALSO_THE_WHOLE_BOX_IN_PORTRAIT`] is a `?:` whose else-arm no landscape box can reach, and
//! [`ALSO_LEAVES_ITS_OUTLINE_IN_LANDSCAPE`] is a corner of an inscribed box that clears a chord in
//! one aspect ratio and not the other. **Neither was visible before this suite had two boxes.**
//!
//! **One of those numbers corrects MJXOFF-204's own brief**, which said four presets are singular
//! in the text rectangle's insets and named `parallelogram` as one. Measured, it is three, and
//! `parallelogram` is not among them: it is singular in `q3`, which its **connection sites** read
//! and its `a:rect` and its paths do not — `a_connector_lands_on_the_outline.rs`'s
//! `SITES_SINGULAR_SOMEWHERE` is where it does appear.
//! [`the_singular_text_rectangles_are_three_and_parallelogram_is_not_one`] is the assertion here.

mod common;

use std::collections::BTreeSet;

use common::{
    box_on_the_page, curve_bounds, extents_of_the_box, portrait_box_on_the_page, portrait_extents,
    reaches_outside,
};
use mjx_geometry::{
    adjustment_domains, preset_outline, preset_text_rectangle, seeded_shapes,
    text_rectangle_of_definition, AdjustmentOverride, Derivation, GeometryError, PathFillMode,
    PresetCoordinate, PresetPath, PresetPathStep, PresetPoint, PresetShapeDefinition,
    PresetShapeType, PresetTextRectangle, Size, TextRectangle,
};
use mjx_ooxml_types::drawingml::PresetGuide;
use mjx_scene::SceneRect;

/// How far a resolved rectangle's edge may sit from where it is expected before it counts as
/// different.
///
/// The same number, for the same reasons, as `every_preset_stands_where_its_box_is.rs`'s: half an
/// EMU of coordinate rounding plus one `f32` narrowing, a hundred times over.
const EDGE_TOLERANCE_PIXELS: f32 = 0.01;

/// The five presets `presetShapeDefinitions.xml` declares no `a:rect` for.
///
/// Three tick marks and two bare lines — none of them a shape text is laid *inside*, which is why
/// the absence is a fact about the shape and not a gap in the extraction.
const NO_TEXT_RECTANGLE: &[PresetShapeType] = &[
    PresetShapeType::ChartPlus,
    PresetShapeType::ChartStar,
    PresetShapeType::ChartX,
    PresetShapeType::StraightLine,
    PresetShapeType::StraightLineInverse,
];

/// The presets whose text rectangle **is** the shape's own box, `l t r b`.
///
/// **This is the list a bounding-box fallback would grow into**, which is the whole reason it is
/// written out rather than counted. Every one of the 45 is explicable:
///
/// * the **twelve callouts** and `wedgeRectCallout` put their text in the body rectangle, which
///   *is* the box — the tail is what leaves it;
/// * the **twelve action buttons** are square faces with a glyph drawn on top, and the glyph is not
///   an inset;
/// * the **nine connectors** have no interior at all, so their `a:rect` is the box by default;
/// * `bentArrow`, `curvedDownArrow`, `curvedLeftArrow`, `curvedRightArrow`, `curvedUpArrow`,
///   `funnel`, `halfFrame`, `swooshArrow` and `uturnArrow` are shapes whose interior is not a
///   rectangle, so the file declines to inset one; and
/// * `rect` and `flowChartProcess` *are* the box.
const TEXT_RECTANGLE_IS_THE_WHOLE_BOX: &[PresetShapeType] = &[
    PresetShapeType::AccentBorderCallout1,
    PresetShapeType::AccentBorderCallout2,
    PresetShapeType::AccentBorderCallout3,
    PresetShapeType::AccentCallout1,
    PresetShapeType::AccentCallout2,
    PresetShapeType::AccentCallout3,
    PresetShapeType::ActionButtonBackPrevious,
    PresetShapeType::ActionButtonBeginning,
    PresetShapeType::ActionButtonBlank,
    PresetShapeType::ActionButtonDocument,
    PresetShapeType::ActionButtonEnd,
    PresetShapeType::ActionButtonForwardNext,
    PresetShapeType::ActionButtonHelp,
    PresetShapeType::ActionButtonHome,
    PresetShapeType::ActionButtonInformation,
    PresetShapeType::ActionButtonMovie,
    PresetShapeType::ActionButtonReturn,
    PresetShapeType::ActionButtonSound,
    PresetShapeType::BentArrow,
    PresetShapeType::BentConnector2,
    PresetShapeType::BentConnector3,
    PresetShapeType::BentConnector4,
    PresetShapeType::BentConnector5,
    PresetShapeType::BorderCallout1,
    PresetShapeType::BorderCallout2,
    PresetShapeType::BorderCallout3,
    PresetShapeType::Callout1,
    PresetShapeType::Callout2,
    PresetShapeType::Callout3,
    PresetShapeType::CurvedConnector2,
    PresetShapeType::CurvedConnector3,
    PresetShapeType::CurvedConnector4,
    PresetShapeType::CurvedConnector5,
    PresetShapeType::CurvedDownArrow,
    PresetShapeType::CurvedLeftArrow,
    PresetShapeType::CurvedRightArrow,
    PresetShapeType::CurvedUpArrow,
    PresetShapeType::FlowChartProcess,
    PresetShapeType::Funnel,
    PresetShapeType::HalfFrame,
    PresetShapeType::Rectangle,
    PresetShapeType::StraightConnector1,
    PresetShapeType::SwooshArrow,
    PresetShapeType::UTurnArrow,
    PresetShapeType::WedgeRectangleCallout,
];

/// The one preset that joins [`TEXT_RECTANGLE_IS_THE_WHOLE_BOX`] when the box is **portrait**.
///
/// **This is the finding the second orientation bought, and it is a branch of ECMA-376's own
/// formula language that no landscape box can take.** `chevron`'s text rectangle is
///
/// ```text
/// <gd name="x1" fmla="*/ ss a 100000" />   <gd name="x2" fmla="+- r 0 x1" />
/// <gd name="dx" fmla="+- x2 0 x1" />
/// <gd name="il" fmla="?: dx x1 l" />       <gd name="ir" fmla="?: dx x2 r" />
/// ```
///
/// `?: a b c` is *"`b` if `a > 0`, else `c`"*, and `dx` is `w - 2·x1` where `x1` is a fraction of
/// **`ss`**. At the default `adj = 50000`, `x1` is half the shorter side, so `dx` is `w - h` when
/// the shape is landscape and `w - w` — exactly zero — when it is portrait or square. **The else
/// arm is therefore unreachable in every landscape box**, and it says something a caller must not
/// get wrong: when the two notches would meet, the shape gives up on insetting and hands text the
/// whole box.
///
/// Before this suite had a portrait box, the crate resolved that `?:` at exactly one value of its
/// condition's sign.
const ALSO_THE_WHOLE_BOX_IN_PORTRAIT: &[PresetShapeType] = &[PresetShapeType::Chevron];

/// The presets whose text rectangle reaches outside the region their own paths **draw**.
///
/// Not a defect, and the reason is the same in all eight: the rectangle is the inscribed box of the
/// shape's *notional* body, and the shape draws a ring or a sliver of that body rather than filling
/// it.
///
/// * the three **circular arrows** draw an annular arrow around a circle and put their text in the
///   middle of the circle, where there is no ink at all — the worst case, 37.1 device pixels in the
///   landscape box; and
/// * the four **curved arrows** overhang by one or two pixels where their curve leaves the box.
///
/// A shape that started doing this would be a scale or a sign error, so the list is asserted in
/// both directions, in both orientations, and bounded by
/// [`FURTHEST_A_RECTANGLE_LEAVES_ITS_OUTLINE`].
const LEAVES_ITS_OWN_OUTLINE: &[PresetShapeType] = &[
    PresetShapeType::CircularArrow,
    PresetShapeType::CurvedDownArrow,
    PresetShapeType::CurvedLeftArrow,
    PresetShapeType::CurvedRightArrow,
    PresetShapeType::CurvedUpArrow,
    PresetShapeType::LeftCircularArrow,
    PresetShapeType::LeftRightCircularArrow,
];

/// The one preset that joins [`LEAVES_ITS_OWN_OUTLINE`] when the box is **landscape**.
///
/// **The second finding a second aspect ratio bought.** `chord` draws the region an ellipse is cut
/// into between 45° and 270°, and insets its text from the *whole* ellipse — `il`/`it`/`ir`/`ib`,
/// the 1/√2 inscribed box. Whether that box's north-east corner pokes out through the chord is a
/// question about the ellipse's aspect ratio, and the two boxes answer it differently: in landscape
/// the corner clears the cut by 8.6 device pixels, in portrait it does not clear it at all.
///
/// Neither is wrong — the shape has one text rectangle and two shapes — and a single-orientation
/// census would have recorded whichever one it happened to take as *the* answer.
const ALSO_LEAVES_ITS_OUTLINE_IN_LANDSCAPE: &[PresetShapeType] = &[PresetShapeType::Chord];

/// How far outside its own drawn outline any text rectangle reaches, at default adjustments, in
/// the landscape box and in the portrait one.
///
/// Both are set by the three circular arrows — the radius of the hole their arrow runs around — and
/// the two differ because the hole is a fraction of the shape and the two shapes are not the same
/// shape. **Stated as a pair rather than as one number bounding both**, because a single bound
/// large enough for the portrait case would stop measuring anything in the landscape one, and
/// [`the_text_rectangles_that_leave_the_outline_are_named`] asserts each is within a pixel of its
/// own worst case.
const FURTHEST_A_RECTANGLE_LEAVES_ITS_OUTLINE: [(&str, f32); 2] =
    [("landscape", 37.2), ("portrait", 51.3)];

/// The presets whose `a:rect` has no value somewhere in their own adjustment domain.
///
/// All three lose `il`, and all three at `adj2 = 0`, which is that adjustment's own **minimum** and
/// therefore a value a handle drag reaches. Their paths draw perfectly well there — which is the
/// whole reason `mjx-geometry`'s resolver evaluates the `gdLst` one guide at a time instead of
/// refusing the shape.
const SINGULAR_TEXT_RECTANGLE: &[PresetShapeType] = &[
    PresetShapeType::LeftRightUpArrow,
    PresetShapeType::LeftUpArrow,
    PresetShapeType::QuadArrow,
];

/// The presets whose `a:rect` **crosses** somewhere in their own adjustment domain.
///
/// Both ribbons, both at `adj1 = 100000` — the top of that adjustment's own range, where the
/// ribbon's body has been squeezed to nothing. The top edge resolves to `q1 = h·a1/100000`, which
/// at `a1 = 100000` is the shape's own bottom, while the bottom edge resolves above it. There is
/// genuinely no text area there, and [`TextRectangle::Inverted`] is the answer that says so instead
/// of a plausible-looking small box.
const INVERTED_SOMEWHERE: &[PresetShapeType] = &[
    PresetShapeType::EllipseRibbon,
    PresetShapeType::EllipseRibbon2,
];

/// The wire tokens of a set of presets, sorted — what a failure message prints.
fn tokens(presets: impl IntoIterator<Item = PresetShapeType>) -> BTreeSet<&'static str> {
    presets.into_iter().map(PresetShapeType::to_wire).collect()
}

/// The two orientations every census below is taken in, labelled for a failure message.
///
/// **Two, and not one, because `ss` is `min(w, h)`.** A great many preset guides — including the
/// corner radius a rounded rectangle's text inset is a fraction of — are written in the shorter
/// side, and in a landscape box the shorter side is always the height, so a census taken only there
/// cannot tell `ss` from `h`. [`the_shorter_side_is_the_shorter_side_in_both_orientations`] is the
/// probe that pins it; these are the censuses that would otherwise all have been taken at one value
/// of one thing.
fn orientations() -> [(&'static str, SceneRect, Size); 2] {
    [
        ("landscape", box_on_the_page(), extents_of_the_box()),
        ("portrait", portrait_box_on_the_page(), portrait_extents()),
    ]
}

/// The text rectangle of `preset` in the landscape box.
fn rectangle_of(preset: PresetShapeType, adjustments: &[AdjustmentOverride]) -> TextRectangle {
    rectangle_in(box_on_the_page(), extents_of_the_box(), preset, adjustments)
}

/// The text rectangle of `preset` in a stated box.
fn rectangle_in(
    within: SceneRect,
    extents: Size,
    preset: PresetShapeType,
    adjustments: &[AdjustmentOverride],
) -> TextRectangle {
    preset_text_rectangle(preset, extents, adjustments, within)
        .unwrap_or_else(|error| panic!("`{}` has no text rectangle: {error}", preset.to_wire()))
}

/// Whether two rectangles' corresponding edges are all within [`EDGE_TOLERANCE_PIXELS`].
fn same_rectangle(left: SceneRect, right: SceneRect) -> bool {
    (left.left - right.left)
        .abs()
        .max((left.top - right.top).abs())
        .max((left.right - right.right).abs())
        .max((left.bottom - right.bottom).abs())
        <= EDGE_TOLERANCE_PIXELS
}

// -------------------------------------------------------------------------------------------
// The census
// -------------------------------------------------------------------------------------------

#[test]
fn the_table_carries_a_text_rectangle_for_181_of_the_186() {
    let mut without: BTreeSet<&'static str> = BTreeSet::new();
    let (mut with, mut edges, mut literals) = (0usize, 0usize, 0usize);
    for definition in seeded_shapes() {
        match definition.text_rectangle {
            None => {
                without.insert(definition.preset.to_wire());
            }
            Some(rectangle) => {
                with += 1;
                for edge in [
                    rectangle.left,
                    rectangle.top,
                    rectangle.right,
                    rectangle.bottom,
                ] {
                    edges += 1;
                    if matches!(edge, PresetCoordinate::Emu(_)) {
                        literals += 1;
                    }
                }
            }
        }
    }
    assert_eq!(with, 181, "the table no longer holds 181 text rectangles");
    assert_eq!(
        without,
        tokens(NO_TEXT_RECTANGLE.iter().copied()),
        "the presets with no `a:rect` have changed"
    );
    assert_eq!(with + without.len(), 186, "the census no longer adds up");
    assert_eq!(edges, 724, "181 rectangles no longer have 724 edges");
    // **Asserted rather than assumed.** `PresetCoordinate` models a literal arm because
    // `ST_AdjCoordinate` has one; ECMA-376's own geometry file never uses it for a text rectangle
    // edge, and an edition that started would be a change this notices rather than absorbs.
    assert_eq!(
        literals, 0,
        "{literals} of the 724 text-rectangle edges are literals; every one used to be a guide name"
    );
    println!(
        "text rectangles: {with} declared, {} absent, {edges} edges, all guide names",
        without.len()
    );
}

// -------------------------------------------------------------------------------------------
// Is it inside the shape?
// -------------------------------------------------------------------------------------------

#[test]
fn every_text_rectangle_is_inside_the_shapes_own_box() {
    // The universal statement, and the one a text layout depends on: text never starts outside the
    // frame the shape was given. All 181, no exceptions — unlike the outline itself, which
    // eighteen presets deliberately leave (a callout's tail, a `cloud`'s puffs).
    //
    // In **both** orientations, because "no exceptions" taken at one aspect ratio is a statement
    // about one aspect ratio.
    for (orientation, within, extents) in orientations() {
        let mut worst = 0.0f32;
        let mut worst_shape = "";
        for definition in seeded_shapes() {
            let token = definition.preset.to_wire();
            let TextRectangle::Declared(rectangle) =
                rectangle_in(within, extents, definition.preset, &[])
            else {
                continue;
            };
            let over = reaches_outside(rectangle, within);
            assert!(
                over <= EDGE_TOLERANCE_PIXELS,
                "in the {orientation} box, `{token}`'s text rectangle {rectangle:?} reaches {over} \
                 device pixels outside the box {within:?} the shape was drawn in"
            );
            if over > worst {
                worst = over;
                worst_shape = token;
            }
        }
        println!(
            "{orientation}: furthest any text rectangle leaves its box: {worst} px ({worst_shape:?})"
        );
    }
}

#[test]
fn the_text_rectangles_that_leave_the_outline_are_named() {
    // The stronger question, against the region the shape's paths actually *draw* rather than
    // against the box. It has eight exceptions and they are named, because "mostly inside" is not
    // a statement a regression could fail.
    //
    // **The membership is asserted in both orientations; the numeric bound only in the landscape
    // one.** Which shapes hide their text in a hole they do not fill is a property of the shape,
    // and it must not change when the box is turned on its side. *How far* the hole reaches is a
    // property of the box, and stating a second number for the portrait case would be two bounds
    // pretending to be one measurement.
    for (index, (orientation, within, extents)) in orientations().into_iter().enumerate() {
        let (bounded_orientation, bound) = FURTHEST_A_RECTANGLE_LEAVES_ITS_OUTLINE[index];
        assert_eq!(
            bounded_orientation, orientation,
            "the bounds are out of step"
        );
        let mut measured: BTreeSet<&'static str> = BTreeSet::new();
        let mut worst = 0.0f32;
        let mut worst_inside = 0.0f32;
        for definition in seeded_shapes() {
            let token = definition.preset.to_wire();
            let TextRectangle::Declared(rectangle) =
                rectangle_in(within, extents, definition.preset, &[])
            else {
                continue;
            };
            let outline = preset_outline(definition.preset, extents, &[], within)
                .unwrap_or_else(|error| panic!("`{token}` did not resolve: {error}"));
            // The *curve* bounds, not the control-point hull: a Bézier lies inside its control
            // polygon, so the hull can sit several per cent outside the shape and would make this
            // question easier than it is.
            let over = reaches_outside(rectangle, curve_bounds(&outline.commands));
            if over > EDGE_TOLERANCE_PIXELS {
                measured.insert(token);
                worst = worst.max(over);
            } else {
                worst_inside = worst_inside.max(over);
            }
            assert!(
                over <= bound,
                "in the {orientation} box, `{token}`'s text rectangle reaches {over} device pixels \
                 outside what the shape draws, past the {bound} the circular arrows' hole accounts \
                 for"
            );
        }
        let expected: BTreeSet<&'static str> = if orientation == "landscape" {
            tokens(
                LEAVES_ITS_OWN_OUTLINE
                    .iter()
                    .chain(ALSO_LEAVES_ITS_OUTLINE_IN_LANDSCAPE)
                    .copied(),
            )
        } else {
            tokens(LEAVES_ITS_OWN_OUTLINE.iter().copied())
        };
        assert_eq!(
            measured, expected,
            "in the {orientation} box, the presets whose text rectangle leaves their own outline \
             have changed"
        );
        println!(
            "{orientation}: furthest a text rectangle leaves its outline: {worst} px (bound \
             {bound}); worst of the other 173: {worst_inside} px"
        );
        assert!(
            worst_inside <= EDGE_TOLERANCE_PIXELS / 2.0,
            "in the {orientation} box the worst of the shapes that stay inside is {worst_inside} \
             px out, more than half the tolerance — the tolerance is doing the work the geometry \
             should"
        );
        // Each bound bites in its own orientation: the worst case is within a pixel of it, so it is
        // a measurement and not a comfortable round number a wrong shape could hide inside.
        assert!(
            worst > bound - 1.0,
            "in the {orientation} box the furthest overhang is {worst}, well inside the {bound} \
             bound — the bound measures nothing"
        );
    }
}

// -------------------------------------------------------------------------------------------
// Is it different from the box?
// -------------------------------------------------------------------------------------------

#[test]
fn the_shapes_whose_text_rectangle_is_the_whole_box_are_named() {
    // **The gate against the invisible defect.** A resolver that lost the `a:rect` and answered
    // with the shape's box would put all 181 presets into `measured`; one that lost it for a single
    // shape would put one extra in. Either fails here, by name.
    //
    // **Asserted in both orientations, and they differ by exactly one shape.** Membership is very
    // nearly a property of the shape — but `chevron`'s inset is guarded by a `?:` whose condition
    // is `w - 2·x1` with `x1` a fraction of `ss`, so it collapses to the whole box in a portrait
    // box and not in a landscape one. See [`ALSO_THE_WHOLE_BOX_IN_PORTRAIT`]. Discovering that is
    // what a second aspect ratio is for.
    for (orientation, within, extents) in orientations() {
        let expected: BTreeSet<&'static str> = if orientation == "portrait" {
            tokens(
                TEXT_RECTANGLE_IS_THE_WHOLE_BOX
                    .iter()
                    .chain(ALSO_THE_WHOLE_BOX_IN_PORTRAIT)
                    .copied(),
            )
        } else {
            tokens(TEXT_RECTANGLE_IS_THE_WHOLE_BOX.iter().copied())
        };
        let mut measured: BTreeSet<&'static str> = BTreeSet::new();
        let mut inset = 0usize;
        for definition in seeded_shapes() {
            let TextRectangle::Declared(rectangle) =
                rectangle_in(within, extents, definition.preset, &[])
            else {
                continue;
            };
            if same_rectangle(rectangle, within) {
                measured.insert(definition.preset.to_wire());
            } else {
                inset += 1;
            }
        }
        assert_eq!(
            measured, expected,
            "in the {orientation} box, the presets whose text rectangle is the whole box have \
             changed"
        );
        assert_eq!(
            inset,
            181 - expected.len(),
            "in the {orientation} box, the inset count no longer complements the census"
        );
        println!(
            "{orientation}: {} presets lay text in the whole box, {inset} inset it",
            measured.len()
        );
    }

    // …and the difference is asserted in both directions, so that a `chevron` which stopped
    // collapsing — or one that started collapsing in landscape too — fails rather than passing by
    // being in a list somewhere.
    for preset in ALSO_THE_WHOLE_BOX_IN_PORTRAIT {
        assert!(
            !TEXT_RECTANGLE_IS_THE_WHOLE_BOX.contains(preset),
            "`{}` is listed as collapsing only in portrait and also as collapsing always",
            preset.to_wire()
        );
    }
}

#[test]
fn the_shorter_side_is_the_shorter_side_in_both_orientations() {
    // **The `ss` ↔ `h` probe.** `roundRect`'s text inset is `il = x1 · 29289/100000` where
    // `x1 = ss · adj/100000`, and `ss` is `min(w, h)`. Both boxes below are 120 points on their
    // shorter side and 160 on their longer, so **`ss` is 120 in both** and the inset must be the
    // same number in both — 120 · 0.16667 · 0.29289 = 5.858 device pixels.
    //
    // That single pair of numbers separates three implementations that agree everywhere else this
    // crate measures:
    //
    // | reads | landscape (160 × 120) | portrait (120 × 160) |
    // |---|---|---|
    // | `ss`, correctly | **5.858** | **5.858** |
    // | `h` | 5.858 | 7.811 |
    // | `w` | 7.811 | 5.858 |
    //
    // Before this test the crate had no portrait box at all outside its degenerate sweep, so the
    // middle row was indistinguishable from the first in every gate it has.
    const EXPECTED_INSET_PIXELS: f32 = 5.858;
    const WHAT_A_LONGER_SIDE_WOULD_GIVE: f32 = 7.811;

    for (orientation, within, extents) in orientations() {
        let TextRectangle::Declared(rectangle) =
            rectangle_in(within, extents, PresetShapeType::RoundedRectangle, &[])
        else {
            panic!("`roundRect` has no text rectangle in the {orientation} box");
        };
        // Both axes: the inset is a length in the shape's own space, so it is the same number
        // horizontally and vertically, and a resolver that scaled it by the wrong axis would differ
        // on one of the two.
        for (edge, measured) in [
            ("left", rectangle.left - within.left),
            ("top", rectangle.top - within.top),
            ("right", within.right - rectangle.right),
            ("bottom", within.bottom - rectangle.bottom),
        ] {
            println!("{orientation:>10} {edge:>6}: {measured:.4} px");
            assert!(
                (measured - EXPECTED_INSET_PIXELS).abs() <= 0.001,
                "in the {orientation} box, `roundRect`'s {edge} inset is {measured} px, not the \
                 {EXPECTED_INSET_PIXELS} that `ss = min(w, h) = 120` gives. \
                 {WHAT_A_LONGER_SIDE_WOULD_GIVE} would mean the shorter side was read as the \
                 {}.",
                if orientation == "landscape" {
                    "width"
                } else {
                    "height"
                }
            );
        }
    }
}

#[test]
fn the_shapes_that_must_inset_their_text_do_inset_it_by_the_amount_they_should() {
    // Named shapes at measured values, so that "different from the box" cannot be satisfied by a
    // rectangle that is different *and wrong*. Each number is the largest gap between the
    // rectangle's edge and the box's, on a 160 × 120-pixel box at default adjustments.
    //
    // `roundRect`'s is the derivation worth reading: its inset is `il = x1 · 29289/100000` where
    // `x1 = ss · adj/100000` — 29.289 % of the corner radius, which is `1 - 1/√2`, the distance
    // from a quarter-circle's corner to the arc. 120 · 0.16667 · 0.29289 = 5.858.
    let cases: &[(PresetShapeType, f32)] = &[
        (PresetShapeType::RoundedRectangle, 5.858),
        (PresetShapeType::Chevron, 60.0),
        (PresetShapeType::Triangle, 60.0),
        (PresetShapeType::Diamond, 40.0),
        (PresetShapeType::Hexagon, 23.333),
        (PresetShapeType::Octagon, 17.573),
        (PresetShapeType::Plaque, 14.143),
    ];
    let within = box_on_the_page();
    for (preset, expected) in cases {
        let token = preset.to_wire();
        let TextRectangle::Declared(rectangle) = rectangle_of(*preset, &[]) else {
            panic!("`{token}` has no text rectangle to measure");
        };
        let inset = (rectangle.left - within.left)
            .abs()
            .max((rectangle.top - within.top).abs())
            .max((rectangle.right - within.right).abs())
            .max((rectangle.bottom - within.bottom).abs());
        println!("{token:>14} insets its text by {inset:.3} px (expected {expected})");
        assert!(
            (inset - expected).abs() <= 0.001,
            "`{token}` insets its text by {inset} px, not the {expected} the shape's own guides give"
        );
    }
}

// -------------------------------------------------------------------------------------------
// Are the four answers four?
// -------------------------------------------------------------------------------------------

#[test]
fn the_four_answers_are_four_and_not_one() {
    // Every arm of `TextRectangle`, from **real** table data at a **real** adjustment value, and
    // each with a consequence that separates it from the others. This is the assertion MJXOFF-204
    // asks for in as many words: absent, singular and broken must not look the same to a caller.
    let declared = rectangle_of(PresetShapeType::RoundedRectangle, &[]);
    let absent = rectangle_of(PresetShapeType::StraightLine, &[]);
    let singular = rectangle_of(
        PresetShapeType::LeftRightUpArrow,
        &[AdjustmentOverride::new("adj2", 0.0)],
    );
    let inverted = rectangle_of(
        PresetShapeType::EllipseRibbon,
        &[AdjustmentOverride::new("adj1", 100_000.0)],
    );

    assert!(matches!(declared, TextRectangle::Declared(_)));
    assert_eq!(absent, TextRectangle::NotDeclared);
    assert!(
        matches!(&singular, TextRectangle::Singular { guide } if guide == "il"),
        "`leftRightUpArrow` at `adj2 = 0` answered {singular:?}, not a singular `il`"
    );
    assert!(matches!(inverted, TextRectangle::Inverted { .. }));

    // Four values, four answers, and no two equal — including the two that are "there is no
    // rectangle" for different reasons.
    let answers = [&declared, &absent, &singular, &inverted];
    for (index, left) in answers.iter().enumerate() {
        for right in &answers[index + 1..] {
            assert_ne!(left, right, "two of the four answers are the same value");
        }
    }

    // The consequence, which is the half that matters: only one of the four has a rectangle, and
    // the other three are three different *reasons* rather than one silence.
    assert!(declared.declared().is_some());
    assert_eq!(absent.declared(), None);
    assert_eq!(singular.declared(), None);
    assert_eq!(inverted.declared(), None);

    // …and an inverted answer keeps the crossed edges rather than throwing them away, which is what
    // let this suite recognise `ellipseRibbon`'s squeezed body as a shape at the end of its range
    // and `pie`'s transposed `a:rect` as a defect in the file.
    let TextRectangle::Inverted { crossed } = inverted else {
        unreachable!("checked above")
    };
    assert!(
        crossed.width() > 0.0,
        "an inverted answer threw away where the edges went"
    );
    println!("the four answers: {declared:?} / {absent:?} / {singular:?} / {inverted:?}");
}

#[test]
fn the_fallback_is_a_call_and_not_a_default() {
    // `or_bounding_box` is the *policy*, and the policy is right — a text layout has no other box.
    // What must not happen is the resolver applying it silently, so the two are shown disagreeing:
    // for a shape with a rectangle they differ, and for the three ways of having none they agree
    // with the box and `declared()` still says there was nothing.
    let within = box_on_the_page();

    let roundrect = rectangle_of(PresetShapeType::RoundedRectangle, &[]);
    assert!(
        !same_rectangle(roundrect.or_bounding_box(within), within),
        "`roundRect`'s fallback equals its box, so the rectangle was lost"
    );

    for answer in [
        rectangle_of(PresetShapeType::StraightLine, &[]),
        rectangle_of(
            PresetShapeType::LeftRightUpArrow,
            &[AdjustmentOverride::new("adj2", 0.0)],
        ),
        rectangle_of(
            PresetShapeType::EllipseRibbon,
            &[AdjustmentOverride::new("adj1", 100_000.0)],
        ),
    ] {
        assert_eq!(
            answer.or_bounding_box(within),
            within,
            "{answer:?} did not fall back to the box when asked"
        );
        assert_eq!(
            answer.declared(),
            None,
            "{answer:?} reports a rectangle it does not have"
        );
    }
}

#[test]
fn the_singular_text_rectangles_are_three_and_parallelogram_is_not_one() {
    // MJXOFF-204's brief says four presets are singular in `il`/`it`/`ir`/`ib` and names
    // `parallelogram` among them. Measured across every adjustment's whole domain, it is three, and
    // `parallelogram` is not one of them: its singular guide is `q3`, which its **connection
    // sites** read and neither its paths nor its `a:rect` do. A singularity belongs to whichever
    // consumer reads the guide, which is exactly what evaluating the `gdLst` one guide at a time
    // buys — and it is why the three lists (paths, text rectangle, connection sites) are three
    // different lists rather than one.
    let mut cases = 0usize;
    for (orientation, within, extents) in orientations() {
        let mut singular: BTreeSet<&'static str> = BTreeSet::new();
        let mut inverted: BTreeSet<&'static str> = BTreeSet::new();
        let mut guides: BTreeSet<String> = BTreeSet::new();

        for definition in seeded_shapes() {
            let preset = definition.preset;
            let token = preset.to_wire();
            let mut sets: Vec<Vec<AdjustmentOverride>> = vec![Vec::new()];
            if let Ok(domains) = adjustment_domains(preset, extents, &[]) {
                for domain in &domains {
                    // Both ends of the domain, its middle, and values well outside it — the shape's
                    // own `pin` is what is meant to bring the last two back.
                    for value in [
                        domain.minimum,
                        domain.maximum,
                        (domain.minimum + domain.maximum) / 2.0,
                        -1_000_000.0,
                        1_000_000.0,
                    ] {
                        sets.push(vec![AdjustmentOverride::new(domain.spec.wire_name, value)]);
                    }
                }
            }
            for adjustments in &sets {
                cases += 1;
                match preset_text_rectangle(preset, extents, adjustments, within) {
                    Ok(TextRectangle::Singular { guide }) => {
                        singular.insert(token);
                        guides.insert(guide);
                    }
                    Ok(TextRectangle::Inverted { .. }) => {
                        inverted.insert(token);
                    }
                    Ok(TextRectangle::Declared(rectangle)) => assert!(
                        rectangle.left.is_finite()
                            && rectangle.top.is_finite()
                            && rectangle.right.is_finite()
                            && rectangle.bottom.is_finite(),
                        "`{token}` produced {rectangle:?} in the {orientation} box"
                    ),
                    Ok(TextRectangle::NotDeclared) => {}
                    Err(error) => panic!("`{token}` at {adjustments:?} ({orientation}): {error}"),
                }
            }
        }

        assert_eq!(
            singular,
            tokens(SINGULAR_TEXT_RECTANGLE.iter().copied()),
            "in the {orientation} box, the presets whose text rectangle has no value somewhere have \
             changed"
        );
        assert_eq!(
            guides,
            BTreeSet::from(["il".to_owned()]),
            "in the {orientation} box, the singular text rectangles are no longer singular only in \
             `il`"
        );
        assert_eq!(
            inverted,
            tokens(INVERTED_SOMEWHERE.iter().copied()),
            "in the {orientation} box, the presets whose text rectangle crosses somewhere have \
             changed"
        );
        assert!(
            !singular.contains("parallelogram"),
            "`parallelogram`'s text rectangle is singular after all, which the brief said and the \
             measurement denied"
        );
        println!(
            "{orientation}: {} singular ({guides:?}), {} inverted",
            singular.len(),
            inverted.len()
        );
    }
    assert!(
        cases > 3_000,
        "the adjustment sweep visited only {cases} cases across the two orientations"
    );
    println!("sweep: {cases} cases across two orientations");
}

// -------------------------------------------------------------------------------------------
// A defect is a defect, and not a missing rectangle
// -------------------------------------------------------------------------------------------

/// A shape whose `a:rect` names a guide it never defines — the one text-rectangle failure that is a
/// table defect rather than an answer.
const A_RECTANGLE_NAMING_A_GUIDE_NOBODY_DEFINES: PresetShapeDefinition = PresetShapeDefinition {
    preset: PresetShapeType::Rectangle,
    derivation: Derivation::FromFirstPrinciples,
    source: "a unit square whose text rectangle names a guide that is not in its guide list",
    adjustment_values: &[],
    guides: &[PresetGuide {
        wire_name: "inset",
        formula: "*/ w 1 10",
    }],
    text_rectangle: Some(PresetTextRectangle {
        left: PresetCoordinate::Guide("inset"),
        top: PresetCoordinate::Guide("inset"),
        right: PresetCoordinate::Guide("nothing_defines_this"),
        bottom: PresetCoordinate::Guide("inset"),
    }),
    connection_sites: &[],
    paths: &[PresetPath {
        width: None,
        height: None,
        fill: PathFillMode::Normal,
        stroke: true,
        extrusion_ok: true,
        steps: &[
            PresetPathStep::MoveTo(PresetPoint::at("l", "t")),
            PresetPathStep::LineTo(PresetPoint::at("r", "b")),
            PresetPathStep::Close,
        ],
    }],
};

/// The same shape with a **transposed** rectangle — the defect `pie` has in ECMA-376's own file,
/// reproduced so that the arm which catches it is shown to catch it.
const A_TRANSPOSED_RECTANGLE: PresetShapeDefinition = PresetShapeDefinition {
    text_rectangle: Some(PresetTextRectangle {
        left: PresetCoordinate::Guide("far"),
        top: PresetCoordinate::Guide("near"),
        right: PresetCoordinate::Guide("near"),
        bottom: PresetCoordinate::Guide("far"),
    }),
    guides: &[
        PresetGuide {
            wire_name: "near",
            formula: "*/ w 1 10",
        },
        PresetGuide {
            wire_name: "far",
            formula: "*/ w 9 10",
        },
    ],
    ..A_RECTANGLE_NAMING_A_GUIDE_NOBODY_DEFINES
};

#[test]
fn a_rectangle_naming_a_guide_the_shape_lacks_is_a_defect_and_not_an_answer() {
    let (within, extents) = (box_on_the_page(), extents_of_the_box());
    let error = text_rectangle_of_definition(
        &A_RECTANGLE_NAMING_A_GUIDE_NOBODY_DEFINES,
        extents,
        &[],
        within,
    )
    .expect_err("a rectangle naming an undefined guide is a table defect");
    assert!(
        matches!(error, GeometryError::TextRectangle { .. }),
        "an undefined guide in an `a:rect` answered with {error}, not a text-rectangle failure"
    );
    assert_eq!(error.shape(), Some("rect"));
    // **And it is not fillable by a stand-in.** A rectangle the table got wrong must not be papered
    // over the way a shape with no geometry at these adjustments may be.
    assert!(
        !error.has_no_geometry_to_draw(),
        "a table defect was classified as `there is nothing to draw`"
    );
    println!("{error}");
}

#[test]
fn a_transposed_rectangle_is_reported_and_not_quietly_put_back_in_order() {
    // `SceneRect::new` normalises, so a crossed rectangle would arrive on the page looking like a
    // perfectly ordinary small box. This is the arm that refuses to let it, and the one that
    // identified `pie`'s `t="ir" r="it"` in the spec file. `xtask`'s `RECT_ERRATA` corrects that
    // shape; nothing corrects this one, because it is written here to fail.
    let (within, extents) = (box_on_the_page(), extents_of_the_box());
    let answer = text_rectangle_of_definition(&A_TRANSPOSED_RECTANGLE, extents, &[], within)
        .expect("a crossed rectangle resolves; it just is not a rectangle");
    let TextRectangle::Inverted { crossed } = answer else {
        panic!("a rectangle written left=0.9w right=0.1w answered {answer:?}")
    };
    // The normalised box is the *evidence*: 80 % of the width across, exactly where the crossed
    // edges are, which is what a caller printing a diagnosis needs.
    assert!(
        (crossed.width() - 0.8 * within.width()).abs() <= 0.5,
        "the crossed rectangle {crossed:?} is not where the two edges were"
    );
    assert_eq!(answer.declared(), None);
    println!("a transposed rectangle answers {answer:?}");
}

#[test]
fn moving_an_adjustment_moves_the_text_rectangle() {
    // The rectangle is *resolved*, not read off a constant: `roundRect`'s corner radius is an
    // adjustment, its inset is 29.289 % of that radius, and the two move together.
    let at = |adjustment: f64| match rectangle_of(
        PresetShapeType::RoundedRectangle,
        &[AdjustmentOverride::new("adj", adjustment)],
    ) {
        TextRectangle::Declared(rectangle) => rectangle,
        other => panic!("`roundRect` answered {other:?}"),
    };
    let (small, large) = (at(5_000.0), at(50_000.0));
    assert!(
        large.left > small.left + 1.0,
        "a tenfold corner radius moved the text rectangle's left edge from {} to {}",
        small.left,
        large.left
    );
    // …and by the amount the shape's own guides say: `ss · adj/100000 · 29289/100000`, with
    // `ss = 120` device pixels.
    for (adjustment, expected) in [(5_000.0, 1.757), (50_000.0, 17.573)] {
        let measured = at(adjustment).left - box_on_the_page().left;
        assert!(
            (measured - expected).abs() <= 0.001,
            "at `adj = {adjustment}` the inset is {measured} px, not the {expected} the guides give"
        );
    }
}
