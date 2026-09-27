//! Every adjustment of every preset, swept from its minimum to its maximum — 285 of them, in two
//! orientations.
//!
//! # Why this file exists, and what it replaces
//!
//! `an_adjustment_moves_the_shape.rs` states a direction in the words a shape's own definition uses
//! and asserts the geometry moves that way. It is the sharpest gate in the crate and it covers
//! **four shapes**. MJXOFF-203's hand-off said the suite would cover 186 "the moment the table
//! grows"; it did not, because the only loop over `seeded_shapes()` there is the degenerate-size
//! sweep, which asks whether a resolution *survives* and never whether it *moved*. For 182 presets
//! no adjustment had ever been checked against a geometric expectation.
//!
//! This is that check, and it is mechanical rather than hand-written, because 285 hand-written
//! directions would be 285 opinions of the same author who wrote the table. Four questions, each
//! answerable without a reference render and each falsifiable by a shape moving into or out of a
//! named list:
//!
//! 1. **Does it move at all?** [`every_adjustment_of_every_preset_moves_the_geometry`] — 285
//!    adjustments, no exceptions, in both orientations. An adjustment wired to a guide nothing
//!    reads, or dropped by the extractor, is a shape that does not move.
//! 2. **Does it move on the axis it says it does?**
//!    [`an_adjustments_declared_axis_is_the_axis_the_geometry_moves_on`] — and the expectation comes
//!    from `a:ahLst`, **a part of ECMA-376's file the path table does not read**. That is what makes
//!    it a differential rather than a restatement.
//! 3. **Does the shape stay in its box while it moves?**
//!    [`a_bounded_adjustment_keeps_its_shape_in_its_box_except_where_named`] — 97 shapes with a
//!    bounded domain, eleven named exceptions with measured bounds, and the rest inside 0.0053 px.
//! 4. **Where does the shape's own arithmetic run out?**
//!    [`the_adjustments_singular_at_a_domain_endpoint_are_named`] — three, each at a bound a handle
//!    drag reaches.
//!
//! # What this found
//!
//! Question 3 found a defect in this crate's own arc decomposition that every existing gate passed
//! over: [`mjx_geometry::parametric_angle`] tested for a degenerate ellipse by asking whether
//! `wR·sin θ` and `hR·cos θ` were *both exactly zero*, and `sin π` is `1.22e-16`. So an `a:arcTo`
//! on an ellipse with one radius zero — which is what `can` and `leftBracket` draw at `adj = 0`,
//! their adjustment's own minimum — derived its centre a whole `wR` away from the pen. `can` drew
//! its top ellipse 80 device pixels left of a box beginning at zero and `leftBracket` drew a full
//! 160 outside a 160-pixel box. **Nothing else in the crate saw it**: the box census resolves at
//! default adjustments, the four monotonicity cases are four other shapes, and the degenerate-size
//! sweep asserts finiteness rather than position. The fix is in `arc.rs` and
//! `a_degenerate_ellipse_keeps_its_arc_where_the_pen_is` is the unit that pins it.
//!
//! # What this deliberately does not claim
//!
//! It does not claim a shape looks like its name. Question 2 says the geometry moved on the axis the
//! file's adjust handle declares, which is a strong statement about wiring and no statement at all
//! about appearance. The authoritative visual check is PowerPoint's and it is the user's
//! (MJXOFF-155 §9 #5).

mod common;

use std::collections::{BTreeMap, BTreeSet};

use common::{curve_overhang, orientations, points_of};
use mjx_geometry::{
    adjustment_domains, preset_outline, seeded_shapes, AdjustmentOverride, GeometryError,
    PresetShapeType, Size,
};
use mjx_ooxml_types::drawingml::{AdjustmentAxis, AdjustmentBound};
use mjx_scene::{PathCommand, SceneRect};

// -------------------------------------------------------------------------------------------
// The numbers, and where each of them comes from
// -------------------------------------------------------------------------------------------

/// How many values a sweep takes across an adjustment's domain, endpoints included.
///
/// Nine rather than two, for `an_adjustment_moves_the_shape.rs`'s reason: the endpoints alone pass
/// on a shape that moves the right way overall and the wrong way in the middle. Nine also puts a
/// sample at each eighth of the domain, which is where `can`'s and `leftBracket`'s arc defect sat —
/// at the very first one.
const SWEEP_SAMPLES: usize = 9;

/// How many presets of the 186 declare at least one `a:avLst` entry an adjust handle references.
///
/// Sixty-seven declare none, and every one of them is a shape with nothing to drag: `rect`,
/// `ellipse`, the flow-chart symbols, the twelve action buttons.
const SHAPES_WITH_AN_ADJUSTMENT: usize = 119;

/// How many (shape, adjustment) pairs the table has in total — the size of this file's sweep.
///
/// Two hundred and eighty-five, against the four `an_adjustment_moves_the_shape.rs` states a
/// direction for by hand. Asserted as a literal so that an adjustment lost from
/// [`adjustments_of`](mjx_ooxml_types::drawingml::adjustments_of) fails here rather than shrinking
/// the sweep in silence.
const ADJUSTMENTS_IN_THE_TABLE: usize = 285;

/// How many of the 285 declare a Cartesian axis — the ones question 2 can ask about.
///
/// The other 27 are [`AdjustmentAxis::Angle`] and [`AdjustmentAxis::Radius`], from an `a:ahPolar`
/// handle. A polar handle makes no claim about `x` versus `y` and the gate does not invent one.
const ADJUSTMENTS_WITH_A_CARTESIAN_AXIS: usize = 258;

