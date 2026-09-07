//! The differential: a hand transcription from ECMA-376's prose against a mechanical extraction
//! from ECMA-376's XML.
//!
//! # What this is worth, shape by shape — read before believing it
//!
//! MJXOFF-202 hand-wrote six presets from the spec's descriptions, before `References/` existed in
//! this worktree, **specifically so that MJXOFF-203's extraction could be checked against something
//! that did not come from the same file.** That is the strongest gate available to a child whose
//! output is 3 923 guide formulas nobody reads.
//!
//! It is also weaker than "six agreements" sounds, and MJXOFF-202 said so itself rather than
//! letting this child bank six confirmations. Each seed row carries a [`Derivation`] and explains
//! itself in prose; [`each_shape_reports_what_its_agreement_is_worth`] prints the classification
//! beside every result so a reader of the test output sees it too.
//!
//! | Shape | What the agreement is worth |
//! |---|---|
//! | `rect` | **Full independence, and it proves the least** — there is one way to draw a rectangle. |
//! | `ellipse` | **Structural coincidence.** Four 90° `a:arcTo` quadrants clockwise from `(l, vc)` is the only structure DrawingML's arc semantics make natural. MJXOFF-202 predicted the file would use it, and it does. This is not a second measurement. |
//! | `triangle` | Paths independent; the `adj` domain came from the generated `adjustments_of`. |
//! | `roundRect` | Paths independent; the `adj` domain came from `adjustments_of`. |
//! | `rightArrow` | **Strong.** Seven points and eight guides, including `dy1 = */ h a1 200000` — the row MJXOFF-202 named as its own weakest point, because its monotonicity sweep asserts the direction its own derivation implies. The file writes the same eight formulas and the same seven points, in the same order. |
//! | `pie` | **Strong on the paths, and it disagreed structurally**, which is the interesting result. See below. |
//!
//! # What disagreed, and how it was settled
//!
//! Two of the six differ from the file in how they *write* the same outline. Neither is an error in
//! either route, and both are the reason this suite compares **resolved outlines and never step
//! lists or guide names** — the comparison MJXOFF-202 specified in advance, for exactly these two
//! reasons:
//!
//! * **`pie` writes its wedge the other way round.** The file draws
//!   `moveTo(rim) → arcTo → lnTo(hc, vc) → close`; the seed draws
//!   `moveTo(hc, vc) → lnTo(rim) → arcTo → close`. Same region, rotated start point. MJXOFF-202
//!   predicted this specific difference before seeing the file.
//! * **`triangle` names its apex differently.** The file's apex guide is `x2`; the seed called it
//!   `x1`. Worse than a rename: the file *also* has an `x1`, and it is a different formula
//!   (`*/ w a 200000`, half the apex offset, for the text rectangle). A diff on guide names would
//!   have reported a contradiction where there is agreement to the EMU.
//!
//! Everything else agrees step for step. Where the two routes genuinely disagreed the file would
//! win — it is the normative artefact and a transcription is not — but on these six they did not.
//!
//! # Why the comparison is a Hausdorff distance
//!
//! Two outlines that draw the same region can differ in start point, in direction, and in how many
//! vertices they place along a straight edge. A point-by-point diff reports all three as failures.
//! The distance below is symmetric, point-to-**segment**, over flattened contours, so it answers
//! *"is every point either route draws on the other route's outline?"* — which is the question, and
//! it is scale-honest: it comes out in device pixels, in the same box every other suite measures
//! in.

mod common;

use common::{box_on_the_page, extents_of_the_box, flattened};
use mjx_geometry::{
    adjustment_domains, definition_of, outline_of_definition, seed::HAND_TRANSCRIBED_SHAPES,
    AdjustmentOverride, Derivation, PresetPath, PresetPathStep, PresetPoint, PresetShapeDefinition,
    PresetShapeType,
};
use mjx_ooxml_types::drawingml::{PathFillMode, PresetGuide};
use mjx_scene::ScenePoint;

