//! The seam, proved by substitution: the same scene through two providers is two vertex buffers.
//!
//! MJX-STAND-IN: substituting the stand-in for a second provider **is** the gate this file is,
//! so the placeholder is its subject rather than a convenience. This crate is also rank 1.7 and
//! `mjx-geometry` is 2.5, so the real provider is an edge the layering test refuses.
//!
//! # Why substitution is the gate and "it tessellates" is not
//!
//! A test that only asked *"does a shape tessellate?"* would pass for ever on a placeholder that is
//! the same rounded rectangle every time, and would still pass on the day the real table arrived and
//! was **wired to nothing** — which is not hypothetical: `mjx-geometry`'s preset tables landed in
//! MJXOFF-203 and the bridge from a document's own `a:prstGeom` was still called by nothing but its
//! own hand-off suite two children later, until MJXOFF-206 asserted it reaches a painter's report.
//!
//! So the claim tested here is the one that actually has to hold: **swapping the provider changes
//! the triangles and changes nothing else.** Everything above the seam — the display list, the
//! command stream, the paint tables, the painter that will read them — is byte-for-byte the same
//! through both providers, and only the meshes differ.
//!
//! # The second provider is DrawingML's, not a second invention
//!
//! [`DrawingMlGeometry`] below resolves its outlines through `mjx_dml`'s **own** `custGeom` model:
//! a [`CustomGeometrySpec`] with adjust values and guide formulas, evaluated by `mjx-dml`'s own
//! guide-formula engine against the shape's extents, and mapped from EMU into device
//! pixels. That work is done and this child does not repeat it; what is proved here is that the
//! seam's one method is **satisfiable by the real thing**.
//!
//! `mjx-dml` is a **dev-dependency and may never be anything else**: it is rank 2.0 and this crate
//! is rank 1.7, so a real edge points up and `xtask/tests/layering.rs` refuses it. That refusal is
//! the whole reason `mjx-scene` sits where it does, and this file is what makes the exemption earn
//! its keep rather than merely be allowed.

use mjx_dml::geometry::{
    AdjustCoordinate, CustomGeometrySpec, DrawCommand, GuideContext, GuideSpec, Path2DSpec, Point,
    ResolvedDrawCommand, ResolvedPoint,
};
use mjx_ooxml_core::measure::Emu;
use mjx_scene::{
    tessellate_scene, Color, Command, DisplayList, FillRule, Geometry, GeometryProvider, MeshRole,
    OutlineProvenance, Paint, PathCommand, PlaceholderGeometry, ResolvedOutline, SceneBuilder,
    SceneError, ScenePoint, SceneRect, StrokeStyle, TessellationOptions, Tessellator,
    PLACEHOLDER_CORNER_FRACTION, PLACEHOLDER_FRAME_FRACTION,
};
use mjx_text::{DeviceScale, ScaleBucket};

/// The handle the scenes below give their one shape.
const SHAPE: u64 = 0x0000_00ff_0000_0042;

/// The box the shape is drawn in, in device pixels.
fn shape_box() -> SceneRect {
    SceneRect::new(10.0, 20.0, 130.0, 100.0)
}

// -------------------------------------------------------------------------------------------
// The second provider: DrawingML's own resolved custom geometry
// -------------------------------------------------------------------------------------------

/// A provider that answers out of `mjx-dml`'s `custGeom` model.
///
/// One shape, with an adjust value and two guide formulas, so that the answer genuinely passes
/// through [`mjx_dml::geometry::formula`] rather than through a table of literals.
struct DrawingMlGeometry;

impl DrawingMlGeometry {
    /// A left-pointing chevron whose notch is `adj` per cent of the width in.
    fn chevron() -> CustomGeometrySpec {
        CustomGeometrySpec {
            adjust_values: vec![GuideSpec {
                name: "adj".to_owned(),
                formula: "val 25000".to_owned(),
            }],
            guides: vec![
                GuideSpec {
                    name: "notch".to_owned(),
                    // A quarter of the width, by way of the adjust value.
                    formula: "*/ w adj 100000".to_owned(),
                },
                GuideSpec {
                    name: "middle".to_owned(),
                    formula: "*/ h 1 2".to_owned(),
                },
            ],
            adjust_handles: Vec::new(),
            connection_sites: Vec::new(),
            text_rectangle: None,
            paths: vec![Path2DSpec {
                width: None,
                height: None,
                fill: None,
                stroke: None,
                extrusion_ok: None,
                commands: vec![
                    DrawCommand::MoveTo(Point::from_emu(0, 0)),
                    DrawCommand::LineTo(named("w", "middle")),
                    DrawCommand::LineTo(Point {
                        x: AdjustCoordinate::Emu(Emu::from_emu(0)),
                        y: AdjustCoordinate::Guide("h".to_owned()),
                    }),
                    DrawCommand::LineTo(named("notch", "middle")),
                    DrawCommand::Close,
                ],
            }],
        }
    }
}

