//! A hand-transcribed preset draws the shape it is, and a scene of presets needs no stand-in.
//!
//! MJX-STAND-IN: the stand-in is what a seeded preset is compared *against* here, so this suite is
//! about it: `a_seeded_shape_is_the_documents_own_geometry_and_not_the_stand_in` needs the shape it
//! must not be, and the placeholder-count cases need a provider that produces one.
//!
//! # What this file is about, now that the table is generated
//!
//! MJXOFF-203 replaced the six-shape seed table with 186 rows extracted from
//! `presetShapeDefinitions.xml`, and the counts, the structure and the box census of all 186 moved
//! to `the_generated_table_is_the_spec_file.rs` and `every_preset_stands_where_its_box_is.rs`.
//! What stayed here is everything whose *expectation* was written by hand, because a count derived
//! from the table it checks would agree with a table that had lost a step:
//!
//! 1. **The counts of the six** — a rectangle is four corners, an ellipse is four arcs, a right
//!    arrow is seven points. Written out per shape, from ECMA-376's prose, and checked against the
//!    **generated** rows. This is a second differential on top of `the_two_routes_agree.rs`: that
//!    one compares outlines, this one compares structure.
//! 2. **The box, and a tolerance shown to bite.** See [`BOX_TOLERANCE_PIXELS`] and
//!    [`the_box_tolerance_is_not_wide_enough_to_pass_a_wrong_shape`], which exhibits a shape that
//!    passes at this tolerance and fails at half of it.
//! 3. **The step vocabulary** — that every `PresetPathStep` maps onto exactly one `DrawCommand`.
//! 4. **The placeholder count in both directions** — zero for a scene of shapes the table has,
//!    non-zero for one that contains `upArrow`, the single preset ECMA-376's own geometry file
//!    defines nothing for. A counter that never increments satisfies the first perfectly, which is
//!    why the second is here.
//!
//! And what it deliberately is **not**: rendering the shapes and finding them plausible. MJXOFF-201
//! §6 says a picture that looks right is not evidence and an agent asserting it looks right is
//! asserting nothing. The authoritative visual check is the user's, against PowerPoint.

mod common;

use common::{
    bounds_of, box_on_the_page, edge_distance, extents_of_the_box, hand_transcribed, overhang,
    StepTally,
};
use mjx_dml::geometry::{AdjustAngle, AdjustCoordinate, DrawCommand, Emu};
use mjx_geometry::PathFillMode;
use mjx_geometry::{
    definition_of, outline_of_definition, preset_outline, seeded_shapes, Derivation, PresetAngle,
    PresetCoordinate, PresetGeometryProvider, PresetPath, PresetPathStep, PresetPoint,
    PresetShapeDefinition, PresetShapeType, ShapeOutline, Size, UnknownShapePolicy,
    PRESETS_WITHOUT_GEOMETRY,
};
use mjx_paint::plan_frame;
use mjx_scene::{
    Color, Command, DeviceScale, DisplayList, Geometry, GeometryProvider, OutlineProvenance, Paint,
    PathCommand, PlaceholderGeometry, SceneBuilder, SceneError, SceneRect, Tessellator,
};

// -------------------------------------------------------------------------------------------
// The tolerances, and why they are the numbers they are
// -------------------------------------------------------------------------------------------

/// How far a resolved outline's bounding box may sit from the shape's own box, **in device
/// pixels**, before [`every_seeded_shape_fills_the_box_it_was_given`] fails it.
///
/// Device pixels because that is the unit the answer is in — the seam hands a [`SceneRect`] in
/// device pixels and the commands come back in the same space — so the number means the thing a
/// reader would want it to mean: *"no edge of this shape is more than a hundredth of a pixel from
/// where the box says it is."*
///
/// **Where the error can come from, and how much of it there is.** Three sources, all bounded:
///
/// * `mjx-dml` rounds every resolved coordinate to a whole [`Emu`](mjx_ooxml_core::measure::Emu),
///   so a coordinate is off by at most half a unit. The suite resolves at one device pixel to the
///   point, so half an EMU is `0.5 / 12_700 ≈ 3.9e-5` pixels.
/// * The map to pixels narrows to `f32` once. At coordinates of a few hundred pixels that is about
///   `2.4e-5` pixels.
/// * A quarter-turn arc's cubic control points span exactly the arc's own box — the control points
///   of the quadrant from 180° to 270° sit at `x = cx - a` and `y = cy - b` — so the arc
///   decomposition contributes **nothing** at the four quadrant boundaries, which is where every
///   seeded arc begins and ends.
///
/// So a correct shape should land within about `1e-4` pixels, and the tolerance is a hundred times
/// that. **A hundred times is only defensible if the tolerance still bites**, which is what
/// [`the_box_tolerance_is_not_wide_enough_to_pass_a_wrong_shape`] shows: it exhibits a shape that
/// passes at this tolerance and *fails at half of it*, and a second whose error is the kind a real
/// transcription slip makes — three orders of magnitude past this number.
const BOX_TOLERANCE_PIXELS: f32 = 0.01;