/// How far apart, in device pixels, the two routes' outlines may lie.
///
/// The same number as `a_preset_renders_as_itself.rs`'s box tolerance and for the same reasons —
/// half an EMU of coordinate rounding and one `f32` narrowing, a hundred times over. It is not a
/// fudge factor for a difference of opinion between the routes: on all six shapes the measured
/// distance is *zero to within the printout below*, because where the two routes agree they agree
/// exactly. [`a_wrong_seed_row_is_caught_by_this_comparison`] shows what a real slip measures.
const AGREEMENT_TOLERANCE_PIXELS: f32 = 0.01;

/// Six shapes, and the six results are reported individually.
#[test]
fn each_shape_reports_what_its_agreement_is_worth() {
    let (within, extents) = (box_on_the_page(), extents_of_the_box());
    assert_eq!(
        HAND_TRANSCRIBED_SHAPES.len(),
        6,
        "the hand-written reference is no longer six shapes"
    );

    let mut worst = 0.0f32;
    for hand in HAND_TRANSCRIBED_SHAPES {
        let token = hand.preset.to_wire();
        let generated = definition_of(hand.preset)
            .unwrap_or_else(|| panic!("the generated table has no `{token}`"));
        assert_eq!(
            generated.derivation,
            Derivation::ExtractedFromTheGeometryFile,
            "`{token}`'s generated row does not claim to have come from the file"
        );
        assert_ne!(
            hand.derivation,
            Derivation::ExtractedFromTheGeometryFile,
            "`{token}`'s hand-written row claims to have come from the file, which would make this \
             comparison self-referential"
        );

        for (label, adjustments) in adjustment_cases(hand.preset, &[]) {
            let mine = outline_of_definition(hand, extents, &adjustments, within).unwrap_or_else(
                |error| panic!("the hand-written `{token}` did not resolve at {label}: {error}"),
            );
            let theirs = outline_of_definition(generated, extents, &adjustments, within)
                .unwrap_or_else(|error| {
                    panic!("the generated `{token}` did not resolve at {label}: {error}")
                });
            let distance = outline_distance(&mine.commands, &theirs.commands);
            worst = worst.max(distance);
            println!(
                "{token:>12} @ {label:<9} {distance:>9.5} px   ({:?})",
                hand.derivation
            );
            assert!(
                distance <= AGREEMENT_TOLERANCE_PIXELS,
                "`{token}` at {label}: the hand-written outline and the generated one are \
                 {distance} device pixels apart, past the {AGREEMENT_TOLERANCE_PIXELS} tolerance. \
                 The file wins — the generated row is the normative artefact and the seed is \
                 somebody reading prose — so the seed row is what to correct.\n\
                 The seed says it was derived thus: {}",
                hand.source
            );
        }
    }
    println!("worst disagreement across the six shapes: {worst} device pixels");
}

#[test]
fn the_two_routes_are_two_routes() {
    // The comparison is worth nothing if both sides are the same data. Three things say they are
    // not, and all three are structural rather than a matter of trust.
    for hand in HAND_TRANSCRIBED_SHAPES {
        let token = hand.preset.to_wire();
        let generated = definition_of(hand.preset).expect("in the generated table");
        assert!(
            !std::ptr::eq(hand, generated),
            "`{token}`'s two rows are the same row"
        );
        assert!(
            hand.source.contains("§20.1.10.56"),
            "`{token}`'s hand-written row cites no clause of the prose it was read from"
        );
        assert!(
            generated.source.contains("presetShapeDefinitions.xml"),
            "`{token}`'s generated row does not name the file it was read from"
        );
    }

    // And the two really do differ: `pie` and `triangle` disagree on how the outline is written,
    // which a diff of step lists would have reported as failures. Asserted, so that a later change
    // that quietly made the seed a copy of the file would fail here rather than pass everywhere.
    let pie = definition_of(PresetShapeType::Pie).expect("in the generated table");
    let hand_pie = HAND_TRANSCRIBED_SHAPES
        .iter()
        .find(|row| row.preset == PresetShapeType::Pie)
        .expect("hand-written");
    assert_ne!(
        step_kinds(pie),
        step_kinds(hand_pie),
        "`pie`'s two rows now write the same step sequence; MJXOFF-202 predicted they would not, \
         and if they do the seed has been edited to match the file"
    );

    let triangle = definition_of(PresetShapeType::Triangle).expect("in the generated table");
    let hand_triangle = HAND_TRANSCRIBED_SHAPES
        .iter()
        .find(|row| row.preset == PresetShapeType::Triangle)
        .expect("hand-written");
    let file_x1 = triangle
        .guides
        .iter()
        .find(|guide| guide.wire_name == "x1")
        .expect("the file's triangle has an x1");
    let seed_x1 = hand_triangle
        .guides
        .iter()
        .find(|guide| guide.wire_name == "x1")
        .expect("the seed's triangle has an x1");
    assert_ne!(
        file_x1.formula, seed_x1.formula,
        "`triangle`'s `x1` now reads the same in both rows; the whole point of comparing resolved \
         outlines rather than guide names is that it does not"
    );
}

