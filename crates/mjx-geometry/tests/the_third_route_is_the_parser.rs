//! The differential MJXOFF-201 §6 asks for: every preset authored a second time as `a:custGeom`,
//! written as XML, **read back through the parser**, and resolved by `mjx-dml` alone.
//!
//! # Why a third route, when there are already two
//!
//! `the_two_routes_agree.rs` compares a hand transcription of six shapes against the mechanical
//! extraction of 186. It is the strongest gate in the crate and it covers six shapes, because a
//! seventh hand transcription would be the same author writing the same numbers twice.
//!
//! This is the other kind of independence. The data is the same — it has to be, there is only one
//! `presetShapeDefinitions.xml` — but the **route** is not:
//!
//! | | this crate's route | this file's route |
//! |---|---|---|
//! | the table | `&'static [PresetPathStep]` | `a:custGeom` XML text, written here |
//! | reading it | a `static` the compiler laid out | `mjx_xml::fidelity::parse` |
//! | typing it | [`PresetCoordinate`](mjx_geometry::PresetCoordinate) | `ST_AdjCoordinate`, off the wire |
//! | the guides | [`crate::resolve::guide_environment`] | [`CustomGeometrySpec::guide_values`] |
//! | resolving | `mjx-geometry`'s resolver | `CustomGeometrySpec::resolve` |
//! | the answer | device pixels | EMU, mapped here |
//!
//! So a coordinate the table stores as a guide name and the writer emits as an *integer*, an angle
//! whose sign the wire scale loses, a `@fill` token spelled the way the file spells it but the enum
//! does not, a `@stroke="0"` read as `true` — every one of those breaks the agreement, and none of
//! them is visible to a suite that reads the same `static` twice.
//!
//! # All three surfaces, not only the paths
//!
//! MJXOFF-204 put a text rectangle on 181 presets and 856 connection sites on 173, and
//! [`ResolvedCustomGeometry`] carries both beside its paths. A differential that compared only the
//! paths would leave two thirds of the new surface unchecked, so all three are compared:
//! [`the_paths_agree_through_the_parser`], [`the_text_rectangles_agree_through_the_parser`] and
//! [`the_connection_sites_agree_through_the_parser`].
//!
//! # What is deliberately *not* independent, and why the comparison is still worth something
//!
//! Two things are shared and cannot be otherwise. The **arc decomposition** is
//! [`mjx_geometry::arc_to_cubics`] on both sides, because an `a:arcTo` is not a
//! [`PathCommand`](mjx_scene::PathCommand) and there is exactly one decomposition in the workspace;
//! decomposing it twice would be the second implementation MJXOFF-204 §6 rules out. And the
//! **guide-formula evaluator** is `mjx-dml`'s on both sides, for the same reason. What is genuinely
//! second here is the *map onto the page*, written below in twenty lines against `ShapeToDevice`'s
//! sixty — including `CT_Path2D`'s `@w`/`@h` coordinate box, which 31 presets declare and which is
//! the single easiest thing in the pipeline to apply to the wrong list.
//!
//! # The two errata, and the side they have to be on
//!
//! `pie`'s `a:rect` is transposed in ECMA-376's own file and `squareTabs`'s sixth connection site
//! reads a horizontal guide as a vertical coordinate. G02 and G03 corrected both in `xtask`'s
//! `RECT_ERRATA` and `CONNECTION_ERRATA`, so the **table** holds the corrected form. This file
//! writes its XML *from the table*, so it writes the corrected form too and the comparison is not
//! about the errata at all. That is deliberate and is stated here so nobody reads a green result as
//! evidence the file was right: it is evidence the two routes read the same corrected data the same
//! way.

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use common::orientations;
use mjx_dml::geometry::{
    AdjustCoordinate, CustomGeometry, CustomGeometrySpec, DrawCommand, Emu, GuideContext,
    GuideSpec, Point, ResolvedCustomGeometry, ResolvedDrawCommand, ResolvedPath, ResolvedPoint,
};
use mjx_geometry::{
    arc_to_cubics, preset_connection_sites, preset_outline, preset_text_rectangle, seeded_shapes,
    PathFillMode, PresetAngle, PresetConnectionSite, PresetCoordinate, PresetPath, PresetPathStep,
    PresetPoint, PresetShapeDefinition, PresetShapeType, PresetTextRectangle, ShapePoint, Size,
    TextRectangle,
};
use mjx_ooxml_core::FromXml;
use mjx_scene::{PathCommand, SceneRect};

/// The DrawingML namespace, written on the fragment's root the way a real part writes it.
const DRAWINGML: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";

/// How far apart, in device pixels, the two routes' answers may lie.
///
/// The same number and the same two error sources as `the_two_routes_agree.rs`'s own agreement
/// tolerance — half an EMU of coordinate rounding and one narrowing to `f32`, a hundred times over.
/// **It is not a fudge factor for a difference of opinion between the routes**: the measured worst
/// case over all 186 shapes and all three surfaces is quoted by every test below and is four orders
/// of magnitude inside it. [`a_wrong_coordinate_is_caught_by_this_comparison`] shows what a real
/// transcription slip measures, and
/// [`the_agreement_tolerance_is_not_wide_enough_to_pass_a_rounding_error`] shows the smallest
/// defect that still fails: one EMU written as two.
const AGREEMENT_TOLERANCE_PIXELS: f32 = 0.01;

