//! The identity-value sweep: for every parameter and every branch this crate has, *is it ever
//! supplied at a value other than the one that makes it a no-op* — and **does that value reach
//! anyone**?
//!
//! # Why both questions
//!
//! R07 found nine defects in one file by asking the first. The audit that reviewed it added the
//! second, and it is the sharper one: a parameter can be varied, be read, change an internal number
//! and reach nobody. `mjx_scene::SceneMesh::provenance` failed exactly that way — written once,
//! read zero times — and it is the field this whole crate exists to make meaningful. So each case
//! below gates on **a relationship plus a downstream consequence**: not a getter answering what was
//! set, but a second observable that has to move with it.
//!
//! # This crate's candidates, and where each branch is
//!
//! | What | Its identity value | Where the branch is |
//! |---|---|---|
//! | a path's `@w`/`@h` coordinate box | `None` — the shape's own space | `ShapeToDevice::new` |
//! | a shape's extents | any positive size; zero is the degenerate arm | `scale`, and the empty-outline arm of `outline_of_definition` |
//! | an adjustment override | absent — the generated default stands | `value_of` |
//! | [`UnknownShapePolicy`] | `Refuse` | `PresetGeometryProvider::outline` |
//! | a `PresetCoordinate` | `Emu` versus `Guide` | `PresetCoordinate::to_adjust_coordinate` |
//! | a `PresetAngle` | `Native` versus `Guide` | `PresetAngle::to_adjust_angle` |
//! | an arc's swing | zero — the arm that draws nothing | `arc_to_cubics` |
//! | an ellipse's radii | equal — the arm where true and parametric angles coincide | `parametric_angle` |
//! | a guide that will not evaluate | finite — the arm that defines it | `guide_environment`'s `is_a_singularity` |
//! | a path's `@fill`/`@stroke` | filled and stroked — the arm that keeps the contour | `outline_of_definition` |
//! | a shape's `a:rect` | absent — the shape says nothing about where its text goes | `text_rectangle_of_definition` |
//! | a text rectangle's edge | `Guide` — the only arm the spec file ever takes | `PresetCoordinate::to_adjust_coordinate` |
//! | a connection site's position | `Guide` — likewise, all 1 712 of them | `PresetPoint::to_point` |
//! | **the box's orientation** | landscape — where `ss` is always `h` | `GuideContext::from_size`, reached through every `ss` formula |
//!
//! **The last of those was added by MJXOFF-204's audit and is the sharpest.** It is not a parameter
//! this crate passes; it is a *shape* of the fixtures. Every box and every non-degenerate extent the
//! crate had was landscape or square, and `ss` is `min(w, h)`, so an implementation that read `h`
//! where a formula says `ss` was indistinguishable from a correct one in every gate — the seed
//! differential measured 0.00000 px, the box census kept its membership, the monotonicity sweep kept
//! its direction. [`a_portrait_box_and_a_landscape_one_are_two_values_of_the_shorter_side`] is the
//! branch, and `text_goes_inside_the_shape.rs` now takes **every** census in both orientations,
//! which is where two further facts turned up that a single aspect ratio had hidden.
//!
//! The lesson is worth stating beside the table, because it generalises past this crate: **asking
//! whether a value reaches somebody is not the same as asking at how many distinct values it was
//! ever supplied.** A measurement taken at exactly one point is a measurement of that point.
//!
//! **Every one of those branches has both arms exercised here, with a consequence.** Two would
//! otherwise never be taken with a non-identity value against *real* data — the anisotropic arc,
//! which is why `outline_of_definition` is public, and the three arms of `is_a_singularity`, of
//! which only one is reachable from the committed table at all. The path coordinate box is no
//! longer among them: thirty-one of the 186 generated shapes declare one, in boxes of 2, 5, 10,
//! 21 600 and 43 200 — but the synthetic case below stays, because it is the one that pins down
//! *which* number the box divides by.

mod common;

use common::{
    bounds_of, box_on_the_page, extents_of_the_box, points_of, portrait_box_on_the_page,
    portrait_extents,
};
use mjx_dml::geometry::{AdjustAngle, AdjustCoordinate};
use mjx_geometry::PathFillMode;
use mjx_geometry::{
    adjustment_domains, arc_to_cubics, connection_sites_of_definition, outline_of_definition,
    parametric_angle, preset_connection_sites, preset_outline, preset_text_rectangle,
    text_rectangle_of_definition, AdjustmentOverride, Derivation, PresetAngle,
    PresetConnectionSite, PresetCoordinate, PresetGeometryProvider, PresetPath, PresetPathStep,
    PresetPoint, PresetShapeDefinition, PresetShapeType, PresetTextRectangle, ShapeOutline,
    ShapePoint, Size, TextRectangle, UnknownShapePolicy, MAXIMUM_ARC_SEGMENT_RADIANS,
};
use mjx_ooxml_types::drawingml::PresetGuide;
use mjx_scene::{GeometryProvider, OutlineProvenance, SceneError, SceneRect};

/// A unit square path, written in whatever coordinate box the definition below declares.
const UNIT_SQUARE: &[PresetPathStep] = &[
    PresetPathStep::MoveTo(PresetPoint {
        x: PresetCoordinate::Emu(0),
        y: PresetCoordinate::Emu(0),
    }),
    PresetPathStep::LineTo(PresetPoint {
        x: PresetCoordinate::Emu(1000),
        y: PresetCoordinate::Emu(0),
    }),
    PresetPathStep::LineTo(PresetPoint {
        x: PresetCoordinate::Emu(1000),
        y: PresetCoordinate::Emu(1000),
    }),
    PresetPathStep::LineTo(PresetPoint {
        x: PresetCoordinate::Emu(0),
        y: PresetCoordinate::Emu(1000),
    }),
    PresetPathStep::Close,
];

