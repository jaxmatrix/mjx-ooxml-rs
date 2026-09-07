//! The geometry seam: who turns a shape's opaque handle into an outline, and the stand-in that
//! answers until somebody real does.
//!
//! # What the seam is for
//!
//! A [`ShapeFragment`](mjx_layout::ShapeFragment) says *this box has that outline* and says it with
//! a bare number. `build_scene` records the number and the box and stops there — a
//! [`Geometry::Unresolved`] — because turning a number into a path means knowing what issued it, and
//! this crate deliberately does not. `docs/UI_PLATFORM_PLAN.md` §4 L4 puts a [`GeometryProvider`]
//! between the two, and everything above the seam — tessellation, painting, hit testing, export —
//! is written against the *outline*, never against the vocabulary the outline came out of.
//!
//! There is exactly one method, it names no OOXML type, and it takes the size as a **rectangle in
//! device pixels**. That last point is not decoration: `mjx_layout::Extent` is a *page count*
//! carrying an [`ExtentPrecision`](mjx_layout::ExtentPrecision) that says whether the count was
//! measured or guessed, so a signature that took an `Extent` as a size would compile, read plausibly
//! and mean something else entirely. The parameter here is called `within` and is a [`SceneRect`],
//! and this paragraph is why.
//!
//! # Why the stand-in is deliberately not a shape
//!
//! **The real provider exists.** `mjx-geometry`'s `PresetGeometryProvider` resolves all 186 presets
//! `presetShapeDefinitions.xml` defines geometry for, through their own guide formulas and their
//! own `a:avLst`, and MJXOFF-206 made it what a document is rendered with: this crate ships no
//! provider that a page of real shapes goes through, and
//! `crates/mjx-geometry/tests/the_stand_in_is_named_wherever_it_is_used.rs` asserts that the only
//! shipped construction of [`PlaceholderGeometry`] left in the workspace is that provider's own
//! fall-through.
//!
//! **This crate cannot name it, and that is the seam working rather than a gap.** `mjx-scene` is
//! rank 1.7 and `mjx-geometry` is 2.5, so the edge points up and `xtask/tests/layering.rs` refuses
//! it — the same refusal that keeps `mjx-scene → mjx-dml` illegal. A display list that could read a
//! preset table would be a display list that knows what a `.pptx` is.
//!
//! So [`PlaceholderGeometry`] stays, as the honest answer for the three cases where there is
//! genuinely no geometry to draw: a handle nobody registered, a preset ECMA-376 defines no geometry
//! for (`upArrow` is the only one), and a shape whose own formulas are singular at the adjustments
//! in force. Deleting it would replace a *visible* placeholder with a silent nothing, which is the
//! one answer the seam forbids.
//!
//! Its shape is a gate, not a cosmetic choice. *"Every shape tessellates"* is trivially true when
//! every shape is the same rounded rectangle, so a placeholder that merely looked plausible would
//! make the obvious test vacuous **and** let a placeholder render be mistaken for a fidelity
//! render. This one cannot be: every outline it produces carries [`OutlineProvenance::Placeholder`]
//! and a label naming the handle it stands in for, `tests/the_geometry_seam_is_swappable.rs`
//! asserts both, and `mjx-paint`'s `DrawReport::placeholders` counts them so that a golden image
//! taken against one can be refused.

use crate::error::SceneError;
use crate::geometry::{FillRule, Geometry, PathCommand, ScenePoint, SceneRect};

/// How much of a placeholder's shorter side each corner rounds away.
pub const PLACEHOLDER_CORNER_FRACTION: f32 = 0.18;

/// How wide a placeholder's frame is, as a fraction of its shorter side.
///
/// The frame and the cross together are what make a placeholder unmistakable at a glance: a filled
/// rounded rectangle could be a document's own rounded rectangle, and a *hollow* one with a cross
/// through it is not a preset shape DrawingML defines.
pub const PLACEHOLDER_FRAME_FRACTION: f32 = 0.08;

/// Where an outline came from.
///
/// Carried on every [`ResolvedOutline`] so that a stand-in can never be mistaken for the real thing
/// — by a reviewer looking at a render, by a golden-image gate, or by a painter that wants to draw
/// placeholders in a warning colour.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum OutlineProvenance {
    /// The document's own geometry, as the layer that issued the handle resolved it.
    Document,
    /// A stand-in, drawn because no geometry table has been supplied for this handle.
    Placeholder,
}

