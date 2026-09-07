//! The tessellator, tested against paths that are **not** the placeholder.
//!
//! MJX-STAND-IN: this crate is rank 1.7 and `mjx-geometry` is 2.5, so the real provider is an
//! upward edge `xtask/tests/layering.rs` refuses by name. The stand-in appears here only to
//! resolve a handle; every shape this file tessellates is written out by hand instead.
//!
//! # The trap this file exists to defeat
//!
//! *"Every shape tessellates"* is trivially true while every shape is the same rounded rectangle.
//! [`PlaceholderGeometry`] makes that gate vacuous **by construction**, so nothing below asks a
//! provider for anything: every path here is hand-built, and every assertion is a triangle count, an
//! enclosed area or a bounding box that follows from the path's own geometry rather than from what
//! the tessellator happened to produce.
//!
//! The two exceptions are the pinned vertex buffers at the end, which are lyon's output written down
//! by hand — and they are the point of R10's golden images: if a platform, a compiler or a `lyon`
//! release changes a single coordinate, those two tests say so before an image comparison does.
//!
//! # Why areas and not vertex counts, mostly
//!
//! A winding rule is a claim about *what a path encloses*, and a triangle count is a claim about how
//! a tessellator chose to cover it. Summing the triangles' areas asks the first question. That is
//! why [`same_winding_contours_are_solid_under_non_zero_and_hollow_under_even_odd`] compares 400
//! against 300 rather than 10 triangles against 8: the areas are what the rules *mean*, and they
//! would still be right if lyon covered them differently.
//!
//! # Proved by mutation
//!
//! Every test here was proved to fail before it was trusted; the neutralisations are recorded in the
//! MJXOFF-162 report. Two are worth naming.
//!
//! The **fill rule**: mapping both [`FillRule`] variants to `NonZero` in `Tessellator::fill` leaves
//! **every** count in this file right except the two areas in the winding-rule test, which is
//! exactly why the areas are here.
//!
//! The **compound bands**: swapping `ThickThin` and `ThinThick` was green against triangle counts,
//! because `Double`, `ThickThin` and `ThinThick` are all two bands and all ten triangles. What tells
//! them apart is *which side the ink is on*, so that test measures area either side of the
//! centreline instead — and asserts the two are each other's mirror, which no swap of the two tables
//! can also satisfy.

use mjx_scene::{
    Color, CompoundStroke, DashPattern, FillRule, FillStyle, Geometry, LineCap, LineEnd, LineJoin,
    Mesh, PathCommand, PlaceholderGeometry, ResourceIndex, ScenePoint, SceneRect, Stroke,
    StrokeAlignment, StrokeGeometry, StrokeStyle, TessellationOptions, Tessellator,
    COORDINATE_LIMIT,
};
use mjx_text::ScaleBucket;

// -------------------------------------------------------------------------------------------
// Helpers
// -------------------------------------------------------------------------------------------

/// A bucket to tessellate at. Which one does not matter here — nothing below reads it — but it has
/// to be *a* bucket, because a tessellation without a scale is not a tessellation.
fn at_scale() -> TessellationOptions {
    TessellationOptions::for_bucket(ScaleBucket::from_steps(8))
}

/// The provider is never consulted by anything in this file, because nothing here is unresolved.
fn unconsulted() -> PlaceholderGeometry {
    PlaceholderGeometry::new()
}

fn at(x: f32, y: f32) -> ScenePoint {
    ScenePoint::new(x, y)
}

/// A closed polygon through `points`, in order.
fn closed(points: &[(f32, f32)]) -> Vec<PathCommand> {
    let mut commands = vec![PathCommand::MoveTo(at(points[0].0, points[0].1))];
    for (x, y) in &points[1..] {
        commands.push(PathCommand::LineTo(at(*x, *y)));
    }
    commands.push(PathCommand::Close);
    commands
}

/// An open polyline through `points`, in order.
fn open(points: &[(f32, f32)]) -> Vec<PathCommand> {
    let mut commands = vec![PathCommand::MoveTo(at(points[0].0, points[0].1))];
    for (x, y) in &points[1..] {
        commands.push(PathCommand::LineTo(at(*x, *y)));
    }
    commands
}