/// How far outside its own box a point of a resolved outline may lie.
///
/// The same unit and the same three error sources as [`BOX_TOLERANCE_PIXELS`], and the same number,
/// because it is the same measurement read the other way round: "the box contains the shape" and
/// "the shape reaches the box" are the two halves of "the shape *is* the box", and a different
/// number for each would be two opinions about one quantity.
const OVERHANG_TOLERANCE_PIXELS: f32 = 0.01;

// -------------------------------------------------------------------------------------------
// Structure
// -------------------------------------------------------------------------------------------

#[test]
fn each_hand_transcribed_shape_is_one_closed_contour() {
    // The six shapes MJXOFF-202 read out of ECMA-376's prose are each a single closed outline, and
    // that is a claim about the *shapes* rather than about the table: a rectangle, an ellipse, a
    // triangle, a rounded rectangle, a right arrow and a pie wedge are all one contour, closed.
    //
    // It is deliberately **not** stated of all 186. Sixty-four of the presets end a contour without
    // an `a:close`, which is what a stroked, unfilled outline is — `straightConnector1` is a line
    // segment — and the universal invariants that *do* hold over the whole table are asserted in
    // `the_generated_table_is_the_spec_file.rs`.
    let (within, extents) = (box_on_the_page(), extents_of_the_box());
    let mut subpaths_seen = 0usize;
    for (preset, token) in hand_transcribed() {
        let outline = preset_outline(preset, extents, &[], within)
            .unwrap_or_else(|error| panic!("`{token}` did not resolve: {error}"));
        assert!(
            !outline.commands.is_empty(),
            "`{token}` resolved to no commands at all, which is the one answer the seam forbids"
        );

        let mut open = false;
        for command in &outline.commands {
            match command {
                PathCommand::MoveTo(_) => {
                    assert!(
                        !open,
                        "`{token}` began a subpath while one was still open — the earlier contour \
                         is never closed and would be filled as though it were"
                    );
                    open = true;
                    subpaths_seen += 1;
                }
                PathCommand::Close => {
                    assert!(open, "`{token}` closed a subpath that was never opened");
                    open = false;
                }
                _ => assert!(
                    open,
                    "`{token}` drew a step before any `MoveTo`; a path that starts with a line has \
                     no start point and is dropped by the tessellator"
                ),
            }
        }
        assert!(
            !open,
            "`{token}` left a subpath open at the end of its path"
        );
    }
    // A walker that found no subpaths would satisfy every assertion above by finding no violations.
    assert_eq!(
        subpaths_seen,
        hand_transcribed().len(),
        "the six shapes are one contour each, and the walk saw {subpaths_seen}"
    );
}

#[test]
fn each_hand_transcribed_shape_fills_the_box_it_was_given() {
    // Six shapes that are their box, and the tolerance stated on [`BOX_TOLERANCE_PIXELS`]. Every
    // arc of every one of the six begins and ends on a quadrant boundary, which is why [`overhang`]
    // — the stricter measure, over control points — is the right one here, and why the census over
    // all 186 in `every_preset_stands_where_its_box_is.rs` must use the flattened one instead.
    let (within, extents) = (box_on_the_page(), extents_of_the_box());
    let mut worst = 0.0f32;
    for (preset, token) in hand_transcribed() {
        let outline = preset_outline(preset, extents, &[], within)
            .unwrap_or_else(|error| panic!("`{token}` did not resolve: {error}"));
        let measured = edge_distance(bounds_of(&outline.commands), within);
        worst = worst.max(measured);
        assert!(
            measured <= BOX_TOLERANCE_PIXELS,
            "`{token}`'s bounding box is {measured} device pixels from its own box, and the \
             tolerance is {BOX_TOLERANCE_PIXELS}"
        );
        let outside = overhang(&outline.commands, within);
        assert!(
            outside <= OVERHANG_TOLERANCE_PIXELS,
            "`{token}` reaches {outside} device pixels outside its own box"
        );
    }
    // The headroom, asserted rather than hoped for: every one is inside *half* the stated
    // tolerance, so the number is not knife-edge and halving it would not start failing correct
    // shapes. It is printed so a reviewer can see how much of the budget is actually used.
    println!("worst bounding-box error across the six shapes: {worst} device pixels");
    assert!(
        worst <= BOX_TOLERANCE_PIXELS / 2.0,
        "the worst correct shape is {worst} pixels off, which is more than half the tolerance — \
         the tolerance is doing no work and should be tightened or the geometry fixed"
    );
}