/// How far outside its own box a point of a swept outline may lie before the shape counts as
/// leaving it, **in device pixels**.
///
/// The same unit and the same three error sources as `a_preset_renders_as_itself.rs`'s
/// `BOX_TOLERANCE_PIXELS` — half an EMU of coordinate rounding, one narrowing to `f32`, and nothing
/// from the arc decomposition at a quadrant boundary — and the same number.
///
/// **It is shown to fail at half of it, by the data rather than by a contrivance.** The worst shape
/// that stays inside its box across its whole adjustment domain measures `0.00528` px, so a
/// tolerance of `0.005` would fail a correct shape;
/// [`a_bounded_adjustment_keeps_its_shape_in_its_box_except_where_named`] asserts exactly that, in
/// the direction that makes the number a measurement rather than decoration. The reason a sweep
/// needs the full hundredth where the default-adjustment census of
/// `every_preset_stands_where_its_box_is.rs` has room to spare is arithmetic and not slack: a sweep
/// visits arc angles that are not quadrant boundaries, so the decomposition's own error — zero at a
/// quadrant — is in play at every sample.
const SWEEP_OVERHANG_TOLERANCE_PIXELS: f32 = 0.01;

/// How far the worst shape that *stays* inside its box reaches outside it across its whole domain.
///
/// Measured, quoted to five decimal places, and asserted from both sides:
/// [`SWEEP_OVERHANG_TOLERANCE_PIXELS`] must be above it or a correct shape fails, and half the
/// tolerance must be below it or the tolerance is passing everything.
const FURTHEST_A_SHAPE_INSIDE_ITS_BOX_REACHES: f32 = 0.00529;

/// How much more one axis must move than the other before the motion counts as *on* that axis.
///
/// **The number is not arbitrary and it is not tuned to make a list short.** At 1.25 the census has
/// 32 contradictions, at 1.5 it has 8, at 2.0 it has 8 — so the population is bimodal, with a gap
/// between 1.5 and 2.0 that nothing lives in. Anything below 1.5 is a shape whose adjustment moves
/// both axes together (a corner radius, a bevel, a mitre), which is a third answer rather than a
/// contradiction, and [`Motion::Both`] is what it is called.
const AXIS_DOMINANCE_RATIO: f64 = 1.5;

/// The value ECMA-376's `a:ahLst` writes for *"this handle has no stop"*.
///
/// `i32::MAX`, and it is the schema's way of saying unbounded rather than a real limit: a callout's
/// tail may be dragged anywhere on the slide. Sweeping to it is meaningful — the coordinates stay
/// finite and 3 435 973.8 device pixels is what a 2 147 483 647 fraction of a 160-pixel box *is* —
/// but "the shape stays in its box" is not a claim anybody makes about such an adjustment, so
/// [`a_bounded_adjustment_keeps_its_shape_in_its_box_except_where_named`] excludes them **by this
/// test rather than by name**, and [`the_unbounded_adjustments_are_the_callouts_and_the_connectors`]
/// says which shapes that turns out to be.
const UNBOUNDED_DOMAIN: i32 = i32::MAX;

// -------------------------------------------------------------------------------------------
// The three lists
// -------------------------------------------------------------------------------------------

/// Which way a shape moved when one of its adjustments was swept.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Motion {
    /// Horizontal travel was at least [`AXIS_DOMINANCE_RATIO`] times the vertical.
    Across,
    /// Vertical travel was at least [`AXIS_DOMINANCE_RATIO`] times the horizontal.
    Down,
    /// Neither dominated — the adjustment moves both axes together.
    Both,
}

impl Motion {
    /// The motion an adjust handle on `axis` predicts, or `None` for a polar handle.
    fn declared(axis: AdjustmentAxis) -> Option<Self> {
        match axis {
            AdjustmentAxis::Horizontal => Some(Self::Across),
            AdjustmentAxis::Vertical => Some(Self::Down),
            AdjustmentAxis::Angle | AdjustmentAxis::Radius => None,
        }
    }

    /// Whether a measured motion contradicts a declared one. [`Both`](Self::Both) never does.
    fn contradicts(self, declared: Self) -> bool {
        self != Self::Both && self != declared
    }
}

/// The eight (shape, adjustment) pairs whose geometry moves **across** the axis their adjust handle
/// declares.
///
/// Every one is a shape whose handle drags along one axis and whose *geometry* is arranged along the
/// other, and none of them is a wiring error — which is the point of naming them individually rather
/// than widening [`AXIS_DOMINANCE_RATIO`] until the list empties.
///
/// * **`bracePair`** — the handle is `ahXY@gdRefY` (a vertical drag on the corner-arc size) and the
///   two braces' spines are vertical, so growing the arc moves the tips **sideways**.
/// * **`horizontalScroll`** and **`verticalScroll`** — each is the other's transpose, and each
///   moves across its own name: a horizontal scroll's curls are at its left and right ends, so
///   growing them moves the top and bottom flaps up and down.
/// * **`leftRightUpArrow`** `adj1`, `adj2` — a three-headed arrow. Both are head and shaft
///   *thicknesses* written in `ss` and dragged horizontally, and the head they size most of is the
///   one pointing **up**.
/// * **`leftRightUpArrow`** `adj3` is the mirror image: a vertical drag sizing the two horizontal
///   heads.
/// * **`mathDivide`** `adj3` is the bar's length; lengthening the bar pushes the two dots apart
///   vertically by more than it extends the bar.
/// * **`star6`** is the one that is an **aspect ratio** rather than a shape: a hexagram's six points
///   are at 0°, 60°, … so the motion is genuinely diagonal, and it reads as horizontal in a 160 × 120
///   box (1.78 : 1) and as exactly balanced in the 120 × 160 one. That is why the measurement is
///   recorded as a pair and not as one number.
///
/// The columns are the landscape motion and the portrait motion, and they differ for `star6` alone.
const MOVES_ACROSS_ITS_DECLARED_AXIS: &[(&str, &str, Motion, Motion)] = &[
    ("bracePair", "adj", Motion::Across, Motion::Across),
    ("horizontalScroll", "adj", Motion::Down, Motion::Down),
    ("leftRightUpArrow", "adj1", Motion::Down, Motion::Down),
    ("leftRightUpArrow", "adj2", Motion::Down, Motion::Down),
    ("leftRightUpArrow", "adj3", Motion::Across, Motion::Across),
    ("mathDivide", "adj3", Motion::Down, Motion::Down),
    ("star6", "adj", Motion::Across, Motion::Both),
    ("verticalScroll", "adj", Motion::Across, Motion::Across),
];