/// A point whose coordinates are both guide references.
fn named(x: &str, y: &str) -> Point {
    Point {
        x: AdjustCoordinate::Guide(x.to_owned()),
        y: AdjustCoordinate::Guide(y.to_owned()),
    }
}

/// How many EMU one device pixel is, for the shape space this provider resolves in.
const EMU_PER_PIXEL: f64 = 12_700.0;

impl GeometryProvider for DrawingMlGeometry {
    fn outline(&self, outline: u64, within: SceneRect) -> Result<ResolvedOutline, SceneError> {
        if outline != SHAPE {
            return Err(SceneError::UnresolvedOutline { outline });
        }
        let extents = |length: f32| Emu::from_emu((f64::from(length) * EMU_PER_PIXEL) as i64);
        let resolved = DrawingMlGeometry::chevron()
            .resolve(GuideContext::from_extents(
                extents(within.width()),
                extents(within.height()),
            ))
            .map_err(|_| SceneError::UnresolvedOutline { outline })?;

        let place = |point: ResolvedPoint| {
            ScenePoint::new(
                within.left + (point.x.emu() as f64 / EMU_PER_PIXEL) as f32,
                within.top + (point.y.emu() as f64 / EMU_PER_PIXEL) as f32,
            )
        };
        let mut commands = Vec::new();
        for path in &resolved.paths {
            for command in &path.commands {
                commands.push(match *command {
                    ResolvedDrawCommand::MoveTo(to) => PathCommand::MoveTo(place(to)),
                    ResolvedDrawCommand::LineTo(to) => PathCommand::LineTo(place(to)),
                    ResolvedDrawCommand::QuadBezierTo(control, end) => PathCommand::QuadraticTo {
                        control: place(control),
                        end: place(end),
                    },
                    ResolvedDrawCommand::CubicBezierTo(first, second, end) => {
                        PathCommand::CubicTo {
                            first_control: place(first),
                            second_control: place(second),
                            end: place(end),
                        }
                    }
                    ResolvedDrawCommand::Close => PathCommand::Close,
                    // An arc is three or four cubics and `mjx-dml` does not decompose one; a shape
                    // that used `a:arcTo` is not this test's shape, and answering with the pen
                    // where it already is is the honest thing for a provider that cannot draw it.
                    ResolvedDrawCommand::ArcTo { .. } => continue,
                });
            }
        }
        Ok(ResolvedOutline {
            commands,
            fill_rule: FillRule::NonZero,
            label: "chevron".to_owned(),
            provenance: OutlineProvenance::Document,
        })
    }
}

// -------------------------------------------------------------------------------------------
// One scene, built once, tessellated twice
// -------------------------------------------------------------------------------------------

/// A page with one shape on it: filled, then stroked. The provider is not consulted while it is
/// built — that is the point of [`Geometry::Unresolved`].
fn a_page_with_one_shape() -> DisplayList {
    let ink = Color {
        red: 0x11,
        green: 0x22,
        blue: 0x33,
        alpha: 0xff,
    };
    let mut builder = SceneBuilder::new(DeviceScale::UNZOOMED, 200.0, 120.0);
    let geometry = builder
        .add_geometry(&Geometry::Unresolved {
            outline: SHAPE,
            bounds: shape_box(),
        })
        .expect("one unresolved geometry");
    let paint = builder.add_paint(Paint::Solid(ink)).expect("one paint");
    builder
        .push(Command::FillPath { geometry, paint })
        .expect("a fill");
    let stroke = builder
        .add_stroke_style(&StrokeStyle::solid(3.0, ink))
        .expect("a stroke interns")
        .expect("a visible stroke");
    builder
        .push(Command::StrokePath { geometry, stroke })
        .expect("a stroke");
    builder.finish().expect("the scene is well formed")
}