/// The width [`common::extents_of_the_box`] states, so the literal coordinates below can be read.
const SHAPE_WIDTH_EMU: i64 = 160 * 12_700;

/// The height it states.
const SHAPE_HEIGHT_EMU: i64 = 120 * 12_700;

/// A corner of a narrow rectangle, at literal EMU.
const fn corner(x: i64, y: i64) -> PresetPoint {
    PresetPoint {
        x: PresetCoordinate::Emu(x),
        y: PresetCoordinate::Emu(y),
    }
}

/// A rectangle whose right edge is 95 EMU — 0.00748 device pixels — short of the box.
const NARROW_BY_NINETY_FIVE_EMU: PresetShapeDefinition = PresetShapeDefinition {
    preset: PresetShapeType::Rectangle,
    derivation: Derivation::FromFirstPrinciples,
    source: "a rectangle 95 EMU narrow, for the tolerance gate alone",
    adjustment_values: &[],
    guides: &[],
    text_rectangle: None,
    connection_sites: &[],
    paths: &[PresetPath {
        width: None,
        height: None,
        fill: PathFillMode::Normal,
        stroke: true,
        extrusion_ok: true,
        steps: &[
            PresetPathStep::MoveTo(corner(0, 0)),
            PresetPathStep::LineTo(corner(SHAPE_WIDTH_EMU - 95, 0)),
            PresetPathStep::LineTo(corner(SHAPE_WIDTH_EMU - 95, SHAPE_HEIGHT_EMU)),
            PresetPathStep::LineTo(corner(0, SHAPE_HEIGHT_EMU)),
            PresetPathStep::Close,
        ],
    }],
};

/// A rectangle whose right edge is one per cent of the width short of the box — 1.6 pixels, the
/// scale a real transcription slip lives at.
const NARROW_BY_ONE_PER_CENT: PresetShapeDefinition = PresetShapeDefinition {
    preset: PresetShapeType::Rectangle,
    derivation: Derivation::FromFirstPrinciples,
    source: "a rectangle one per cent narrow, for the tolerance gate alone",
    adjustment_values: &[],
    guides: &[],
    text_rectangle: None,
    connection_sites: &[],
    paths: &[PresetPath {
        width: None,
        height: None,
        fill: PathFillMode::Normal,
        stroke: true,
        extrusion_ok: true,
        steps: &[
            PresetPathStep::MoveTo(corner(0, 0)),
            PresetPathStep::LineTo(corner(SHAPE_WIDTH_EMU - SHAPE_WIDTH_EMU / 100, 0)),
            PresetPathStep::LineTo(corner(
                SHAPE_WIDTH_EMU - SHAPE_WIDTH_EMU / 100,
                SHAPE_HEIGHT_EMU,
            )),
            PresetPathStep::LineTo(corner(0, SHAPE_HEIGHT_EMU)),
            PresetPathStep::Close,
        ],
    }],
};

#[test]
fn the_box_tolerance_is_not_wide_enough_to_pass_a_wrong_shape() {
    let (within, extents) = (box_on_the_page(), extents_of_the_box());

    // Half a device pixel would be a tolerance that passes everything; a hundredth is not, and this
    // is the demonstration. A rectangle whose right edge is short by `inset` EMU has a bounding box
    // exactly `inset / 12_700` device pixels narrow, because the suite resolves at one pixel to the
    // point — so the error is a number chosen, not measured, and the check below is a comparison a
    // reader can do in their head. The two insets are written as literal coordinates rather than
    // computed, so the arithmetic a reader has to trust is visible on the page.
    assert_eq!(
        (extents.width.emu(), extents.height.emu()),
        (SHAPE_WIDTH_EMU, SHAPE_HEIGHT_EMU),
        "the literal coordinates below assume this suite's extents"
    );
    let measure = |definition: &PresetShapeDefinition| {
        let outline = outline_of_definition(definition, extents, &[], within)
            .expect("the narrow rectangle resolves");
        edge_distance(bounds_of(&outline.commands), within)
    };

    // 95 EMU is 0.00748 pixels: inside the stated tolerance, outside half of it. **This is the
    // shape that fails at half the tolerance**, which is what makes the number a real threshold
    // rather than a value large enough to accept anything.
    let just_inside = measure(&NARROW_BY_NINETY_FIVE_EMU);
    assert!(
        just_inside <= BOX_TOLERANCE_PIXELS,
        "a 95 EMU inset measured {just_inside} pixels, which should be inside the tolerance"
    );
    assert!(
        just_inside > BOX_TOLERANCE_PIXELS / 2.0,
        "a 95 EMU inset measured {just_inside} pixels; it must fail at half the tolerance, or \
         `every_seeded_shape_fills_the_box_it_was_given`'s headroom assertion proves nothing"
    );

    // And the scale a real mistake lives at: one per cent of the width — the kind of slip a `*/ w a
    // 100000` written as `*/ w a 101000` makes — is 1.6 pixels, a hundred and sixty times the
    // tolerance. A tolerance that could not tell these apart would be the defect this test is for.
    let a_real_slip = measure(&NARROW_BY_ONE_PER_CENT);
    assert!(
        a_real_slip > BOX_TOLERANCE_PIXELS * 100.0,
        "a one-per-cent transcription error measured only {a_real_slip} pixels"
    );
    println!(
        "box tolerance {BOX_TOLERANCE_PIXELS}: a 95 EMU inset measures {just_inside}, a \
         one-per-cent slip {a_real_slip}"
    );
}