/// The presets that reach outside their own box somewhere in a **bounded** adjustment's domain,
/// with how far they reach in each orientation.
///
/// Eleven, against the eighteen that already spill at their *default* adjustments
/// (`every_preset_stands_where_its_box_is.rs`'s `REACHES_OUTSIDE_ITS_BOX`) — and the two lists
/// barely overlap, because a callout spills by default and has an unbounded handle, while these
/// spill only when a bounded handle is pushed to an end of its range.
///
/// Each is ECMA-376's own arithmetic with no `pin` on the result:
///
/// * **`teardrop`** — `adj` runs to 200 000, twice full scale, and the tail is *meant* to leave: at
///   the maximum it reaches half the box past two edges.
/// * **`bentUpArrow`** — at `adj2 = 0` the shaft has no width, so the arrowhead's half-width `dx2`
///   is added to a right edge that is already `r`. Fifteen pixels, and the file writes no `pin`.
/// * **`star5`, `star6`, `star7`, `star10`** — the star presets carry `hf`/`vf` shape factors above
///   100 000 in their own `a:avLst`, which deliberately scale the circumscribed ellipse past the
///   box so the star *looks* the right size; at an extreme point depth the points follow.
/// * **`curvedUpArrow`, `curvedDownArrow`, `curvedLeftArrow`, `swooshArrow`** — arrowheads sized
///   in `ss` against a curve whose radius is sized in the other side.
/// * **`mathNotEqual`** — in portrait only, where the slash's own length outgrows the box it is
///   struck through.
///
/// `curvedLeftArrow` is here at **0.0117** px, which is a hundredth of a pixel and only just past
/// the tolerance. It is kept in the list rather than absorbed by widening the tolerance, because a
/// tolerance chosen to swallow the smallest exception swallows the next defect of that size too.
const LEAVES_ITS_BOX_SOMEWHERE_IN_A_BOUNDED_DOMAIN: &[(&str, f32, f32)] = &[
    ("bentUpArrow", 15.0000, 15.0000),
    ("curvedDownArrow", 12.6978, 14.0689),
    ("curvedLeftArrow", 0.0117, 0.0178),
    ("curvedUpArrow", 12.6978, 14.0689),
    ("mathNotEqual", 0.0000, 11.6847),
    ("star10", 4.1168, 3.0876),
    ("star5", 12.6684, 16.8912),
    ("star6", 12.3760, 9.2820),
    ("star7", 6.2520, 8.3360),
    ("swooshArrow", 5.0703, 6.1970),
    ("teardrop", 80.0000, 80.0000),
];

/// The (shape, adjustment) pairs whose own formulas have no finite value **at an endpoint of that
/// adjustment's own domain**, with the guide that loses its value.
///
/// A strict subset of `an_adjustment_moves_the_shape.rs`'s `SINGULAR_SOMEWHERE`, and the difference
/// is the point: that list is measured by crossing every preset with values *outside* every domain,
/// while these three are reached by dragging one handle to its own stop. `curvedDownArrow`,
/// `curvedUpArrow` and `noSmoking` are in that list and not in this one — their singular points are
/// outside the domain their handle can reach.
///
/// Every one answers [`GeometryError::SingularGeometry`], which
/// [`GeometryError::has_no_geometry_to_draw`] classifies as *"there is nothing to draw"*, so a
/// provider stands in for it with a counted placeholder rather than failing the page.
const SINGULAR_AT_A_DOMAIN_ENDPOINT: &[(&str, &str, &str)] = &[
    ("circularArrow", "adj5", "swAng"),
    ("leftCircularArrow", "adj5", "istAng"),
    ("leftRightCircularArrow", "adj5", "xKp"),
];

/// The presets with at least one adjustment whose domain is [`UNBOUNDED_DOMAIN`].
///
/// Twenty-two, and every one of them is a callout or a connector — a shape whose handle names a
/// point somewhere else on the slide rather than a proportion of itself. Asserted by name so that a
/// shape acquiring an unbounded handle, which would silently leave the in-the-box census, fails
/// here instead.
const UNBOUNDED_DOMAIN_SHAPES: &[&str] = &[
    "accentBorderCallout1",
    "accentBorderCallout2",
    "accentBorderCallout3",
    "accentCallout1",
    "accentCallout2",
    "accentCallout3",
    "bentConnector3",
    "bentConnector4",
    "bentConnector5",
    "borderCallout1",
    "borderCallout2",
    "borderCallout3",
    "callout1",
    "callout2",
    "callout3",
    "cloudCallout",
    "curvedConnector3",
    "curvedConnector4",
    "curvedConnector5",
    "wedgeEllipseCallout",
    "wedgeRectCallout",
    "wedgeRoundRectCallout",
];

// -------------------------------------------------------------------------------------------
// The sweep itself
// -------------------------------------------------------------------------------------------