/// How much area the mesh's triangles cover, counted without regard to their orientation.
///
/// The question a winding rule answers, asked of the triangles rather than of the tessellator.
fn area_of(mesh: &Mesh) -> f32 {
    let positions = mesh.positions();
    let mut total = 0.0_f32;
    for triangle in mesh.indices().as_chunks::<3>().0 {
        let corner = |which: usize| -> (f32, f32) {
            let at = triangle[which] as usize * 2;
            (positions[at], positions[at + 1])
        };
        let (ax, ay) = corner(0);
        let (bx, by) = corner(1);
        let (cx, cy) = corner(2);
        total += ((bx - ax) * (cy - ay) - (cx - ax) * (by - ay)).abs() / 2.0;
    }
    total
}

/// A plain stroke of `width`, with everything else at its least surprising setting.
fn plain_stroke(width: f32) -> StrokeGeometry {
    StrokeGeometry {
        width,
        cap: LineCap::Flat,
        join: LineJoin::Bevel,
        dash: DashPattern::Solid,
        alignment: StrokeAlignment::Centered,
        compound: CompoundStroke::Single,
    }
}

#[track_caller]
fn assert_close(what: &str, produced: f32, expected: f32, slack: f32) {
    assert!(
        (produced - expected).abs() <= slack,
        "{what}: {produced} is not within {slack} of {expected}"
    );
}

// -------------------------------------------------------------------------------------------
// Fills
// -------------------------------------------------------------------------------------------

/// Two squares, the inner inside the outer, wound the **same** way.
fn nested_squares_wound_alike() -> Vec<PathCommand> {
    let mut commands = closed(&[(0.0, 0.0), (20.0, 0.0), (20.0, 20.0), (0.0, 20.0)]);
    commands.extend(closed(&[
        (5.0, 5.0),
        (15.0, 5.0),
        (15.0, 15.0),
        (5.0, 15.0),
    ]));
    commands
}

#[test]
fn same_winding_contours_are_solid_under_non_zero_and_hollow_under_even_odd() {
    let mut tessellator = Tessellator::new();

    // Both contours wind the same way, so inside the inner square the winding number is two and the
    // crossing count is even. Non-zero fills 20 × 20; even-odd fills that less the 10 × 10 hole.
    let solid = tessellator
        .fill(
            &Geometry::path(nested_squares_wound_alike(), FillRule::NonZero),
            &unconsulted(),
            at_scale(),
        )
        .expect("two nested squares tessellate");
    let hollow = tessellator
        .fill(
            &Geometry::path(nested_squares_wound_alike(), FillRule::EvenOdd),
            &unconsulted(),
            at_scale(),
        )
        .expect("the same two squares tessellate under the other rule");

    assert_close("the non-zero fill", area_of(&solid), 400.0, 0.01);
    assert_close("the even-odd fill", area_of(&hollow), 300.0, 0.01);
    assert_eq!(solid.triangle_count(), 10);
    assert_eq!(hollow.triangle_count(), 8);
    assert_eq!(solid.bounds(), SceneRect::new(0.0, 0.0, 20.0, 20.0));
    assert_eq!(hollow.bounds(), solid.bounds(), "the box is the same box");
}

#[test]
fn a_hole_is_a_hole_under_either_rule_when_the_contours_wind_oppositely() {
    let mut commands = closed(&[(0.0, 0.0), (20.0, 0.0), (20.0, 20.0), (0.0, 20.0)]);
    commands.extend(closed(&[
        (5.0, 5.0),
        (5.0, 15.0),
        (15.0, 15.0),
        (15.0, 5.0),
    ]));
    let mut tessellator = Tessellator::new();

    // The way a glyph, a `custGeom` with a counter-contour and every donut in every document says
    // "hole": reverse the inner contour. Then the winding number inside it is zero as well as the
    // crossing count being even, and the two rules agree.
    for rule in [FillRule::NonZero, FillRule::EvenOdd] {
        let mesh = tessellator
            .fill(
                &Geometry::path(commands.clone(), rule),
                &unconsulted(),
                at_scale(),
            )
            .expect("a square with a hole tessellates");
        assert_close(&format!("the {rule:?} fill"), area_of(&mesh), 300.0, 0.01);
        assert_eq!(mesh.triangle_count(), 8, "under {rule:?}");
    }
}

#[test]
fn a_self_intersecting_path_encloses_half_the_box_it_spans() {
    // A bowtie: the diagonals cross, and the two lobes it encloses are a quarter of the square each.
    let bowtie = closed(&[(0.0, 0.0), (10.0, 10.0), (10.0, 0.0), (0.0, 10.0)]);
    let mut tessellator = Tessellator::new();
    for rule in [FillRule::NonZero, FillRule::EvenOdd] {
        let mesh = tessellator
            .fill(
                &Geometry::path(bowtie.clone(), rule),
                &unconsulted(),
                at_scale(),
            )
            .expect("a self-intersecting path tessellates");
        assert_close(&format!("the {rule:?} fill"), area_of(&mesh), 50.0, 0.01);
        assert_eq!(mesh.bounds(), SceneRect::new(0.0, 0.0, 10.0, 10.0));
        assert!(
            mesh.vertex_count() > 4,
            "the crossing has to become a vertex; {rule:?} produced only {}",
            mesh.vertex_count()
        );
    }
}

