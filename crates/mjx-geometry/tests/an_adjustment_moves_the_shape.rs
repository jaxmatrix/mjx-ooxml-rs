//! Sweeping an adjustment moves the geometry the way the shape says it should — and a degenerate
//! size moves nothing at all rather than panicking.
//!
//! # Why this is the sharpest gate in the crate
//!
//! Every static check in `a_preset_renders_as_itself.rs` passes on a shape whose guides are wired
//! to the *wrong variable*. A `triangle` whose apex tracked the height instead of the width still
//! closes, still fills its box and still has three points; so does a `rightArrow` whose two
//! adjustments are swapped. What separates them from the right shape is what happens when the
//! adjustment moves, and nothing but a sweep can see it.
//!
//! So each case below states a direction **in the words the shape's own definition uses** — "adj
//! places the apex horizontally", "adj is the corner radius", "adj1 is the shaft's thickness" — and
//! asserts the geometry moves that way across the adjustment's whole domain, strictly and without
//! ever leaving the box.
//!
//! What it cannot do, stated so nobody over-reads it: if the transcription in `seed.rs` is wrong in
//! the *same* direction as the sweep asserts, both agree and both are wrong. That is what
//! MJXOFF-203's independent extraction is for, and it is why `seed.rs` records how independent each
//! row's derivation really was.

mod common;

use common::{box_on_the_page, curve_overhang, enclosed_area, extents_of_the_box, points_of};
use std::collections::BTreeSet;

use mjx_geometry::{
    adjustment_domains, preset_outline, seeded_shapes, AdjustmentOverride, GeometryError,
    PresetGeometryProvider, PresetShapeType, ShapeOutline, Size, UnknownShapePolicy,
};
use mjx_scene::{GeometryProvider, OutlineProvenance, PathCommand, ScenePoint, SceneRect};

/// How far outside its box a point may lie at any point of a sweep, in device pixels.
///
/// The same number and the same three error sources as `a_preset_renders_as_itself.rs`'s box
/// tolerance: half an EMU of coordinate rounding, one narrowing to `f32`, and nothing from the arc
/// decomposition at a quadrant boundary. It is restated rather than shared because a sweep visits
/// adjustment values the other suite does not, and a tolerance that had to cover both would be the
/// larger of the two with no note saying why.
const OVERHANG_TOLERANCE_PIXELS: f32 = 0.01;

/// How many values a sweep takes across an adjustment's domain.
///
/// Nine rather than two: the endpoints alone would pass on a shape that moved the right way overall
/// and the wrong way in the middle, which is exactly what a `pin` written against the wrong bound
/// does.
const SWEEP_SAMPLES: usize = 9;

/// The values a sweep visits between `from` and `to`, inclusive of both.
fn sweep(from: f64, to: f64) -> Vec<f64> {
    (0..SWEEP_SAMPLES)
        .map(|index| from + (to - from) * index as f64 / (SWEEP_SAMPLES - 1) as f64)
        .collect()
}

/// Resolve one shape at one adjustment value, in the suite's box.
fn at(preset: PresetShapeType, adjustments: &[(&str, f64)]) -> Vec<PathCommand> {
    let overrides: Vec<AdjustmentOverride> = adjustments
        .iter()
        .map(|(name, value)| AdjustmentOverride::new(*name, *value))
        .collect();
    let outline = preset_outline(preset, extents_of_the_box(), &overrides, box_on_the_page())
        .unwrap_or_else(|error| panic!("`{}` did not resolve: {error}", preset.to_wire()));
    // The curve, not the control hull: a sweep visits arc angles that are not quadrant
    // boundaries, and a Bézier's control point legitimately sits outside the shape's box there.
    // `common::curve_overhang` says why at length.
    let outside = curve_overhang(&outline.commands, box_on_the_page());
    assert!(
        outside <= OVERHANG_TOLERANCE_PIXELS,
        "`{}` reaches {outside} device pixels outside its box at {adjustments:?}",
        preset.to_wire()
    );
    outline.commands
}

/// Assert a sequence is strictly increasing (or strictly decreasing), naming where it is not.
fn strictly(direction: &str, what: &str, values: &[f64]) {
    let rising = match direction {
        "rises" => true,
        "falls" => false,
        other => panic!("a sweep's direction is `rises` or `falls`, not `{other}`"),
    };
    for pair in values.windows(2) {
        let moved = if rising {
            pair[1] > pair[0]
        } else {
            pair[1] < pair[0]
        };
        assert!(
            moved,
            "{what} did not go on {direction}: it went {} then {} across {values:?}",
            pair[0], pair[1]
        );
    }
}

