//! What the suites share: the boxes and sizes they resolve in, and the geometry they measure.
//!
//! Nothing here asserts. Every function is a *measurement* — a bounding box, a step tally, a signed
//! area — so that a suite's assertion is a comparison between two numbers a reader can check rather
//! than a call to something that decides for itself whether a shape is right.

#![allow(
    dead_code,
    unreachable_pub,
    reason = "each suite uses a different part of this scaffolding"
)]

use mjx_geometry::{PresetShapeType, Size};
use mjx_scene::{Geometry, PathCommand, ScenePoint, SceneRect};

/// The device-pixel box every suite draws in unless it says otherwise.
///
/// Deliberately **not** square and not at the origin: a mapping that dropped the offset, swapped the
/// axes or scaled both by the same factor would still land inside a square box at `(0, 0)`, and the
/// bounding-box gate would pass on all three.
pub fn box_on_the_page() -> SceneRect {
    SceneRect::new(37.0, 11.0, 197.0, 131.0)
}

/// The extents a shape in [`box_on_the_page`] has in the document, at one device pixel per point.
///
/// 160 × 120 pixels at 12 700 EMU to the point. A real number rather than a round one so that a
/// resolver that ignored the extents and used a fixed scale would be off by a visible amount.
pub fn extents_of_the_box() -> Size {
    Size::from_emu(160 * 12_700, 120 * 12_700)
}

/// Every shape this build seeds, with the wire token a failure should name it by.
pub fn seeded() -> Vec<(PresetShapeType, &'static str)> {
    mjx_geometry::seeded_shapes()
        .iter()
        .map(|definition| (definition.preset, definition.preset.to_wire()))
        .collect()
}

/// How many steps of each kind a command list holds: moves, lines, quadratics, cubics, closes.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct StepTally {
    pub moves: usize,
    pub lines: usize,
    pub quadratics: usize,
    pub cubics: usize,
    pub closes: usize,
}

impl StepTally {
    /// The tally of `commands`.
    pub fn of(commands: &[PathCommand]) -> Self {
        let mut tally = Self::default();
        for command in commands {
            match command {
                PathCommand::MoveTo(_) => tally.moves += 1,
                PathCommand::LineTo(_) => tally.lines += 1,
                PathCommand::QuadraticTo { .. } => tally.quadratics += 1,
                PathCommand::CubicTo { .. } => tally.cubics += 1,
                PathCommand::Close => tally.closes += 1,
            }
        }
        tally
    }

    /// A tally written the way a shape's definition reads: moves, lines, cubics, closes.
    pub fn expected(moves: usize, lines: usize, cubics: usize, closes: usize) -> Self {
        Self {
            moves,
            lines,
            quadratics: 0,
            cubics,
            closes,
        }
    }
}

/// The smallest rectangle containing every point a command list names, control points included.
///
/// Computed through `mjx-scene`'s own [`Geometry::path`], not by a second walk: the box a viewport
/// cull uses is the box a gate must measure, or the gate is measuring something else.
pub fn bounds_of(commands: &[PathCommand]) -> SceneRect {
    Geometry::path(commands.to_vec(), mjx_scene::FillRule::NonZero).bounds()
}

/// How far, in device pixels, the two rectangles' corresponding edges are apart at worst.
pub fn edge_distance(left: SceneRect, right: SceneRect) -> f32 {
    (left.left - right.left)
        .abs()
        .max((left.top - right.top).abs())
        .max((left.right - right.right).abs())
        .max((left.bottom - right.bottom).abs())
}

/// How far outside `box_` the furthest **named point** of `commands` lies, control points included.
///
/// The stricter of the two overhang measures, and the one to use where every curve is known to
/// begin and end on a quadrant boundary — as every arc of every seeded shape does at its default
/// adjustments. There the control points of a quarter-turn arc sit exactly on the box, so this
/// measures the shape.
pub fn overhang(commands: &[PathCommand], box_: SceneRect) -> f32 {
    outside(points_of(commands).into_iter(), box_)
}

/// How far outside `box_` the furthest point **of the curve itself** lies.
///
/// A Bézier lies inside its control polygon and not on it, so a control point may sit outside a
/// shape's box while every point the shape actually draws is inside. That is not a defect and it is
/// not rare: an arc split into equal segments that do *not* land on a quadrant boundary — a `pie`
/// swept 100°, say — has a control point about 3 per cent past the ellipse, which on a 120-pixel
/// box is 1.7 pixels. The shape is still inside its box; the *hull* is not.
///
/// So a sweep, which visits exactly those angles, asks this question instead. [`overhang`] is what
/// a shape whose curves are quadrant-aligned should be measured with, because there the two agree
/// and the stricter one is free.
pub fn curve_overhang(commands: &[PathCommand], box_: SceneRect) -> f32 {
    outside(
        flattened(commands).into_iter().flat_map(Vec::into_iter),
        box_,
    )
}

