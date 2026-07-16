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