/// The first point at the top edge of `within` — a shape's apex, where it has one.
fn apex(commands: &[PathCommand], within: SceneRect) -> ScenePoint {
    points_of(commands)
        .into_iter()
        .min_by(|left, right| left.y.total_cmp(&right.y))
        .unwrap_or_else(|| panic!("a shape with no points cannot have an apex; box {within:?}"))
}

// -------------------------------------------------------------------------------------------
// One adjustment at a time
// -------------------------------------------------------------------------------------------

#[test]
fn a_triangles_apex_follows_its_adjustment_across_the_width() {
    // `seed.rs`: *"adj places the apex horizontally as a fraction of the width"*. So the apex's `x`
    // rises with `adj`, from the box's left edge at 0 to its right edge at 100 000.
    let within = box_on_the_page();
    let domain = domain_of(PresetShapeType::Triangle, "adj");
    assert_eq!((domain.minimum, domain.maximum), (0.0, 100_000.0));

    let apexes: Vec<f64> = sweep(domain.minimum, domain.maximum)
        .into_iter()
        .map(|value| f64::from(apex(&at(PresetShapeType::Triangle, &[("adj", value)]), within).x))
        .collect();
    strictly("rises", "a triangle's apex", &apexes);

    // And the endpoints are the box's own edges, which is what "a fraction of the width" means.
    assert!((apexes[0] - f64::from(within.left)).abs() < 0.01);
    assert!((apexes[apexes.len() - 1] - f64::from(within.right)).abs() < 0.01);
}

#[test]
fn a_rounded_rectangles_corners_grow_with_its_adjustment() {
    // `seed.rs`: *"adj is the corner radius as a fraction of the shorter side"*. Two consequences,
    // and both are asserted because either alone has a wrong shape that satisfies it: the corner's
    // end — where the top edge begins — moves right, **and** the area the shape encloses falls,
    // because a rounder corner cuts more off the box.
    let within = box_on_the_page();
    let domain = domain_of(PresetShapeType::RoundedRectangle, "adj");
    assert_eq!((domain.minimum, domain.maximum), (0.0, 50_000.0));

    let mut radii = Vec::new();
    let mut areas = Vec::new();
    for value in sweep(domain.minimum, domain.maximum) {
        let commands = at(PresetShapeType::RoundedRectangle, &[("adj", value)]);
        // The path starts at `(l, x1)` — the point where the left edge meets the top-left corner —
        // so the `MoveTo`'s distance below the box's top edge *is* the corner radius. Read from the
        // `MoveTo` rather than from the corner's own cubic because at `adj = 0` there is no cubic:
        // a zero-radius arc draws nothing, which is correct and is what makes the shape the plain
        // rectangle at that end of its domain.
        match commands[0] {
            PathCommand::MoveTo(start) => radii.push(f64::from(start.y - within.top)),
            ref other => panic!("a rounded rectangle starts with {other:?}"),
        }
        areas.push(enclosed_area(&commands));
    }
    strictly("rises", "a rounded rectangle's corner radius", &radii);
    strictly("falls", "a rounded rectangle's area", &areas);

    // At zero the shape *is* the rectangle: no radius, and five steps rather than nine.
    assert!(
        radii[0].abs() < 0.01,
        "at adj = 0 the radius is {}",
        radii[0]
    );
    assert_eq!(
        at(PresetShapeType::RoundedRectangle, &[("adj", 0.0)]).len(),
        5,
        "at adj = 0 a rounded rectangle is a rectangle: four corners and a close"
    );
    // At the maximum the radius is half the shorter side — 60 of 120 pixels, which is where two
    // corners would meet, and is why 50 000 is the only bound the adjustment could have.
    let widest = radii[radii.len() - 1];
    assert!(
        (widest - f64::from(within.height() / 2.0)).abs() < 0.01,
        "at adj = 50 000 the corner radius is {widest} and half the shorter side is {}",
        within.height() / 2.0
    );
}