#[test]
fn the_flattening_tolerance_decides_how_many_triangles_a_curve_becomes() {
    let curve = vec![
        PathCommand::MoveTo(at(0.0, 0.0)),
        PathCommand::CubicTo {
            first_control: at(0.0, 40.0),
            second_control: at(60.0, 40.0),
            end: at(60.0, 0.0),
        },
        PathCommand::Close,
    ];
    let mut tessellator = Tessellator::new();
    let bucket = ScaleBucket::from_steps(8);
    let fine = tessellator
        .fill(
            &Geometry::path(curve.clone(), FillRule::NonZero),
            &unconsulted(),
            TessellationOptions::with_tolerance(bucket, 0.25),
        )
        .expect("a cubic flattens finely")
        .triangle_count();
    let coarse = tessellator
        .fill(
            &Geometry::path(curve, FillRule::NonZero),
            &unconsulted(),
            TessellationOptions::with_tolerance(bucket, 4.0),
        )
        .expect("and coarsely")
        .triangle_count();

    assert_eq!(fine, 14);
    assert_eq!(coarse, 3);
    assert!(
        fine > coarse * 3,
        "a sixteen-fold coarser tolerance produced {coarse} triangles against {fine}, which is not \
         a flattener responding to its tolerance at all"
    );
}

// -------------------------------------------------------------------------------------------
// Strokes
// -------------------------------------------------------------------------------------------

/// A right angle: two segments of twenty pixels meeting at `(20, 0)`.
fn a_right_angle() -> Geometry {
    Geometry::path(
        open(&[(0.0, 0.0), (20.0, 0.0), (20.0, 20.0)]),
        FillRule::NonZero,
    )
}

#[test]
fn each_line_join_makes_a_different_corner() {
    let mut tessellator = Tessellator::new();
    let mut triangles = Vec::new();
    for join in [
        LineJoin::Miter { limit: 4.0 },
        LineJoin::Bevel,
        LineJoin::Round,
    ] {
        let mesh = tessellator
            .stroke(
                &a_right_angle(),
                StrokeGeometry {
                    join,
                    ..plain_stroke(4.0)
                },
                &unconsulted(),
                at_scale(),
            )
            .expect("a right angle strokes");
        // Flat caps, so the stroke reaches exactly the end points and no further, and the outside
        // of the corner reaches half a width past it.
        assert_eq!(
            mesh.bounds(),
            SceneRect::new(0.0, -2.0, 22.0, 20.0),
            "with {join:?}"
        );
        triangles.push(mesh.triangle_count());
    }
    // A mitre is one quadrilateral at the corner, a bevel adds a triangle, and a round join adds
    // the arc's segments on top of that. Two joins that produced the same triangles would be two
    // joins the stroker is not distinguishing.
    assert_eq!(triangles, vec![4, 5, 6]);
}

#[test]
fn each_line_cap_makes_a_different_end() {
    let mut tessellator = Tessellator::new();
    let flat = tessellator
        .stroke(
            &a_right_angle(),
            plain_stroke(4.0),
            &unconsulted(),
            at_scale(),
        )
        .expect("a flat cap");
    let square = tessellator
        .stroke(
            &a_right_angle(),
            StrokeGeometry {
                cap: LineCap::Square,
                ..plain_stroke(4.0)
            },
            &unconsulted(),
            at_scale(),
        )
        .expect("a square cap");
    let round = tessellator
        .stroke(
            &a_right_angle(),
            StrokeGeometry {
                cap: LineCap::Round,
                ..plain_stroke(4.0)
            },
            &unconsulted(),
            at_scale(),
        )
        .expect("a round cap");

    // A flat cap stops at the end point; the other two reach half a width past it, at both ends.
    assert_eq!(flat.bounds(), SceneRect::new(0.0, -2.0, 22.0, 20.0));
    assert_eq!(square.bounds(), SceneRect::new(-2.0, -2.0, 22.0, 22.0));
    assert_eq!(round.bounds(), SceneRect::new(-2.0, -2.0, 22.0, 22.0));
    // A square cap is the same rectangle extended, so it costs no triangles; a round one is an arc
    // at each end and costs six.
    assert_eq!(square.triangle_count(), flat.triangle_count());
    assert_eq!(round.triangle_count(), flat.triangle_count() + 6);
}