/// A `rightArrow` whose shaft is twice as thick as the file says — one digit changed.
///
/// `dy1` is half the shaft's thickness, hence the `200000` divisor MJXOFF-202 called its own
/// sharpest case. `100000` is the slip a transcription actually makes, and it is invisible to every
/// other gate in this crate: the shape still closes, still fills its box, still has seven points,
/// and still moves the right way when its adjustment moves. Only the differential sees it.
const A_RIGHT_ARROW_WITH_THE_WRONG_DIVISOR: PresetShapeDefinition = PresetShapeDefinition {
    preset: PresetShapeType::RightArrow,
    derivation: Derivation::FromFirstPrinciples,
    source: "a deliberately wrong `rightArrow`, for this gate alone",
    adjustment_values: &[
        PresetGuide {
            wire_name: "adj1",
            formula: "val 50000",
        },
        PresetGuide {
            wire_name: "adj2",
            formula: "val 50000",
        },
    ],
    guides: &[
        PresetGuide {
            wire_name: "maxAdj2",
            formula: "*/ 100000 w ss",
        },
        PresetGuide {
            wire_name: "a1",
            formula: "pin 0 adj1 100000",
        },
        PresetGuide {
            wire_name: "a2",
            formula: "pin 0 adj2 maxAdj2",
        },
        PresetGuide {
            wire_name: "dx1",
            formula: "*/ ss a2 100000",
        },
        PresetGuide {
            wire_name: "x1",
            formula: "+- r 0 dx1",
        },
        // The slip: `200000` in the file and in the seed, `100000` here.
        PresetGuide {
            wire_name: "dy1",
            formula: "*/ h a1 100000",
        },
        PresetGuide {
            wire_name: "y1",
            formula: "+- vc 0 dy1",
        },
        PresetGuide {
            wire_name: "y2",
            formula: "+- vc dy1 0",
        },
    ],
    paths: &[PresetPath {
        width: None,
        height: None,
        fill: PathFillMode::Normal,
        stroke: true,
        extrusion_ok: true,
        steps: &[
            PresetPathStep::MoveTo(PresetPoint::at("l", "y1")),
            PresetPathStep::LineTo(PresetPoint::at("x1", "y1")),
            PresetPathStep::LineTo(PresetPoint::at("x1", "t")),
            PresetPathStep::LineTo(PresetPoint::at("r", "vc")),
            PresetPathStep::LineTo(PresetPoint::at("x1", "b")),
            PresetPathStep::LineTo(PresetPoint::at("x1", "y2")),
            PresetPathStep::LineTo(PresetPoint::at("l", "y2")),
            PresetPathStep::Close,
        ],
    }],
};

#[test]
fn a_wrong_seed_row_is_caught_by_this_comparison() {
    // A comparison that passes is only evidence once it is shown to be capable of failing. This is
    // the exhibit: the same shape with one digit changed, against the same generated row, measured
    // by the same function.
    let (within, extents) = (box_on_the_page(), extents_of_the_box());
    let generated = definition_of(PresetShapeType::RightArrow).expect("in the generated table");

    let wrong = outline_of_definition(&A_RIGHT_ARROW_WITH_THE_WRONG_DIVISOR, extents, &[], within)
        .expect("the wrong shape still resolves, which is the point");
    let right = outline_of_definition(generated, extents, &[], within).expect("the file's shape");
    let slip = outline_distance(&wrong.commands, &right.commands);
    println!("a one-digit slip in `rightArrow`'s `dy1` measures {slip} device pixels");
    assert!(
        slip > AGREEMENT_TOLERANCE_PIXELS * 100.0,
        "a doubled shaft thickness measures only {slip} device pixels, which is not a distance the \
         tolerance would catch — the comparison is not measuring the geometry"
    );

    // And the slip really is invisible to the structural gates, which is why the differential is
    // the one that has to catch it: seven points and a close, either way.
    assert_eq!(
        wrong.commands.len(),
        right.commands.len(),
        "the two right arrows no longer have the same number of steps, so a count would catch this"
    );
}