#[test]
fn a_right_arrows_shaft_thickens_and_its_head_lengthens_independently() {
    // Two adjustments that interact, which is why the shape is in the seed table. `seed.rs`:
    // *"adj1 is the shaft's thickness as a fraction of the height and adj2 the head's length as a
    // fraction of the shorter side"*. So adj1 moves the shaft's top edge **up** and adj2 moves the
    // head's base **left**, and neither may move the other's.
    let within = box_on_the_page();
    let shaft_top = |commands: &[PathCommand]| match commands[0] {
        PathCommand::MoveTo(at) => f64::from(at.y),
        ref other => panic!("the first command is {other:?}"),
    };
    let head_base = |commands: &[PathCommand]| match commands[1] {
        PathCommand::LineTo(at) => f64::from(at.x),
        ref other => panic!("the second command is {other:?}"),
    };

    let thickness = domain_of(PresetShapeType::RightArrow, "adj1");
    assert_eq!((thickness.minimum, thickness.maximum), (0.0, 100_000.0));
    let mut tops = Vec::new();
    let mut bases_while_thickening = Vec::new();
    for value in sweep(thickness.minimum, thickness.maximum) {
        let commands = at(PresetShapeType::RightArrow, &[("adj1", value)]);
        tops.push(shaft_top(&commands));
        bases_while_thickening.push(head_base(&commands));
    }
    strictly("falls", "a right arrow's shaft edge", &tops);
    // The head is `adj2`'s business and must not move while `adj1` does — a swapped pair of guides
    // would move both, and the two `strictly` calls above would still pass.
    assert!(
        bases_while_thickening
            .windows(2)
            .all(|pair| (pair[0] - pair[1]).abs() < 0.01),
        "thickening the shaft moved the head's base: {bases_while_thickening:?}"
    );

    // `maxAdj2` is `*/ 100000 w ss`, which at 160 × 120 pixels is 100 000 × 160 / 120.
    let length = domain_of(PresetShapeType::RightArrow, "adj2");
    assert_eq!(length.minimum, 0.0);
    assert!(
        (length.maximum - 100_000.0 * 160.0 / 120.0).abs() < 1.0,
        "maxAdj2 evaluated to {} rather than 100000·w/ss",
        length.maximum
    );

    let mut bases = Vec::new();
    for value in sweep(length.minimum, length.maximum) {
        bases.push(head_base(&at(
            PresetShapeType::RightArrow,
            &[("adj2", value)],
        )));
    }
    strictly("falls", "a right arrow's head base", &bases);
    // At the maximum the head fills the shape, so its base is the box's left edge; at zero there is
    // no head at all and the base is the right edge.
    assert!((bases[0] - f64::from(within.right)).abs() < 0.01);
    assert!((bases[bases.len() - 1] - f64::from(within.left)).abs() < 0.01);
}

#[test]
fn a_pies_wedge_grows_with_its_end_angle_and_shrinks_with_its_start() {
    // `seed.rs`: the wedge runs from `adj1` to `adj2`, the long way round when the end precedes the
    // start. So a later end angle sweeps more of the ellipse and a later start angle sweeps less —
    // and the area is the right measure because the wedge's *shape* changes as it crosses a
    // quadrant and its point count changes with it.
    //
    // The sweep deliberately starts above zero. At `adj1 == adj2` the swing is `?: 0 0 21600000`,
    // which takes the else branch and draws the **whole** ellipse; that is the shape's own rule and
    // not a discontinuity in the resolver, but it is not part of a monotone run.
    let widening: Vec<f64> = sweep(1_000_000.0, 21_000_000.0)
        .into_iter()
        .map(|value| enclosed_area(&at(PresetShapeType::Pie, &[("adj1", 0.0), ("adj2", value)])))
        .collect();
    strictly("rises", "a pie's wedge as its end angle rises", &widening);

    let narrowing: Vec<f64> = sweep(0.0, 20_000_000.0)
        .into_iter()
        .map(|value| {
            enclosed_area(&at(
                PresetShapeType::Pie,
                &[("adj1", value), ("adj2", 21_500_000.0)],
            ))
        })
        .collect();
    strictly(
        "falls",
        "a pie's wedge as its start angle rises",
        &narrowing,
    );

    // And the shape's own wrap-around rule, asserted rather than assumed: equal angles are a whole
    // turn, which is the entire ellipse and therefore the largest wedge there is.
    let whole = enclosed_area(&at(
        PresetShapeType::Pie,
        &[("adj1", 5_000_000.0), ("adj2", 5_000_000.0)],
    ));
    assert!(
        whole > widening[widening.len() - 1],
        "a pie with equal angles enclosed {whole}, which is not more than the widest wedge {}",
        widening[widening.len() - 1]
    );
}