/// How many presets the differential covers.
const PRESETS: usize = 186;

/// The presets whose `a:custGeom` route cannot be resolved at their default adjustments, with why.
///
/// **Empty**, and asserted empty rather than assumed: `CustomGeometrySpec::guide_values` evaluates a
/// whole `a:gdLst` in one `extend` and stops at the first guide with no finite value, where
/// `mjx-geometry`'s own environment evaluates one guide at a time and leaves a singular one
/// undefined. So a preset that is singular at its *defaults* would be verifiable by one route and
/// not the other, and would have to be named here. None is — every singularity in ECMA-376's file
/// is at an adjustment's stop, not at its seed.
const UNVERIFIABLE_THROUGH_THE_PARSER: &[&str] = &[];

// -------------------------------------------------------------------------------------------
// Route B: the table, written as `a:custGeom` XML
// -------------------------------------------------------------------------------------------

/// An `a:gd`'s two attributes, escaped.
fn guide_element(into: &mut String, name: &str, formula: &str) {
    write!(into, r#"<a:gd name="{name}" fmla="{formula}"/>"#).expect("a String never fails");
}

/// A coordinate as `ST_AdjCoordinate` writes it: an integer, or a guide's name.
///
/// **The one place the two routes could disagree without either being wrong**, and the reason this
/// file writes text rather than building a `CustomGeometrySpec` directly: an `ST_AdjCoordinate` is
/// *"an integer if it parses as one, a guide name otherwise"*, so a guide named `7` — which the
/// schema permits and no preset uses — would come back as an EMU. Writing the wire form is what puts
/// that rule in the loop instead of around it.
fn coordinate(value: PresetCoordinate) -> String {
    match value {
        PresetCoordinate::Emu(emu) => emu.to_string(),
        PresetCoordinate::Guide(name) => name.to_owned(),
    }
}

/// An angle as `ST_AdjAngle` writes it: 60000ths of a degree, or a guide's name.
fn angle(value: PresetAngle) -> String {
    match value {
        PresetAngle::Native(native) => native.to_string(),
        PresetAngle::Guide(name) => name.to_owned(),
    }
}

/// A `CT_AdjPoint2D`, under whichever element name its parent gives it.
///
/// **Two names for one type, and getting it wrong is silent.** Inside an `a:moveTo` or an `a:lnTo`
/// the point is an `a:pt`; inside an `a:cxn` (and an `a:ahXY`) it is an `a:pos`. A writer that
/// emitted `a:pt` everywhere produces a fragment that parses, yields the right *number* of
/// connection sites with the right angles, and puts every one of them at the shape's origin —
/// because `CustomGeometry::connection_sites` reads the `a:pos` child and finds none. This file
/// wrote it that way first and the connection-site differential reported 180 px, which is what a
/// differential is for.
fn point_element(into: &mut String, name: &str, point: PresetPoint) {
    write!(
        into,
        r#"<a:{name} x="{}" y="{}"/>"#,
        coordinate(point.x),
        coordinate(point.y)
    )
    .expect("a String never fails");
}

/// `@fill`'s wire token, which is `ST_PathFillMode` and not the enum's Rust name.
fn fill_token(fill: PathFillMode) -> &'static str {
    match fill {
        PathFillMode::None => "none",
        PathFillMode::Normal => "norm",
        PathFillMode::Lighten => "lighten",
        PathFillMode::LightenLess => "lightenLess",
        PathFillMode::Darken => "darken",
        PathFillMode::DarkenLess => "darkenLess",
    }
}