/// The same square with **no** coordinate box: its coordinates are lengths in the shape's space.
const NO_COORDINATE_BOX: PresetShapeDefinition = PresetShapeDefinition {
    preset: PresetShapeType::Rectangle,
    derivation: Derivation::FromFirstPrinciples,
    source: "a 1000 EMU square in the shape's own space, for the identity-value probe",
    adjustment_values: &[],
    guides: &[],
    text_rectangle: None,
    connection_sites: &[],
    paths: &[PresetPath {
        width: None,
        height: None,
        fill: PathFillMode::Normal,
        stroke: true,
        extrusion_ok: true,
        steps: UNIT_SQUARE,
    }],
};

/// The same square **with** a 1000 × 1000 coordinate box: its coordinates are fractions of it, so
/// it fills the shape.
const A_COORDINATE_BOX: PresetShapeDefinition = PresetShapeDefinition {
    preset: PresetShapeType::Rectangle,
    derivation: Derivation::FromFirstPrinciples,
    source: "a 1000 × 1000 path box, for the identity-value probe",
    adjustment_values: &[],
    guides: &[],
    text_rectangle: None,
    connection_sites: &[],
    paths: &[PresetPath {
        width: Some(1000),
        height: Some(1000),
        fill: PathFillMode::Normal,
        stroke: true,
        extrusion_ok: true,
        steps: UNIT_SQUARE,
    }],
};

/// The same square with a coordinate box declared as **zero**, which the schema's default is and
/// which means "no box" rather than "a box of no size".
const A_ZERO_COORDINATE_BOX: PresetShapeDefinition = PresetShapeDefinition {
    preset: PresetShapeType::Rectangle,
    derivation: Derivation::FromFirstPrinciples,
    source: "a path box declared as zero, which is the schema default and means no box",
    adjustment_values: &[],
    guides: &[],
    text_rectangle: None,
    connection_sites: &[],
    paths: &[PresetPath {
        width: Some(0),
        height: Some(0),
        fill: PathFillMode::Normal,
        stroke: true,
        extrusion_ok: true,
        steps: UNIT_SQUARE,
    }],
};

// -------------------------------------------------------------------------------------------
// A guide that will not evaluate: which of the three arms, and what each one costs
// -------------------------------------------------------------------------------------------

/// A shape one of whose guides has no finite value, read by nothing.
///
/// `wide` divides by a guide that is zero, so it has no value; `x1` never mentions it and the path
/// draws through `x1` alone. The whole shape must still draw — that is the arm that turns four of
/// the ten singular presets back into drawable ones.
const A_SINGULAR_GUIDE_NOBODY_READS: PresetShapeDefinition = PresetShapeDefinition {
    preset: PresetShapeType::Rectangle,
    derivation: Derivation::FromFirstPrinciples,
    source: "a guide with no finite value that no path reads, for the identity-value probe",
    adjustment_values: &[],
    guides: &[
        PresetGuide {
            wire_name: "zero",
            formula: "val 0",
        },
        PresetGuide {
            wire_name: "wide",
            formula: "*/ w h zero",
        },
        PresetGuide {
            wire_name: "x1",
            formula: "*/ w 1 2",
        },
    ],
    text_rectangle: None,
    connection_sites: &[],
    paths: &[PresetPath {
        width: None,
        height: None,
        fill: PathFillMode::Normal,
        stroke: true,
        extrusion_ok: true,
        steps: &[
            PresetPathStep::MoveTo(PresetPoint::at("l", "t")),
            PresetPathStep::LineTo(PresetPoint::at("x1", "b")),
            PresetPathStep::Close,
        ],
    }],
};

/// The same singular guide, this time read by the path.
const A_SINGULAR_GUIDE_A_PATH_READS: PresetShapeDefinition = PresetShapeDefinition {
    preset: PresetShapeType::Rectangle,
    derivation: Derivation::FromFirstPrinciples,
    source: "a guide with no finite value that a path reads, for the identity-value probe",
    adjustment_values: &[],
    guides: &[
        PresetGuide {
            wire_name: "zero",
            formula: "val 0",
        },
        PresetGuide {
            wire_name: "wide",
            formula: "*/ w h zero",
        },
    ],
    text_rectangle: None,
    connection_sites: &[],
    paths: &[PresetPath {
        width: None,
        height: None,
        fill: PathFillMode::Normal,
        stroke: true,
        extrusion_ok: true,
        steps: &[
            PresetPathStep::MoveTo(PresetPoint::at("l", "t")),
            PresetPathStep::LineTo(PresetPoint::at("wide", "b")),
            PresetPathStep::Close,
        ],
    }],
};

/// A guide naming something nothing ever defines — a defect in the table, not a singularity.
const A_GUIDE_WITH_A_NAME_NOTHING_DEFINES: PresetShapeDefinition = PresetShapeDefinition {
    preset: PresetShapeType::Rectangle,
    derivation: Derivation::FromFirstPrinciples,
    source: "a guide naming an undefined name, for the identity-value probe",
    adjustment_values: &[],
    guides: &[PresetGuide {
        wire_name: "x1",
        formula: "*/ w nowhere 100000",
    }],
    text_rectangle: None,
    connection_sites: &[],
    paths: &[PresetPath {
        width: None,
        height: None,
        fill: PathFillMode::Normal,
        stroke: true,
        extrusion_ok: true,
        steps: &[
            PresetPathStep::MoveTo(PresetPoint::at("l", "t")),
            PresetPathStep::LineTo(PresetPoint::at("x1", "b")),
            PresetPathStep::Close,
        ],
    }],
};