/// One adjustment's whole domain, resolved into outlines — or the failure that stopped it.
struct Sweep {
    /// The outline at each sample that resolved, in order.
    outlines: Vec<Vec<PathCommand>>,
    /// The guide that had no finite value, at the first sample where one did not.
    singular: Option<String>,
    /// The furthest any sample reached outside the box.
    furthest_outside: f32,
}

/// Sweep one adjustment of one shape across its own domain, holding every other at its default.
fn sweep(
    preset: PresetShapeType,
    wire_name: &'static str,
    from: f64,
    to: f64,
    extents: Size,
    within: SceneRect,
) -> Sweep {
    let mut outlines = Vec::with_capacity(SWEEP_SAMPLES);
    let mut singular = None;
    let mut furthest_outside = 0.0f32;
    for index in 0..SWEEP_SAMPLES {
        let value = from + (to - from) * index as f64 / (SWEEP_SAMPLES - 1) as f64;
        let overrides = [AdjustmentOverride::new(wire_name, value)];
        match preset_outline(preset, extents, &overrides, within) {
            Ok(outline) => {
                // The curve and not the control hull: a sweep visits arc angles that are not
                // quadrant boundaries, where a Bézier's control point legitimately sits outside the
                // shape's box. `common::curve_overhang` says why at length.
                furthest_outside = furthest_outside.max(curve_overhang(&outline.commands, within));
                for point in points_of(&outline.commands) {
                    assert!(
                        point.x.is_finite() && point.y.is_finite(),
                        "`{}` at `{wire_name}` = {value} produced {point:?}, which `SceneBuilder` \
                         would sanitise to zero and draw in the page's corner",
                        preset.to_wire()
                    );
                }
                outlines.push(outline.commands);
            }
            Err(GeometryError::SingularGeometry { guide, .. }) => {
                singular.get_or_insert(guide);
            }
            Err(error) => panic!(
                "`{}` at `{wire_name}` = {value} failed with {error}, which is a defect in the \
                 table rather than a point its own formulas have no value at",
                preset.to_wire()
            ),
        }
    }
    Sweep {
        outlines,
        singular,
        furthest_outside,
    }
}

/// The kinds of a command list, so two samples can be told apart from two *structures*.
///
/// An arc gains a cubic whenever its swing crosses a quarter turn, so a sweep legitimately changes
/// the length of its own command list — 112 of the 570 sweeps do. Where that happens the two samples
/// have no point-to-point correspondence and the displacement between them is not defined; the axis
/// census skips exactly those steps and asserts it did not skip them all.
fn structure(commands: &[PathCommand]) -> Vec<u8> {
    commands
        .iter()
        .map(|command| match command {
            PathCommand::MoveTo(_) => 0,
            PathCommand::LineTo(_) => 1,
            PathCommand::QuadraticTo { .. } => 2,
            PathCommand::CubicTo { .. } => 3,
            PathCommand::Close => 4,
        })
        .collect()
}

/// How far the outline travelled horizontally and vertically across a sweep, summed over every
/// consecutive pair of samples that share a structure.
///
/// Returns the totals and how many steps had a correspondence at all.
fn travel(outlines: &[Vec<PathCommand>]) -> (f64, f64, usize) {
    let (mut across, mut down, mut compared) = (0.0f64, 0.0f64, 0usize);
    for pair in outlines.windows(2) {
        if structure(&pair[0]) != structure(&pair[1]) {
            continue;
        }
        compared += 1;
        for (before, after) in points_of(&pair[0]).iter().zip(points_of(&pair[1]).iter()) {
            across += f64::from((after.x - before.x).abs());
            down += f64::from((after.y - before.y).abs());
        }
    }
    (across, down, compared)
}

/// Which axis a measured travel is on.
fn motion_of(across: f64, down: f64) -> Motion {
    let (larger, smaller) = if across > down {
        (across, down)
    } else {
        (down, across)
    };
    let dominant = smaller <= 0.0 || larger / smaller >= AXIS_DOMINANCE_RATIO;
    if !dominant {
        Motion::Both
    } else if across > down {
        Motion::Across
    } else {
        Motion::Down
    }
}

/// Whether either end of `spec`'s domain is the schema's *"no stop"* sentinel.
fn is_unbounded(minimum: AdjustmentBound, maximum: AdjustmentBound) -> bool {
    let sentinel =
        |bound| matches!(bound, AdjustmentBound::Literal(value) if value.abs() == UNBOUNDED_DOMAIN);
    sentinel(minimum) || sentinel(maximum)
}

// -------------------------------------------------------------------------------------------
// 1 · Does it move at all?
// -------------------------------------------------------------------------------------------