#[test]
fn a_miter_limit_bevels_a_corner_that_would_spike() {
    // A corner of about three degrees: the mitre would reach roughly forty times the line width
    // past it, so a limit of four has to cut it off and a limit of forty does not.
    let spike = Geometry::path(
        open(&[(0.0, 0.0), (40.0, 0.0), (0.0, 2.0)]),
        FillRule::NonZero,
    );
    let mut tessellator = Tessellator::new();
    let held = tessellator
        .stroke(
            &spike,
            StrokeGeometry {
                join: LineJoin::Miter { limit: 4.0 },
                ..plain_stroke(4.0)
            },
            &unconsulted(),
            at_scale(),
        )
        .expect("a limited mitre");
    let loosed = tessellator
        .stroke(
            &spike,
            StrokeGeometry {
                join: LineJoin::Miter { limit: 40.0 },
                ..plain_stroke(4.0)
            },
            &unconsulted(),
            at_scale(),
        )
        .expect("an unlimited one");

    assert_close("the bevelled corner", held.bounds().right, 40.1, 0.1);
    assert_close("the mitred corner", loosed.bounds().right, 120.05, 0.1);
    assert!(
        loosed.bounds().width() > held.bounds().width() * 3.0,
        "the limit did not change the corner: {} against {}",
        loosed.bounds().width(),
        held.bounds().width()
    );
}

#[test]
fn a_dashed_compound_line_is_more_pieces_than_a_solid_single_one() {
    let mut tessellator = Tessellator::new();
    let solid_single = tessellator
        .stroke(
            &a_right_angle(),
            plain_stroke(6.0),
            &unconsulted(),
            at_scale(),
        )
        .expect("one solid line")
        .triangle_count();

    // Each compound is a different number of parallel bands, so each is a different number of
    // triangles even before the line is dashed.
    let mut by_compound = Vec::new();
    for compound in [
        CompoundStroke::Single,
        CompoundStroke::Double,
        CompoundStroke::ThickThin,
        CompoundStroke::ThinThick,
        CompoundStroke::Triple,
    ] {
        let mesh = tessellator
            .stroke(
                &a_right_angle(),
                StrokeGeometry {
                    compound,
                    ..plain_stroke(6.0)
                },
                &unconsulted(),
                at_scale(),
            )
            .expect("a compound line strokes");
        by_compound.push(mesh.triangle_count());
    }
    assert_eq!(by_compound, vec![5, 10, 10, 10, 15]);

    // And dashing cuts every band into pieces: `sysDot` is one width on, one width off, so a forty
    // pixel path at a six pixel width is a handful of dashes per band.
    let dashed = tessellator
        .stroke(
            &a_right_angle(),
            StrokeGeometry {
                dash: DashPattern::SystemDot,
                compound: CompoundStroke::Triple,
                ..plain_stroke(6.0)
            },
            &unconsulted(),
            at_scale(),
        )
        .expect("a dashed compound line strokes");
    assert_eq!(dashed.triangle_count(), 25);
    assert!(
        dashed.triangle_count() > solid_single * 4,
        "three dashed bands produced {} triangles against one solid line's {solid_single}, which \
         is not a dash and a compound at all",
        dashed.triangle_count()
    );
    // A dash removes ink; the box it is drawn in is the box the solid line was drawn in.
    assert_eq!(dashed.bounds().right, 23.0);
}

/// How much of the mesh's area lies on each side of the line `y = 0`.
///
/// The question a compound stroke's *asymmetry* is made of, and one a triangle count cannot answer.
fn area_either_side_of_the_centreline(mesh: &Mesh) -> (f32, f32) {
    let positions = mesh.positions();
    let (mut above, mut below) = (0.0_f32, 0.0_f32);
    for triangle in mesh.indices().as_chunks::<3>().0 {
        let corner = |which: usize| -> (f32, f32) {
            let at = triangle[which] as usize * 2;
            (positions[at], positions[at + 1])
        };
        let (ax, ay) = corner(0);
        let (bx, by) = corner(1);
        let (cx, cy) = corner(2);
        let area = ((bx - ax) * (cy - ay) - (cx - ax) * (by - ay)).abs() / 2.0;
        // `y` increases downward, so a smaller `y` is higher on the page.
        if (ay + by + cy) / 3.0 < 0.0 {
            above += area;
        } else {
            below += area;
        }
    }
    (above, below)
}

