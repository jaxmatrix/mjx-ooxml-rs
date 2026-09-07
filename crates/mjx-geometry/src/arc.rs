//! `a:arcTo` as cubic Béziers — the one piece of geometry this crate computes for itself.
//!
//! # Why it is here and not in `mjx-dml`
//!
//! `mjx-dml` resolves an `a:arcTo` into numbers — two radii and two angles — and stops, because an
//! arc is what the *file* says and decomposing it is a rendering decision: how many cubics, and how
//! close, depends on who is drawing. A [`PathCommand`](mjx_scene::PathCommand) has no arc, so
//! somebody has to. `mjx-scene`'s own `the_geometry_seam_is_swappable.rs` says as much and skips
//! arcs outright; this module is what stops that being the shipped behaviour.
//!
//! # The angles are true angles, not parameters
//!
//! An `a:arcTo` names an ellipse by its radii `wR`/`hR` and the arc on it by `stAng`/`swAng`, and
//! the arc begins **at the pen's current position** — so the ellipse's centre is derived, not
//! given. Whether `stAng` is the angle of a ray from that centre (a *true* angle) or the parameter
//! `t` of `(wR·cos t, hR·sin t)` matters for every shape that is not square, and the spec settles
//! it in the definition of its own `arc` preset. `mjx-dml` quotes that definition on
//! [`GuideOperator::CosineArcTangent`](mjx_dml::geometry::GuideOperator::CosineArcTangent): the arc
//! begins at `hc + cat2 wd2 ht1 wt1` with `wt1 = sin wd2 stAng` and `ht1 = cos hd2 stAng`, which is
//!
//! ```text
//!     x = hc + (w/2)·cos( atan2( (w/2)·sin θ, (h/2)·cos θ ) )
//! ```
//!
//! and the centre a pen at that point derives is `(hc, vc)` — the shape's own centre, which is
//! plainly what `arc` means — **only** if `stAng` is a true angle. Under the parametric reading the
//! ellipse would be displaced by however much the shape is not square, and the spec would have
//! written `cos wd2 stAng` instead of `cat2`. So: true angles, converted here.
//!
//! [`parametric_angle`] is that conversion, and it is written so the conversion cannot lose a turn:
//! the parametric and true angles are always in the same quadrant, so their difference is strictly
//! within a quarter turn, and unwrapping it into `(-π, π]` recovers it exactly. That keeps
//! `θ ↦ φ` monotone across an arc of any length, which is what makes a 350° swing three hundred and
//! fifty degrees rather than minus ten.
//!
//! # Coordinates here are the shape's own, and `f64`
//!
//! An arc is decomposed **before** the shape is mapped onto its device-pixel box, because the map
//! is affine and an affine image of a cubic is the cubic of the affine images of its control
//! points. Doing it the other way would need the map applied to the radii and the angles, which is
//! not a thing an anisotropic map does.

use std::f64::consts::{FRAC_PI_2, PI, TAU};

/// A point in a shape's own coordinate space, as a real number of EMU.
///
/// `f64` rather than [`Emu`](mjx_ooxml_core::measure::Emu) because these are intermediate values of
/// a decomposition — a Bézier control point is not a coordinate the file states, and rounding one
/// to a whole EMU before the map to pixels would quantise the curve for no benefit.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct ShapePoint {
    /// Rightward from the shape's own origin, in EMU.
    pub x: f64,
    /// Downward from it, in EMU. **`y` increases downward**, which is why a positive angle turns
    /// clockwise.
    pub y: f64,
}

impl ShapePoint {
    /// The point at `(x, y)`.
    #[must_use]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Whether both coordinates are finite — the guard every value entering a decomposition passes.
    #[must_use]
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

/// One cubic of a decomposed arc: two control points and an end point, in the shape's own space.
///
/// The start point is the previous segment's end, or the pen's position for the first, exactly as
/// [`PathCommand::CubicTo`](mjx_scene::PathCommand::CubicTo) has it.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ArcSegment {
    /// The control point nearer the start.
    pub first_control: ShapePoint,
    /// The control point nearer the end.
    pub second_control: ShapePoint,
    /// Where this cubic ends.
    pub end: ShapePoint,
}

/// The widest parametric span one cubic may cover — a quarter turn.
///
/// A cubic approximation of a circular arc has a maximum radial error of roughly `2.7e-4·r` at 90°
/// and about sixteen times that at 180°, so a quarter turn is the standard split and is what every
/// vector graphics stack uses. It is a constant rather than a literal so that the count of cubics a
/// shape produces — which `tests/a_preset_renders_as_itself.rs` asserts exactly, four for an
/// ellipse and three for the default `pie` — is derived from the same number the code splits on.
pub const MAXIMUM_ARC_SEGMENT_RADIANS: f64 = FRAC_PI_2;

