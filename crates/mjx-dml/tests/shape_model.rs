//! Tests for the typed shape-geometry tier (`ShapeGeometry`, `shape()`, `set_shape()`), driven
//! through the public API. Named values are checked against spec-derived expectations; every
//! serialized assertion is paired with a structural/re-parse one.

use mjx_dml::{Fraction, PresetGeometry, ShapeGeometry};
use mjx_ooxml_core::{FromXml, Interner, RawDocument, ToXml};
use mjx_ooxml_types::drawingml::PresetShapeType;
use mjx_xml::fidelity;

const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";

fn parse_typed(fragment: &[u8]) -> (PresetGeometry, RawDocument) {
    let doc = fidelity::parse(fragment).expect("fragment parses");
    let geom = PresetGeometry::from_xml(&doc.root, &doc.interner).expect("from_xml");
    (geom, doc)
}

fn serialize_built(mut interner: Interner, geom: &PresetGeometry) -> String {
    let root = geom.to_xml(&mut interner);
    let doc = RawDocument {
        interner,
        bom: false,
        prologue: Vec::new(),
        root,
        epilogue: Vec::new(),
    };
    String::from_utf8(fidelity::serialize_to_vec(&doc)).expect("utf-8")
}

#[track_caller]
fn assert_close(actual: Fraction, expected: f64) {
    assert!(
        (actual.ratio() - expected).abs() < 1e-9,
        "expected ≈{expected}, got {}",
        actual.ratio()
    );
}