#[test]
fn the_command_counts_are_the_shapes_own() {
    // Written out per shape from ECMA-376's prose rather than derived from the table, deliberately:
    // a count computed from the same data it checks would agree with a table that had lost a step.
    // These are what the shape *is* — a rectangle is four corners, an ellipse four quadrant arcs
    // and therefore four cubics, a right arrow seven points, and `roundRect` has three lines and
    // four corners because its left edge is drawn by the close.
    //
    // **The counts are checked against the generated rows**, which is what makes this a second
    // differential rather than a restatement: MJXOFF-202 wrote them from the prose and MJXOFF-203
    // supplies the geometry they are measured against. `the_two_routes_agree.rs` compares the same
    // six shapes' resolved outlines; this compares their structure.
    //
    // **What the 186-shape equivalent is, since there cannot be one of these per shape.** A
    // hand-written tally for every preset would be a transcription of the file into a second file,
    // with the same error rate and no independent source. What replaces it is a count of the *whole
    // table* against the file's own element counts, in
    // `the_generated_table_is_the_spec_file.rs` — 445 `moveTo`, 1 689 `lnTo`, 393 `arcTo`, 33
    // `quadBezTo`, 28 `cubicBezTo`, 319 `close` — obtained by counting XML elements, which is a
    // different reading of the same bytes from the one the extractor does. A shape that lost a step
    // moves that total.
    let expected = [
        (PresetShapeType::Rectangle, StepTally::expected(1, 3, 0, 1)),
        (PresetShapeType::Ellipse, StepTally::expected(1, 0, 4, 1)),
        (PresetShapeType::Triangle, StepTally::expected(1, 2, 0, 1)),
        (
            PresetShapeType::RoundedRectangle,
            StepTally::expected(1, 3, 4, 1),
        ),
        (PresetShapeType::RightArrow, StepTally::expected(1, 6, 0, 1)),
        (PresetShapeType::Pie, StepTally::expected(1, 1, 3, 1)),
    ];
    assert_eq!(
        expected.len(),
        hand_transcribed().len(),
        "the hand-written reference has {} shapes and this list has {}; a shape added without a \
         count is a shape nothing checks the structure of",
        hand_transcribed().len(),
        expected.len()
    );

    let (within, extents) = (box_on_the_page(), extents_of_the_box());
    for (preset, tally) in expected {
        let token = preset.to_wire();
        let outline = preset_outline(preset, extents, &[], within)
            .unwrap_or_else(|error| panic!("`{token}` did not resolve: {error}"));
        assert_eq!(
            StepTally::of(&outline.commands),
            tally,
            "`{token}` drew the wrong steps"
        );
    }
}

#[test]
fn the_hand_written_table_says_how_independently_each_shape_was_transcribed() {
    // `the_two_routes_agree.rs` is only worth what the two routes' independence is worth. So every
    // hand-written row records it, both hand-written values are used, and every row explains itself
    // in prose long enough to be a real answer.
    let mut seen = Vec::new();
    for definition in mjx_geometry::seed::HAND_TRANSCRIBED_SHAPES {
        let token = definition.preset.to_wire();
        assert!(
            definition.source.len() > 200,
            "`{token}`'s source note is {} characters; it has to say what the geometry was derived \
             from and what was not independent, which does not fit in a line",
            definition.source.len()
        );
        assert!(
            definition.source.contains("§20.1.10.56"),
            "`{token}`'s source note cites no clause"
        );
        if definition.derivation == Derivation::ConstantsFromTheGeneratedTables {
            assert!(
                definition.source.contains("NOT independent")
                    || definition.source.contains("NOT independently"),
                "`{token}` is marked as taking constants from the generated tables but its note \
                 does not say which are not independent, so the differential cannot tell what it \
                 is worth"
            );
        }
        seen.push(definition.derivation);
    }
    // The third value belongs to the generated rows, not to these; between the two tables every
    // value of the enumeration is carried by something.
    seen.extend(
        seeded_shapes()
            .iter()
            .map(|definition| definition.derivation),
    );
    for value in Derivation::ALL {
        assert!(
            seen.contains(&value),
            "no shape in either table is marked {value:?}; an enumeration one value of which \
             nothing carries is a distinction nothing makes"
        );
    }
}

