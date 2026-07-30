//! Unit tests for the custom-geometry foundation (`a:custGeom` value types), through the public API.
//!
//! The two properties these carry the whole model on: an [`AdjustCoordinate`] / [`AdjustAngle`]
//! distinguishes a literal from a **guide reference**, and an [`AdjustPoint`] round-trips
//! byte-for-byte with any attribute it does not model preserved verbatim.

use mjx_dml::{AdjustAngle, AdjustCoordinate, AdjustPoint, Emu};
use mjx_ooxml_core::{FromXml, Interner, RawDocument, ToXml};
use mjx_xml::fidelity;

const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";

fn parse_typed<T: FromXml>(fragment: &[u8]) -> (T, RawDocument) {
    let doc = fidelity::parse(fragment).expect("fragment parses");
    let typed = T::from_xml(&doc.root, &doc.interner).expect("from_xml");
    (typed, doc)
}

#[track_caller]
fn assert_round_trips<T: ToXml>(typed: &T, mut doc: RawDocument, expected: &[u8]) {
    doc.root = typed.to_xml(&mut doc.interner);
    let out = fidelity::serialize_to_vec(&doc);
    assert_eq!(
        String::from_utf8_lossy(&out),
        String::from_utf8_lossy(expected),
        "round-trip byte mismatch"
    );
}

// ---------------------------------------------------------------------------------------------
// AdjustCoordinate — a literal EMU, or a guide reference
// ---------------------------------------------------------------------------------------------

#[test]
fn an_adjust_coordinate_reads_an_integer_as_emu_and_anything_else_as_a_guide() {
    assert_eq!(
        AdjustCoordinate::from_wire("100"),
        AdjustCoordinate::Emu(Emu::from_emu(100))
    );
    assert_eq!(
        AdjustCoordinate::from_wire("-2540"),
        AdjustCoordinate::Emu(Emu::from_emu(-2540))
    );
    assert_eq!(
        AdjustCoordinate::from_wire("hc"),
        AdjustCoordinate::Guide("hc".to_owned())
    );
}

#[test]
fn an_adjust_coordinate_round_trips_through_its_wire_form() {
    assert_eq!(AdjustCoordinate::Emu(Emu::from_emu(100)).to_wire(), "100");
    assert_eq!(AdjustCoordinate::Guide("adj1".to_owned()).to_wire(), "adj1");
    for wire in ["0", "914400", "-50", "cd2", "wd8"] {
        assert_eq!(AdjustCoordinate::from_wire(wire).to_wire(), wire);
    }
}

// ---------------------------------------------------------------------------------------------
// AdjustAngle — a literal angle, or a guide reference
// ---------------------------------------------------------------------------------------------

#[test]
fn an_adjust_angle_reads_an_integer_as_an_angle_and_anything_else_as_a_guide() {
    // 5_400_000 sixtieths-of-a-thousandth-of-a-degree is a quarter turn.
    match AdjustAngle::from_wire("5400000") {
        AdjustAngle::Angle(angle) => assert!((angle.degrees() - 90.0).abs() < 1e-9),
        AdjustAngle::Guide(_) => panic!("integer angle read as a guide"),
    }
    assert_eq!(
        AdjustAngle::from_wire("adj"),
        AdjustAngle::Guide("adj".to_owned())
    );
}

#[test]
fn an_adjust_angle_round_trips_through_its_wire_form() {
    for wire in ["0", "5400000", "-1200000", "21600000", "adj2"] {
        assert_eq!(AdjustAngle::from_wire(wire).to_wire(), wire);
    }
}

// ---------------------------------------------------------------------------------------------
// AdjustPoint — a fidelity leaf that reads x/y typed and preserves the rest
// ---------------------------------------------------------------------------------------------

#[test]
fn an_adjust_point_reads_its_coordinates_typed() {
    let xml = format!(r#"<a:pt xmlns:a="{A}" x="914400" y="hc"/>"#);
    let (pt, doc) = parse_typed::<AdjustPoint>(xml.as_bytes());
    assert_eq!(
        pt.x(&doc.interner),
        Some(AdjustCoordinate::Emu(Emu::from_emu(914_400)))
    );
    assert_eq!(
        pt.y(&doc.interner),
        Some(AdjustCoordinate::Guide("hc".to_owned()))
    );
}

#[test]
fn an_adjust_point_round_trips_byte_for_byte_with_an_unknown_attribute() {
    // The `foo` attribute is not modeled; it must survive verbatim, in its original position.
    let xml = format!(r#"<a:pt xmlns:a="{A}" x="100" foo="bar" y="200"/>"#);
    let (pt, doc) = parse_typed::<AdjustPoint>(xml.as_bytes());
    assert_eq!(
        pt.x(&doc.interner),
        Some(AdjustCoordinate::Emu(Emu::from_emu(100)))
    );
    assert_round_trips(&pt, doc, xml.as_bytes());
}

#[test]
fn a_built_adjust_point_reads_back_the_coordinates_it_was_given() {
    let mut interner = Interner::new();
    let pt = AdjustPoint::new(
        &mut interner,
        "pt",
        &AdjustCoordinate::Emu(Emu::from_emu(100)),
        &AdjustCoordinate::Guide("hc".to_owned()),
    );
    assert_eq!(
        pt.x(&interner),
        Some(AdjustCoordinate::Emu(Emu::from_emu(100)))
    );
    assert_eq!(
        pt.y(&interner),
        Some(AdjustCoordinate::Guide("hc".to_owned()))
    );

    // And it serializes to the expected `pt` element with both coordinates as attributes.
    let root = pt.to_xml(&mut interner);
    let doc = RawDocument {
        interner,
        bom: false,
        prologue: Vec::new(),
        root,
        epilogue: Vec::new(),
    };
    let out = String::from_utf8(fidelity::serialize_to_vec(&doc)).expect("utf-8");
    assert!(out.contains(r#"x="100""#), "missing x: {out}");
    assert!(out.contains(r#"y="hc""#), "missing y: {out}");
}