#[test]
fn reads_default_corner_radius() {
    let fragment = format!(r#"<a:prstGeom xmlns:a="{A}" prst="roundRect"/>"#);
    let (geom, doc) = parse_typed(fragment.as_bytes());
    let Some(ShapeGeometry::RoundedRectangle { corner_radius }) = geom.shape(&doc.interner) else {
        panic!("expected RoundedRectangle");
    };
    assert_close(corner_radius, 0.16667); // default adj 16667 / 100000
}

#[test]
fn reads_overridden_corner_radius() {
    let fragment = format!(
        r#"<a:prstGeom xmlns:a="{A}" prst="roundRect"><a:avLst><a:gd name="adj" fmla="val 25000"/></a:avLst></a:prstGeom>"#
    );
    let (geom, doc) = parse_typed(fragment.as_bytes());
    let Some(ShapeGeometry::RoundedRectangle { corner_radius }) = geom.shape(&doc.interner) else {
        panic!("expected RoundedRectangle");
    };
    assert_close(corner_radius, 0.25);
}

#[test]
fn reads_star_inner_radius_over_the_star_denominator() {
    // star4 default adj 12500; inner radius is a/50000 of the outer point radius → 0.25.
    let fragment = format!(r#"<a:prstGeom xmlns:a="{A}" prst="star4"/>"#);
    let (geom, doc) = parse_typed(fragment.as_bytes());
    let Some(ShapeGeometry::FourPointStar { inner_radius }) = geom.shape(&doc.interner) else {
        panic!("expected FourPointStar");
    };
    assert_close(inner_radius, 0.25);
}

#[test]
fn unported_shape_is_unmodeled_and_unknown_prst_is_none() {
    // teardrop is single-adjustment but deferred (spec-ambiguous) → Unmodeled.
    let teardrop = format!(r#"<a:prstGeom xmlns:a="{A}" prst="teardrop"/>"#);
    let (geom, doc) = parse_typed(teardrop.as_bytes());
    assert_eq!(
        geom.shape(&doc.interner),
        Some(ShapeGeometry::Unmodeled(PresetShapeType::Teardrop))
    );

    // An unknown/future prst token → None.
    let unknown = format!(r#"<a:prstGeom xmlns:a="{A}" prst="notAShape"/>"#);
    let (geom, doc) = parse_typed(unknown.as_bytes());
    assert_eq!(geom.shape(&doc.interner), None);
}

#[test]
fn set_shape_writes_prst_and_adjustment() {
    // Start from a bare rectangle and turn it into a rounded rectangle with a 25% corner radius.
    let mut interner = Interner::new();
    let mut geom = PresetGeometry::new(&mut interner, PresetShapeType::Rectangle, None);
    geom.set_shape(
        &mut interner,
        ShapeGeometry::RoundedRectangle {
            corner_radius: Fraction::from_ratio(0.25),
        },
    );
    assert_eq!(
        serialize_built(interner, &geom),
        r#"<a:prstGeom prst="roundRect"><a:avLst><a:gd name="adj" fmla="val 25000"/></a:avLst></a:prstGeom>"#
    );
}

#[test]
fn set_shape_then_shape_round_trips_a_star() {
    // 0.375 inner radius → native 0.375 * 50000 = 18750 → back to 0.375.
    let mut interner = Interner::new();
    let mut geom = PresetGeometry::new(&mut interner, PresetShapeType::Rectangle, None);
    geom.set_shape(
        &mut interner,
        ShapeGeometry::EightPointStar {
            inner_radius: Fraction::from_ratio(0.375),
        },
    );
    assert_eq!(geom.adjustment(&interner, "adj"), Some(18750));
    let Some(ShapeGeometry::EightPointStar { inner_radius }) = geom.shape(&interner) else {
        panic!("expected EightPointStar");
    };
    assert_close(inner_radius, 0.375);
}

// --- Remaining single-adjustment families ---

#[test]
fn reads_triangle_apex_as_fraction_of_width() {
    // triangle default adj 50000 → apex centered at 0.5 of the width.
    let fragment = format!(r#"<a:prstGeom xmlns:a="{A}" prst="triangle"/>"#);
    let (geom, doc) = parse_typed(fragment.as_bytes());
    let Some(ShapeGeometry::Triangle { apex_x }) = geom.shape(&doc.interner) else {
        panic!("expected Triangle");
    };
    assert_close(apex_x, 0.5);
}

#[test]
fn reads_math_plus_from_adj1() {
    // mathPlus uses adj1 (not adj); default 23520 → 0.2352 of the shorter side.
    let fragment = format!(r#"<a:prstGeom xmlns:a="{A}" prst="mathPlus"/>"#);
    let (geom, doc) = parse_typed(fragment.as_bytes());
    let Some(ShapeGeometry::MathPlus { arm_thickness }) = geom.shape(&doc.interner) else {
        panic!("expected MathPlus");
    };
    assert_close(arm_thickness, 0.2352);
}

#[test]
fn reads_donut_ring_from_radius_handle() {
    let fragment = format!(r#"<a:prstGeom xmlns:a="{A}" prst="donut"/>"#);
    let (geom, doc) = parse_typed(fragment.as_bytes());
    let Some(ShapeGeometry::Donut { ring_thickness }) = geom.shape(&doc.interner) else {
        panic!("expected Donut");
    };
    assert_close(ring_thickness, 0.25);
}

#[test]
fn reads_chevron_point_depth_default() {
    // chevron's max is a computed guide (maxAdj); the default still reads cleanly.
    let fragment = format!(r#"<a:prstGeom xmlns:a="{A}" prst="chevron"/>"#);
    let (geom, doc) = parse_typed(fragment.as_bytes());
    let Some(ShapeGeometry::Chevron { point_depth }) = geom.shape(&doc.interner) else {
        panic!("expected Chevron");
    };
    assert_close(point_depth, 0.5);
}

#[test]
fn smiley_mouth_curve_is_signed_and_round_trips() {
    // Default is a positive (smile) curvature.
    let fragment = format!(r#"<a:prstGeom xmlns:a="{A}" prst="smileyFace"/>"#);
    let (mut geom, mut doc) = parse_typed(fragment.as_bytes());
    let Some(ShapeGeometry::SmileyFace { mouth_curve }) = geom.shape(&doc.interner) else {
        panic!("expected SmileyFace");
    };
    assert_close(mouth_curve, 0.04653);

    // A negative value (frown) survives the native round-trip.
    geom.set_shape(
        &mut doc.interner,
        ShapeGeometry::SmileyFace {
            mouth_curve: Fraction::from_ratio(-0.02),
        },
    );
    assert_eq!(geom.adjustment(&doc.interner, "adj"), Some(-2000));
    let Some(ShapeGeometry::SmileyFace { mouth_curve }) = geom.shape(&doc.interner) else {
        panic!("expected SmileyFace");
    };
    assert_close(mouth_curve, -0.02);
}

#[test]
fn set_shape_writes_adj1_shapes() {
    // mathPlus writes to adj1, not adj.
    let mut interner = Interner::new();
    let mut geom = PresetGeometry::new(&mut interner, PresetShapeType::Rectangle, None);
    geom.set_shape(
        &mut interner,
        ShapeGeometry::MathPlus {
            arm_thickness: Fraction::from_ratio(0.3),
        },
    );
    assert_eq!(
        serialize_built(interner, &geom),
        r#"<a:prstGeom prst="mathPlus"><a:avLst><a:gd name="adj1" fmla="val 30000"/></a:avLst></a:prstGeom>"#
    );
}

// --- Two-adjustment shapes (Batch 4) ---

use mjx_dml::Angle;

#[track_caller]
fn assert_deg(actual: Angle, expected: f64) {
    assert!(
        (actual.degrees() - expected).abs() < 1e-6,
        "expected ≈{expected}°, got {}°",
        actual.degrees()
    );
}

#[test]
fn reads_and_writes_pie_angles() {
    // pie default: adj1 = 0 (start), adj2 = 16_200_000 = 270° (end).
    let fragment = format!(r#"<a:prstGeom xmlns:a="{A}" prst="pie"/>"#);
    let (mut geom, mut doc) = parse_typed(fragment.as_bytes());
    let Some(ShapeGeometry::Pie {
        start_angle,
        end_angle,
    }) = geom.shape(&doc.interner)
    else {
        panic!("expected Pie");
    };
    assert_deg(start_angle, 0.0);
    assert_deg(end_angle, 270.0);

    // set_shape writes native 60000ths of a degree and round-trips.
    geom.set_shape(
        &mut doc.interner,
        ShapeGeometry::Pie {
            start_angle: Angle::from_degrees(90.0),
            end_angle: Angle::from_degrees(180.0),
        },
    );
    assert_eq!(geom.adjustment(&doc.interner, "adj1"), Some(5_400_000));
    assert_eq!(geom.adjustment(&doc.interner, "adj2"), Some(10_800_000));
    let Some(ShapeGeometry::Pie {
        start_angle,
        end_angle,
    }) = geom.shape(&doc.interner)
    else {
        panic!("expected Pie");
    };
    assert_deg(start_angle, 90.0);
    assert_deg(end_angle, 180.0);
}

#[test]
fn reads_arrow_two_fields() {
    // rightArrow defaults: adj1 = 50000 (shaft), adj2 = 50000 (head) → 0.5 each.
    let fragment = format!(r#"<a:prstGeom xmlns:a="{A}" prst="rightArrow"/>"#);
    let (geom, doc) = parse_typed(fragment.as_bytes());
    let Some(ShapeGeometry::RightArrow {
        shaft_thickness,
        head_length,
    }) = geom.shape(&doc.interner)
    else {
        panic!("expected RightArrow");
    };
    assert_close(shaft_thickness, 0.5);
    assert_close(head_length, 0.5);
}

#[test]
fn reads_callout_signed_tail() {
    // wedgeRectCallout defaults: adj1 = -20833 (tail_x), adj2 = 62500 (tail_y).
    let fragment = format!(r#"<a:prstGeom xmlns:a="{A}" prst="wedgeRectCallout"/>"#);
    let (geom, doc) = parse_typed(fragment.as_bytes());
    let Some(ShapeGeometry::WedgeRectangleCallout { tail_x, tail_y }) = geom.shape(&doc.interner)
    else {
        panic!("expected WedgeRectangleCallout");
    };
    assert_close(tail_x, -0.20833);
    assert_close(tail_y, 0.625);
}

#[test]
fn reads_diagonal_corner_rectangle() {
    // round2DiagRect defaults: adj1 = 16667 (tl/br), adj2 = 0 (tr/bl).
    let fragment = format!(r#"<a:prstGeom xmlns:a="{A}" prst="round2DiagRect"/>"#);
    let (geom, doc) = parse_typed(fragment.as_bytes());
    let Some(ShapeGeometry::RoundDiagonalCornersRectangle {
        top_left_bottom_right_radius,
        top_right_bottom_left_radius,
    }) = geom.shape(&doc.interner)
    else {
        panic!("expected RoundDiagonalCornersRectangle");
    };
    assert_close(top_left_bottom_right_radius, 0.16667);
    assert_close(top_right_bottom_left_radius, 0.0);
}

#[test]
fn wave_skew_is_signed() {
    let mut interner = Interner::new();
    let mut geom = PresetGeometry::new(&mut interner, PresetShapeType::Rectangle, None);
    geom.set_shape(
        &mut interner,
        ShapeGeometry::Wave {
            amplitude: Fraction::from_ratio(0.1),
            skew: Fraction::from_ratio(-0.05),
        },
    );
    assert_eq!(geom.adjustment(&interner, "adj1"), Some(10000));
    assert_eq!(geom.adjustment(&interner, "adj2"), Some(-5000));
    let Some(ShapeGeometry::Wave { amplitude, skew }) = geom.shape(&interner) else {
        panic!("expected Wave");
    };
    assert_close(amplitude, 0.1);
    assert_close(skew, -0.05);
}

#[test]
fn set_shape_writes_both_adjustments() {
    let mut interner = Interner::new();
    let mut geom = PresetGeometry::new(&mut interner, PresetShapeType::Rectangle, None);
    geom.set_shape(
        &mut interner,
        ShapeGeometry::RightArrow {
            shaft_thickness: Fraction::from_ratio(0.4),
            head_length: Fraction::from_ratio(0.6),
        },
    );
    assert_eq!(
        serialize_built(interner, &geom),
        r#"<a:prstGeom prst="rightArrow"><a:avLst><a:gd name="adj1" fmla="val 40000"/><a:gd name="adj2" fmla="val 60000"/></a:avLst></a:prstGeom>"#
    );
}

// --- Callouts (Batch 5a) ---

#[test]
fn reads_callout1_signed_vertices() {
    // callout1 defaults: adj1=18750(y1), adj2=-8333(x1), adj3=112500(y2), adj4=-38333(x2).
    let fragment = format!(r#"<a:prstGeom xmlns:a="{A}" prst="callout1"/>"#);
    let (geom, doc) = parse_typed(fragment.as_bytes());
    let Some(ShapeGeometry::Callout1 {
        vertex1_x,
        vertex1_y,
        vertex2_x,
        vertex2_y,
    }) = geom.shape(&doc.interner)
    else {
        panic!("expected Callout1");
    };
    assert_close(vertex1_x, -0.08333); // box anchor, left of box
    assert_close(vertex1_y, 0.1875);
    assert_close(vertex2_x, -0.38333); // tip
    assert_close(vertex2_y, 1.125); // below the box (> 1)
}

#[test]
fn accent_border_callout_shares_callout_structure() {
    // accentBorderCallout2 has the same adjustment mapping as callout2 (accent/border are render-only).
    let fragment = format!(r#"<a:prstGeom xmlns:a="{A}" prst="accentBorderCallout2"/>"#);
    let (geom, doc) = parse_typed(fragment.as_bytes());
    let Some(ShapeGeometry::AccentBorderCallout2 {
        vertex1_x,
        vertex2_x,
        vertex3_x,
        vertex3_y,
        ..
    }) = geom.shape(&doc.interner)
    else {
        panic!("expected AccentBorderCallout2");
    };
    assert_close(vertex1_x, -0.08333);
    assert_close(vertex2_x, -0.16667);
    assert_close(vertex3_x, -0.46667); // tip x
    assert_close(vertex3_y, 1.125); // tip y
}

#[test]
fn set_callout3_writes_all_eight_adjustments() {
    let mut interner = Interner::new();
    let mut geom = PresetGeometry::new(&mut interner, PresetShapeType::Rectangle, None);
    geom.set_shape(
        &mut interner,
        ShapeGeometry::Callout3 {
            vertex1_x: Fraction::from_ratio(-0.1),
            vertex1_y: Fraction::from_ratio(0.2),
            vertex2_x: Fraction::from_ratio(-0.2),
            vertex2_y: Fraction::from_ratio(0.2),
            vertex3_x: Fraction::from_ratio(-0.2),
            vertex3_y: Fraction::from_ratio(1.0),
            vertex4_x: Fraction::from_ratio(-0.1),
            vertex4_y: Fraction::from_ratio(1.1),
        },
    );
    // adj order is y1,x1,y2,x2,… → adj1=20000, adj2=-10000, …, adj8=-10000.
    assert_eq!(geom.adjustment(&interner, "adj1"), Some(20000));
    assert_eq!(geom.adjustment(&interner, "adj2"), Some(-10000));
    assert_eq!(geom.adjustment(&interner, "adj7"), Some(110000));
    assert_eq!(geom.adjustment(&interner, "adj8"), Some(-10000));

    // Re-parse the serialized bytes and read back.
    let serialized = serialize_built(interner, &geom);
    let fragment = serialized.replacen("<a:prstGeom", &format!(r#"<a:prstGeom xmlns:a="{A}""#), 1);
    let (reparsed, doc) = parse_typed(fragment.as_bytes());
    let Some(ShapeGeometry::Callout3 {
        vertex3_y,
        vertex4_x,
        ..
    }) = reparsed.shape(&doc.interner)
    else {
        panic!("expected Callout3");
    };
    assert_close(vertex3_y, 1.0);
    assert_close(vertex4_x, -0.1);
}

// --- 3-adjustment arrows / ribbons / connectors (Batch 5b-i) ---

#[test]
fn reads_and_sets_unbounded_connector_bends() {
    let fragment = format!(r#"<a:prstGeom xmlns:a="{A}" prst="bentConnector5"/>"#);
    let (mut geom, mut doc) = parse_typed(fragment.as_bytes());
    let Some(ShapeGeometry::BentConnector5 {
        bend1_x,
        bend2_y,
        bend3_x,
    }) = geom.shape(&doc.interner)
    else {
        panic!("expected BentConnector5");
    };
    assert_close(bend1_x, 0.5);
    assert_close(bend2_y, 0.5);
    assert_close(bend3_x, 0.5);

    // Bend positions are unbounded — a value beyond 0..1 round-trips.
    geom.set_shape(
        &mut doc.interner,
        ShapeGeometry::BentConnector5 {
            bend1_x: Fraction::from_ratio(1.2),
            bend2_y: Fraction::from_ratio(0.5),
            bend3_x: Fraction::from_ratio(-0.1),
        },
    );
    assert_eq!(geom.adjustment(&doc.interner, "adj1"), Some(120000));
    assert_eq!(geom.adjustment(&doc.interner, "adj3"), Some(-10000));
    let Some(ShapeGeometry::BentConnector5 {
        bend1_x, bend3_x, ..
    }) = geom.shape(&doc.interner)
    else {
        panic!("expected BentConnector5");
    };
    assert_close(bend1_x, 1.2);
    assert_close(bend3_x, -0.1);
}

#[test]
fn reads_curved_arrow_body_head() {
    let fragment = format!(r#"<a:prstGeom xmlns:a="{A}" prst="curvedRightArrow"/>"#);
    let (geom, doc) = parse_typed(fragment.as_bytes());
    let Some(ShapeGeometry::CurvedRightArrow {
        body_thickness,
        head_width,
        head_length,
    }) = geom.shape(&doc.interner)
    else {
        panic!("expected CurvedRightArrow");
    };
    assert_close(body_thickness, 0.25);
    assert_close(head_width, 0.5);
    assert_close(head_length, 0.25);
}

#[test]
fn reads_ellipse_ribbon() {
    let fragment = format!(r#"<a:prstGeom xmlns:a="{A}" prst="ellipseRibbon"/>"#);
    let (geom, doc) = parse_typed(fragment.as_bytes());
    let Some(ShapeGeometry::EllipseRibbon {
        arch_height,
        center_width,
        fold_thickness,
    }) = geom.shape(&doc.interner)
    else {
        panic!("expected EllipseRibbon");
    };
    assert_close(arch_height, 0.25);
    assert_close(center_width, 0.5);
    assert_close(fold_thickness, 0.125);
}

#[test]
fn set_quad_arrow_writes_three_adjustments() {
    let mut interner = Interner::new();
    let mut geom = PresetGeometry::new(&mut interner, PresetShapeType::Rectangle, None);
    geom.set_shape(
        &mut interner,
        ShapeGeometry::QuadArrow {
            shaft_thickness: Fraction::from_ratio(0.3),
            head_width: Fraction::from_ratio(0.4),
            head_length: Fraction::from_ratio(0.35),
        },
    );
    assert_eq!(
        serialize_built(interner, &geom),
        r#"<a:prstGeom prst="quadArrow"><a:avLst><a:gd name="adj1" fmla="val 30000"/><a:gd name="adj2" fmla="val 40000"/><a:gd name="adj3" fmla="val 35000"/></a:avLst></a:prstGeom>"#
    );
}

// --- Arrow-callouts + bentArrow + uTurnArrow (Batch 5b-ii) ---

#[test]
fn reads_down_arrow_callout() {
    let fragment = format!(r#"<a:prstGeom xmlns:a="{A}" prst="downArrowCallout"/>"#);
    let (geom, doc) = parse_typed(fragment.as_bytes());
    let Some(ShapeGeometry::DownArrowCallout {
        shaft_thickness,
        arrowhead_width,
        arrowhead_length,
        text_box_size,
    }) = geom.shape(&doc.interner)
    else {
        panic!("expected DownArrowCallout");
    };
    assert_close(shaft_thickness, 0.25);
    assert_close(arrowhead_width, 0.25);
    assert_close(arrowhead_length, 0.25);
    assert_close(text_box_size, 0.64977);
}

#[test]
fn reads_double_arrow_callout_smaller_box() {
    let fragment = format!(r#"<a:prstGeom xmlns:a="{A}" prst="leftRightArrowCallout"/>"#);
    let (geom, doc) = parse_typed(fragment.as_bytes());
    let Some(ShapeGeometry::LeftRightArrowCallout { text_box_size, .. }) =
        geom.shape(&doc.interner)
    else {
        panic!("expected LeftRightArrowCallout");
    };
    assert_close(text_box_size, 0.48123); // double-headed → smaller box default
}

#[test]
fn reads_bent_arrow_bend_radius() {
    let fragment = format!(r#"<a:prstGeom xmlns:a="{A}" prst="bentArrow"/>"#);
    let (geom, doc) = parse_typed(fragment.as_bytes());
    let Some(ShapeGeometry::BentArrow { bend_radius, .. }) = geom.shape(&doc.interner) else {
        panic!("expected BentArrow");
    };
    assert_close(bend_radius, 0.4375);
}

#[test]
fn uturn_arrow_reads_five_fields_and_sets() {
    let fragment = format!(r#"<a:prstGeom xmlns:a="{A}" prst="uturnArrow"/>"#);
    let (mut geom, mut doc) = parse_typed(fragment.as_bytes());
    let Some(ShapeGeometry::UTurnArrow { tip_height, .. }) = geom.shape(&doc.interner) else {
        panic!("expected UTurnArrow");
    };
    assert_close(tip_height, 0.75);

    geom.set_shape(
        &mut doc.interner,
        ShapeGeometry::UTurnArrow {
            shaft_thickness: Fraction::from_ratio(0.3),
            arrowhead_width: Fraction::from_ratio(0.2),
            arrowhead_length: Fraction::from_ratio(0.25),
            bend_radius: Fraction::from_ratio(0.4),
            tip_height: Fraction::from_ratio(0.8),
        },
    );
    assert_eq!(geom.adjustment(&doc.interner, "adj1"), Some(30000));
    assert_eq!(geom.adjustment(&doc.interner, "adj5"), Some(80000));
    let Some(ShapeGeometry::UTurnArrow {
        arrowhead_width,
        tip_height,
        ..
    }) = geom.shape(&doc.interner)
    else {
        panic!("expected UTurnArrow");
    };
    assert_close(arrowhead_width, 0.2);
    assert_close(tip_height, 0.8);
}

// --- Angle/math shapes (Batch 5c) ---

#[test]
fn reads_block_arc_angles_and_thickness() {
    let fragment = format!(r#"<a:prstGeom xmlns:a="{A}" prst="blockArc"/>"#);
    let (geom, doc) = parse_typed(fragment.as_bytes());
    let Some(ShapeGeometry::BlockArc {
        start_angle,
        end_angle,
        ring_thickness,
    }) = geom.shape(&doc.interner)
    else {
        panic!("expected BlockArc");
    };
    assert_deg(start_angle, 180.0); // adj1 = 10_800_000
    assert_deg(end_angle, 0.0); // adj2 = 0
    assert_close(ring_thickness, 0.25);
}

#[test]
fn reads_math_not_equal_slash_angle() {
    let fragment = format!(r#"<a:prstGeom xmlns:a="{A}" prst="mathNotEqual"/>"#);
    let (geom, doc) = parse_typed(fragment.as_bytes());
    let Some(ShapeGeometry::MathNotEqual {
        bar_thickness,
        slash_angle,
        bar_gap,
    }) = geom.shape(&doc.interner)
    else {
        panic!("expected MathNotEqual");
    };
    assert_close(bar_thickness, 0.2352);
    assert_deg(slash_angle, 110.0); // adj2 = 6_600_000
    assert_close(bar_gap, 0.1176);
}

#[test]
fn reads_math_divide_fractions() {
    let fragment = format!(r#"<a:prstGeom xmlns:a="{A}" prst="mathDivide"/>"#);
    let (geom, doc) = parse_typed(fragment.as_bytes());
    let Some(ShapeGeometry::MathDivide {
        bar_thickness,
        dot_gap,
        dot_radius,
    }) = geom.shape(&doc.interner)
    else {
        panic!("expected MathDivide");
    };
    assert_close(bar_thickness, 0.2352);
    assert_close(dot_gap, 0.0588);
    assert_close(dot_radius, 0.1176);
}

#[test]
fn circular_arrow_reads_mixed_and_round_trips() {
    let fragment = format!(r#"<a:prstGeom xmlns:a="{A}" prst="circularArrow"/>"#);
    let (mut geom, mut doc) = parse_typed(fragment.as_bytes());
    let Some(ShapeGeometry::CircularArrow {
        body_thickness,
        start_angle,
        head_width,
        ..
    }) = geom.shape(&doc.interner)
    else {
        panic!("expected CircularArrow");
    };
    assert_close(body_thickness, 0.125); // adj1 (radius → fraction of ss)
    assert_deg(start_angle, 180.0); // adj4 = 10_800_000
    assert_close(head_width, 0.125); // adj5

    // set_shape mixes fraction + angle adjustments (adj1..adj5).
    geom.set_shape(
        &mut doc.interner,
        ShapeGeometry::CircularArrow {
            body_thickness: Fraction::from_ratio(0.1),
            head_pointer_angle: Angle::from_degrees(20.0),
            end_angle: Angle::from_degrees(340.0),
            start_angle: Angle::from_degrees(170.0),
            head_width: Fraction::from_ratio(0.15),
        },
    );
    assert_eq!(geom.adjustment(&doc.interner, "adj1"), Some(10000));
    assert_eq!(geom.adjustment(&doc.interner, "adj4"), Some(10_200_000)); // 170° · 60000
    assert_eq!(geom.adjustment(&doc.interner, "adj5"), Some(15000));
    let Some(ShapeGeometry::CircularArrow {
        head_pointer_angle,
        end_angle,
        ..
    }) = geom.shape(&doc.interner)
    else {
        panic!("expected CircularArrow");
    };
    assert_deg(head_pointer_angle, 20.0);
    assert_deg(end_angle, 340.0);
}