/// A guide whose formula gives its operator the wrong number of arguments.
const A_GUIDE_WITH_A_MALFORMED_FORMULA: PresetShapeDefinition = PresetShapeDefinition {
    preset: PresetShapeType::Rectangle,
    derivation: Derivation::FromFirstPrinciples,
    source: "a guide with a four-argument `+-`, for the identity-value probe",
    adjustment_values: &[],
    guides: &[PresetGuide {
        wire_name: "x1",
        formula: "+- w 0 h 0",
    }],
    text_rectangle: None,
    connection_sites: &[],
    paths: &[PresetPath {
        width: None,
        height: None,
        fill: PathFillMode::Normal,
        stroke: true,
        extrusion_ok: true,
        steps: &[
            PresetPathStep::MoveTo(PresetPoint::at("l", "t")),
            PresetPathStep::LineTo(PresetPoint::at("x1", "b")),
            PresetPathStep::Close,
        ],
    }],
};

#[test]
fn all_three_arms_of_a_guide_that_will_not_evaluate_are_taken_and_answer_differently() {
    // `is_a_singularity` has three answers and the committed table only ever reaches one of them,
    // so the other two are exhibited here. They must not collapse into each other: a shape's own
    // arithmetic leaving the reals is not a defect and must not fail the page, while a table that
    // names a guide it does not define, or writes a formula the language does not have, is a defect
    // and must.
    let (within, extents) = (box_on_the_page(), extents_of_the_box());

    // 1. Not finite, and nothing reads it — the shape draws. This is the arm that matters: four of
    //    the ten presets whose guide list has a singular point are singular only in the text
    //    rectangle's insets, and would otherwise be undrawable at those adjustments.
    let drawn = outline_of_definition(&A_SINGULAR_GUIDE_NOBODY_READS, extents, &[], within)
        .expect("a guide nothing reads cannot stop the shape drawing");
    assert_eq!(drawn.commands.len(), 3, "the triangle still drew");

    // 2. Not finite, and a path reads it — no geometry, and a *counted* stand-in may fill it.
    let singular = outline_of_definition(&A_SINGULAR_GUIDE_A_PATH_READS, extents, &[], within)
        .expect_err("a path reading a guide with no value cannot resolve");
    assert!(
        matches!(&singular, mjx_geometry::GeometryError::SingularGeometry { guide, .. } if guide == "wide"),
        "a path reading a singular guide reported {singular}, which names the wrong cause"
    );
    assert!(
        singular.has_no_geometry_to_draw(),
        "a singularity must be fillable by a counted stand-in"
    );

    // 3. A name nothing defines — a table defect, and no stand-in may hide it.
    let undefined =
        outline_of_definition(&A_GUIDE_WITH_A_NAME_NOTHING_DEFINES, extents, &[], within)
            .expect_err("a guide naming nothing cannot resolve");
    assert!(
        matches!(undefined, mjx_geometry::GeometryError::Guides { .. }),
        "an undefined name reported {undefined}, which is not a guide-list failure"
    );
    assert!(
        !undefined.has_no_geometry_to_draw(),
        "a table defect reported itself as having nothing to draw, so a stand-in would hide it"
    );
    assert!(
        format!("{undefined}").contains("nowhere"),
        "the failure does not name the guide it could not find: {undefined}"
    );

    // 4. A malformed formula — the same, by the other arm of `is_a_singularity`.
    let malformed = outline_of_definition(&A_GUIDE_WITH_A_MALFORMED_FORMULA, extents, &[], within)
        .expect_err("a four-argument `+-` cannot resolve");
    assert!(
        matches!(malformed, mjx_geometry::GeometryError::Guides { .. }),
        "a malformed formula reported {malformed}"
    );
    assert!(
        !malformed.has_no_geometry_to_draw(),
        "a malformed formula reported itself as having nothing to draw"
    );
}

#[test]
fn a_paths_own_coordinate_box_changes_where_its_points_land() {
    // The branch no seeded shape takes. `@w`/`@h` is *"the maximum x coordinate that should be used
    // within the path coordinate system"*, so the identical thousand-unit square is a thousandth of
    // the shape without a box and the whole of it with one. A resolver that ignored the attribute
    // would produce the same tiny square either way, and every seeded shape would still pass.
    let (within, extents) = (box_on_the_page(), extents_of_the_box());

    let unboxed = outline_of_definition(&NO_COORDINATE_BOX, extents, &[], within)
        .expect("a square in the shape's own space resolves");
    let boxed = outline_of_definition(&A_COORDINATE_BOX, extents, &[], within)
        .expect("a square in its own coordinate box resolves");
    assert_ne!(
        unboxed.commands, boxed.commands,
        "a path's `@w`/`@h` box changed nothing, so the attribute is being ignored"
    );

    // The consequence, quantified rather than merely different: with the box the square *is* the
    // shape's box; without it the square is 1000 of 2 032 000 EMU across, which is 0.079 pixels.
    let filled = bounds_of(&boxed.commands);
    assert!(
        (filled.width() - within.width()).abs() < 0.01
            && (filled.height() - within.height()).abs() < 0.01,
        "with a 1000 × 1000 coordinate box the square should fill the shape, and it is {filled:?}"
    );
    let tiny = bounds_of(&unboxed.commands);
    assert!(
        tiny.width() < 0.1 && tiny.width() > 0.0,
        "without a box the 1000 EMU square should be a fraction of a pixel, and it is {}",
        tiny.width()
    );

    // And zero is *"no box"*, not *"a box of no size"* — the schema's own default is zero, so a
    // resolver that treated it as a box would collapse every path that omitted the attribute.
    let zeroed = outline_of_definition(&A_ZERO_COORDINATE_BOX, extents, &[], within)
        .expect("a zero coordinate box resolves");
    assert_eq!(
        zeroed.commands, unboxed.commands,
        "a coordinate box declared as zero is not the same as none"
    );
}