#[test]
fn every_adjustment_of_every_preset_moves_the_geometry() {
    // The weakest of the four questions and the one with no exceptions: an adjustment that can be
    // dragged from one end of its domain to the other and leaves the outline **identical** is an
    // adjustment nothing reads. It would survive every other gate in the crate — the shape still
    // closes, still fills its box, still has the right step count — and it is exactly what a guide
    // wired to `adj1` where the file writes `adj2` looks like from the second adjustment's side.
    let mut swept = 0usize;
    let mut shapes = BTreeSet::new();
    let mut motionless: BTreeSet<String> = BTreeSet::new();

    for (orientation, within, extents) in orientations() {
        for definition in seeded_shapes() {
            let preset = definition.preset;
            let domains = adjustment_domains(preset, extents, &[])
                .unwrap_or_else(|error| panic!("`{}` has no domains: {error}", preset.to_wire()));
            if domains.is_empty() {
                continue;
            }
            shapes.insert(preset.to_wire());
            for domain in &domains {
                swept += 1;
                let swept_shape = sweep(
                    preset,
                    domain.spec.wire_name,
                    domain.minimum,
                    domain.maximum,
                    extents,
                    within,
                );
                if swept_shape.outlines.len() < 2
                    || swept_shape
                        .outlines
                        .windows(2)
                        .all(|pair| pair[0] == pair[1])
                {
                    motionless.insert(format!(
                        "{orientation} `{}`.`{}` over [{}, {}]",
                        preset.to_wire(),
                        domain.spec.wire_name,
                        domain.minimum,
                        domain.maximum
                    ));
                }
            }
        }
    }

    assert!(
        motionless.is_empty(),
        "these adjustments were dragged across their whole domain and the outline never changed, \
         which is what an adjustment nothing reads looks like: {motionless:#?}"
    );
    assert_eq!(shapes.len(), SHAPES_WITH_AN_ADJUSTMENT);
    assert_eq!(
        swept,
        ADJUSTMENTS_IN_THE_TABLE * 2,
        "the sweep covered {swept} (shape, adjustment, orientation) triples rather than \
         {ADJUSTMENTS_IN_THE_TABLE} × 2 — an adjustment has been lost from the table"
    );
    println!(
        "{ADJUSTMENTS_IN_THE_TABLE} adjustments across {SHAPES_WITH_AN_ADJUSTMENT} presets, swept \
         in two orientations; every one moved its geometry"
    );
}

// -------------------------------------------------------------------------------------------
// 2 · Does it move on the axis it says it does?
// -------------------------------------------------------------------------------------------

#[test]
fn an_adjustments_declared_axis_is_the_axis_the_geometry_moves_on() {
    // **The differential in this file.** `AdjustmentAxis` is not derived from the path table: it is
    // read out of `a:ahLst`, where ECMA-376 states which guide a handle's *horizontal* drag writes
    // to and which its *vertical* one does. So "the geometry moved horizontally" and "the file says
    // this is a horizontal handle" are two readings of two different parts of the same file, and a
    // guide wired to the wrong variable breaks the agreement between them.
    //
    // Three answers rather than two, because a corner radius moves both axes at once and calling
    // that a contradiction would make the gate meaningless. [`AXIS_DOMINANCE_RATIO`] is where the
    // population's own gap is.
    let mut asked = 0usize;
    let mut measured: BTreeMap<(&'static str, &'static str), [Motion; 2]> = BTreeMap::new();
    let mut uncomparable: BTreeSet<String> = BTreeSet::new();

    for (index, (orientation, within, extents)) in orientations().into_iter().enumerate() {
        for definition in seeded_shapes() {
            let preset = definition.preset;
            let Ok(domains) = adjustment_domains(preset, extents, &[]) else {
                continue;
            };
            for domain in &domains {
                if Motion::declared(domain.spec.axis).is_none() {
                    continue;
                }
                asked += 1;
                let swept = sweep(
                    preset,
                    domain.spec.wire_name,
                    domain.minimum,
                    domain.maximum,
                    extents,
                    within,
                );
                let (across, down, compared) = travel(&swept.outlines);
                if compared == 0 {
                    uncomparable.insert(format!(
                        "{orientation} `{}`.`{}`",
                        preset.to_wire(),
                        domain.spec.wire_name
                    ));
                    continue;
                }
                measured
                    .entry((preset.to_wire(), domain.spec.wire_name))
                    .or_insert([Motion::Both; 2])[index] = motion_of(across, down);
            }
        }
    }

    // Every Cartesian adjustment has at least one pair of samples with a point-to-point
    // correspondence, so none is measured by default. The two that change their arc structure at
    // *every* step of the sweep — `circularArrow`'s and `leftCircularArrow`'s `adj3` — are both
    // `a:ahPolar` handles and were never asked. Kept as an assertion rather than as a comment
    // because "the census measured nothing and reported no contradictions" is the failure this
    // whole file is written against.
    assert!(
        uncomparable.is_empty(),
        "these adjustments changed their command structure at every step of the sweep, so no \
         displacement could be taken and the axis census silently skipped them: {uncomparable:#?}"
    );
    assert_eq!(asked, ADJUSTMENTS_WITH_A_CARTESIAN_AXIS * 2);

    let mut contradicting: BTreeMap<(&str, &str), [Motion; 2]> = BTreeMap::new();
    for ((shape, wire_name), motions) in &measured {
        let preset = seeded_shapes()
            .iter()
            .find(|definition| definition.preset.to_wire() == *shape)
            .map(|definition| definition.preset)
            .expect("a measured shape is in the table");
        let spec = mjx_ooxml_types::drawingml::adjustments_of(preset)
            .iter()
            .find(|spec| spec.wire_name == *wire_name)
            .expect("a measured adjustment is in the table");
        let declared = Motion::declared(spec.axis).expect("only Cartesian handles were asked");
        if motions.iter().any(|motion| motion.contradicts(declared)) {
            contradicting.insert((shape, wire_name), *motions);
        }
    }

    let expected: BTreeMap<(&str, &str), [Motion; 2]> = MOVES_ACROSS_ITS_DECLARED_AXIS
        .iter()
        .map(|(shape, wire_name, landscape, portrait)| {
            ((*shape, *wire_name), [*landscape, *portrait])
        })
        .collect();
    assert_eq!(
        contradicting, expected,
        "the adjustments whose geometry moves across the axis their own `a:ahLst` handle declares \
         have changed — each entry is (shape, adjustment) with its landscape and portrait motion"
    );
    println!(
        "{ADJUSTMENTS_WITH_A_CARTESIAN_AXIS} adjustments declare a Cartesian axis; {} move across \
         it and are named",
        contradicting.len()
    );
}