#[test]
fn a_thick_thin_compound_puts_its_ink_on_the_other_side_from_a_thin_thick_one() {
    // **Triangle counts cannot tell these two apart** — `Double`, `ThickThin` and `ThinThick` are
    // all two bands and all ten triangles, so swapping the thick and the thin band was a green
    // mutation. What distinguishes them is *where the ink is*, and that is what is asserted.
    //
    // The bands are placed along the left normal, which for a left-to-right segment points down the
    // page. `ThickThin` is thick-then-thin along that normal, so its thick band is the *upper* one.
    let line = Geometry::path(open(&[(0.0, 0.0), (200.0, 0.0)]), FillRule::NonZero);
    let mut tessellator = Tessellator::new();
    let measure = |tessellator: &mut Tessellator, compound| {
        let mesh = tessellator
            .stroke(
                &line,
                StrokeGeometry {
                    compound,
                    ..plain_stroke(12.0)
                },
                &unconsulted(),
                at_scale(),
            )
            .expect("a compound line strokes");
        (
            area_either_side_of_the_centreline(&mesh),
            mesh.vertex_bytes(),
        )
    };

    let ((thick_above, thin_below), thick_thin) =
        measure(&mut tessellator, CompoundStroke::ThickThin);
    let ((thin_above, thick_below), thin_thick) =
        measure(&mut tessellator, CompoundStroke::ThinThick);
    let ((upper, lower), double) = measure(&mut tessellator, CompoundStroke::Double);

    // The thick band is twice the thin one, so it carries twice the ink.
    assert!(
        thick_above > thin_below * 1.5,
        "a thick-thin compound put {thick_above:.0} above the centreline and {thin_below:.0} \
         below, which is not a thick band and a thin one"
    );
    assert!(
        thick_below > thin_above * 1.5,
        "a thin-thick compound put {thin_above:.0} above and {thick_below:.0} below"
    );
    // And the two are each other's mirror, which no swap of the two tables can also satisfy.
    assert_close("the mirrored thick band", thick_above, thick_below, 0.01);
    assert_close("the mirrored thin band", thin_below, thin_above, 0.01);
    assert_ne!(
        thick_thin, thin_thick,
        "the two compounds produced identical triangles"
    );

    // A double line is symmetric, which is what makes the asymmetry above meaningful rather than an
    // artefact of how bands are placed at all.
    assert_close("the two halves of a double line", upper, lower, 0.01);
    assert_ne!(double, thick_thin);
    assert_ne!(double, thin_thick);
}

#[test]
fn an_inset_stroke_lands_inside_whichever_way_the_contour_is_wound() {
    // `Inset` means *the whole width inside the path*, and "inside" is decided by the contour's
    // winding — there is no other way to know. Reversing a square reverses the sign of its area, so
    // a stroker that ignored the sign would put the ink outside for exactly half of all documents.
    //
    // Every closed contour in the suite wound the same way, so losing the sign was a green mutation.
    let clockwise = Geometry::path(
        closed(&[(0.0, 0.0), (20.0, 0.0), (20.0, 20.0), (0.0, 20.0)]),
        FillRule::NonZero,
    );
    let widdershins = Geometry::path(
        closed(&[(0.0, 0.0), (0.0, 20.0), (20.0, 20.0), (20.0, 0.0)]),
        FillRule::NonZero,
    );
    let inset = StrokeGeometry {
        join: LineJoin::Miter { limit: 4.0 },
        alignment: StrokeAlignment::Inset,
        ..plain_stroke(4.0)
    };

    let mut tessellator = Tessellator::new();
    for (name, geometry) in [("clockwise", &clockwise), ("anticlockwise", &widdershins)] {
        let bounds = tessellator
            .stroke(geometry, inset, &unconsulted(), at_scale())
            .expect("an inset stroke")
            .bounds();
        assert_close(
            &format!("the {name} inset stroke's left edge"),
            bounds.left,
            0.0,
            0.001,
        );
        assert_close(
            &format!("the {name} inset stroke's top edge"),
            bounds.top,
            0.0,
            0.001,
        );
        assert_close(
            &format!("the {name} inset stroke's right edge"),
            bounds.right,
            20.0,
            0.001,
        );
        assert_close(
            &format!("the {name} inset stroke's bottom edge"),
            bounds.bottom,
            20.0,
            0.001,
        );
    }
}