#[test]
fn a_shapes_extents_change_the_answer_and_a_zero_extent_takes_the_other_arm() {
    // `scale`'s two arms, both with a consequence. The identity here is *any* positive extent; the
    // non-identity is a second positive extent that must give a different answer, and the zero arm
    // is the one that must give an answer at all rather than an infinity.
    let within = box_on_the_page();
    let at = |extents: Size| {
        preset_outline(PresetShapeType::Triangle, extents, &[], within)
            .expect("a triangle resolves")
            .commands
    };

    // **A finding, and it is why this case is written the way it is.** Every coordinate of a
    // `triangle` is a fraction of `w` or of `h`, and the map back onto the pixel box divides by the
    // same two numbers — so the shape is invariant under *any* change of extents, proportional or
    // not. That is correct, and it means a triangle can prove nothing about whether the extents are
    // read at all. Asserted rather than assumed, because it is the reason the shape below is a
    // `roundRect`.
    assert_eq!(
        at(Size::from_emu(2_032_000, 1_524_000)),
        at(Size::from_emu(1_016_000, 762_000)),
        "a triangle should not care about the scale of its own space"
    );
    assert_eq!(
        at(Size::from_emu(1_000_000, 1_000_000)),
        at(Size::from_emu(2_032_000, 1_524_000)),
        "a triangle should not care about the proportions of its own space either"
    );

    // `roundRect` is the seeded shape that does care, because its radius is a fraction of `ss` —
    // the *shorter* side — and `ss` is where the aspect ratio of the shape's own space reaches the
    // answer. A resolver that ignored the extents and used a fixed scale would give the same corner
    // for both of these.
    let rounded = |extents: Size| {
        preset_outline(PresetShapeType::RoundedRectangle, extents, &[], within)
            .expect("a rounded rectangle resolves")
            .commands
    };
    // Proportional extents describe the same shape — *within the coordinate rounding*, and not
    // byte for byte. **A finding worth keeping:** `mjx-dml` rounds every resolved coordinate to a
    // whole EMU, so a shape resolved in a space half as large is quantised twice as coarsely, and
    // halving these extents moves a corner by 8e-5 pixels. That is the first of the three error
    // terms `a_preset_renders_as_itself.rs`'s box tolerance is built out of, measured here rather
    // than only reasoned about, and it is two orders of magnitude inside that tolerance.
    let large = rounded(Size::from_emu(2_032_000, 1_524_000));
    let half = rounded(Size::from_emu(1_016_000, 762_000));
    let (large_points, half_points) = (points_of(&large), points_of(&half));
    assert_eq!(large_points.len(), half_points.len());
    let drift = large_points
        .iter()
        .zip(&half_points)
        .map(|(left, right)| (left.x - right.x).abs().max((left.y - right.y).abs()))
        .fold(0.0f32, f32::max);
    assert!(
        drift < 0.001,
        "halving a shape's own space moved it by {drift} device pixels, which is more than the \
         coordinate rounding can account for"
    );
    println!("halving the shape's own coordinate space moves it by {drift} device pixels");
    assert_ne!(
        rounded(Size::from_emu(1_000_000, 1_000_000)),
        rounded(Size::from_emu(2_032_000, 1_524_000)),
        "a rounded rectangle resolved in a square space and in a 4:3 one produced the same corner, \
         so `ss` is not being taken from the extents"
    );

    // The zero arm: finite coordinates, all of them on the box's corner, and not an empty answer —
    // `rect` and `triangle` have no guide that divides by `ss`, so they reach `scale` rather than
    // the empty-outline arm that answers `rightArrow`.
    let collapsed = at(Size::from_emu(0, 0));
    assert!(!collapsed.is_empty(), "a triangle is still three points");
    for point in points_of(&collapsed) {
        assert!(point.x.is_finite() && point.y.is_finite());
        assert!(
            (point.x - within.left).abs() < f32::EPSILON
                && (point.y - within.top).abs() < f32::EPSILON,
            "a shape with no space of its own collapses onto the box's corner, not to {point:?}"
        );
    }
    // And the *other* arm of the degenerate case, the one `rightArrow` takes, answers with nothing.
    assert!(preset_outline(
        PresetShapeType::RightArrow,
        Size::from_emu(0, 0),
        &[],
        within
    )
    .expect("a zero-extent arrow is not a failure")
    .commands
    .is_empty());
}