#[test]
fn the_axis_gate_is_answering_about_the_shape_and_not_about_the_box() {
    // The identity-value question for question 2, asked of the *instrument*. A census whose two
    // orientations produce identical measurements everywhere is a census that never looked at the
    // box, and the two boxes here are transposes with the same `ss` — so a quantity written in `ss`
    // comes out the same in both and one written in `w` or `h` does not.
    //
    // `star6` is the shape that proves the instrument is looking: its motion is `Across` in a
    // 160 × 120 box and `Both` in a 120 × 160 one, from the same guides and the same sweep.
    let mut motions = [Motion::Both; 2];
    for (index, (_, within, extents)) in orientations().into_iter().enumerate() {
        let domain = adjustment_domains(PresetShapeType::SixPointStar, extents, &[])
            .expect("`star6` has a domain")
            .into_iter()
            .find(|domain| domain.spec.wire_name == "adj")
            .expect("`star6` has an `adj`");
        let swept = sweep(
            PresetShapeType::SixPointStar,
            "adj",
            domain.minimum,
            domain.maximum,
            extents,
            within,
        );
        let (across, down, compared) = travel(&swept.outlines);
        assert!(compared > 0);
        motions[index] = motion_of(across, down);
    }
    assert_eq!(
        motions,
        [Motion::Across, Motion::Both],
        "`star6` no longer measures differently in the two orientations, so the axis census can no \
         longer tell a box it looked at from one it did not"
    );
}

// -------------------------------------------------------------------------------------------
// 3 · Does the shape stay in its box while it moves?
// -------------------------------------------------------------------------------------------

#[test]
fn a_bounded_adjustment_keeps_its_shape_in_its_box_except_where_named() {
    let mut furthest: BTreeMap<&'static str, [f32; 2]> = BTreeMap::new();

    for (index, (_, within, extents)) in orientations().into_iter().enumerate() {
        for definition in seeded_shapes() {
            let preset = definition.preset;
            let Ok(domains) = adjustment_domains(preset, extents, &[]) else {
                continue;
            };
            for domain in &domains {
                if is_unbounded(domain.spec.min, domain.spec.max) {
                    continue;
                }
                let swept = sweep(
                    preset,
                    domain.spec.wire_name,
                    domain.minimum,
                    domain.maximum,
                    extents,
                    within,
                );
                let slot = furthest.entry(preset.to_wire()).or_insert([0.0; 2]);
                slot[index] = slot[index].max(swept.furthest_outside);
            }
        }
    }

    let leaves: BTreeMap<&str, [f32; 2]> = furthest
        .iter()
        .filter(|(_, reach)| {
            reach
                .iter()
                .any(|value| *value > SWEEP_OVERHANG_TOLERANCE_PIXELS)
        })
        .map(|(shape, reach)| (*shape, *reach))
        .collect();
    let named: BTreeSet<&str> = LEAVES_ITS_BOX_SOMEWHERE_IN_A_BOUNDED_DOMAIN
        .iter()
        .map(|(shape, _, _)| *shape)
        .collect();
    assert_eq!(
        leaves.keys().copied().collect::<BTreeSet<_>>(),
        named,
        "the presets that leave their box somewhere in a bounded adjustment's domain have changed"
    );

    // …and by how much, to four decimal places, so that a shape which stayed in the list but grew a
    // tail fails as loudly as one that joined it.
    for (shape, landscape, portrait) in LEAVES_ITS_BOX_SOMEWHERE_IN_A_BOUNDED_DOMAIN {
        let measured = leaves[shape];
        for (expected, measured, orientation) in [
            (*landscape, measured[0], "landscape"),
            (*portrait, measured[1], "portrait"),
        ] {
            assert!(
                (measured - expected).abs() <= 0.001,
                "`{shape}` reaches {measured} px outside its box in {orientation}, not the \
                 {expected} px recorded"
            );
        }
    }

    // The two halves that make the tolerance a measurement. The worst shape that stays inside must
    // be *under* the tolerance — or a correct shape fails — and *over half* of it, or the tolerance
    // is passing everything and the number is decoration.
    let worst_inside = furthest
        .iter()
        .filter(|(shape, _)| !named.contains(*shape))
        .flat_map(|(_, reach)| reach.iter().copied())
        .fold(0.0f32, f32::max);
    println!("worst overhang among the shapes that stay in their box: {worst_inside} px");
    assert!(
        worst_inside <= SWEEP_OVERHANG_TOLERANCE_PIXELS,
        "a shape that is supposed to stay in its box reaches {worst_inside} px outside it"
    );
    assert!(
        worst_inside > SWEEP_OVERHANG_TOLERANCE_PIXELS / 2.0,
        "the worst correct shape is only {worst_inside} px out, so halving \
         {SWEEP_OVERHANG_TOLERANCE_PIXELS} would still pass everything — the tolerance has stopped \
         measuring anything and should be tightened"
    );
    assert!(
        (worst_inside - FURTHEST_A_SHAPE_INSIDE_ITS_BOX_REACHES).abs() < 0.0001,
        "the worst overhang among the shapes that stay in their box is {worst_inside}, not the \
         {FURTHEST_A_SHAPE_INSIDE_ITS_BOX_REACHES} this file's tolerance is justified against"
    );
    assert_eq!(
        furthest.len(),
        SHAPES_WITH_AN_ADJUSTMENT - UNBOUNDED_DOMAIN_SHAPES.len(),
        "the census no longer covers every preset with a bounded adjustment"
    );
}