#[test]
fn an_inset_stroke_stays_inside_the_path_and_a_centred_one_straddles_it() {
    let square = Geometry::path(
        closed(&[(0.0, 0.0), (20.0, 0.0), (20.0, 20.0), (0.0, 20.0)]),
        FillRule::NonZero,
    );
    let mut tessellator = Tessellator::new();
    let centred = tessellator
        .stroke(
            &square,
            StrokeGeometry {
                join: LineJoin::Miter { limit: 4.0 },
                ..plain_stroke(4.0)
            },
            &unconsulted(),
            at_scale(),
        )
        .expect("a centred stroke");
    let inset = tessellator
        .stroke(
            &square,
            StrokeGeometry {
                join: LineJoin::Miter { limit: 4.0 },
                alignment: StrokeAlignment::Inset,
                ..plain_stroke(4.0)
            },
            &unconsulted(),
            at_scale(),
        )
        .expect("an inset stroke");

    assert_eq!(centred.bounds(), SceneRect::new(-2.0, -2.0, 22.0, 22.0));
    let bounds = inset.bounds();
    assert_close("the inset stroke's left edge", bounds.left, 0.0, 0.001);
    assert_close("the inset stroke's top edge", bounds.top, 0.0, 0.001);
    assert_close("the inset stroke's right edge", bounds.right, 20.0, 0.001);
    assert_close("the inset stroke's bottom edge", bounds.bottom, 20.0, 0.001);
}

// -------------------------------------------------------------------------------------------
// Paths a document can produce and a tessellator must survive
// -------------------------------------------------------------------------------------------

#[test]
fn degenerate_paths_produce_empty_or_clamped_meshes_and_never_panic() {
    let cases: Vec<(&str, Vec<PathCommand>)> = vec![
        ("nothing at all", Vec::new()),
        (
            "one move and no line",
            vec![PathCommand::MoveTo(at(1.0, 1.0))],
        ),
        (
            "a zero-length contour",
            closed(&[(1.0, 1.0), (1.0, 1.0), (1.0, 1.0)]),
        ),
        (
            "a line before any move",
            vec![PathCommand::LineTo(at(5.0, 5.0)), PathCommand::Close],
        ),
        (
            "a NaN and an infinity",
            vec![
                PathCommand::MoveTo(ScenePoint {
                    x: f32::NAN,
                    y: 0.0,
                }),
                PathCommand::LineTo(ScenePoint {
                    x: f32::INFINITY,
                    y: f32::NAN,
                }),
                PathCommand::LineTo(at(3.0, 3.0)),
                PathCommand::Close,
            ],
        ),
        (
            "coordinates past every limit",
            vec![
                PathCommand::MoveTo(ScenePoint {
                    x: 1.0e30,
                    y: -1.0e30,
                }),
                PathCommand::LineTo(ScenePoint {
                    x: 1.0e30,
                    y: 1.0e30,
                }),
                PathCommand::LineTo(at(0.0, 0.0)),
                PathCommand::Close,
            ],
        ),
    ];
    let mut tessellator = Tessellator::new();
    for (what, commands) in cases {
        let geometry = Geometry::path(commands, FillRule::NonZero);
        let filled = tessellator
            .fill(&geometry, &unconsulted(), at_scale())
            .unwrap_or_else(|error| panic!("filling {what}: {error}"));
        // Everything that survives is inside the clamp, and no coordinate is a NaN — a NaN in a
        // vertex buffer is a triangle a GPU discards silently and a bounding box that never
        // compares equal to itself.
        for value in filled.positions() {
            assert!(
                value.is_finite() && value.abs() <= COORDINATE_LIMIT,
                "filling {what} produced the coordinate {value}"
            );
        }
        // Everything a stroker can be asked to do at once, on the same path.
        let stroked = tessellator
            .stroke(
                &geometry,
                StrokeGeometry {
                    width: 2.0,
                    cap: LineCap::Round,
                    join: LineJoin::Miter { limit: 0.0 },
                    dash: DashPattern::LargeDashDotDot,
                    alignment: StrokeAlignment::Inset,
                    compound: CompoundStroke::Triple,
                },
                &unconsulted(),
                at_scale(),
            )
            .unwrap_or_else(|error| panic!("stroking {what}: {error}"));
        for value in stroked.positions() {
            assert!(
                value.is_finite() && value.abs() <= COORDINATE_LIMIT * 2.0,
                "stroking {what} produced the coordinate {value}"
            );
        }
    }

    // A hairline dash on a page-long path is tens of millions of segments; it is drawn solid rather
    // than drawn for an hour. Same triangles as an undashed hairline, and no more.
    let long = Geometry::path(
        open(&[(0.0, 0.0), (COORDINATE_LIMIT, 0.0)]),
        FillRule::NonZero,
    );
    let hairline = StrokeGeometry {
        width: 0.001,
        dash: DashPattern::SystemDot,
        ..plain_stroke(0.001)
    };
    let mesh = tessellator
        .stroke(&long, hairline, &unconsulted(), at_scale())
        .expect("a hairline dash over a million pixels does not run away");
    assert_eq!(
        mesh.triangle_count(),
        2,
        "the dash was not refused; it produced {} triangles",
        mesh.triangle_count()
    );

    // And a stroke of no width draws nothing rather than dividing by it.
    let nothing = tessellator
        .stroke(
            &a_right_angle(),
            plain_stroke(0.0),
            &unconsulted(),
            at_scale(),
        )
        .expect("a zero-width stroke");
    assert!(nothing.is_empty());
    assert_eq!(nothing.bounds(), SceneRect::EMPTY);
    assert_eq!(nothing.vertex_bytes(), Vec::<u8>::new());
    assert_eq!(nothing.index_bytes(), Vec::<u8>::new());
}