#[test]
fn an_absent_adjustment_is_the_generated_default_and_a_present_one_is_not() {
    // `value_of`'s two arms. The identity value is *absence*, and what makes the assertion worth
    // making is that absence must resolve to the **generated table's** default rather than to zero:
    // a `roundRect` whose missing `adj` read as zero would be a plain rectangle, and would still
    // close, still fill its box and still have the right number of corners.
    let (within, extents) = (box_on_the_page(), extents_of_the_box());
    let at = |adjustments: &[AdjustmentOverride]| {
        preset_outline(
            PresetShapeType::RoundedRectangle,
            extents,
            adjustments,
            within,
        )
        .expect("a rounded rectangle resolves")
        .commands
    };

    let defaulted = at(&[]);
    let zeroed = at(&[AdjustmentOverride::new("adj", 0.0)]);
    assert_ne!(
        defaulted, zeroed,
        "an absent adjustment read as zero, so the generated default never reaches the shape"
    );
    assert_eq!(
        defaulted,
        at(&[AdjustmentOverride::new("adj", 16_667.0)]),
        "the default that reaches the shape is not the 16 667 the generated table states"
    );
    // An override for an adjustment the shape does not have changes nothing — the arm that would
    // otherwise let a typo in a wire name silently move a different adjustment.
    assert_eq!(
        defaulted,
        at(&[AdjustmentOverride::new("adj7", 0.0)]),
        "an override naming an adjustment this shape does not have changed the shape"
    );
    // And the domain a caller reads moves with it, in both directions.
    let domains = adjustment_domains(PresetShapeType::RoundedRectangle, extents, &[])
        .expect("roundRect has a domain");
    assert!(!domains[0].is_overridden && domains[0].value == 16_667.0);
    let overridden = adjustment_domains(
        PresetShapeType::RoundedRectangle,
        extents,
        &[AdjustmentOverride::new("adj", 1.0)],
    )
    .expect("roundRect has a domain");
    assert!(overridden[0].is_overridden && overridden[0].value == 1.0);
}

#[test]
fn the_unknown_shape_policy_decides_what_an_unseeded_shape_becomes() {
    // Two arms, and the identity is `Refuse` because it is `Default`. What makes this more than a
    // getter is that both arms are asked the same question and answer differently *at the seam* —
    // through `GeometryProvider::outline`, which is the only method anything above will call.
    let within = box_on_the_page();
    // `upArrow` — the one `ST_ShapeType` value ECMA-376's own geometry file defines nothing for,
    // and therefore the only preset this build genuinely cannot draw.
    let shape = ShapeOutline::new(
        mjx_geometry::PRESETS_WITHOUT_GEOMETRY[0],
        extents_of_the_box(),
    );

    let mut refusing = PresetGeometryProvider::new();
    assert_eq!(refusing.unknown_shape_policy(), UnknownShapePolicy::Refuse);
    assert_eq!(
        PresetGeometryProvider::default().unknown_shape_policy(),
        UnknownShapePolicy::Refuse,
        "the default policy is the identity one"
    );
    refusing.register(1, shape.clone());

    let mut standing_in = PresetGeometryProvider::standing_in_for_unknown_shapes();
    assert_eq!(
        standing_in.unknown_shape_policy(),
        UnknownShapePolicy::StandIn
    );
    standing_in.register(1, shape);

    assert!(matches!(
        refusing.outline(1, within),
        Err(SceneError::UnresolvedOutline { outline: 1 })
    ));
    let stood_in = standing_in.outline(1, within).expect("a stand-in answers");
    assert_eq!(stood_in.provenance, OutlineProvenance::Placeholder);
    assert!(!stood_in.commands.is_empty());

    // The policy must not change what a shape this build *can* draw looks like — an arm that also
    // altered the seeded path would be a policy that decides geometry.
    let mut both = [
        PresetGeometryProvider::new(),
        PresetGeometryProvider::standing_in_for_unknown_shapes(),
    ];
    for provider in &mut both {
        provider.register(
            2,
            ShapeOutline::new(PresetShapeType::Triangle, extents_of_the_box()),
        );
    }
    let answers: Vec<_> = both
        .iter()
        .map(|provider| provider.outline(2, within).expect("a triangle resolves"))
        .collect();
    assert_eq!(answers[0].commands, answers[1].commands);
    assert_eq!(answers[0].provenance, OutlineProvenance::Document);
}

#[test]
fn a_literal_coordinate_and_a_guide_reference_are_two_different_answers() {
    // `PresetCoordinate` and `PresetAngle` each have two arms and the *literal* arm is the identity
    // one — a table of literals needs no evaluator at all. Both arms must reach `DrawCommand`
    // distinctly, or a guide name would silently become a length.
    assert_eq!(
        PresetCoordinate::Emu(42).to_adjust_coordinate(),
        AdjustCoordinate::Emu(mjx_dml::geometry::Emu::from_emu(42))
    );
    assert_eq!(
        PresetCoordinate::Guide("hc").to_adjust_coordinate(),
        AdjustCoordinate::Guide("hc".to_owned())
    );
    assert_eq!(
        PresetAngle::Guide("swAng").to_adjust_angle(),
        AdjustAngle::Guide("swAng".to_owned())
    );
    match PresetAngle::Native(10_800_000).to_adjust_angle() {
        AdjustAngle::Angle(angle) => assert!((angle.degrees() - 180.0).abs() < 1e-9),
        other => panic!("a literal half turn became {other:?}"),
    }

    // The consequence, downstream: the same shape written with literals and with guides resolves to
    // the same picture, which is what proves the guide arm is *evaluated* rather than dropped.
    let (within, extents) = (box_on_the_page(), extents_of_the_box());
    let by_guide = preset_outline(PresetShapeType::Rectangle, extents, &[], within)
        .expect("`rect` is written in guides");
    let by_literal = outline_of_definition(&A_COORDINATE_BOX, extents, &[], within)
        .expect("the square is written in literals");
    assert_eq!(
        bounds_of(&by_guide.commands),
        bounds_of(&by_literal.commands),
        "the same rectangle written in guides and in literals landed in different places"
    );
}