// -------------------------------------------------------------------------------------------
// The measurement
// -------------------------------------------------------------------------------------------

/// The adjustment settings both routes are compared at: the defaults, and each adjustment at each
/// end of its domain with the others left alone.
fn adjustment_cases(
    preset: PresetShapeType,
    _: &[AdjustmentOverride],
) -> Vec<(String, Vec<AdjustmentOverride>)> {
    let extents = extents_of_the_box();
    let mut cases = vec![("defaults".to_owned(), Vec::new())];
    let Ok(domains) = adjustment_domains(preset, extents, &[]) else {
        return cases;
    };
    for domain in domains {
        for (edge, value) in [("min", domain.minimum), ("max", domain.maximum)] {
            cases.push((
                format!("{}={edge}", domain.spec.wire_name),
                vec![AdjustmentOverride::new(domain.spec.wire_name, value)],
            ));
        }
    }
    cases
}

/// The step kinds of a definition's first path, as a string — enough to compare two step sequences
/// without comparing the coordinates in them.
fn step_kinds(definition: &PresetShapeDefinition) -> String {
    definition
        .paths
        .iter()
        .flat_map(|path| path.steps)
        .map(|step| match step {
            PresetPathStep::Close => "close",
            PresetPathStep::MoveTo(_) => "moveTo",
            PresetPathStep::LineTo(_) => "lnTo",
            PresetPathStep::ArcTo { .. } => "arcTo",
            PresetPathStep::QuadBezierTo { .. } => "quadBezTo",
            PresetPathStep::CubicBezierTo { .. } => "cubicBezTo",
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The symmetric Hausdorff distance between two outlines, in device pixels.
///
/// Point-to-**segment**, over flattened contours, in both directions: *"how far is the furthest
/// point either outline draws from the other outline?"*. Insensitive to start point, to direction
/// and to where vertices fall along a shared edge, all three of which differ legitimately between
/// the two routes.
fn outline_distance(left: &[mjx_scene::PathCommand], right: &[mjx_scene::PathCommand]) -> f32 {
    let (left, right) = (flattened(left), flattened(right));
    one_way(&left, &right).max(one_way(&right, &left))
}

/// The furthest any point of `from` lies from the nearest segment of `to`.
fn one_way(from: &[Vec<ScenePoint>], to: &[Vec<ScenePoint>]) -> f32 {
    let mut worst = 0.0f32;
    for point in from.iter().flatten() {
        let mut nearest = f32::MAX;
        for contour in to {
            for pair in contour.windows(2) {
                nearest = nearest.min(distance_to_segment(*point, pair[0], pair[1]));
            }
            // A flattened contour is a closed ring when the outline closed it; the closing segment
            // is not in `windows(2)`, so it is added here. Leaving it out would report a spurious
            // distance for a point on an edge the other route draws by its `a:close`.
            if let (Some(first), Some(last)) = (contour.first(), contour.last()) {
                nearest = nearest.min(distance_to_segment(*point, *last, *first));
            }
        }
        worst = worst.max(nearest);
    }
    worst
}

/// How far `point` is from the segment `start`–`end`.
fn distance_to_segment(point: ScenePoint, start: ScenePoint, end: ScenePoint) -> f32 {
    let (dx, dy) = (end.x - start.x, end.y - start.y);
    let length_squared = dx * dx + dy * dy;
    let t = if length_squared <= f32::EPSILON {
        0.0
    } else {
        (((point.x - start.x) * dx + (point.y - start.y) * dy) / length_squared).clamp(0.0, 1.0)
    };
    let (nearest_x, nearest_y) = (start.x + t * dx, start.y + t * dy);
    ((point.x - nearest_x).powi(2) + (point.y - nearest_y).powi(2)).sqrt()
}