/// What a provider answers with: an outline in device pixels, what fills it, and where it came from.
#[derive(Clone, PartialEq, Debug)]
pub struct ResolvedOutline {
    /// The steps, in order, in the same device pixels every other coordinate in a display list is
    /// in.
    pub commands: Vec<PathCommand>,
    /// Which side of the outline is inside it.
    pub fill_rule: FillRule,
    /// What this outline is, in words — a preset's name for a real provider, and
    /// [`PlaceholderGeometry::label_for`] for the stand-in.
    pub label: String,
    /// Whether this is the document's geometry or a stand-in for it.
    pub provenance: OutlineProvenance,
}

impl ResolvedOutline {
    /// The outline as a [`Geometry`], with its bounding box computed from the points it names.
    #[must_use]
    pub fn into_geometry(self) -> Geometry {
        Geometry::path(self.commands, self.fill_rule)
    }
}

/// Turns the opaque outline handle a box model issued into a path.
///
/// The companion of [`ResourceResolver`](crate::ResourceResolver), and for the same reason: the
/// handle means something only to the layer that issued it. A PowerPoint scene pairs its fragment
/// tree with a provider that reads `mjx-dml`'s preset and custom geometry; a Markdown one has no
/// shapes at all and never calls this. Nothing here can be answered by this crate.
pub trait GeometryProvider {
    /// The outline `outline` names, drawn to fill `within`.
    ///
    /// `within` is the shape's box **in device pixels** — the rectangle
    /// [`Geometry::Unresolved`] carries — and the returned commands are in the same space, so the
    /// answer is ready to tessellate with no further transform. It is a [`SceneRect`] and never an
    /// [`mjx_layout::Extent`], which is a page count and not a size.
    ///
    /// # Errors
    ///
    /// [`SceneError::UnresolvedOutline`] when the provider does not know the handle, and whatever
    /// else a provider's own resolution can fail with. Answering with an error rather than with an
    /// empty path is deliberate: a shape that silently drew nothing is a defect a reader reports as
    /// *"my slide is missing a box"* and nobody finds.
    fn outline(&self, outline: u64, within: SceneRect) -> Result<ResolvedOutline, SceneError>;
}

/// The provider that stands in for a shape there is no geometry to draw for: a framed, crossed
/// rounded rectangle at the shape's own box.
///
/// **Not what a real document renders with** — that is `mjx-geometry`'s `PresetGeometryProvider`,
/// which this crate may not name because 2.5 is above 1.7. This is what that provider falls through
/// to under `UnknownShapePolicy::StandIn`, and what a caller with no geometry table of its own has.
///
/// Answers every handle, never fails, and labels every answer with the handle it stands in for. See
/// this module's documentation for why the shape is deliberately wrong.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct PlaceholderGeometry;

impl PlaceholderGeometry {
    /// The provider.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// What a placeholder for `outline` is labelled.
    ///
    /// A function rather than a format string written twice, so that a test asserting the label
    /// cannot pass by restating the bug.
    #[must_use]
    pub fn label_for(outline: u64) -> String {
        format!("placeholder for outline {outline}")
    }
}

impl GeometryProvider for PlaceholderGeometry {
    fn outline(&self, outline: u64, within: SceneRect) -> Result<ResolvedOutline, SceneError> {
        Ok(ResolvedOutline {
            commands: placeholder_commands(within),
            // Even-odd, so that the outer and inner rounded rectangles read as a *frame* rather
            // than as a filled box with an invisible contour inside it.
            fill_rule: FillRule::EvenOdd,
            label: PlaceholderGeometry::label_for(outline),
            provenance: OutlineProvenance::Placeholder,
        })
    }
}