// -------------------------------------------------------------------------------------------
// Determinism
// -------------------------------------------------------------------------------------------

#[test]
fn the_same_path_at_the_same_tolerance_is_the_same_bytes() {
    // Everything with a transcendental in it — a round join, a round cap, a flattened cubic —
    // tessellated twice through two *different* tessellators, so no cache can make this pass.
    let curvy = Geometry::path(
        vec![
            PathCommand::MoveTo(at(1.5, 2.5)),
            PathCommand::CubicTo {
                first_control: at(1.5, 42.5),
                second_control: at(61.5, 42.5),
                end: at(61.5, 2.5),
            },
            PathCommand::QuadraticTo {
                control: at(30.0, -20.0),
                end: at(1.5, 2.5),
            },
            PathCommand::Close,
        ],
        FillRule::EvenOdd,
    );
    let round = StrokeGeometry {
        width: 3.0,
        cap: LineCap::Round,
        join: LineJoin::Round,
        dash: DashPattern::DashDot,
        alignment: StrokeAlignment::Inset,
        compound: CompoundStroke::ThickThin,
    };

    let mut first = Tessellator::new();
    let mut second = Tessellator::new();
    let one = first
        .fill(&curvy, &unconsulted(), at_scale())
        .expect("a curve fills");
    let other = second
        .fill(&curvy, &unconsulted(), at_scale())
        .expect("the same curve fills again");
    assert_eq!(one.vertex_bytes(), other.vertex_bytes());
    assert_eq!(one.index_bytes(), other.index_bytes());
    assert!(!one.vertex_bytes().is_empty());

    let one = first
        .stroke(&curvy, round, &unconsulted(), at_scale())
        .expect("a curve strokes");
    let other = second
        .stroke(&curvy, round, &unconsulted(), at_scale())
        .expect("the same curve strokes again");
    assert_eq!(one.vertex_bytes(), other.vertex_bytes());
    assert_eq!(one.index_bytes(), other.index_bytes());
    assert!(
        one.triangle_count() > 20,
        "a dashed compound curve is more than twenty triangles"
    );
}

#[test]
fn a_stroke_described_two_ways_is_the_same_triangles() {
    // A decoration carries a `StrokeStyle`, a display list carries a `Stroke` record, and the two
    // differ only in whether the paint is owned or addressed. Neither has any effect on the
    // triangles, and a painter that reached the tessellator through one door must not get a
    // different outline from one that reached it through the other.
    let style = StrokeStyle {
        fill: FillStyle::Solid(Color {
            red: 0x11,
            green: 0x22,
            blue: 0x33,
            alpha: 0xff,
        }),
        width: 5.0,
        cap: LineCap::Square,
        join: LineJoin::Miter { limit: 2.5 },
        dash: DashPattern::LargeDashDot,
        alignment: StrokeAlignment::Centered,
        compound: CompoundStroke::ThinThick,
        head: LineEnd::default(),
        tail: LineEnd::default(),
    };
    let record = Stroke {
        paint: ResourceIndex::new(7),
        width: style.width,
        cap: style.cap,
        join: style.join,
        dash: style.dash,
        alignment: style.alignment,
        compound: style.compound,
        head: style.head,
        tail: style.tail,
    };
    assert_eq!(
        StrokeGeometry::from_style(&style),
        StrokeGeometry::from_record(record)
    );

    let mut tessellator = Tessellator::new();
    let from_style = tessellator
        .stroke(
            &a_right_angle(),
            StrokeGeometry::from_style(&style),
            &unconsulted(),
            at_scale(),
        )
        .expect("a decoration's stroke tessellates");
    let from_record = tessellator
        .stroke(
            &a_right_angle(),
            StrokeGeometry::from_record(record),
            &unconsulted(),
            at_scale(),
        )
        .expect("and so does a display list's");
    assert_eq!(from_style.vertex_bytes(), from_record.vertex_bytes());
    assert!(!from_style.is_empty());
    // The same key, so the second ask was answered from the cache rather than tessellated again.
    assert_eq!(tessellator.cache().hits(), 1);
}