/// How far the furthest of `points` lies outside `box_`; zero when every one is inside.
fn outside(points: impl Iterator<Item = ScenePoint>, box_: SceneRect) -> f32 {
    let mut worst = 0.0f32;
    for point in points {
        worst = worst
            .max(box_.left - point.x)
            .max(point.x - box_.right)
            .max(box_.top - point.y)
            .max(point.y - box_.bottom);
    }
    worst
}

/// Every point a command list names, control points included, in order.
pub fn points_of(commands: &[PathCommand]) -> Vec<ScenePoint> {
    let mut points = Vec::new();
    for command in commands {
        match *command {
            PathCommand::MoveTo(at) | PathCommand::LineTo(at) => points.push(at),
            PathCommand::QuadraticTo { control, end } => {
                points.push(control);
                points.push(end);
            }
            PathCommand::CubicTo {
                first_control,
                second_control,
                end,
            } => {
                points.push(first_control);
                points.push(second_control);
                points.push(end);
            }
            PathCommand::Close => {}
        }
    }
    points
}

/// The outline as a polygon, with every curve flattened into `FLATTENING_STEPS` line segments.
///
/// Used for the signed area a monotonicity sweep compares. Flattening rather than taking the curve
/// endpoints alone matters: an arc that crosses a quarter-turn boundary gains a cubic, and a
/// polygon of endpoints would step whenever it did, which would read as a break in monotonicity
/// that is an artefact of the decomposition rather than of the shape.
pub fn flattened(commands: &[PathCommand]) -> Vec<Vec<ScenePoint>> {
    let mut contours: Vec<Vec<ScenePoint>> = Vec::new();
    let mut current: Vec<ScenePoint> = Vec::new();
    let mut pen = ScenePoint::ORIGIN;
    for command in commands {
        match *command {
            PathCommand::MoveTo(at) => {
                if current.len() > 1 {
                    contours.push(std::mem::take(&mut current));
                } else {
                    current.clear();
                }
                current.push(at);
                pen = at;
            }
            PathCommand::LineTo(at) => {
                current.push(at);
                pen = at;
            }
            PathCommand::QuadraticTo { control, end } => {
                for step in 1..=FLATTENING_STEPS {
                    let t = step as f32 / FLATTENING_STEPS as f32;
                    current.push(quadratic_at(pen, control, end, t));
                }
                pen = end;
            }
            PathCommand::CubicTo {
                first_control,
                second_control,
                end,
            } => {
                for step in 1..=FLATTENING_STEPS {
                    let t = step as f32 / FLATTENING_STEPS as f32;
                    current.push(cubic_at(pen, first_control, second_control, end, t));
                }
                pen = end;
            }
            PathCommand::Close => {
                if current.len() > 1 {
                    contours.push(std::mem::take(&mut current));
                } else {
                    current.clear();
                }
                pen = contours
                    .last()
                    .and_then(|contour| contour.first().copied())
                    .unwrap_or(pen);
            }
        }
    }
    if current.len() > 1 {
        contours.push(current);
    }
    contours
}

/// How many line segments one curve becomes when flattened.
const FLATTENING_STEPS: usize = 24;

fn quadratic_at(from: ScenePoint, control: ScenePoint, to: ScenePoint, t: f32) -> ScenePoint {
    let u = 1.0 - t;
    ScenePoint::new(
        u * u * from.x + 2.0 * u * t * control.x + t * t * to.x,
        u * u * from.y + 2.0 * u * t * control.y + t * t * to.y,
    )
}

fn cubic_at(
    from: ScenePoint,
    first: ScenePoint,
    second: ScenePoint,
    to: ScenePoint,
    t: f32,
) -> ScenePoint {
    let u = 1.0 - t;
    let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
    ScenePoint::new(
        a * from.x + b * first.x + c * second.x + d * to.x,
        a * from.y + b * first.y + c * second.y + d * to.y,
    )
}

/// The unsigned area the outline encloses, in square device pixels.
pub fn enclosed_area(commands: &[PathCommand]) -> f64 {
    flattened(commands)
        .iter()
        .map(|contour| shoelace(contour).abs())
        .sum()
}

/// Twice the signed area of a closed polygon, halved — the shoelace formula.
fn shoelace(points: &[ScenePoint]) -> f64 {
    let mut sum = 0.0f64;
    for index in 0..points.len() {
        let here = points[index];
        let next = points[(index + 1) % points.len()];
        sum += f64::from(here.x) * f64::from(next.y) - f64::from(next.x) * f64::from(here.y);
    }
    sum / 2.0
}