/// The four contours of a placeholder: the outer rounded rectangle, the inset one that hollows it
/// into a frame, and the two diagonal bars that cross it.
fn placeholder_commands(within: SceneRect) -> Vec<PathCommand> {
    if within.is_empty() {
        return Vec::new();
    }
    let shorter = within.width().min(within.height());
    let corner = shorter * PLACEHOLDER_CORNER_FRACTION;
    let frame = (shorter * PLACEHOLDER_FRAME_FRACTION).max(f32::MIN_POSITIVE);

    let mut commands = Vec::with_capacity(48);
    rounded_rectangle(&mut commands, within, corner);
    let inner = SceneRect::new(
        within.left + frame,
        within.top + frame,
        within.right - frame,
        within.bottom - frame,
    );
    if !inner.is_empty() {
        rounded_rectangle(&mut commands, inner, (corner - frame).max(0.0));
    }
    diagonal_bar(&mut commands, inner, frame, Diagonal::Falling);
    diagonal_bar(&mut commands, inner, frame, Diagonal::Rising);
    commands
}

/// Which way a placeholder's diagonal bar runs across the box.
#[derive(Clone, Copy)]
enum Diagonal {
    /// Top-left to bottom-right.
    Falling,
    /// Bottom-left to top-right.
    Rising,
}

/// Append one closed rounded rectangle, drawn clockwise from the top-left corner's end.
///
/// The corners are quadratic rather than the usual four cubics: a quadratic through the corner
/// point is not a circular arc, and for a stand-in that is a feature — it is one more way the shape
/// is not something DrawingML draws.
fn rounded_rectangle(into: &mut Vec<PathCommand>, rect: SceneRect, corner: f32) {
    let corner = corner
        .max(0.0)
        .min(rect.width() / 2.0)
        .min(rect.height() / 2.0);
    let (left, top, right, bottom) = (rect.left, rect.top, rect.right, rect.bottom);
    into.push(PathCommand::MoveTo(ScenePoint::new(left + corner, top)));
    into.push(PathCommand::LineTo(ScenePoint::new(right - corner, top)));
    into.push(PathCommand::QuadraticTo {
        control: ScenePoint::new(right, top),
        end: ScenePoint::new(right, top + corner),
    });
    into.push(PathCommand::LineTo(ScenePoint::new(right, bottom - corner)));
    into.push(PathCommand::QuadraticTo {
        control: ScenePoint::new(right, bottom),
        end: ScenePoint::new(right - corner, bottom),
    });
    into.push(PathCommand::LineTo(ScenePoint::new(left + corner, bottom)));
    into.push(PathCommand::QuadraticTo {
        control: ScenePoint::new(left, bottom),
        end: ScenePoint::new(left, bottom - corner),
    });
    into.push(PathCommand::LineTo(ScenePoint::new(left, top + corner)));
    into.push(PathCommand::QuadraticTo {
        control: ScenePoint::new(left, top),
        end: ScenePoint::new(left + corner, top),
    });
    into.push(PathCommand::Close);
}

/// Append one closed quadrilateral running corner to corner across `rect`, `thickness` wide.
fn diagonal_bar(into: &mut Vec<PathCommand>, rect: SceneRect, thickness: f32, which: Diagonal) {
    if rect.is_empty() {
        return;
    }
    let half = (thickness / 2.0).max(f32::MIN_POSITIVE);
    let (start, end) = match which {
        Diagonal::Falling => (
            ScenePoint::new(rect.left, rect.top),
            ScenePoint::new(rect.right, rect.bottom),
        ),
        Diagonal::Rising => (
            ScenePoint::new(rect.left, rect.bottom),
            ScenePoint::new(rect.right, rect.top),
        ),
    };
    let (dx, dy) = (end.x - start.x, end.y - start.y);
    let length = dx.hypot(dy);
    if length <= 0.0 || !length.is_finite() {
        return;
    }
    // The bar's normal, unit length, so the two edges sit `half` either side of the diagonal.
    let (nx, ny) = (-dy / length * half, dx / length * half);
    into.push(PathCommand::MoveTo(ScenePoint::new(
        start.x + nx,
        start.y + ny,
    )));
    into.push(PathCommand::LineTo(ScenePoint::new(end.x + nx, end.y + ny)));
    into.push(PathCommand::LineTo(ScenePoint::new(end.x - nx, end.y - ny)));
    into.push(PathCommand::LineTo(ScenePoint::new(
        start.x - nx,
        start.y - ny,
    )));
    into.push(PathCommand::Close);
}