/// The parametric angle `φ` at which the ellipse `(wR·cos φ, hR·sin φ)` meets the ray leaving its
/// centre at the true angle `θ`.
///
/// Both angles are in radians, measured clockwise because `y` increases downward. The identity is
/// `tan φ = (wR/hR)·tan θ`, taken as a two-argument arc tangent so the quadrant survives, and then
/// unwrapped against `θ` so an arc that crosses the `atan2` branch cut does not lose a turn.
///
/// A degenerate ellipse — both radii zero, or the ray landing exactly on the collapsed axis —
/// answers with `θ` itself, which is the only value that keeps the sweep's sign and magnitude.
#[must_use]
pub fn parametric_angle(true_angle: f64, width_radius: f64, height_radius: f64) -> f64 {
    let across = width_radius * true_angle.sin();
    let along = height_radius * true_angle.cos();
    if across == 0.0 && along == 0.0 {
        return true_angle;
    }
    let raw = across.atan2(along);
    // The two angles share a quadrant, so their difference is inside a quarter turn; wrapping into
    // `(-π, π]` therefore recovers it exactly rather than merely approximately.
    true_angle + wrapped_into_half_turns(raw - true_angle)
}

/// `angle` moved by whole turns into `(-π, π]`.
fn wrapped_into_half_turns(angle: f64) -> f64 {
    let wrapped = angle - TAU * (angle / TAU).round();
    // `round` breaks ties away from zero, so exactly -π survives as -π; normalise it to +π so the
    // interval is half-open on one side only and the function is a true inverse of adding turns.
    if wrapped <= -PI {
        wrapped + TAU
    } else {
        wrapped
    }
}

/// The cubics that draw `a:arcTo`'s arc, starting from the pen's position.
///
/// `start_angle` and `swing_angle` are **true** angles in radians, clockwise-positive; the radii
/// are in the shape's own units. The returned segments are consecutive: the first begins at
/// `start`, and each subsequent one at its predecessor's `end`.
///
/// Answers with no segments — and therefore leaves the pen where it is — when the arc draws
/// nothing: a zero swing, an ellipse with both radii zero, or any non-finite input. That is not the
/// *"silently drew nothing"* defect [`crate::error`] is written against: those three are arcs that
/// genuinely have no extent, and a shape whose whole path failed to resolve is an error long before
/// it reaches here.
#[must_use]
pub fn arc_to_cubics(
    start: ShapePoint,
    width_radius: f64,
    height_radius: f64,
    start_angle: f64,
    swing_angle: f64,
) -> Vec<ArcSegment> {
    if !start.is_finite()
        || !width_radius.is_finite()
        || !height_radius.is_finite()
        || !start_angle.is_finite()
        || !swing_angle.is_finite()
        || swing_angle == 0.0
        || (width_radius == 0.0 && height_radius == 0.0)
    {
        return Vec::new();
    }

    let from = parametric_angle(start_angle, width_radius, height_radius);
    let to = parametric_angle(start_angle + swing_angle, width_radius, height_radius);
    let sweep = to - from;
    if sweep == 0.0 {
        return Vec::new();
    }

    // The centre is what makes the arc begin where the pen already is.
    let centre = ShapePoint::new(
        start.x - width_radius * from.cos(),
        start.y - height_radius * from.sin(),
    );

    let count = (sweep.abs() / MAXIMUM_ARC_SEGMENT_RADIANS).ceil().max(1.0);
    // A swing large enough to overflow a `usize` is not a shape; the guard is here because the
    // input is a document's own number.
    let count = if count.is_finite() && count < 4096.0 {
        count as usize
    } else {
        4096
    };
    let step = sweep / count as f64;

    let on = |angle: f64| {
        ShapePoint::new(
            centre.x + width_radius * angle.cos(),
            centre.y + height_radius * angle.sin(),
        )
    };
    // The derivative of the parametrisation, which is what a Bézier's control points travel along.
    let along =
        |angle: f64| ShapePoint::new(-width_radius * angle.sin(), height_radius * angle.cos());
    // The classical control-point distance for a cubic through a parametric arc of `step`.
    let reach = 4.0 / 3.0 * (step / 4.0).tan();

    let mut segments = Vec::with_capacity(count);
    for index in 0..count {
        let first = from + step * index as f64;
        let second = from + step * (index + 1) as f64;
        let (at_first, at_second) = (on(first), on(second));
        let (dir_first, dir_second) = (along(first), along(second));
        segments.push(ArcSegment {
            first_control: ShapePoint::new(
                at_first.x + reach * dir_first.x,
                at_first.y + reach * dir_first.y,
            ),
            second_control: ShapePoint::new(
                at_second.x - reach * dir_second.x,
                at_second.y - reach * dir_second.y,
            ),
            end: at_second,
        });
    }
    segments
}