#[test]
fn the_unbounded_adjustments_are_the_callouts_and_the_connectors() {
    // The exclusion from the census above, named — because an exclusion list nobody reads is how a
    // catalogue of quiet failures starts. These handles have no stop in ECMA-376's own `a:ahLst`,
    // so "the shape stays in its box" is not a claim anybody makes about them; what *is* claimed,
    // and asserted here, is that dragging one to `i32::MAX` still produces finite coordinates.
    let (_, within, extents) = orientations()[0];
    let mut unbounded: BTreeSet<&'static str> = BTreeSet::new();
    let mut furthest = 0.0f32;
    for definition in seeded_shapes() {
        let preset = definition.preset;
        let Ok(domains) = adjustment_domains(preset, extents, &[]) else {
            continue;
        };
        for domain in &domains {
            if !is_unbounded(domain.spec.min, domain.spec.max) {
                continue;
            }
            unbounded.insert(preset.to_wire());
            // `sweep` asserts finiteness at every sample and panics on any failure that is not a
            // singularity, so reaching here at all is the assertion.
            let swept = sweep(
                preset,
                domain.spec.wire_name,
                domain.minimum,
                domain.maximum,
                extents,
                within,
            );
            assert!(swept.singular.is_none());
            furthest = furthest.max(swept.furthest_outside);
        }
    }
    assert_eq!(
        unbounded,
        UNBOUNDED_DOMAIN_SHAPES
            .iter()
            .copied()
            .collect::<BTreeSet<_>>(),
        "the presets with an unbounded adjustment handle have changed"
    );
    // 2 147 483 647 hundred-thousandths of a 160-pixel box is 3 435 973.8 pixels, and that is what
    // an unbounded handle dragged to its schema limit *is*. Asserted so the exemption cannot be
    // quietly satisfied by a shape that no longer moves.
    assert!(
        furthest > 3_000_000.0,
        "an unbounded handle dragged to `i32::MAX` moved its shape only {furthest} px, so the \
         domain is no longer the schema's sentinel"
    );
    println!(
        "{} presets have an unbounded handle; at its limit they reach {furthest} px outside their \
         box, finitely",
        unbounded.len()
    );
}

// -------------------------------------------------------------------------------------------
// 4 · Where does the shape's own arithmetic run out?
// -------------------------------------------------------------------------------------------

#[test]
fn the_adjustments_singular_at_a_domain_endpoint_are_named() {
    let mut singular: BTreeSet<(&'static str, &'static str, String)> = BTreeSet::new();
    for (_, within, extents) in orientations() {
        for definition in seeded_shapes() {
            let preset = definition.preset;
            let Ok(domains) = adjustment_domains(preset, extents, &[]) else {
                continue;
            };
            for domain in &domains {
                let swept = sweep(
                    preset,
                    domain.spec.wire_name,
                    domain.minimum,
                    domain.maximum,
                    extents,
                    within,
                );
                if let Some(guide) = swept.singular {
                    singular.insert((preset.to_wire(), domain.spec.wire_name, guide));
                }
            }
        }
    }
    assert_eq!(
        singular,
        SINGULAR_AT_A_DOMAIN_ENDPOINT
            .iter()
            .map(|(shape, wire_name, guide)| (*shape, *wire_name, (*guide).to_owned()))
            .collect::<BTreeSet<_>>(),
        "the adjustments whose own domain reaches a point their formulas have no value at have \
         changed"
    );

    // And the classification a stand-in depends on, asserted from the error rather than from the
    // list: `has_no_geometry_to_draw` is what tells a provider to count a placeholder instead of
    // failing the page.
    let (_, within, extents) = orientations()[0];
    for (shape, wire_name, guide) in SINGULAR_AT_A_DOMAIN_ENDPOINT {
        let preset = seeded_shapes()
            .iter()
            .find(|definition| definition.preset.to_wire() == *shape)
            .map(|definition| definition.preset)
            .expect("a named shape is in the table");
        let domain = adjustment_domains(preset, extents, &[])
            .expect("a named shape has domains")
            .into_iter()
            .find(|domain| domain.spec.wire_name == *wire_name)
            .expect("a named adjustment is in the table");
        let at_a_stop = [
            AdjustmentOverride::new(*wire_name, domain.minimum),
            AdjustmentOverride::new(*wire_name, domain.maximum),
        ];
        let failures: Vec<GeometryError> = at_a_stop
            .into_iter()
            .filter_map(|override_| preset_outline(preset, extents, &[override_], within).err())
            .collect();
        assert_eq!(
            failures.len(),
            1,
            "`{shape}`.`{wire_name}` is singular at neither or both of its own stops"
        );
        assert!(
            matches!(&failures[0], GeometryError::SingularGeometry { guide: named, .. } if named == guide),
            "`{shape}` at a stop of `{wire_name}` failed with {}, not a singularity in `{guide}`",
            failures[0]
        );
        assert!(failures[0].has_no_geometry_to_draw());
    }
}

// -------------------------------------------------------------------------------------------
// The check shown able to fail
// -------------------------------------------------------------------------------------------

/// `triangle`'s guide list with its apex pinned to the middle — `adj` seeded, declared, offered to
/// the user, and read by nothing.
///
/// The mutation MJXOFF-205 asks the monotonicity check to be proved against, and it is the exact
/// defect the check exists for. The file's `x2` is `*/ w a 100000`; this one is `*/ w 50000 100000`,
/// a constant that happens to equal the default. So the shape is **identical at its defaults** —
/// the box census, the step census, the `custGeom` differential and every structural claim in the
/// crate pass on it unchanged — and the adjustment is dead. The only thing that sees it is a sweep.
static A_TRIANGLE_WHOSE_APEX_IGNORES_ITS_ADJUSTMENT: &[mjx_ooxml_types::drawingml::PresetGuide] = &[
    mjx_ooxml_types::drawingml::PresetGuide {
        wire_name: "a",
        formula: "pin 0 adj 100000",
    },
    mjx_ooxml_types::drawingml::PresetGuide {
        wire_name: "x1",
        formula: "*/ w a 200000",
    },
    mjx_ooxml_types::drawingml::PresetGuide {
        wire_name: "x2",
        // The file writes `*/ w a 100000`. This is the same number at `adj = 50000` and a constant
        // everywhere else.
        formula: "*/ w 50000 100000",
    },
    mjx_ooxml_types::drawingml::PresetGuide {
        wire_name: "x3",
        formula: "+- x1 wd2 0",
    },
];

