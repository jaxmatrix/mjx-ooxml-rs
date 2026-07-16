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
    // chevron is single-adjustment but not in this batch → Unmodeled.
    let chevron = format!(r#"<a:prstGeom xmlns:a="{A}" prst="chevron"/>"#);
    let (geom, doc) = parse_typed(chevron.as_bytes());
    assert_eq!(
        geom.shape(&doc.interner),
        Some(ShapeGeometry::Unmodeled(PresetShapeType::Chevron))
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