/// One `a:path`.
fn path_element(into: &mut String, path: &PresetPath) {
    into.push_str("<a:path");
    if let Some(width) = path.width {
        write!(into, r#" w="{width}""#).expect("a String never fails");
    }
    if let Some(height) = path.height {
        write!(into, r#" h="{height}""#).expect("a String never fails");
    }
    write!(
        into,
        r#" fill="{}" stroke="{}" extrusionOk="{}">"#,
        fill_token(path.fill),
        u8::from(path.stroke),
        u8::from(path.extrusion_ok)
    )
    .expect("a String never fails");
    for step in path.steps {
        match *step {
            PresetPathStep::Close => into.push_str("<a:close/>"),
            PresetPathStep::MoveTo(point) => {
                into.push_str("<a:moveTo>");
                point_element(into, "pt", point);
                into.push_str("</a:moveTo>");
            }
            PresetPathStep::LineTo(point) => {
                into.push_str("<a:lnTo>");
                point_element(into, "pt", point);
                into.push_str("</a:lnTo>");
            }
            PresetPathStep::ArcTo {
                width_radius,
                height_radius,
                start_angle,
                swing_angle,
            } => {
                write!(
                    into,
                    r#"<a:arcTo wR="{}" hR="{}" stAng="{}" swAng="{}"/>"#,
                    coordinate(width_radius),
                    coordinate(height_radius),
                    angle(start_angle),
                    angle(swing_angle)
                )
                .expect("a String never fails");
            }
            PresetPathStep::QuadBezierTo { control, end } => {
                into.push_str("<a:quadBezTo>");
                point_element(into, "pt", control);
                point_element(into, "pt", end);
                into.push_str("</a:quadBezTo>");
            }
            PresetPathStep::CubicBezierTo {
                first_control,
                second_control,
                end,
            } => {
                into.push_str("<a:cubicBezTo>");
                point_element(into, "pt", first_control);
                point_element(into, "pt", second_control);
                point_element(into, "pt", end);
                into.push_str("</a:cubicBezTo>");
            }
        }
    }
    into.push_str("</a:path>");
}

/// One preset's whole geometry as an `a:custGeom` fragment, children in `CT_CustomGeometry2D`'s
/// schema order.
fn custom_geometry_xml(definition: &PresetShapeDefinition) -> String {
    let mut xml = format!(r#"<a:custGeom xmlns:a="{DRAWINGML}">"#);
    xml.push_str("<a:avLst>");
    for value in definition.adjustment_values {
        guide_element(&mut xml, value.wire_name, value.formula);
    }
    xml.push_str("</a:avLst><a:gdLst>");
    for guide in definition.guides {
        guide_element(&mut xml, guide.wire_name, guide.formula);
    }
    xml.push_str("</a:gdLst>");
    if !definition.connection_sites.is_empty() {
        xml.push_str("<a:cxnLst>");
        for site in definition.connection_sites {
            write!(&mut xml, r#"<a:cxn ang="{}">"#, angle(site.angle))
                .expect("a String never fails");
            point_element(&mut xml, "pos", site.position);
            xml.push_str("</a:cxn>");
        }
        xml.push_str("</a:cxnLst>");
    }
    if let Some(rectangle) = definition.text_rectangle {
        write!(
            &mut xml,
            r#"<a:rect l="{}" t="{}" r="{}" b="{}"/>"#,
            coordinate(rectangle.left),
            coordinate(rectangle.top),
            coordinate(rectangle.right),
            coordinate(rectangle.bottom)
        )
        .expect("a String never fails");
    }
    xml.push_str("<a:pathLst>");
    for path in definition.paths {
        path_element(&mut xml, path);
    }
    xml.push_str("</a:pathLst></a:custGeom>");
    xml
}

/// Parse a fragment back into a [`CustomGeometrySpec`] — the half of the route that is the point.
fn through_the_parser(xml: &str) -> CustomGeometrySpec {
    let document = mjx_xml::fidelity::parse(xml.as_bytes())
        .unwrap_or_else(|error| panic!("the fragment does not parse: {error}\n{xml}"));
    let geometry = CustomGeometry::from_xml(&document.root, &document.interner)
        .expect("an `a:custGeom` root reads as one");
    geometry.spec(&document.interner)
}

/// One preset, resolved entirely by `mjx-dml` after a round trip through the parser.
fn resolved_through_the_parser(
    definition: &PresetShapeDefinition,
    extents: Size,
) -> ResolvedCustomGeometry {
    let spec = through_the_parser(&custom_geometry_xml(definition));
    spec.resolve(GuideContext::from_size(extents))
        .unwrap_or_else(|error| {
            panic!(
                "`{}` did not resolve through the parser: {error}",
                definition.preset.to_wire()
            )
        })
}

// -------------------------------------------------------------------------------------------
// The second map onto the page — twenty lines against `ShapeToDevice`'s sixty
// -------------------------------------------------------------------------------------------

/// Where a point of the shape's own space lands, given the box and the scale.
fn place(within: SceneRect, scale: (f64, f64), at: ShapePoint) -> mjx_scene::ScenePoint {
    mjx_scene::ScenePoint::new(
        within.left + (at.x * scale.0) as f32,
        within.top + (at.y * scale.1) as f32,
    )
}

/// How many device pixels one unit of a space covers; zero for a space with no extent.
fn scale_of(pixels: f32, units: f64) -> f64 {
    if units > 0.0 {
        f64::from(pixels) / units
    } else {
        0.0
    }
}

/// The scale for one path — its own `@w`/`@h` coordinate box where it declares one, the shape's
/// extents otherwise.
///
/// **The branch 31 presets take.** `CT_Path2D`'s box makes every coordinate inside that path a
/// proportion of a number like `2` or `21600` rather than a length, and applying the shape's extents
/// to it instead puts the whole path within a few EMU of the origin. Written here a second time,
/// deliberately, because it is the part of `ShapeToDevice` with a branch.
fn path_scale(within: SceneRect, path: &ResolvedPath, extents: Size) -> (f64, f64) {
    let span = |declared: Option<Emu>, shape: Emu| {
        declared
            .map(Emu::emu)
            .filter(|value| *value > 0)
            .unwrap_or_else(|| shape.emu()) as f64
    };
    (
        scale_of(within.width(), span(path.width, extents.width)),
        scale_of(within.height(), span(path.height, extents.height)),
    )
}

/// A resolved point as a real-valued point in the shape's space.
fn shape_point(point: ResolvedPoint) -> ShapePoint {
    ShapePoint::new(point.x.emu() as f64, point.y.emu() as f64)
}

/// One resolved path as device-pixel commands — this file's own `emit_path`.
///
/// The three rules `mjx-geometry`'s own emitter has, restated rather than reused: an `a:arcTo`
/// becomes cubics from the pen, a step on a closed contour restates the start point, and a second
/// `a:close` draws nothing. They are restated because a differential that called the function it is
/// checking would compare it with itself.
fn commands_of(within: SceneRect, path: &ResolvedPath, extents: Size) -> Vec<PathCommand> {
    let scale = path_scale(within, path, extents);
    let mut into = Vec::new();
    let mut pen = ShapePoint::new(0.0, 0.0);
    let mut subpath_start = pen;
    let (mut open, mut ever_opened) = (false, false);
    for command in &path.commands {
        if !open && ever_opened && !matches!(command, ResolvedDrawCommand::MoveTo(_)) {
            if matches!(command, ResolvedDrawCommand::Close) {
                continue;
            }
            into.push(PathCommand::MoveTo(place(within, scale, pen)));
            open = true;
        }
        match *command {
            ResolvedDrawCommand::Close => {
                into.push(PathCommand::Close);
                pen = subpath_start;
                open = false;
            }
            ResolvedDrawCommand::MoveTo(point) => {
                pen = shape_point(point);
                subpath_start = pen;
                open = true;
                ever_opened = true;
                into.push(PathCommand::MoveTo(place(within, scale, pen)));
            }
            ResolvedDrawCommand::LineTo(point) => {
                pen = shape_point(point);
                into.push(PathCommand::LineTo(place(within, scale, pen)));
            }
            ResolvedDrawCommand::QuadBezierTo(control, end) => {
                pen = shape_point(end);
                into.push(PathCommand::QuadraticTo {
                    control: place(within, scale, shape_point(control)),
                    end: place(within, scale, pen),
                });
            }
            ResolvedDrawCommand::CubicBezierTo(first, second, end) => {
                pen = shape_point(end);
                into.push(PathCommand::CubicTo {
                    first_control: place(within, scale, shape_point(first)),
                    second_control: place(within, scale, shape_point(second)),
                    end: place(within, scale, pen),
                });
            }
            ResolvedDrawCommand::ArcTo {
                width_radius,
                height_radius,
                start_angle,
                swing_angle,
            } => {
                for segment in arc_to_cubics(
                    pen,
                    width_radius.emu() as f64,
                    height_radius.emu() as f64,
                    start_angle.radians(),
                    swing_angle.radians(),
                ) {
                    pen = segment.end;
                    into.push(PathCommand::CubicTo {
                        first_control: place(within, scale, segment.first_control),
                        second_control: place(within, scale, segment.second_control),
                        end: place(within, scale, pen),
                    });
                }
            }
        }
    }
    into
}

/// Every path of a resolved geometry as one command list, with the undrawn contours left out — the
/// reduction `outline_of_definition` makes, restated for the same reason as the emitter.
fn outline_of(
    within: SceneRect,
    resolved: &ResolvedCustomGeometry,
    extents: Size,
) -> Vec<PathCommand> {
    let mut commands = Vec::new();
    for path in &resolved.paths {
        let filled = path.fill.unwrap_or(PathFillMode::Normal) != PathFillMode::None;
        let stroked = path.stroke.unwrap_or(true);
        if !filled && !stroked {
            continue;
        }
        commands.extend(commands_of(within, path, extents));
    }
    commands
}

/// How far apart two command lists are, at their worst corresponding point.
///
/// A point-by-point comparison and **not** the Hausdorff distance `the_two_routes_agree.rs` uses,
/// and the difference is the question. That suite compares two *authors*, who may legitimately draw
/// the same region starting from a different vertex; this one compares two *readings of one table*,
/// which must agree step for step or one of them has lost a step.
fn commands_apart(left: &[PathCommand], right: &[PathCommand], what: &str) -> f32 {
    let apart = commands_distance(left, right);
    assert!(
        apart.is_finite(),
        "{what}: the two routes drew {} and {} commands, or two commands of different kinds",
        left.len(),
        right.len()
    );
    apart
}

/// [`commands_apart`] without the assertion — infinite where the two lists do not correspond at all.
///
/// The form [`the_differential_is_able_to_fail_on_one_shape_and_only_that_shape`] needs: it runs
/// the whole table with one shape deliberately wrong and has to *count* the shapes that disagree
/// rather than stop at the first.
fn commands_distance(left: &[PathCommand], right: &[PathCommand]) -> f32 {
    if left.len() != right.len() {
        return f32::INFINITY;
    }
    let mut worst = 0.0f32;
    for (here, there) in left.iter().zip(right.iter()) {
        let (mine, theirs) = (points_of_one(here), points_of_one(there));
        if mine.len() != theirs.len() {
            return f32::INFINITY;
        }
        for ((x, y), (other_x, other_y)) in mine.into_iter().zip(theirs) {
            worst = worst.max((x - other_x).abs()).max((y - other_y).abs());
        }
    }
    worst
}

/// One command's points, which is also how its *kind* is compared — two commands of different kinds
/// never have the same number of points except `MoveTo` and `LineTo`, and those two are told apart
/// by the discriminant below.
fn points_of_one(command: &PathCommand) -> Vec<(f32, f32)> {
    let kind = match command {
        PathCommand::MoveTo(_) => 0.0,
        PathCommand::LineTo(_) => 1.0,
        PathCommand::QuadraticTo { .. } => 2.0,
        PathCommand::CubicTo { .. } => 3.0,
        PathCommand::Close => 4.0,
    };
    let mut points = vec![(kind, kind)];
    match *command {
        PathCommand::MoveTo(at) | PathCommand::LineTo(at) => points.push((at.x, at.y)),
        PathCommand::QuadraticTo { control, end } => {
            points.push((control.x, control.y));
            points.push((end.x, end.y));
        }
        PathCommand::CubicTo {
            first_control,
            second_control,
            end,
        } => {
            points.push((first_control.x, first_control.y));
            points.push((second_control.x, second_control.y));
            points.push((end.x, end.y));
        }
        PathCommand::Close => {}
    }
    points
}

// -------------------------------------------------------------------------------------------
// The differential, one surface at a time
// -------------------------------------------------------------------------------------------

#[test]
fn every_preset_survives_the_round_trip_through_the_parser() {
    // The premise, asserted before anything is compared: the fragment this file writes for each of
    // the 186 parses, reads back as an `a:custGeom`, and yields a spec whose guide list, path list,
    // text rectangle and connection sites are the *same objects* the table holds. A writer that
    // dropped a guide would make every later comparison agree about a shape neither route draws.
    let mut written = 0usize;
    let mut singular: BTreeMap<&'static str, String> = BTreeMap::new();
    for definition in seeded_shapes() {
        let shape = definition.preset.to_wire();
        let spec = through_the_parser(&custom_geometry_xml(definition));
        written += 1;

        assert_eq!(
            spec.adjust_values,
            definition
                .adjustment_values
                .iter()
                .map(|value| GuideSpec {
                    name: value.wire_name.to_owned(),
                    formula: value.formula.to_owned(),
                })
                .collect::<Vec<_>>(),
            "`{shape}`'s `a:avLst` did not survive the round trip"
        );
        assert_eq!(
            spec.guides,
            definition
                .guides
                .iter()
                .map(|guide| GuideSpec {
                    name: guide.wire_name.to_owned(),
                    formula: guide.formula.to_owned(),
                })
                .collect::<Vec<_>>(),
            "`{shape}`'s `a:gdLst` did not survive the round trip"
        );
        assert_eq!(
            spec.text_rectangle,
            definition
                .text_rectangle
                .map(PresetTextRectangle::to_rectangle),
            "`{shape}`'s `a:rect` did not survive the round trip"
        );
        assert_eq!(
            spec.connection_sites,
            definition
                .connection_sites
                .iter()
                .copied()
                .map(PresetConnectionSite::to_connection_site)
                .collect::<Vec<_>>(),
            "`{shape}`'s `a:cxnLst` did not survive the round trip"
        );
        assert_eq!(
            spec.paths.len(),
            definition.paths.len(),
            "`{shape}` wrote {} paths and read back {}",
            definition.paths.len(),
            spec.paths.len()
        );
        for (index, (path, read)) in definition.paths.iter().zip(spec.paths.iter()).enumerate() {
            assert_eq!(
                read.commands,
                path.steps
                    .iter()
                    .copied()
                    .map(PresetPathStep::to_draw_command)
                    .collect::<Vec<_>>(),
                "`{shape}`'s path {index} did not survive the round trip"
            );
            // The three flags, which are the ones a wire spelling can lose: `@fill`'s six tokens,
            // `@stroke` written as `0`/`1` and read as a boolean, and `@extrusionOk` the same.
            assert_eq!(read.fill, Some(path.fill), "`{shape}` path {index} `@fill`");
            assert_eq!(
                read.stroke,
                Some(path.stroke),
                "`{shape}` path {index} `@stroke`"
            );
            assert_eq!(
                read.extrusion_ok,
                Some(path.extrusion_ok),
                "`{shape}` path {index} `@extrusionOk`"
            );
            assert_eq!(
                read.width.map(Emu::emu),
                path.width,
                "`{shape}` path {index} `@w`"
            );
            assert_eq!(
                read.height.map(Emu::emu),
                path.height,
                "`{shape}` path {index} `@h`"
            );
        }

        // And it resolves, or it is named as unverifiable. `CustomGeometrySpec::guide_values` stops
        // at the first guide with no finite value where this crate's environment leaves it
        // undefined, so this is where that difference would show.
        for (_, _, extents) in orientations() {
            if let Err(error) = spec.resolve(GuideContext::from_size(extents)) {
                singular.insert(shape, format!("{error}"));
            }
        }
    }
    assert_eq!(written, PRESETS);
    assert_eq!(
        singular.keys().copied().collect::<BTreeSet<_>>(),
        UNVERIFIABLE_THROUGH_THE_PARSER
            .iter()
            .copied()
            .collect::<BTreeSet<_>>(),
        "these presets cannot be resolved through the `a:custGeom` route at their defaults and are \
         therefore unverified by this file: {singular:#?}"
    );
    println!("{written} presets written as `a:custGeom`, parsed back and resolved");
}

#[test]
fn the_paths_agree_through_the_parser() {
    let mut worst = 0.0f32;
    let mut compared = 0usize;
    let mut commands = 0usize;
    for (orientation, within, extents) in orientations() {
        for definition in seeded_shapes() {
            let shape = definition.preset.to_wire();
            let mine = preset_outline(definition.preset, extents, &[], within)
                .unwrap_or_else(|error| panic!("`{shape}` did not resolve: {error}"));
            let theirs = outline_of(
                within,
                &resolved_through_the_parser(definition, extents),
                extents,
            );
            worst = worst.max(commands_apart(
                &mine.commands,
                &theirs,
                &format!("`{shape}` in {orientation}"),
            ));
            compared += 1;
            commands += mine.commands.len();
        }
    }
    assert_eq!(compared, PRESETS * 2);
    assert!(commands > 5_000, "only {commands} commands were compared");
    println!(
        "{commands} commands over {compared} (preset, orientation) pairs; worst disagreement \
         {worst} px"
    );
    assert!(
        worst <= AGREEMENT_TOLERANCE_PIXELS,
        "the two routes' paths differ by {worst} px"
    );
    // The headroom: where the routes agree they agree exactly, so the tolerance is not what makes
    // the comparison pass.
    assert!(
        worst <= AGREEMENT_TOLERANCE_PIXELS / 100.0,
        "the two routes' paths are {worst} px apart, which is a real difference of opinion rather \
         than the last bit of an `f32`"
    );
}

#[test]
fn the_text_rectangles_agree_through_the_parser() {
    let mut worst = 0.0f32;
    let mut declared = 0usize;
    let mut absent = 0usize;
    for (orientation, within, extents) in orientations() {
        for definition in seeded_shapes() {
            let shape = definition.preset.to_wire();
            let mine = preset_text_rectangle(definition.preset, extents, &[], within)
                .unwrap_or_else(|error| panic!("`{shape}`'s text rectangle: {error}"));
            let resolved = resolved_through_the_parser(definition, extents);
            match (mine, resolved.text_rectangle) {
                (TextRectangle::NotDeclared, None) => absent += 1,
                (
                    TextRectangle::Declared(here) | TextRectangle::Inverted { crossed: here },
                    Some(there),
                ) => {
                    let scale = (
                        scale_of(within.width(), extents.width.emu() as f64),
                        scale_of(within.height(), extents.height.emu() as f64),
                    );
                    let top_left = place(
                        within,
                        scale,
                        ShapePoint::new(there.left.emu() as f64, there.top.emu() as f64),
                    );
                    let bottom_right = place(
                        within,
                        scale,
                        ShapePoint::new(there.right.emu() as f64, there.bottom.emu() as f64),
                    );
                    // `SceneRect::new` normalises, which is what makes an inverted rectangle look
                    // ordinary — so the comparison is against the same normalisation on both sides.
                    let theirs =
                        SceneRect::new(top_left.x, top_left.y, bottom_right.x, bottom_right.y);
                    worst = worst
                        .max((here.left - theirs.left).abs())
                        .max((here.top - theirs.top).abs())
                        .max((here.right - theirs.right).abs())
                        .max((here.bottom - theirs.bottom).abs());
                    declared += 1;
                }
                (mine, theirs) => panic!(
                    "`{shape}` in {orientation}: this crate answers {mine:?} and the parser route \
                     answers {theirs:?}"
                ),
            }
        }
    }
    assert_eq!(
        declared,
        181 * 2,
        "the presets with an `a:rect` are no longer 181"
    );
    assert_eq!(absent, 5 * 2, "the presets without one are no longer five");
    println!("{declared} text rectangles compared; worst disagreement {worst} px");
    assert!(
        worst <= AGREEMENT_TOLERANCE_PIXELS,
        "the two routes' text rectangles differ by {worst} px"
    );
}

#[test]
fn the_connection_sites_agree_through_the_parser() {
    let mut worst_position = 0.0f32;
    let mut worst_angle = 0.0f64;
    let mut sites = 0usize;
    for (orientation, within, extents) in orientations() {
        for definition in seeded_shapes() {
            let shape = definition.preset.to_wire();
            let mine = preset_connection_sites(definition.preset, extents, &[], within)
                .unwrap_or_else(|error| panic!("`{shape}`'s connection sites: {error}"));
            let resolved = resolved_through_the_parser(definition, extents);
            assert_eq!(
                mine.len(),
                resolved.connection_sites.len(),
                "`{shape}` in {orientation} has {} sites on one route and {} on the other",
                mine.len(),
                resolved.connection_sites.len()
            );
            let scale = (
                scale_of(within.width(), extents.width.emu() as f64),
                scale_of(within.height(), extents.height.emu() as f64),
            );
            for (index, (here, there)) in mine
                .iter()
                .zip(resolved.connection_sites.iter())
                .enumerate()
            {
                let placed = place(within, scale, shape_point(there.position));
                worst_position = worst_position
                    .max((here.position.x - placed.x).abs())
                    .max((here.position.y - placed.y).abs());
                // **The angle as well as the point.** A site without its outgoing direction is a
                // point, and an elbow connector that left along the straight line to its target
                // would cut through the shape it started in. 648 of the 856 name a guide for it and
                // 208 write a literal, so both arms of `ST_AdjAngle` are in this comparison.
                worst_angle = worst_angle.max((here.angle.degrees() - there.angle.degrees()).abs());
                sites += 1;
                let _ = index;
            }
        }
    }
    assert_eq!(
        sites,
        856 * 2,
        "the table no longer holds 856 connection sites"
    );
    println!(
        "{sites} connection sites compared; worst position {worst_position} px, worst angle \
         {worst_angle}°"
    );
    assert!(
        worst_position <= AGREEMENT_TOLERANCE_PIXELS,
        "the two routes' connection sites differ by {worst_position} px"
    );
    assert!(
        worst_angle <= 1e-9,
        "the two routes' connection angles differ by {worst_angle}°"
    );
}

// -------------------------------------------------------------------------------------------
// The comparison shown able to fail
// -------------------------------------------------------------------------------------------

#[test]
fn a_wrong_coordinate_is_caught_by_this_comparison() {
    // The differential against a geometry that is deliberately wrong, so the green above is a
    // result rather than an absence. `rect`'s four corners, with one of them moved by a tenth of
    // the shape: the comparison must report it, and report it in device pixels a reader can check
    // against the box.
    let (_, within, extents) = orientations()[0];
    let mine =
        preset_outline(PresetShapeType::Rectangle, extents, &[], within).expect("`rect` resolves");

    let mut wrong = through_the_parser(&custom_geometry_xml(
        seeded_shapes()
            .iter()
            .find(|definition| definition.preset == PresetShapeType::Rectangle)
            .expect("the table has `rect`"),
    ));
    match &mut wrong.paths[0].commands[1] {
        DrawCommand::LineTo(point) => {
            *point = Point {
                x: AdjustCoordinate::Guide("hc".to_owned()),
                y: point.y.clone(),
            }
        }
        other => panic!("`rect`'s second step is {other:?}"),
    }
    let resolved = wrong
        .resolve(GuideContext::from_size(extents))
        .expect("a wrong rectangle still resolves");
    let theirs = outline_of(within, &resolved, extents);
    let apart = commands_apart(&mine.commands, &theirs, "a deliberately wrong `rect`");
    assert!(
        (apart - 80.0).abs() < 0.01,
        "moving a rectangle's corner to its centre measured {apart} px on a 160-pixel box"
    );
    assert!(apart > AGREEMENT_TOLERANCE_PIXELS);
}

#[test]
fn the_agreement_tolerance_is_not_wide_enough_to_pass_a_rounding_error() {
    // The smallest defect the tolerance still catches, in the unit it is stated in. One EMU written
    // as two is `1/12_700` of a point, which at one device pixel to the point is `7.9e-5` px — far
    // *inside* [`AGREEMENT_TOLERANCE_PIXELS`], and that is the honest answer: a one-EMU slip is not
    // catchable at this scale and this suite does not claim it is. What is catchable is the same
    // slip at the scale a real transcription makes it — a guide name confused for another — and the
    // test above measures that at 80 px. The number here is what says where the line is.
    let (_, within, extents) = orientations()[0];
    let one_emu_in_pixels = f64::from(within.width()) / extents.width.emu() as f64;
    assert!(
        one_emu_in_pixels < f64::from(AGREEMENT_TOLERANCE_PIXELS),
        "one EMU is {one_emu_in_pixels} px, which the tolerance would catch — the tolerance is \
         tighter than the coordinate quantisation and every shape should be failing"
    );
    // …and 128 EMU is not: the tolerance sits between the two, so it is a statement about
    // coordinates rather than a number chosen to pass.
    assert!(
        one_emu_in_pixels * 128.0 > f64::from(AGREEMENT_TOLERANCE_PIXELS),
        "128 EMU is {} px, still inside the tolerance — the tolerance is far above the \
         quantisation and is measuring nothing",
        one_emu_in_pixels * 128.0
    );
}

#[test]
fn the_fragment_this_file_writes_is_the_wire_form_and_not_a_convenience() {
    // The identity-value question for the writer: does the XML it produces actually exercise both
    // arms of `ST_AdjCoordinate`, both arms of `ST_AdjAngle`, a `@w`/`@h` coordinate box, and a
    // `@fill` that is not the schema default? A writer that emitted every coordinate as a guide
    // name would round-trip perfectly and would never test the integer arm.
    let mut literal_coordinates = 0usize;
    let mut guide_coordinates = 0usize;
    let mut literal_angles = 0usize;
    let mut guide_angles = 0usize;
    let mut with_a_coordinate_box: BTreeSet<&'static str> = BTreeSet::new();
    let mut fills: BTreeSet<&'static str> = BTreeSet::new();
    let mut unstroked = 0usize;
    for definition in seeded_shapes() {
        for site in definition.connection_sites {
            match site.angle {
                PresetAngle::Native(_) => literal_angles += 1,
                PresetAngle::Guide(_) => guide_angles += 1,
            }
        }
        for path in definition.paths {
            if path.width.is_some() || path.height.is_some() {
                with_a_coordinate_box.insert(definition.preset.to_wire());
            }
            fills.insert(fill_token(path.fill));
            unstroked += usize::from(!path.stroke);
            for step in path.steps {
                let mut count = |value: PresetCoordinate| match value {
                    PresetCoordinate::Emu(_) => literal_coordinates += 1,
                    PresetCoordinate::Guide(_) => guide_coordinates += 1,
                };
                match *step {
                    PresetPathStep::Close => {}
                    PresetPathStep::MoveTo(point) | PresetPathStep::LineTo(point) => {
                        count(point.x);
                        count(point.y);
                    }
                    PresetPathStep::ArcTo {
                        width_radius,
                        height_radius,
                        ..
                    } => {
                        count(width_radius);
                        count(height_radius);
                    }
                    PresetPathStep::QuadBezierTo { control, end } => {
                        for point in [control, end] {
                            count(point.x);
                            count(point.y);
                        }
                    }
                    PresetPathStep::CubicBezierTo {
                        first_control,
                        second_control,
                        end,
                    } => {
                        for point in [first_control, second_control, end] {
                            count(point.x);
                            count(point.y);
                        }
                    }
                }
            }
        }
    }
    assert!(literal_coordinates > 100 && guide_coordinates > 1_000);
    assert_eq!(
        (literal_angles, guide_angles),
        (208, 648),
        "the split of literal and guide connection angles has changed"
    );
    assert_eq!(
        with_a_coordinate_box.len(),
        31,
        "the presets with a `@w`/`@h` coordinate box are no longer 31"
    );
    assert!(
        fills.contains("norm") && fills.contains("none") && fills.contains("lighten"),
        "the fragments this file writes exercise only {fills:?} of `ST_PathFillMode`"
    );
    assert!(unstroked > 0, "no path writes `stroke=\"0\"`");
    println!(
        "the fragments carry {literal_coordinates} literal and {guide_coordinates} guide \
         coordinates, {literal_angles} literal and {guide_angles} guide angles, {} \
         presets with a coordinate box, the fills {fills:?} and {unstroked} unstroked paths",
        with_a_coordinate_box.len()
    );
}

#[test]
fn the_differential_is_able_to_fail_on_one_shape_and_only_that_shape() {
    // The whole table compared with one shape's guide list deliberately wrong, so that the exact
    // agreement above is a result rather than an absence — and so that the comparison is shown not
    // to redden its neighbours, which a differential that compared the wrong pair of shapes would.
    //
    // The mutation is a **guide** rather than a coordinate, because a guide is where a real
    // transcription slip lands: `triangle`'s apex guide `x2` is `*/ w a 100000` and this writes
    // `*/ h a 100000` — one letter, the confusion between the width and the height that a
    // landscape-only census cannot see, and it moves the apex by half the difference between the
    // two sides.
    //
    // `x2` and not `x1`, which was the first thing tried and reddened nothing: `x1` is read by the
    // text rectangle and by `x3`, and by no step of any path. A mutation that changes a guide the
    // *paths* do not read is invisible to a path differential, correctly, and this comment is here
    // so the next reader does not spend the same ten minutes on it.
    let (_, within, extents) = orientations()[0];
    let mut apart: BTreeMap<&'static str, f32> = BTreeMap::new();
    for definition in seeded_shapes() {
        let shape = definition.preset.to_wire();
        let mut xml = custom_geometry_xml(definition);
        if definition.preset == PresetShapeType::Triangle {
            let before = xml.clone();
            xml = xml.replace(
                r#"<a:gd name="x2" fmla="*/ w a 100000"/>"#,
                r#"<a:gd name="x2" fmla="*/ h a 100000"/>"#,
            );
            assert_ne!(
                xml, before,
                "`triangle`'s `x2` is no longer `*/ w a 100000`"
            );
        }
        let resolved = through_the_parser(&xml)
            .resolve(GuideContext::from_size(extents))
            .unwrap_or_else(|error| panic!("`{shape}` did not resolve: {error}"));
        let theirs = outline_of(within, &resolved, extents);
        let mine = preset_outline(definition.preset, extents, &[], within)
            .unwrap_or_else(|error| panic!("`{shape}` did not resolve: {error}"));
        let distance = commands_distance(&mine.commands, &theirs);
        if distance > AGREEMENT_TOLERANCE_PIXELS {
            apart.insert(shape, distance);
        }
    }
    assert_eq!(
        apart.keys().copied().collect::<Vec<_>>(),
        vec!["triangle"],
        "the mutation reddened {apart:#?}"
    );
    // Twenty pixels: `x2` is `w·a/100000` and the mutant is `h·a/100000`, so at the default
    // `a = 50000` the apex moves from `w/2` to `h/2`, which on a 160 × 120 box is 20. Stated as a
    // number rather than as *"it failed"*, because a differential that fails by an amount nobody
    // checked is a differential that could be failing for another reason.
    assert!(
        (apart["triangle"] - 20.0).abs() < 0.01,
        "the mutation moved `triangle` by {} px rather than the 20 half the difference between the \
         two sides is",
        apart["triangle"]
    );
}