#[test]
fn an_override_reaches_the_domain_and_the_default_is_the_tables_own() {
    // The identity-value question for `AdjustmentOverride`, asked from the consumer's side: does
    // supplying one change what comes out, and does *not* supplying one fall back to the generated
    // default rather than to zero?
    let extents = extents_of_the_box();
    let defaulted = adjustment_domains(PresetShapeType::RoundedRectangle, extents, &[])
        .expect("roundRect has a domain");
    assert_eq!(defaulted.len(), 1);
    assert!(!defaulted[0].is_overridden);
    assert_eq!(defaulted[0].value, 16_667.0, "the generated default");
    assert_eq!(defaulted[0].spec.wire_name, "adj");

    let overridden = adjustment_domains(
        PresetShapeType::RoundedRectangle,
        extents,
        &[AdjustmentOverride::new("adj", 40_000.0)],
    )
    .expect("roundRect has a domain");
    assert!(overridden[0].is_overridden);
    assert_eq!(overridden[0].value, 40_000.0);

    // …and the downstream consequence, which is the half that matters: the geometry moved.
    let with_default = at(PresetShapeType::RoundedRectangle, &[]);
    let with_override = at(PresetShapeType::RoundedRectangle, &[("adj", 40_000.0)]);
    assert_ne!(
        with_default, with_override,
        "an adjustment override changed the domain and not the shape"
    );
}

/// One adjustment's domain at the suite's own extents.
fn domain_of(preset: PresetShapeType, wire_name: &str) -> mjx_geometry::AdjustmentDomain {
    adjustment_domains(preset, extents_of_the_box(), &[])
        .unwrap_or_else(|error| panic!("`{}` has no domain: {error}", preset.to_wire()))
        .into_iter()
        .find(|domain| domain.spec.wire_name == wire_name)
        .unwrap_or_else(|| panic!("`{}` has no `{wire_name}`", preset.to_wire()))
}

// -------------------------------------------------------------------------------------------
// Sizes that come out of real files
// -------------------------------------------------------------------------------------------

/// The presets whose own guide formulas have no value somewhere a *path* reads, at some size and
/// some adjustment value.
///
/// **This is ECMA-376's arithmetic, not a defect in the extraction.** The spec's formulas divide
/// and take square roots, and at the ends of an adjustment's domain a divisor can be zero:
/// `circularArrow`'s `swAng` is derived through `dxF1 = "+/ q11 q10 q4"`, and `q4` is zero at
/// `adj5 = 0` — which is that adjustment's own **minimum**, and therefore a value a handle drag
/// reaches. `noSmoking` is the same story with a `sqrt` of a negative.
///
/// Ten of the 186 have such a point *somewhere in their guide list*; these six are the ones where a
/// path reads it. In the other four the singular guide is `il`/`it`/`ir`/`ib` — the **text
/// rectangle**'s insets, which draw nothing — and [`mjx_geometry::resolve`] therefore leaves it
/// undefined and the shape draws normally. That distinction is the whole reason the resolver
/// evaluates the `gdLst` one guide at a time.
///
/// The answer for these six is [`GeometryError::SingularGeometry`], which
/// [`GeometryError::has_no_geometry_to_draw`] classifies as *"there is nothing to draw"* rather
/// than *"the table is wrong"* — so a provider standing in for unknown shapes answers with a
/// **counted** placeholder instead of failing the page, and one refusing answers with an error.
/// Neither silently draws nothing. `a_singular_shape_is_stood_in_for_and_never_silently_nothing`
/// gates both.
const SINGULAR_SOMEWHERE: &[PresetShapeType] = &[
    PresetShapeType::CircularArrow,
    PresetShapeType::CurvedDownArrow,
    PresetShapeType::CurvedUpArrow,
    PresetShapeType::LeftCircularArrow,
    PresetShapeType::LeftRightCircularArrow,
    PresetShapeType::NoSmoking,
];

