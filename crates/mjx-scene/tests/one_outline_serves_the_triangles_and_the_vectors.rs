//! **One interpretation of what a shape is**, shared by the rasterisers and the vector exporters.
//!
//! MJX-STAND-IN: this crate is rank 1.7 and `mjx-geometry` is 2.5, so the real provider is an
//! upward edge the layering test refuses by name — and the case that resolves a handle is about
//! the stand-in's own provenance surviving into both the triangles and the vectors.
//!
//! # Why this function had to become public, and what would have happened otherwise
//!
//! [`Tessellator::fill_resolved`] answers with triangles. That is what a rasteriser wants and what a
//! *vector* exporter cannot use: MJXOFF-164's PDF and SVG exporters write paths, and a path written
//! as a shape's trapezoidation is a hundred times the file with a hairline seam along every interior
//! edge in any viewer that antialiases.
//!
//! So an exporter needs the **outline**. Before MJXOFF-164 the only way to get one was to decide for
//! itself what a [`Geometry::Rectangle`] is, which side of a [`Geometry::Path`] is inside it, and
//! what to do with a [`Geometry::Unresolved`] — a second interpretation of the same shape, in a
//! crate above the display list, which is exactly the thing a display list exists to prevent. The
//! answer taken is one public function that the tessellator itself resolves its input with.
//!
//! # What is asserted here
//!
//! 1. **The tessellator resolves through this call and no other.** A rectangle, a path and an
//!    unresolved handle each tessellate into triangles whose bounding box is the bounding box of the
//!    outline [`resolve_outline`] answers with — so an exporter that draws the outline and a painter
//!    that draws the triangles are drawing the same shape.
//! 2. **The provenance travels with the outline**, so a vector exporter can refuse to call a page of
//!    stand-ins a fidelity export exactly as a raster painter can.
//! 3. **A path is borrowed, not copied.** The case a page is full of costs no allocation, which is
//!    the reason the answer is a `Cow` rather than a `Vec`.
//! 4. **The fill rule is the geometry's own**, not a constant: an even-odd path stays even-odd, and a
//!    rectangle — which has no rule of its own to carry — is non-zero.
//!
//! # Proved by mutation
//!
//! * Returning `FillRule::NonZero` unconditionally from `resolve_outline` → case four fails.
//! * Giving `Geometry::Rectangle` the corners in the wrong order (a bow tie) → case one fails, on
//!   the triangles rather than on the outline, because the tessellator reads the same function.
//! * Answering `Cow::Owned(commands.clone())` for a `Geometry::Path` → case three fails.
//! * Dropping the provenance in `resolve_outline` → case two fails.

use std::borrow::Cow;

use mjx_scene::{
    resolve_outline, FillRule, Geometry, GeometryProvider, OutlineProvenance, PathCommand,
    PlaceholderGeometry, ResolvedOutline, SceneError, ScenePoint, SceneRect, TessellationOptions,
    Tessellator,
};

/// The bounding box of a list of path steps, which is what the triangles are compared against.
fn outline_bounds(commands: &[PathCommand]) -> SceneRect {
    let mut bounds: Option<SceneRect> = None;
    let mut absorb = |point: ScenePoint| {
        bounds = Some(match bounds {
            Some(rect) => rect.including(point),
            None => SceneRect::new(point.x, point.y, point.x, point.y),
        });
    };
    for command in commands {
        match command {
            PathCommand::MoveTo(point) | PathCommand::LineTo(point) => absorb(*point),
            PathCommand::QuadraticTo { control, end } => {
                absorb(*control);
                absorb(*end);
            }
            PathCommand::CubicTo {
                first_control,
                second_control,
                end,
            } => {
                absorb(*first_control);
                absorb(*second_control);
                absorb(*end);
            }
            PathCommand::Close => {}
        }
    }
    bounds.unwrap_or(SceneRect::new(0.0, 0.0, 0.0, 0.0))
}

fn triangle_bounds(geometry: &Geometry, provider: &dyn GeometryProvider) -> SceneRect {
    let mut tessellator = Tessellator::new();
    let mesh = tessellator
        .fill(
            geometry,
            provider,
            TessellationOptions::for_bucket(mjx_scene::ScaleBucket::enclosing(16.0)),
        )
        .expect("the shape tessellates");
    mesh.bounds()
}

/// A provider that answers with a triangle, so that case one can tell a real answer from a
/// placeholder's rounded rectangle.
struct ATriangle;

impl GeometryProvider for ATriangle {
    fn outline(&self, handle: u64, within: SceneRect) -> Result<ResolvedOutline, SceneError> {
        Ok(ResolvedOutline {
            commands: vec![
                PathCommand::MoveTo(ScenePoint::new(within.left, within.bottom)),
                PathCommand::LineTo(ScenePoint::new(within.right, within.bottom)),
                PathCommand::LineTo(ScenePoint::new(
                    (within.left + within.right) / 2.0,
                    within.top,
                )),
                PathCommand::Close,
            ],
            fill_rule: FillRule::NonZero,
            label: format!("a triangle for {handle}"),
            provenance: OutlineProvenance::Document,
        })
    }
}

/// How close two edges of a box may be and still be the same box. The tessellator flattens curves,
/// so a box compared to the control points of the outline that produced it is close, not equal;
/// these three shapes are straight-edged, so the tolerance is for floating point alone.
const TOLERANCE: f32 = 0.01;