// -------------------------------------------------------------------------------------------
// The step vocabulary
// -------------------------------------------------------------------------------------------

#[test]
fn every_step_maps_onto_the_one_draw_command_vocabulary() {
    // MJXOFF-201 §2: *do not define a second command vocabulary*. `PresetPathStep` is
    // `DrawCommand` in a form a `static` can hold, and this is the proof that the mapping is total
    // and carries the payload rather than merely returning the right variant. All seven, because a
    // mapping that lost one would produce a shape missing a step and nothing else would notice.
    let point = |x, y| PresetPoint {
        x: PresetCoordinate::Emu(x),
        y: PresetCoordinate::Emu(y),
    };
    let emu = |command: &DrawCommand| format!("{command:?}");

    assert!(matches!(
        PresetPathStep::Close.to_draw_command(),
        DrawCommand::Close
    ));

    let moved = PresetPathStep::MoveTo(point(11, 22)).to_draw_command();
    match &moved {
        DrawCommand::MoveTo(at) => {
            assert_eq!(at.x, AdjustCoordinate::Emu(Emu::from_emu(11)));
            assert_eq!(at.y, AdjustCoordinate::Emu(Emu::from_emu(22)));
        }
        other => panic!("a MoveTo became {}", emu(other)),
    }

    let lined = PresetPathStep::LineTo(PresetPoint::at("hc", "vc")).to_draw_command();
    match &lined {
        DrawCommand::LineTo(at) => {
            assert_eq!(at.x, AdjustCoordinate::Guide("hc".to_owned()));
            assert_eq!(at.y, AdjustCoordinate::Guide("vc".to_owned()));
        }
        other => panic!("a LineTo became {}", emu(other)),
    }

    let arced = PresetPathStep::ArcTo {
        width_radius: PresetCoordinate::Emu(100),
        height_radius: PresetCoordinate::Guide("hd2"),
        start_angle: PresetAngle::Native(5_400_000),
        swing_angle: PresetAngle::Guide("swAng"),
    }
    .to_draw_command();
    match &arced {
        DrawCommand::ArcTo {
            width_radius,
            height_radius,
            start_angle,
            swing_angle,
        } => {
            assert_eq!(*width_radius, AdjustCoordinate::Emu(Emu::from_emu(100)));
            assert_eq!(*height_radius, AdjustCoordinate::Guide("hd2".to_owned()));
            match start_angle {
                AdjustAngle::Angle(angle) => assert!(
                    (angle.degrees() - 90.0).abs() < 1e-9,
                    "a literal quarter turn became {} degrees",
                    angle.degrees()
                ),
                other => panic!("a literal angle became {other:?}"),
            }
            assert_eq!(*swing_angle, AdjustAngle::Guide("swAng".to_owned()));
        }
        other => panic!("an ArcTo became {}", emu(other)),
    }

    let quadratic = PresetPathStep::QuadBezierTo {
        control: point(1, 2),
        end: point(3, 4),
    }
    .to_draw_command();
    match &quadratic {
        DrawCommand::QuadBezierTo(control, end) => {
            assert_eq!(control.x, AdjustCoordinate::Emu(Emu::from_emu(1)));
            assert_eq!(end.y, AdjustCoordinate::Emu(Emu::from_emu(4)));
        }
        other => panic!("a QuadBezierTo became {}", emu(other)),
    }

    let cubic = PresetPathStep::CubicBezierTo {
        first_control: point(1, 2),
        second_control: point(3, 4),
        end: point(5, 6),
    }
    .to_draw_command();
    match &cubic {
        DrawCommand::CubicBezierTo(first, second, end) => {
            assert_eq!(first.x, AdjustCoordinate::Emu(Emu::from_emu(1)));
            assert_eq!(second.x, AdjustCoordinate::Emu(Emu::from_emu(3)));
            assert_eq!(end.x, AdjustCoordinate::Emu(Emu::from_emu(5)));
        }
        other => panic!("a CubicBezierTo became {}", emu(other)),
    }
}