#[test]
fn a_triangle_is_pinned_to_the_vertex_buffer_it_produces() {
    // Written out as IEEE-754 bit patterns rather than as `8.0_f32.to_le_bytes()`, for the reason
    // `tests/the_encoding_is_pinned_to_literals.rs` gives: restating the arithmetic pins nothing.
    // 8.0 is 0x4100_0000 and 4.0 is 0x4080_0000.
    let mesh = Tessellator::new()
        .fill(
            &Geometry::path(
                closed(&[(0.0, 0.0), (8.0, 0.0), (0.0, 4.0)]),
                FillRule::NonZero,
            ),
            &unconsulted(),
            at_scale(),
        )
        .expect("a triangle tessellates");

    #[rustfmt::skip]
    let vertices: Vec<u8> = vec![
        0x00, 0x00, 0x00, 0x00,   0x00, 0x00, 0x00, 0x00,   // (0, 0)
        0x00, 0x00, 0x00, 0x41,   0x00, 0x00, 0x00, 0x00,   // (8, 0)
        0x00, 0x00, 0x00, 0x00,   0x00, 0x00, 0x80, 0x40,   // (0, 4)
    ];
    #[rustfmt::skip]
    let indices: Vec<u8> = vec![
        0x01, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
        0x02, 0x00, 0x00, 0x00,
    ];
    assert_eq!(
        mesh.vertex_bytes(),
        vertices,
        "the vertex buffer of a straight-edged triangle is exact arithmetic and cannot drift; a \
         difference here is a platform difference or a `lyon` release changing its output, and \
         either one invalidates every golden image above"
    );
    assert_eq!(mesh.index_bytes(), indices);
    assert_eq!(mesh.vertex_count(), 3);
    assert_eq!(mesh.triangle_count(), 1);
    assert_eq!(mesh.byte_len(), 24 + 12);
}

#[test]
fn a_rectangle_is_pinned_to_the_two_triangles_it_produces() {
    // 10.0 is 0x4120_0000 and 6.0 is 0x40c0_0000. A `Geometry::Rectangle` is not special-cased
    // anywhere in the tessellator: it goes through the same path builder every other geometry does,
    // and these are the four corners it names.
    let mesh = Tessellator::new()
        .fill(
            &Geometry::Rectangle(SceneRect::new(0.0, 0.0, 10.0, 6.0)),
            &unconsulted(),
            at_scale(),
        )
        .expect("a rectangle tessellates");

    #[rustfmt::skip]
    let vertices: Vec<u8> = vec![
        0x00, 0x00, 0x00, 0x00,   0x00, 0x00, 0x00, 0x00,   // (0, 0)
        0x00, 0x00, 0x20, 0x41,   0x00, 0x00, 0x00, 0x00,   // (10, 0)
        0x00, 0x00, 0x00, 0x00,   0x00, 0x00, 0xc0, 0x40,   // (0, 6)
        0x00, 0x00, 0x20, 0x41,   0x00, 0x00, 0xc0, 0x40,   // (10, 6)
    ];
    #[rustfmt::skip]
    let indices: Vec<u8> = vec![
        0x01, 0x00, 0x00, 0x00,   0x00, 0x00, 0x00, 0x00,   0x02, 0x00, 0x00, 0x00,
        0x01, 0x00, 0x00, 0x00,   0x02, 0x00, 0x00, 0x00,   0x03, 0x00, 0x00, 0x00,
    ];
    assert_eq!(mesh.vertex_bytes(), vertices);
    assert_eq!(mesh.index_bytes(), indices);
    assert_eq!(
        mesh.positions(),
        &[0.0, 0.0, 10.0, 0.0, 0.0, 6.0, 10.0, 6.0]
    );
    assert_eq!(mesh.indices(), &[1, 0, 2, 1, 2, 3]);
    assert_eq!(mesh.bounds(), SceneRect::new(0.0, 0.0, 10.0, 6.0));
    assert!(!mesh.is_empty());
}
