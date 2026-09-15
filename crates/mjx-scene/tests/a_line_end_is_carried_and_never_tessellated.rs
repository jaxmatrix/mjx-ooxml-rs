//! An arrowhead reaches the display list and never a triangle (MJXOFF-297).
//!
//! MJX-LEDGER-LIMITATION: an arrowhead (`a:headEnd`, `a:tailEnd`) is carried through the display
//! list and never tessellated, so no line draws its line ends.

use mjx_scene::{
    CompoundStroke, DashPattern, FillRule, Geometry, LineCap, LineEnd, LineEndShape, LineEndSize,
    LineJoin, PathCommand, PlaceholderGeometry, ResourceIndex, ScenePoint, Stroke, StrokeAlignment,
    StrokeGeometry, TessellationOptions, Tessellator,
};
use mjx_text::ScaleBucket;

// A four-pixel line record with the given line ends.
fn a_line(head: LineEnd, tail: LineEnd) -> Stroke {
    Stroke {
        paint: ResourceIndex::new(0),
        width: 4.0,
        cap: LineCap::Flat,
        join: LineJoin::Bevel,
        dash: DashPattern::Solid,
        alignment: StrokeAlignment::Centered,
        compound: CompoundStroke::Single,
        head,
        tail,
    }
}

// The largest triangle arrowhead the schema can state.
fn an_arrowhead() -> LineEnd {
    LineEnd {
        shape: LineEndShape::Triangle,
        width: LineEndSize::Large,
        length: LineEndSize::Large,
    }
}

// The triangles of one open horizontal line, from a tessellator with an empty cache.
fn triangles_of(stroke: Stroke) -> (Vec<f32>, Vec<u32>) {
    let line = Geometry::path(
        vec![
            PathCommand::MoveTo(ScenePoint::new(0.0, 10.0)),
            PathCommand::LineTo(ScenePoint::new(100.0, 10.0)),
        ],
        FillRule::NonZero,
    );
    let mesh = Tessellator::new()
        .stroke(
            &line,
            StrokeGeometry::from_record(stroke),
            &PlaceholderGeometry::new(),
            TessellationOptions::for_bucket(ScaleBucket::from_steps(8)),
        )
        .expect("an open line strokes");
    (mesh.positions().to_vec(), mesh.indices().to_vec())
}

#[test]
fn the_record_carries_the_arrowheads_and_the_stroke_geometry_drops_them() {
    let plain = a_line(LineEnd::default(), LineEnd::default());
    let arrowed = a_line(an_arrowhead(), an_arrowhead());
    assert_ne!(
        plain, arrowed,
        "the display-list record carries both line ends"
    );
    assert_eq!(
        StrokeGeometry::from_record(plain),
        StrokeGeometry::from_record(arrowed),
        "if the stroke geometry now differs, line ends reach the tessellator: delete this suite"
    );
}

#[test]
fn an_arrowed_line_tessellates_to_exactly_the_triangles_of_a_plain_one() {
    let plain = triangles_of(a_line(LineEnd::default(), LineEnd::default()));
    let arrowed = triangles_of(a_line(an_arrowhead(), an_arrowhead()));
    assert!(!plain.1.is_empty(), "the plain line produced no triangles");
    assert_eq!(
        plain, arrowed,
        "an arrowed line drew different triangles, so arrowheads are tessellated: delete this suite"
    );
}