#[test]
fn an_arcs_swing_and_its_radii_each_have_both_arms_taken() {
    // `arc_to_cubics` has three no-op arms — a zero swing, two zero radii, a non-finite input — and
    // one that draws. All four, with the consequence being how many cubics come back.
    let start = ShapePoint::new(100.0, 0.0);
    assert!(
        arc_to_cubics(start, 100.0, 100.0, 0.0, 0.0).is_empty(),
        "a zero swing draws nothing"
    );
    assert!(
        arc_to_cubics(start, 0.0, 0.0, 0.0, 1.0).is_empty(),
        "an ellipse with no radii draws nothing"
    );
    assert!(
        arc_to_cubics(start, f64::NAN, 100.0, 0.0, 1.0).is_empty(),
        "a non-finite radius draws nothing"
    );

    // The splitting arm: a swing of one, two and five quarter turns is one, two and five cubics.
    for turns in 1..=5 {
        let segments = arc_to_cubics(
            start,
            100.0,
            100.0,
            0.0,
            MAXIMUM_ARC_SEGMENT_RADIANS * f64::from(turns),
        );
        assert_eq!(
            segments.len(),
            turns as usize,
            "a swing of {turns} quarter turn(s) became {} cubic(s)",
            segments.len()
        );
    }
    // A swing that is not a whole number of quarter turns rounds up rather than down: a segment
    // wider than a quarter turn is where a cubic's error stops being negligible.
    assert_eq!(
        arc_to_cubics(start, 100.0, 100.0, 0.0, MAXIMUM_ARC_SEGMENT_RADIANS * 1.01).len(),
        2
    );

    // `parametric_angle`'s identity arm is a **circle**, where the true and parametric angles are
    // the same number, and that is exactly the arm every seeded shape's `roundRect` corner takes.
    // The other arm is an ellipse, and it is why the conversion exists at all.
    for angle in [0.3, 1.0, 2.5, -2.0] {
        assert!(
            (parametric_angle(angle, 50.0, 50.0) - angle).abs() < 1e-12,
            "on a circle the parametric angle is the true angle, and {angle} became {}",
            parametric_angle(angle, 50.0, 50.0)
        );
        let stretched = parametric_angle(angle, 100.0, 25.0);
        assert!(
            (stretched - angle).abs() > 1e-3,
            "on a 4:1 ellipse the parametric angle must differ from the true angle {angle}, and it \
             was {stretched}"
        );
        // …and never by as much as a quarter turn, which is what keeps a long sweep monotone.
        assert!((stretched - angle).abs() < std::f64::consts::FRAC_PI_2);
    }
    // The quadrant boundaries are where the two agree on *any* ellipse, which is why a seeded
    // shape's arcs are exact there and why the bounding-box tolerance can be as tight as it is.
    for quarters in 0..8 {
        let angle = MAXIMUM_ARC_SEGMENT_RADIANS * f64::from(quarters);
        assert!((parametric_angle(angle, 100.0, 25.0) - angle).abs() < 1e-9);
    }
}

#[test]
fn an_anisotropic_arc_is_centred_where_a_true_angle_puts_it() {
    // The consequence of the paragraph above, in a shape rather than in a number. An `a:arcTo` at a
    // start angle of 45° on a 4:1 ellipse begins at a different point under the two readings, so a
    // provider that used the parametric one would put the ellipse's centre somewhere else — and the
    // arc would leave the shape's box. Nothing in the seed table can catch that, because every
    // seeded arc starts on a quadrant boundary where the two readings agree.
    let within = SceneRect::new(0.0, 0.0, 400.0, 100.0);
    let extents = Size::from_emu(400 * 12_700, 100 * 12_700);
    let outline = outline_of_definition(&A_DIAGONAL_ARC, extents, &[], within)
        .expect("a 45° arc on a 4:1 ellipse resolves");

    // The sharpest thing that can be said about the derived centre, said in the shape rather than
    // in a number: the arc runs from 45° to 135°, which is symmetric about the vertical centre
    // line, **so its end point is its start point mirrored about `hc`.** Under the parametric
    // reading the centre would land about 93 pixels left of the shape's own and the end point would
    // be at x ≈ -34, off the page.
    let start = match outline.commands[0] {
        mjx_scene::PathCommand::MoveTo(at) => at,
        ref other => panic!("the arc starts with {other:?}"),
    };
    let end = match outline.commands[1] {
        mjx_scene::PathCommand::CubicTo { end, .. } => end,
        ref other => panic!("the arc's second command is {other:?}"),
    };
    let centre_line = within.left + within.width() / 2.0;
    assert!(
        ((start.x - centre_line) + (end.x - centre_line)).abs() < 0.05,
        "the arc runs from {} to {}, which is not symmetric about the centre line at {centre_line} \
         — the derived ellipse centre is not the shape's own",
        start.x,
        end.x
    );
    assert!(
        (start.y - end.y).abs() < 0.05,
        "the two ends of a symmetric arc are at {} and {}",
        start.y,
        end.y
    );

    // And every point of the curve is inside the box. The *control* points are not, and that is
    // expected rather than a defect: this arc's single cubic spans 28° of parametric angle across
    // the ellipse's lowest point, and a Bézier's hull bulges about half a pixel past the curve
    // there. `common::curve_overhang` says why the two measures differ.
    let outside = common::curve_overhang(&outline.commands, within);
    assert!(
        outside < 0.05,
        "the arc leaves its box by {outside} device pixels"
    );
    assert!(
        bounds_of(&outline.commands).bottom > within.bottom,
        "the control hull of this arc is expected to bulge past the box; if it no longer does, the \
         distinction between the two overhang measures has gone and one of them should go with it"
    );
}