#[test]
fn a_degenerate_size_produces_finite_coordinates_and_never_a_panic() {
    // Five sizes that arrive from real documents — a shape scaled to nothing, a shape one EMU
    // across, a negative extent from a file that lies — crossed with every preset in the table and
    // with each adjustment pushed to both ends of its domain and well outside it. Nothing here may
    // panic, and nothing may put a `NaN` or an infinity into a display list: `SceneBuilder` would
    // sanitise it to zero and the shape would silently jump to the page's corner.
    //
    // Twenty-six thousand combinations, and the ones that do not resolve are **named**: they are
    // exactly [`SINGULAR_SOMEWHERE`], every one answers [`GeometryError::SingularGeometry`], and
    // every one is a point where ECMA-376's own formula has no value.
    let boxes = [
        ("a box with no area", SceneRect::new(20.0, 20.0, 20.0, 20.0)),
        (
            "a box with no width",
            SceneRect::new(20.0, 20.0, 20.0, 90.0),
        ),
        (
            "a box with no height",
            SceneRect::new(20.0, 20.0, 90.0, 20.0),
        ),
        ("an ordinary box", box_on_the_page()),
    ];
    let sizes = [
        ("no extent at all", Size::from_emu(0, 0)),
        ("one EMU square", Size::from_emu(1, 1)),
        ("one EMU wide", Size::from_emu(1, 1_524_000)),
        ("a negative extent", Size::from_emu(-2_032_000, -1_524_000)),
        ("ordinary extents", extents_of_the_box()),
    ];

    let mut resolved = 0usize;
    let mut with_commands = 0usize;
    let mut singular: BTreeSet<&'static str> = BTreeSet::new();
    for definition in seeded_shapes() {
        let preset = definition.preset;
        // Both ends of every adjustment's domain, plus values well outside it, because the shape's
        // own `pin` is what is meant to bring them back and a `pin` written against the wrong bound
        // is exactly what this crosses with a degenerate size.
        let mut adjustment_sets: Vec<Vec<AdjustmentOverride>> = vec![Vec::new()];
        for spec in mjx_ooxml_types::drawingml::adjustments_of(preset) {
            for value in [-1_000_000.0, 0.0, 50_000.0, 100_000_000.0] {
                adjustment_sets.push(vec![AdjustmentOverride::new(spec.wire_name, value)]);
            }
        }

        for (size_label, extents) in sizes {
            for (box_label, within) in boxes {
                for adjustments in &adjustment_sets {
                    let outline = match preset_outline(preset, extents, adjustments, within) {
                        Ok(outline) => outline,
                        Err(error) => {
                            assert!(
                                matches!(error, GeometryError::SingularGeometry { .. }),
                                "`{}` at {size_label} in {box_label} failed with {error}, which is \
                                 a defect in the table rather than a point its own formulas have \
                                 no value at",
                                preset.to_wire()
                            );
                            assert!(
                                error.has_no_geometry_to_draw(),
                                "`{}` answered with a failure no stand-in may fill",
                                preset.to_wire()
                            );
                            singular.insert(preset.to_wire());
                            continue;
                        }
                    };
                    resolved += 1;
                    with_commands += usize::from(!outline.commands.is_empty());
                    for point in points_of(&outline.commands) {
                        assert!(
                            point.x.is_finite() && point.y.is_finite(),
                            "`{}` at {size_label} in {box_label} produced {point:?}",
                            preset.to_wire()
                        );
                    }
                }
            }
        }
    }

    assert_eq!(
        singular,
        SINGULAR_SOMEWHERE
            .iter()
            .map(|preset| preset.to_wire())
            .collect::<BTreeSet<_>>(),
        "the presets with a point their own formulas have no value at have changed"
    );
    // A loop that resolved nothing would satisfy every assertion inside it, and one that answered
    // every case with an empty path would satisfy the finiteness check by having nothing to check.
    assert!(
        resolved > 20_000,
        "the degenerate-size sweep resolved only {resolved} outlines"
    );
    assert!(
        with_commands > resolved / 2,
        "only {with_commands} of {resolved} degenerate resolutions drew anything at all"
    );
    println!(
        "degenerate-size sweep: {resolved} outlines resolved without a panic, {with_commands} of \
         them with commands, {} presets singular somewhere",
        singular.len()
    );
}