/// A shape drawn with the two step kinds no seeded preset uses.
const A_CURVE: PresetShapeDefinition = PresetShapeDefinition {
    preset: PresetShapeType::Rectangle,
    derivation: Derivation::FromFirstPrinciples,
    source: "a curve, for the two step kinds no seeded shape uses",
    adjustment_values: &[],
    guides: &[],
    text_rectangle: None,
    connection_sites: &[],
    paths: &[PresetPath {
        width: None,
        height: None,
        fill: PathFillMode::Normal,
        stroke: true,
        extrusion_ok: true,
        steps: &[
            PresetPathStep::MoveTo(PresetPoint::at("l", "t")),
            PresetPathStep::QuadBezierTo {
                control: PresetPoint::at("hc", "t"),
                end: PresetPoint::at("r", "vc"),
            },
            PresetPathStep::CubicBezierTo {
                first_control: PresetPoint::at("r", "b"),
                second_control: PresetPoint::at("hc", "b"),
                end: PresetPoint::at("l", "b"),
            },
            PresetPathStep::Close,
        ],
    }],
};

#[test]
fn a_bezier_step_reaches_the_page_as_a_bezier() {
    // The two step kinds no seeded shape uses. They are not dead: MJXOFF-203's `heart`, `cloud`,
    // `moon` and `wave` are all curves, and a table that emitted them into a resolver that dropped
    // them would draw four straight-edged shapes and no test would say so. So the path from step to
    // `PathCommand` is exercised here on both, end to end, through the same call the provider uses.
    let within = box_on_the_page();
    let outline = outline_of_definition(&A_CURVE, extents_of_the_box(), &[], within)
        .expect("a curve resolves");
    assert_eq!(
        StepTally::of(&outline.commands),
        StepTally {
            moves: 1,
            lines: 0,
            quadratics: 1,
            cubics: 1,
            closes: 1,
        }
    );
    // And the control points landed where the guides put them, rather than at the origin.
    match outline.commands[1] {
        PathCommand::QuadraticTo { control, end } => {
            assert!((control.x - (within.left + within.width() / 2.0)).abs() < 0.01);
            assert!((end.y - (within.top + within.height() / 2.0)).abs() < 0.01);
        }
        other => panic!("the second command is {other:?}"),
    }
}

// -------------------------------------------------------------------------------------------
// A seeded shape is not the stand-in
// -------------------------------------------------------------------------------------------

#[test]
fn a_seeded_shape_is_the_documents_own_geometry_and_not_the_stand_in() {
    let (within, extents) = (box_on_the_page(), extents_of_the_box());
    let stand_in = PlaceholderGeometry::new()
        .outline(7, within)
        .expect("the placeholder answers every handle");

    for definition in seeded_shapes() {
        let (preset, token) = (definition.preset, definition.preset.to_wire());
        let outline = preset_outline(preset, extents, &[], within)
            .unwrap_or_else(|error| panic!("`{token}` did not resolve: {error}"));
        assert_eq!(
            outline.provenance,
            OutlineProvenance::Document,
            "`{token}` answered with a provenance that would let a golden image taken against it \
             be recorded as fidelity"
        );
        assert_eq!(outline.label, token, "`{token}` labelled itself wrongly");
        assert_ne!(
            outline.commands, stand_in.commands,
            "`{token}` drew the placeholder's own path"
        );
        // The placeholder is a frame and two diagonal bars, drawn with *quadratic* corners — a
        // quadratic through a corner point is not a circular arc, which is one more way the
        // stand-in is not something DrawingML draws. **No preset in the file uses a `quadBezTo`
        // for a corner of an axis-aligned rounded box**, and thirty-three `quadBezTo`s across the
        // 186 are enough that this is a real distinction rather than an empty one. A substitution
        // that produced the placeholder's shape by another route would pass the inequality above
        // and fail this.
        assert!(
            StepTally::of(&outline.commands).quadratics
                < StepTally::of(&stand_in.commands).quadratics
                || StepTally::of(&outline.commands).closes
                    != StepTally::of(&stand_in.commands).closes,
            "`{token}` has the placeholder's own quadratic corners and its contour count"
        );
    }
}

// -------------------------------------------------------------------------------------------
// The placeholder count, in both directions
// -------------------------------------------------------------------------------------------

/// The handle each seeded shape is registered under in the scenes below.
fn handle_of(index: usize) -> u64 {
    0x0000_1000_0000_0000 | index as u64
}

/// A page with one filled shape per handle, none of them resolved while it is built.
fn a_page_of(handles: &[u64]) -> DisplayList {
    let mut builder = SceneBuilder::new(DeviceScale::UNZOOMED, 640.0, 480.0);
    let paint = builder
        .add_paint(Paint::Solid(Color {
            red: 0x22,
            green: 0x44,
            blue: 0x88,
            alpha: 0xff,
        }))
        .expect("one paint");
    for (index, handle) in handles.iter().enumerate() {
        let top = 8.0 + 76.0 * index as f32;
        let geometry = builder
            .add_geometry(&Geometry::Unresolved {
                outline: *handle,
                bounds: SceneRect::new(8.0, top, 168.0, top + 68.0),
            })
            .expect("an unresolved geometry");
        builder
            .push(Command::FillPath { geometry, paint })
            .expect("a fill");
    }
    builder.finish().expect("the scene is well formed")
}