/// An arc of the inscribed ellipse from 45° to 135°, drawn from the point a true-angle reading puts
/// its start at — the same `cat2`/`sat2` idiom `pie` uses.
const A_DIAGONAL_ARC: PresetShapeDefinition = PresetShapeDefinition {
    preset: PresetShapeType::Pie,
    derivation: Derivation::ConstantsFromTheGeneratedTables,
    source: "a 45° to 135° arc of the inscribed ellipse, for the identity-value probe",
    adjustment_values: &[],
    guides: &[
        mjx_ooxml_types::drawingml::PresetGuide {
            wire_name: "wt1",
            formula: "sin wd2 2700000",
        },
        mjx_ooxml_types::drawingml::PresetGuide {
            wire_name: "ht1",
            formula: "cos hd2 2700000",
        },
        mjx_ooxml_types::drawingml::PresetGuide {
            wire_name: "dx1",
            formula: "cat2 wd2 ht1 wt1",
        },
        mjx_ooxml_types::drawingml::PresetGuide {
            wire_name: "dy1",
            formula: "sat2 hd2 ht1 wt1",
        },
        mjx_ooxml_types::drawingml::PresetGuide {
            wire_name: "x1",
            formula: "+- hc dx1 0",
        },
        mjx_ooxml_types::drawingml::PresetGuide {
            wire_name: "y1",
            formula: "+- vc dy1 0",
        },
    ],
    text_rectangle: None,
    connection_sites: &[],
    paths: &[PresetPath {
        width: None,
        height: None,
        fill: PathFillMode::Normal,
        stroke: true,
        extrusion_ok: true,
        steps: &[
            PresetPathStep::MoveTo(PresetPoint::at("x1", "y1")),
            PresetPathStep::ArcTo {
                width_radius: PresetCoordinate::Guide("wd2"),
                height_radius: PresetCoordinate::Guide("hd2"),
                start_angle: PresetAngle::Native(2_700_000),
                swing_angle: PresetAngle::Native(5_400_000),
            },
            PresetPathStep::Close,
        ],
    }],
};

// -------------------------------------------------------------------------------------------
// MJXOFF-204's branches
// -------------------------------------------------------------------------------------------

/// A rounded rectangle whose text rectangle is written in a **literal** rather than in a guide
/// name.
///
/// The arm of [`PresetCoordinate`] that ECMA-376's own file never takes for an `a:rect`: all 724 of
/// its edges are guide names, which `text_goes_inside_the_shape.rs` asserts rather than assumes.
/// The type models both because `ST_AdjCoordinate` does, so the untaken arm is taken here — with a
/// consequence, not merely constructed.
const A_LITERAL_TEXT_RECTANGLE: PresetShapeDefinition = PresetShapeDefinition {
    preset: PresetShapeType::Rectangle,
    derivation: Derivation::FromFirstPrinciples,
    source: "a square whose text rectangle is four literals, for the identity-value probe",
    adjustment_values: &[],
    guides: &[],
    // A quarter of the way in on every side of a 2 032 000 x 1 524 000 EMU shape.
    text_rectangle: Some(PresetTextRectangle {
        left: PresetCoordinate::Emu(508_000),
        top: PresetCoordinate::Emu(381_000),
        right: PresetCoordinate::Emu(1_524_000),
        bottom: PresetCoordinate::Emu(1_143_000),
    }),
    connection_sites: &[],
    paths: &[PresetPath {
        width: None,
        height: None,
        fill: PathFillMode::Normal,
        stroke: true,
        extrusion_ok: true,
        steps: UNIT_SQUARE,
    }],
};

/// The same square with the same rectangle written as **guide names** instead.
const A_GUIDE_NAMED_TEXT_RECTANGLE: PresetShapeDefinition = PresetShapeDefinition {
    guides: &[
        PresetGuide {
            wire_name: "il",
            formula: "*/ w 1 4",
        },
        PresetGuide {
            wire_name: "it",
            formula: "*/ h 1 4",
        },
        PresetGuide {
            wire_name: "ir",
            formula: "*/ w 3 4",
        },
        PresetGuide {
            wire_name: "ib",
            formula: "*/ h 3 4",
        },
    ],
    text_rectangle: Some(PresetTextRectangle {
        left: PresetCoordinate::Guide("il"),
        top: PresetCoordinate::Guide("it"),
        right: PresetCoordinate::Guide("ir"),
        bottom: PresetCoordinate::Guide("ib"),
    }),
    ..A_LITERAL_TEXT_RECTANGLE
};

/// The same square with a connection site whose position is written in **literals**.
///
/// The other arm ECMA-376's file never takes: all 1 712 of its site coordinates are guide names.
const A_LITERAL_CONNECTION_SITE: PresetShapeDefinition = PresetShapeDefinition {
    text_rectangle: None,
    connection_sites: &[PresetConnectionSite {
        angle: PresetAngle::Native(5_400_000),
        position: PresetPoint::new(
            PresetCoordinate::Emu(508_000),
            PresetCoordinate::Emu(381_000),
        ),
    }],
    ..A_LITERAL_TEXT_RECTANGLE
};

#[test]
fn a_text_rectangle_a_shape_declares_and_one_it_does_not_are_two_different_answers() {
    // The `Option<PresetTextRectangle>` branch of a row, taken both ways against **real** table
    // data, each with a consequence a caller can see.
    let (within, extents) = (box_on_the_page(), extents_of_the_box());

    let declared = preset_text_rectangle(PresetShapeType::RoundedRectangle, extents, &[], within)
        .expect("`roundRect` resolves");
    let absent = preset_text_rectangle(PresetShapeType::StraightLine, extents, &[], within)
        .expect("`line` resolves");

    assert_ne!(declared, absent, "the two arms answer the same value");
    // The downstream consequence: one of them has a rectangle and the other's fallback is the box,
    // so a text layout would put the two shapes' first line in different places.
    assert_ne!(
        declared.or_bounding_box(within),
        absent.or_bounding_box(within),
        "a shape with an inset text rectangle and one with none lay text in the same place"
    );
    assert_eq!(absent.or_bounding_box(within), within);
}