#[test]
fn a_singular_shape_is_stood_in_for_and_never_silently_nothing() {
    // `circularArrow` at `adj5 = 0` — its own domain minimum, so a handle drag reaches it — has no
    // geometry, and this is what happens instead. Two policies, two answers, and neither is an
    // empty path: the seam's rule is that a shape which silently drew nothing is a defect nobody
    // finds.
    let (within, extents) = (box_on_the_page(), extents_of_the_box());
    let domains = adjustment_domains(PresetShapeType::CircularArrow, extents, &[])
        .expect("`circularArrow` has domains at an ordinary size");
    let adj5 = domains
        .iter()
        .find(|domain| domain.spec.wire_name == "adj5")
        .expect("`circularArrow` has an `adj5`");
    assert_eq!(
        adj5.minimum, 0.0,
        "`circularArrow`'s `adj5` no longer bottoms out at the value its formulas are singular at"
    );
    let at_the_stop = vec![AdjustmentOverride::new("adj5", adj5.minimum)];

    let error = preset_outline(
        PresetShapeType::CircularArrow,
        extents,
        &at_the_stop,
        within,
    )
    .expect_err("`circularArrow` has no geometry at `adj5 = 0`");
    assert!(matches!(error, GeometryError::SingularGeometry { .. }));
    assert_eq!(error.shape(), Some("circularArrow"));

    let outline = ShapeOutline::new(PresetShapeType::CircularArrow, extents)
        .with_adjustment("adj5", adj5.minimum);
    for (policy, mut provider) in [
        (UnknownShapePolicy::Refuse, PresetGeometryProvider::new()),
        (
            UnknownShapePolicy::StandIn,
            PresetGeometryProvider::standing_in_for_unknown_shapes(),
        ),
    ] {
        provider.register(3, outline.clone());
        let answer = provider.outline(3, within);
        match policy {
            UnknownShapePolicy::Refuse => assert!(
                answer.is_err(),
                "refusing answered a singular shape with {answer:?}"
            ),
            UnknownShapePolicy::StandIn => {
                let stand_in = answer.expect("a stand-in is an answer");
                assert_eq!(stand_in.provenance, OutlineProvenance::Placeholder);
                assert!(
                    !stand_in.commands.is_empty(),
                    "the stand-in for a singular shape is an empty path, which is the one answer \
                     the seam forbids"
                );
            }
        }
    }
}

#[test]
fn a_shape_with_no_extent_draws_nothing_and_says_so_as_the_documents_own_geometry() {
    // The one case where an empty command list is the *right* answer, gated in both directions so
    // that "it drew nothing" cannot become the answer to everything. `rightArrow`'s own `maxAdj2`
    // is `*/ 100000 w ss`, which is infinite when a side is zero, so this is also the shape whose
    // guide list genuinely cannot be evaluated at a degenerate size.
    let within = box_on_the_page();
    let nothing = preset_outline(
        PresetShapeType::RightArrow,
        Size::from_emu(0, 0),
        &[],
        within,
    )
    .expect("a shape with no extent is not a failure");
    assert!(
        nothing.commands.is_empty(),
        "a shape with no extent drew {} command(s)",
        nothing.commands.len()
    );
    // Still the document's own geometry, not a stand-in: nothing about the shape is unknown.
    assert_eq!(
        nothing.provenance,
        mjx_scene::OutlineProvenance::Document,
        "a zero-extent shape is not a placeholder"
    );
    assert_eq!(nothing.label, "rightArrow");

    // The other direction. One EMU is a real extent, and the same shape at the same box draws.
    let something = preset_outline(
        PresetShapeType::RightArrow,
        Size::from_emu(1, 1),
        &[],
        within,
    )
    .expect("a one-EMU arrow resolves");
    assert_eq!(
        something.commands.len(),
        8,
        "a right arrow is eight steps at every extent that has an area"
    );
}

#[test]
fn a_shape_smaller_than_its_own_corner_radius_still_draws() {
    // MJXOFF-202 names this case specifically: *"a size smaller than an adjustment's own radius"*.
    // A `roundRect` two EMU across with the radius pinned to its maximum has a corner radius of one
    // EMU, so every one of its four arcs is the whole of an edge and the three straight edges have
    // zero length. It must still be a closed path in the right box rather than nothing.
    let within = SceneRect::new(4.0, 4.0, 6.0, 6.0);
    let outline = preset_outline(
        PresetShapeType::RoundedRectangle,
        Size::from_emu(2, 2),
        &[AdjustmentOverride::new("adj", 50_000.0)],
        within,
    )
    .expect("a two-EMU rounded rectangle resolves");
    assert_eq!(
        outline.commands.len(),
        9,
        "a rounded rectangle is nine steps at every size"
    );
    for point in points_of(&outline.commands) {
        assert!(point.x.is_finite() && point.y.is_finite());
        assert!(
            point.x >= within.left - 0.01 && point.x <= within.right + 0.01,
            "{point:?} is outside {within:?}"
        );
    }
    assert!(enclosed_area(&outline.commands) > 0.0);
}