#[test]
fn the_same_scene_through_two_providers_is_two_different_vertex_buffers() {
    let list = a_page_with_one_shape();
    let before = list.as_bytes().to_vec();

    let mut tessellator = Tessellator::new();
    let placeholder = tessellate_scene(&list, &PlaceholderGeometry::new(), &mut tessellator)
        .expect("the placeholder resolves every handle");
    let drawing_ml = tessellate_scene(&list, &DrawingMlGeometry, &mut tessellator)
        .expect("DrawingML's own geometry resolves this one");

    // Nothing above the seam moved. This is the claim that matters: dropping the real preset table
    // in later changes the triangles and nothing else.
    assert_eq!(
        list.as_bytes(),
        before.as_slice(),
        "tessellating a display list must not touch it"
    );

    // The same commands produced the same *kinds* of mesh in the same order.
    assert_eq!(placeholder.len(), 2);
    assert_eq!(
        placeholder
            .iter()
            .map(|mesh| (mesh.command, mesh.role))
            .collect::<Vec<_>>(),
        vec![(0, MeshRole::Fill), (1, MeshRole::Stroke)]
    );
    assert_eq!(
        drawing_ml
            .iter()
            .map(|mesh| (mesh.command, mesh.role))
            .collect::<Vec<_>>(),
        placeholder
            .iter()
            .map(|mesh| (mesh.command, mesh.role))
            .collect::<Vec<_>>()
    );

    // And the triangles are different, in both roles.
    for (stand_in, real) in placeholder.iter().zip(drawing_ml.iter()) {
        assert!(
            !stand_in.mesh.is_empty() && !real.mesh.is_empty(),
            "the {:?} mesh is empty through one of the providers",
            stand_in.role
        );
        assert_ne!(
            stand_in.mesh.vertex_bytes(),
            real.mesh.vertex_bytes(),
            "the {:?} mesh is byte-identical through two providers that draw different shapes — \
             either the provider is not being consulted, or the seam is not where it says it is",
            stand_in.role
        );
    }

    // The chevron is four points and a close: far fewer triangles than a framed, crossed rounded
    // rectangle. A substitution that produced *the same shape by another route* would pass the
    // inequality above and fail this.
    assert!(
        drawing_ml[0].mesh.triangle_count() * 4 < placeholder[0].mesh.triangle_count(),
        "the chevron became {} triangles and the placeholder {}, which are too close to be two \
         different shapes",
        drawing_ml[0].mesh.triangle_count(),
        placeholder[0].mesh.triangle_count()
    );
}

#[test]
fn the_drawing_ml_provider_answers_with_the_shape_its_guide_formulas_describe() {
    // The notch is `adj` — 25 000 per hundred thousand — of the width, and the middle is half the
    // height. Nothing here is a literal in the provider: both come out of `mjx-dml`'s guide
    // formula engine.
    let within = shape_box();
    let outline = DrawingMlGeometry
        .outline(SHAPE, within)
        .expect("the chevron resolves");
    assert_eq!(outline.provenance, OutlineProvenance::Document);
    assert_eq!(outline.label, "chevron");

    let points: Vec<ScenePoint> = outline
        .commands
        .iter()
        .filter_map(|command| match *command {
            PathCommand::MoveTo(at) | PathCommand::LineTo(at) => Some(at),
            _ => None,
        })
        .collect();
    assert_eq!(
        points,
        vec![
            ScenePoint::new(within.left, within.top),
            ScenePoint::new(within.right, within.top + within.height() / 2.0),
            ScenePoint::new(within.left, within.bottom),
            ScenePoint::new(
                within.left + within.width() * 0.25,
                within.top + within.height() / 2.0
            ),
        ],
        "the guide formulas `*/ w adj 100000` and `*/ h 1 2` did not reach the outline"
    );

    // And a handle it does not know is an error rather than an empty shape nobody notices.
    let unknown = DrawingMlGeometry.outline(SHAPE + 1, within);
    assert!(matches!(
        unknown,
        Err(SceneError::UnresolvedOutline { outline }) if outline == SHAPE + 1
    ));
}

#[test]
fn a_scene_whose_provider_cannot_answer_fails_loudly_rather_than_drawing_nothing() {
    let mut builder = SceneBuilder::new(DeviceScale::UNZOOMED, 64.0, 64.0);
    let geometry = builder
        .add_geometry(&Geometry::Unresolved {
            outline: 999,
            bounds: SceneRect::new(0.0, 0.0, 32.0, 32.0),
        })
        .expect("one unresolved geometry");
    let paint = builder
        .add_paint(Paint::Solid(Color {
            red: 0,
            green: 0,
            blue: 0,
            alpha: 0xff,
        }))
        .expect("one paint");
    builder
        .push(Command::FillPath { geometry, paint })
        .expect("a fill");
    let list = builder.finish().expect("the scene is well formed");

    let mut tessellator = Tessellator::new();
    let refused = tessellate_scene(&list, &DrawingMlGeometry, &mut tessellator);
    assert!(
        matches!(refused, Err(SceneError::UnresolvedOutline { outline }) if outline == 999),
        "a provider paired with a box model that is not its own must say so; it produced {refused:?}"
    );
}