/// A provider holding every seeded shape, plus whatever `extra` adds.
fn a_provider(policy: UnknownShapePolicy) -> (PresetGeometryProvider, Vec<u64>) {
    let mut provider = match policy {
        UnknownShapePolicy::Refuse => PresetGeometryProvider::new(),
        UnknownShapePolicy::StandIn => PresetGeometryProvider::standing_in_for_unknown_shapes(),
    };
    let mut handles = Vec::new();
    for (index, definition) in seeded_shapes().iter().enumerate() {
        let handle = handle_of(index);
        provider.register(
            handle,
            ShapeOutline::new(definition.preset, Size::from_emu(160 * 12_700, 68 * 12_700)),
        );
        handles.push(handle);
    }
    (provider, handles)
}

#[test]
fn an_adjustment_registered_on_a_handle_reaches_the_shape_it_draws() {
    // The registry is the whole of what the seam's opaque handle means, so the question worth
    // asking of it is not "does it store what I put in" but **"does what I put in change the
    // picture"**. Two handles, the same preset, one adjustment apart.
    let (within, extents) = (box_on_the_page(), extents_of_the_box());
    let mut provider = PresetGeometryProvider::new();
    provider.register(
        1,
        ShapeOutline::new(PresetShapeType::RoundedRectangle, extents),
    );
    provider.register(
        2,
        ShapeOutline::new(PresetShapeType::RoundedRectangle, extents)
            .with_adjustment("adj", 50_000.0),
    );

    let plain = provider.outline(1, within).expect("the default resolves");
    let adjusted = provider.outline(2, within).expect("the override resolves");
    assert_ne!(
        plain.commands, adjusted.commands,
        "an adjustment recorded on a handle did not reach the outline it draws"
    );
    assert_eq!(plain.label, adjusted.label, "both are still `roundRect`");

    // And re-registering answers with what stood there before, so a caller replacing a shape can
    // tell whether it replaced anything.
    let previous = provider.register(1, ShapeOutline::new(PresetShapeType::Ellipse, extents));
    assert_eq!(
        previous.map(|shape| shape.preset),
        Some(PresetShapeType::RoundedRectangle)
    );
    assert_eq!(
        provider.shape(1).map(|shape| shape.preset),
        Some(PresetShapeType::Ellipse)
    );
    assert!(provider.shape(3).is_none());
}

#[test]
fn a_scene_of_seeded_shapes_needs_no_stand_in_at_all() {
    let (provider, handles) = a_provider(UnknownShapePolicy::Refuse);
    assert_eq!(provider.len(), seeded_shapes().len());
    assert!(!provider.is_empty());

    let list = a_page_of(&handles);
    let mut tessellator = Tessellator::new();
    let plan = plan_frame(&list, &provider, &mut tessellator).expect("the page lowers");
    let report = plan.report();

    assert_eq!(
        report.placeholders,
        0,
        "a page of {} presets drew {} stand-in(s)",
        handles.len(),
        report.placeholders
    );
    // A counter that never increments satisfies the line above perfectly, so this is the half that
    // makes it mean something: the walk really did reach six fills and really did make triangles.
    assert_eq!(report.draw_calls, handles.len());
    assert!(
        report.triangles > handles.len() * 2,
        "{} shapes became {} triangle(s), which cannot be that many filled outlines",
        handles.len(),
        report.triangles
    );
}

#[test]
fn one_unseeded_shape_raises_the_count_off_zero() {
    // `upArrow` is a real preset that this build genuinely cannot draw — ECMA-376's own geometry
    // file has no element for it — which is what makes it the honest way to ask this question:
    // nothing about the scene is different except that one of its shapes has no table.
    //
    // MJXOFF-202 left this test keyed on `cloud` and said it would break when MJXOFF-203 seeded
    // all 186, deliberately, so that the count would still be provable in both directions. It is:
    // `upArrow` took `cloud`'s place, and the guard below fails loudly if a later child hand-writes
    // the one shape the file omits.
    let unseeded_preset = PRESETS_WITHOUT_GEOMETRY[0];
    assert!(
        definition_of(unseeded_preset).is_none(),
        "`{}` is now seeded; pick a preset this build still cannot draw",
        unseeded_preset.to_wire()
    );

    let (mut provider, mut handles) = a_provider(UnknownShapePolicy::StandIn);
    let unseeded = handle_of(handles.len());
    provider.register(
        unseeded,
        ShapeOutline::new(unseeded_preset, Size::from_emu(160 * 12_700, 68 * 12_700)),
    );
    handles.push(unseeded);

    let list = a_page_of(&handles);
    let mut tessellator = Tessellator::new();
    let plan = plan_frame(&list, &provider, &mut tessellator).expect("the page lowers");
    assert_eq!(
        plan.report().placeholders,
        1,
        "one unseeded shape among the table's own produced {} stand-in(s)",
        plan.report().placeholders
    );
    assert_eq!(plan.report().draw_calls, handles.len());
}