#[test]
fn the_sweep_is_able_to_fail_on_one_shape_and_only_that_shape() {
    let (_, within, extents) = orientations()[0];
    let real = seeded_shapes()
        .iter()
        .find(|definition| definition.preset == PresetShapeType::Triangle)
        .expect("the table has `triangle`");
    let mutant = mjx_geometry::PresetShapeDefinition {
        guides: A_TRIANGLE_WHOSE_APEX_IGNORES_ITS_ADJUSTMENT,
        ..*real
    };

    // The mutant is invisible everywhere else: at the default adjustment it draws the same shape,
    // to the pixel. That is the claim that makes this mutation worth making.
    let honest = preset_outline(PresetShapeType::Triangle, extents, &[], within)
        .expect("`triangle` resolves");
    let mutated = mjx_geometry::outline_of_definition(&mutant, extents, &[], within)
        .expect("the mutant resolves");
    assert_eq!(
        honest.commands, mutated.commands,
        "the mutation changed the shape at its defaults, so it is not the invisible defect this \
         test is about"
    );

    // …and the whole table swept, with the mutant standing in for `triangle`. Exactly one shape
    // must report, and it must be that one.
    let mut motionless: BTreeSet<&'static str> = BTreeSet::new();
    for definition in seeded_shapes() {
        let shape = definition.preset.to_wire();
        let definition = if definition.preset == PresetShapeType::Triangle {
            &mutant
        } else {
            definition
        };
        let Ok(domains) = adjustment_domains(definition.preset, extents, &[]) else {
            continue;
        };
        for domain in &domains {
            let outlines: Vec<Vec<PathCommand>> = (0..SWEEP_SAMPLES)
                .filter_map(|index| {
                    let value = domain.minimum
                        + (domain.maximum - domain.minimum) * index as f64
                            / (SWEEP_SAMPLES - 1) as f64;
                    mjx_geometry::outline_of_definition(
                        definition,
                        extents,
                        &[AdjustmentOverride::new(domain.spec.wire_name, value)],
                        within,
                    )
                    .ok()
                    .map(|outline| outline.commands)
                })
                .collect();
            if outlines.len() >= 2 && outlines.windows(2).all(|pair| pair[0] == pair[1]) {
                motionless.insert(shape);
            }
        }
    }
    assert_eq!(
        motionless.into_iter().collect::<Vec<_>>(),
        vec!["triangle"],
        "the mutation should have made exactly `triangle`'s adjustment dead"
    );
}

// -------------------------------------------------------------------------------------------
// The defect this file found, pinned where it happened
// -------------------------------------------------------------------------------------------

#[test]
fn a_degenerate_ellipse_keeps_its_arc_where_the_pen_is() {
    // The unit behind `arc.rs`'s fix, stated in the terms the bug was in. An `a:arcTo` whose
    // ellipse has one radius zero is a *line segment*, and the arc must still begin at the pen and
    // sweep the angle the file asks for. The old guard tested `wR·sin θ == 0 && hR·cos θ == 0` and
    // `sin π` is `1.22e-16`, so it never fired: `atan2(1.22e-16, -0.0)` is `π/2` rather than `π`,
    // and the quarter turn that invents puts the derived centre a whole `wR` from the pen.
    //
    // `can`'s top ellipse at `adj = 0` is exactly this arc, and it is the shape whose 80-pixel
    // excursion found it.
    let pen = mjx_geometry::ShapePoint::new(0.0, 0.0);
    let segments =
        mjx_geometry::arc_to_cubics(pen, 80.0, 0.0, std::f64::consts::PI, std::f64::consts::PI);
    assert!(!segments.is_empty(), "a half turn on a flat ellipse draws");
    let last = segments.last().expect("at least one segment");
    assert!(
        (last.end.x - 160.0).abs() < 1e-9 && last.end.y.abs() < 1e-9,
        "a half turn from the pen along a flat ellipse of half-width 80 ends at (160, 0), not \
         {:?}",
        last.end
    );
    for segment in &segments {
        for point in [segment.first_control, segment.second_control, segment.end] {
            assert!(
                point.x >= -1e-9 && point.x <= 160.0 + 1e-9,
                "{point:?} is outside the segment the ellipse collapsed to"
            );
        }
    }
    // The conversion itself, at the two angles that made the old guard miss.
    for angle in [std::f64::consts::PI, std::f64::consts::TAU] {
        assert_eq!(
            mjx_geometry::parametric_angle(angle, 80.0, 0.0),
            angle,
            "a degenerate ellipse's parametric angle is the true angle"
        );
        assert_eq!(mjx_geometry::parametric_angle(angle, 0.0, 80.0), angle);
    }
    // And the shape it was found on, in its own box: at `adj = 0` `can` is a flat-topped cylinder
    // and every point of it is inside.
    let (_, within, extents) = orientations()[0];
    let outline = preset_outline(
        PresetShapeType::Can,
        extents,
        &[AdjustmentOverride::new("adj", 0.0)],
        within,
    )
    .expect("`can` resolves at its adjustment's own minimum");
    let outside = curve_overhang(&outline.commands, within);
    assert!(
        outside <= SWEEP_OVERHANG_TOLERANCE_PIXELS,
        "`can` at `adj = 0` reaches {outside} device pixels outside its box"
    );
}