#[test]
fn a_literal_text_rectangle_and_a_guide_named_one_are_two_ways_to_the_same_place() {
    // Both arms of `PresetCoordinate` inside an `a:rect` — the literal one, which the spec file
    // never takes, and the guide-named one, which it takes 724 times. Written to describe the
    // *same* rectangle, so the assertion is that the two roads meet: a resolver that ignored the
    // literal arm, or scaled it as though it were a fraction, would land somewhere else.
    let (within, extents) = (box_on_the_page(), extents_of_the_box());
    let literal = text_rectangle_of_definition(&A_LITERAL_TEXT_RECTANGLE, extents, &[], within)
        .expect("a literal rectangle resolves");
    let named = text_rectangle_of_definition(&A_GUIDE_NAMED_TEXT_RECTANGLE, extents, &[], within)
        .expect("a guide-named rectangle resolves");
    assert_eq!(
        literal, named,
        "the same rectangle written two ways resolved to two places"
    );

    // ...and it is not the box, so the comparison is not two fallbacks agreeing with each other.
    let TextRectangle::Declared(rectangle) = literal else {
        panic!("a literal rectangle answered {literal:?}");
    };
    assert!(
        (rectangle.left - within.left - 40.0).abs() < 0.01,
        "a quarter of a 160-pixel box is 40 px in, and this is {}",
        rectangle.left - within.left
    );
}

#[test]
fn a_literal_site_coordinate_and_a_guide_named_one_are_two_ways_to_the_same_point() {
    // The same probe for `a:cxn`'s position. `PresetPoint::new` is the constructor the generated
    // table uses wherever either coordinate is a literal — which for connection sites is never, so
    // this is the case that keeps the arm honest.
    let (within, extents) = (box_on_the_page(), extents_of_the_box());
    let sites = connection_sites_of_definition(&A_LITERAL_CONNECTION_SITE, extents, &[], within)
        .expect("a literal site resolves");
    assert_eq!(sites.len(), 1);
    // A quarter of the way in on both axes: 40 px of 160, 30 px of 120.
    assert!(
        (sites[0].position.x - within.left - 40.0).abs() < 0.01
            && (sites[0].position.y - within.top - 30.0).abs() < 0.01,
        "a literal site landed at {:?}, not a quarter of the way into {within:?}",
        sites[0].position
    );
    // The angle's literal arm, with its own consequence: 5 400 000 in the wire scale is 90 degrees.
    assert!((sites[0].angle.degrees() - 90.0).abs() < 1e-9);

    // And the guide-named arm, from the committed table, answering a *different* number — so the
    // two arms are not both quietly returning zero.
    let named = preset_connection_sites(PresetShapeType::Rectangle, extents, &[], within)
        .expect("`rect` resolves");
    assert!((named[0].angle.degrees() - 270.0).abs() < 1e-9);
    assert_ne!(named[0].position, sites[0].position);
}

#[test]
fn a_portrait_box_and_a_landscape_one_are_two_values_of_the_shorter_side() {
    // **"The box is landscape" is an identity value, and it was never varied.** `ss` is
    // `min(w, h)`, so in a landscape box the shorter side is always the height — and every box and
    // every non-degenerate extent this crate had was landscape or square. An implementation that
    // read `h` where a formula says `ss` produced identical numbers in every gate.
    //
    // The two boxes below have the *same* `ss` (120 points) and differ only in which side it is, so
    // a quantity written in `ss` must come out the same in both. `roundRect`'s text inset is such a
    // quantity — `x1 = ss·adj/100000`, inset `= x1·29289/100000` — and
    // `text_goes_inside_the_shape.rs` pins its value in both orientations. Here is the branch
    // itself, with the consequence that separates the three readings.
    let landscape = preset_text_rectangle(
        PresetShapeType::RoundedRectangle,
        extents_of_the_box(),
        &[],
        box_on_the_page(),
    )
    .expect("`roundRect` resolves in a landscape box");
    let portrait = preset_text_rectangle(
        PresetShapeType::RoundedRectangle,
        portrait_extents(),
        &[],
        portrait_box_on_the_page(),
    )
    .expect("`roundRect` resolves in a portrait box");

    let (TextRectangle::Declared(landscape_rectangle), TextRectangle::Declared(portrait_rectangle)) =
        (&landscape, &portrait)
    else {
        panic!("`roundRect` lost its text rectangle in one of the two boxes");
    };
    let landscape_inset = landscape_rectangle.left - box_on_the_page().left;
    let portrait_inset = portrait_rectangle.left - portrait_box_on_the_page().left;

    // Equal, because `ss` is the same in both — and *not* equal to what either side alone would
    // give, which is what makes this a probe rather than a restatement.
    assert!(
        (landscape_inset - portrait_inset).abs() < 0.001,
        "the inset is {landscape_inset} px landscape and {portrait_inset} px portrait; a quantity \
         written in `ss` cannot differ between two boxes with the same shorter side"
    );
    assert!(
        (landscape_inset - 7.811).abs() > 0.5,
        "the inset is what the *longer* side would give, so `ss` is being read as `w` or `h`"
    );

    // The downstream consequence: the two boxes really are different boxes, so this is not two
    // identical resolutions agreeing with each other.
    assert!(
        (box_on_the_page().width() - portrait_box_on_the_page().width()).abs() > 1.0,
        "the two orientations are the same box"
    );
    assert_ne!(
        landscape, portrait,
        "the two rectangles are the same rectangle"
    );
}
