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

/// A device-pixel box that is **taller than it is wide** — 120 × 160, the transpose of
/// [`box_on_the_page`].
///
/// # Why a second box, and why portrait specifically
///
/// Every other box and every non-degenerate extent in this crate's suites is landscape or square,
/// and that makes one confusion invisible to all of them. `ss` is `min(w, h)` — ECMA-376's "shorter
/// side", and the unit a great many preset guides are written in — so **in a landscape box `ss` is
/// always `h`**, and an implementation that read `h` where the formula says `ss` would produce
/// identical numbers everywhere the crate measures. It would keep its box-census membership,
/// measure zero in the seed differential and draw the same command tally.
///
/// Here `ss` is `w`. Together the two boxes pin `ss` to `min(w, h)` rather than to either side:
/// a quantity written in `ss` must come out the *same* in both, an `h`-based one comes out larger
/// here, and a `w`-based one comes out larger there.
///
/// Not square, not at the origin, and at a different offset from the landscape box — so a mapping
/// that dropped the offset or swapped the axes would fail in one of the two.
pub fn portrait_box_on_the_page() -> SceneRect {
    SceneRect::new(23.0, 17.0, 143.0, 177.0)
}

/// The extents a shape in [`portrait_box_on_the_page`] has in the document, at one device pixel per
/// point.
///
/// 120 × 160 points at 12 700 EMU to the point — the transpose of [`extents_of_the_box`], so the
/// two boxes have the *same* `ss` and differ only in which side it is.
pub fn portrait_extents() -> Size {
    Size::from_emu(120 * 12_700, 160 * 12_700)
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

/// The smallest rectangle containing every point the outline actually **draws**, curves flattened.
///
/// The companion of [`bounds_of`], and the one a suite over the generated table wants: a Bézier
/// lies inside its control polygon and not on it, so [`bounds_of`] measures a hull that can sit
/// several per cent outside the shape wherever an arc is not quadrant-aligned. `heart`'s control
/// points are 83 device pixels outside a 160-pixel box and its curve reaches 0.57.
pub fn curve_bounds(commands: &[PathCommand]) -> SceneRect {
    let (mut left, mut top, mut right, mut bottom) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for point in flattened(commands).into_iter().flatten() {
        left = left.min(point.x);
        top = top.min(point.y);
        right = right.max(point.x);
        bottom = bottom.max(point.y);
    }
    SceneRect::new(left, top, right, bottom)
}

/// Every preset MJXOFF-202 transcribed by hand, with the wire token a failure should name it by.
///
/// Six of the 186, and the ones whose *expected* structure a suite may write out: they were
/// derived from ECMA-376's prose rather than from the table, so a count stated beside them is an
/// independent claim rather than a restatement of the data it checks.
pub fn hand_transcribed() -> Vec<(PresetShapeType, &'static str)> {
    mjx_geometry::seed::HAND_TRANSCRIBED_SHAPES
        .iter()
        .map(|definition| (definition.preset, definition.preset.to_wire()))
        .collect()
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

/// How far `point` is from the segment `start`–`end`.
///
/// The one point-to-segment primitive the suites share. `the_two_routes_agree.rs`'s Hausdorff
/// distance and `a_connector_lands_on_the_outline.rs`'s *"is this site on the outline"* are the
/// same measurement asked twice, and a second implementation of it would be free to disagree with
/// the first about a degenerate segment.
pub fn distance_to_segment(point: ScenePoint, start: ScenePoint, end: ScenePoint) -> f32 {
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

/// How far `point` lies from the nearest point of the outline `commands` draws, curves flattened.
///
/// Zero for a point on the outline. The closing segment of every contour is included, because a
/// connection site on the edge an `a:close` draws is on the outline as surely as one on an edge a
/// `a:lnTo` draws — and a contour the outline left open (a `fill="none"` path) gets the same
/// treatment, which can only make the answer smaller and never larger.
pub fn distance_to_outline(point: ScenePoint, commands: &[PathCommand]) -> f32 {
    let mut nearest = f32::MAX;
    for contour in flattened(commands) {
        for pair in contour.windows(2) {
            nearest = nearest.min(distance_to_segment(point, pair[0], pair[1]));
        }
        if let (Some(first), Some(last)) = (contour.first(), contour.last()) {
            nearest = nearest.min(distance_to_segment(point, *last, *first));
        }
    }
    nearest
}

/// How far `inner` reaches outside `outer`, on the worst of its four edges; zero when it is inside.
pub fn reaches_outside(inner: SceneRect, outer: SceneRect) -> f32 {
    (outer.left - inner.left)
        .max(inner.right - outer.right)
        .max(outer.top - inner.top)
        .max(inner.bottom - outer.bottom)
        .max(0.0)
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