fn assert_same_box(outline: SceneRect, triangles: SceneRect, what: &str) {
    for (name, a, b) in [
        ("left", outline.left, triangles.left),
        ("top", outline.top, triangles.top),
        ("right", outline.right, triangles.right),
        ("bottom", outline.bottom, triangles.bottom),
    ] {
        assert!(
            (a - b).abs() <= TOLERANCE,
            "{what}: the outline's {name} edge is {a} and the triangles' is {b}. The tessellator \
             resolves its input with `resolve_outline`, so these are the same shape read twice; a \
             difference means an exporter drawing the outline and a painter drawing the triangles \
             would draw two different pictures."
        );
    }
}

#[test]
fn the_triangles_and_the_outline_are_the_same_shape() {
    let placeholder = PlaceholderGeometry::new();

    // A rectangle: computed, with no commands of its own in the display list.
    let rectangle = Geometry::Rectangle(SceneRect::new(4.0, 6.0, 44.0, 26.0));
    let resolved = resolve_outline(&rectangle, &placeholder).expect("a rectangle resolves");
    assert_same_box(
        outline_bounds(&resolved.commands),
        triangle_bounds(&rectangle, &placeholder),
        "a rectangle",
    );

    // A path: the document's own steps.
    let path = Geometry::path(
        vec![
            PathCommand::MoveTo(ScenePoint::new(10.0, 10.0)),
            PathCommand::LineTo(ScenePoint::new(70.0, 12.0)),
            PathCommand::LineTo(ScenePoint::new(40.0, 55.0)),
            PathCommand::Close,
        ],
        FillRule::EvenOdd,
    );
    let resolved = resolve_outline(&path, &placeholder).expect("a path resolves");
    assert_same_box(
        outline_bounds(&resolved.commands),
        triangle_bounds(&path, &placeholder),
        "a path",
    );

    // An unresolved handle, through a provider that answers with something no placeholder draws.
    let unresolved = Geometry::Unresolved {
        outline: 77,
        bounds: SceneRect::new(0.0, 0.0, 30.0, 20.0),
    };
    let resolved = resolve_outline(&unresolved, &ATriangle).expect("a handle resolves");
    assert_same_box(
        outline_bounds(&resolved.commands),
        triangle_bounds(&unresolved, &ATriangle),
        "an unresolved handle",
    );
}

#[test]
fn the_provenance_travels_with_the_outline() {
    let unresolved = Geometry::Unresolved {
        outline: 12,
        bounds: SceneRect::new(0.0, 0.0, 40.0, 30.0),
    };

    let stood_in = resolve_outline(&unresolved, &PlaceholderGeometry::new())
        .expect("a placeholder answers for every handle");
    assert!(
        stood_in.provenance.is_placeholder(),
        "an outline that came out of the placeholder provider must say so — a vector exporter has \
         the same duty a raster painter has, and `DrawReport::placeholders` is what R10 refuses a \
         fidelity claim on"
    );
    assert!(
        stood_in
            .provenance
            .label
            .as_deref()
            .is_some_and(|label| label.contains("12")),
        "the label must name the handle it stands in for, and it says {:?}",
        stood_in.provenance.label
    );

    let real = resolve_outline(&unresolved, &ATriangle).expect("a real provider answers");
    assert!(
        !real.provenance.is_placeholder(),
        "the same handle through a provider that answers with the document's own geometry is not a \
         stand-in; a field that said `Placeholder` either way would be a constant"
    );
}

#[test]
fn a_path_is_borrowed_and_the_two_computed_shapes_are_owned() {
    let placeholder = PlaceholderGeometry::new();
    let path = Geometry::path(
        vec![
            PathCommand::MoveTo(ScenePoint::new(0.0, 0.0)),
            PathCommand::LineTo(ScenePoint::new(10.0, 0.0)),
            PathCommand::LineTo(ScenePoint::new(10.0, 10.0)),
            PathCommand::Close,
        ],
        FillRule::NonZero,
    );
    let resolved = resolve_outline(&path, &placeholder).expect("a path resolves");
    assert!(
        matches!(resolved.commands, Cow::Borrowed(_)),
        "a `Geometry::Path` is the case a page is full of, and copying its steps on every ask would \
         make a cache hit most of a cache miss. That is the whole reason the answer is a `Cow`."
    );

    let rectangle = Geometry::Rectangle(SceneRect::new(0.0, 0.0, 1.0, 1.0));
    let resolved = resolve_outline(&rectangle, &placeholder).expect("a rectangle resolves");
    assert!(
        matches!(resolved.commands, Cow::Owned(_)),
        "a rectangle has no steps stored anywhere, so its outline has to be computed"
    );
}

#[test]
fn the_fill_rule_is_the_geometrys_own() {
    let placeholder = PlaceholderGeometry::new();

    let even_odd = Geometry::path(
        vec![
            PathCommand::MoveTo(ScenePoint::new(0.0, 0.0)),
            PathCommand::LineTo(ScenePoint::new(20.0, 0.0)),
            PathCommand::LineTo(ScenePoint::new(20.0, 20.0)),
            PathCommand::Close,
        ],
        FillRule::EvenOdd,
    );
    assert_eq!(
        resolve_outline(&even_odd, &placeholder)
            .expect("a path resolves")
            .fill_rule,
        FillRule::EvenOdd,
        "an even-odd path resolves even-odd; a constant here would make every self-intersecting \
         shape solid in the exporters and hollow in the rasterisers"
    );

    let rectangle = Geometry::Rectangle(SceneRect::new(0.0, 0.0, 5.0, 5.0));
    assert_eq!(
        resolve_outline(&rectangle, &placeholder)
            .expect("a rectangle resolves")
            .fill_rule,
        FillRule::NonZero,
        "a rectangle carries no rule of its own and is wound once"
    );
}