// -------------------------------------------------------------------------------------------
// The placeholder is unmistakable
// -------------------------------------------------------------------------------------------

#[test]
fn the_placeholder_carries_its_label_and_its_provenance() {
    let outline = PlaceholderGeometry::new()
        .outline(SHAPE, shape_box())
        .expect("the placeholder answers every handle");

    assert_eq!(outline.provenance, OutlineProvenance::Placeholder);
    assert_eq!(outline.label, PlaceholderGeometry::label_for(SHAPE));
    assert!(
        outline.label.contains(&SHAPE.to_string()),
        "the label `{}` does not name the handle it stands in for, so a reviewer looking at two \
         placeholders cannot tell which shape either of them is",
        outline.label
    );
    assert_eq!(
        outline.fill_rule,
        FillRule::EvenOdd,
        "the frame is a frame because the two rounded rectangles are filled even-odd"
    );
}

#[test]
fn the_placeholder_is_not_a_shape_a_document_contains() {
    let within = shape_box();
    let outline = PlaceholderGeometry::new()
        .outline(SHAPE, within)
        .expect("the placeholder answers");

    // Four closed contours: the outer rounded rectangle, the inset one, and the two diagonal bars.
    // A rounded rectangle a document drew would be one.
    let contours = outline
        .commands
        .iter()
        .filter(|command| matches!(command, PathCommand::Close))
        .count();
    assert_eq!(
        contours, 4,
        "a placeholder is a frame and a cross, which is four contours and not {contours}"
    );
    assert!(
        outline
            .commands
            .iter()
            .any(|command| matches!(command, PathCommand::QuadraticTo { .. })),
        "the corners are rounded, and a rounded corner is a curve"
    );

    // It fills its box exactly, so a reviewer sees the placeholder where the shape would have been.
    let geometry = outline.clone().into_geometry();
    assert_eq!(geometry.bounds(), within);

    // And it really is drawn hollow with a cross through it: the triangles cover well under the
    // whole box, and well over none of it.
    let mesh = Tessellator::new()
        .fill(
            &geometry,
            &PlaceholderGeometry::new(),
            TessellationOptions::for_bucket(ScaleBucket::from_steps(8)),
        )
        .expect("the placeholder tessellates");
    let box_area = within.width() * within.height();
    let covered = mesh
        .indices()
        .as_chunks::<3>()
        .0
        .iter()
        .map(|triangle| {
            let corner = |which: usize| {
                let at = triangle[which] as usize * 2;
                (mesh.positions()[at], mesh.positions()[at + 1])
            };
            let (ax, ay) = corner(0);
            let (bx, by) = corner(1);
            let (cx, cy) = corner(2);
            ((bx - ax) * (cy - ay) - (cx - ax) * (by - ay)).abs() / 2.0
        })
        .sum::<f32>();
    assert!(
        covered > box_area * 0.1 && covered < box_area * 0.6,
        "a placeholder covers {covered:.0} of its box's {box_area:.0} square pixels, which is \
         either invisible or a solid rectangle; it is meant to be a frame with a cross in it"
    );
    assert!(mesh.triangle_count() > 30);

    // The proportions are constants rather than magic numbers, and both are used.
    const { assert!(PLACEHOLDER_CORNER_FRACTION > 0.0 && PLACEHOLDER_CORNER_FRACTION < 0.5) };
    const { assert!(PLACEHOLDER_FRAME_FRACTION > 0.0 && PLACEHOLDER_FRAME_FRACTION < 0.5) };
}

#[test]
fn a_placeholder_for_a_box_with_no_area_draws_nothing() {
    // A shape laid out to nothing is a shape a document can contain, and the stand-in for it must
    // not be a division by its width.
    let outline = PlaceholderGeometry::new()
        .outline(SHAPE, SceneRect::EMPTY)
        .expect("the placeholder answers even here");
    assert!(outline.commands.is_empty());
    assert_eq!(outline.provenance, OutlineProvenance::Placeholder);
}