#[test]
fn a_shape_this_build_cannot_draw_is_never_silently_nothing() {
    // The seam's own rule, in this crate's terms: *"a shape that silently drew nothing is a defect
    // a reader reports as 'my slide is missing a box' and nobody finds"*. There are exactly two
    // permitted answers and this asserts both, on both of the two ways a shape can be undrawable.
    let within = box_on_the_page();
    let extents = extents_of_the_box();
    let unseeded = ShapeOutline::new(PRESETS_WITHOUT_GEOMETRY[0], extents);

    for (label, policy) in [
        ("refusing", UnknownShapePolicy::Refuse),
        ("standing in", UnknownShapePolicy::StandIn),
    ] {
        let mut provider = match policy {
            UnknownShapePolicy::Refuse => PresetGeometryProvider::new(),
            UnknownShapePolicy::StandIn => PresetGeometryProvider::standing_in_for_unknown_shapes(),
        };
        assert_eq!(provider.unknown_shape_policy(), policy);
        provider.register(1, unseeded.clone());

        for (what, handle) in [("an unseeded shape", 1u64), ("an unregistered handle", 2)] {
            let answer = provider.outline(handle, within);
            match policy {
                UnknownShapePolicy::Refuse => assert!(
                    matches!(answer, Err(SceneError::UnresolvedOutline { outline }) if outline == handle),
                    "{label} answered {what} with {answer:?} rather than an error"
                ),
                UnknownShapePolicy::StandIn => {
                    let outline = answer.expect("a stand-in is an answer");
                    assert_eq!(outline.provenance, OutlineProvenance::Placeholder);
                    assert!(
                        !outline.commands.is_empty(),
                        "{label} answered {what} with an empty path, which is the one answer the \
                         seam forbids"
                    );
                    assert_eq!(outline.label, PlaceholderGeometry::label_for(handle));
                }
            }
        }
    }
}

/// A definition whose path names a guide nothing defines — a table defect, not a table gap.
const A_MISSING_GUIDE: PresetShapeDefinition = PresetShapeDefinition {
    preset: PresetShapeType::Rectangle,
    derivation: Derivation::FromFirstPrinciples,
    source: "a path naming a guide that does not exist, for this gate alone",
    adjustment_values: &[],
    guides: &[],
    text_rectangle: None,
    connection_sites: &[],
    paths: &[PresetPath {
        width: None,
        height: None,
        fill: PathFillMode::Normal,
        stroke: true,
        extrusion_ok: true,
        steps: &[
            PresetPathStep::MoveTo(PresetPoint::at("l", "t")),
            PresetPathStep::LineTo(PresetPoint::at("nowhere", "t")),
            PresetPathStep::Close,
        ],
    }],
};

#[test]
fn a_broken_table_is_an_error_under_both_policies() {
    // The policy is a fall-through for *"there is no geometry to draw"*, not a blanket "answer with
    // something". A shape the table has, whose path names a guide the table does not define, is a
    // defect in this crate's own data, and a rounded rectangle drawn over it would hide the one
    // failure the table's gates exist to catch. **It is deliberately distinguishable from the
    // singular case**, which looks identical from `DrawCommand::resolve` — an undefined guide name
    // — and is told apart by whether the table defined the guide and the resolver left it out; see
    // `an_adjustment_moves_the_shape.rs`.
    let error = outline_of_definition(
        &A_MISSING_GUIDE,
        extents_of_the_box(),
        &[],
        box_on_the_page(),
    )
    .expect_err("a path naming an undefined guide cannot resolve");
    assert!(
        !error.has_no_geometry_to_draw(),
        "a table whose path names a missing guide reported itself as having nothing to draw, so a \
         stand-in would have been drawn over it: {error}"
    );
    assert!(
        matches!(error, mjx_geometry::GeometryError::PathCommand { .. }),
        "a path naming a guide nothing ever defined is not a singularity: {error}"
    );
    assert_eq!(error.shape(), Some("rect"));
    assert!(
        format!("{error}").contains("rect"),
        "the failure does not name the shape: {error}"
    );
}
