//! Unit tests for the DrawingML color + solidFill model, through the public API only. Every round-trip
//! assertion is paired with a structural one so byte-identity can't pass by dumping into `Raw`.

use mjx_dml::{Color, ColorKind, SchemeColor, SolidFill, SolidFillContent};
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

fn serialize_built<T: ToXml>(mut interner: Interner, typed: &T) -> String {
    let root = typed.to_xml(&mut interner);
    let doc = RawDocument {
        interner,
        bom: false,
        prologue: Vec::new(),
        root,
        epilogue: Vec::new(),
    };
    String::from_utf8(fidelity::serialize_to_vec(&doc)).expect("utf-8")
}

#[test]
fn srgb_color_reads_and_preserves_transforms() {
    let fragment =
        format!(r#"<a:srgbClr xmlns:a="{A}" val="FF0000"><a:lumMod val="50000"/></a:srgbClr>"#);
    let (color, doc): (Color, _) = parse_typed(fragment.as_bytes());
    assert_eq!(color.kind(&doc.interner), ColorKind::Srgb);
    assert_eq!(color.hex(&doc.interner), Some("FF0000"));
    assert_eq!(color.scheme_color(&doc.interner), None);
    // The lumMod transform is preserved opaquely.
    assert_eq!(color.transforms().len(), 1);
    assert_round_trips(&color, doc, fragment.as_bytes());
}

#[test]
fn scheme_color_reads_the_theme_slot() {
    let fragment = format!(r#"<a:schemeClr xmlns:a="{A}" val="accent1"/>"#);
    let (color, doc): (Color, _) = parse_typed(fragment.as_bytes());
    assert_eq!(color.kind(&doc.interner), ColorKind::Scheme);
    assert_eq!(
        color.scheme_color(&doc.interner),
        Some(SchemeColor::Accent1)
    );
    assert_eq!(color.hex(&doc.interner), None);
    assert_round_trips(&color, doc, fragment.as_bytes());
}

#[test]
fn system_and_preset_colors_round_trip() {
    // sysClr keeps its optional lastClr; both read as their kind and round-trip verbatim.
    let sys = format!(r#"<a:sysClr xmlns:a="{A}" val="windowText" lastClr="000000"/>"#);
    let (color, doc): (Color, _) = parse_typed(sys.as_bytes());
    assert_eq!(color.kind(&doc.interner), ColorKind::System);
    assert_round_trips(&color, doc, sys.as_bytes());

    let prst = format!(r#"<a:prstClr xmlns:a="{A}" val="red"/>"#);
    let (color, doc): (Color, _) = parse_typed(prst.as_bytes());
    assert_eq!(color.kind(&doc.interner), ColorKind::Preset);
    assert_round_trips(&color, doc, prst.as_bytes());
}

#[test]
fn builds_srgb_and_scheme_colors() {
    let mut interner = Interner::new();
    let srgb = Color::srgb(&mut interner, "FF0000");
    assert_eq!(
        serialize_built(interner, &srgb),
        r#"<a:srgbClr val="FF0000"/>"#
    );

    let mut interner = Interner::new();
    let scheme = Color::scheme(&mut interner, SchemeColor::Background1);
    assert_eq!(
        serialize_built(interner, &scheme),
        r#"<a:schemeClr val="bg1"/>"#
    );
}

#[test]
fn solid_fill_round_trips_and_exposes_its_color() {
    let fragment = format!(r#"<a:solidFill xmlns:a="{A}"><a:srgbClr val="00FF00"/></a:solidFill>"#);
    let (fill, doc): (SolidFill, _) = parse_typed(fragment.as_bytes());
    assert_eq!(fill.content().len(), 1);
    assert!(matches!(fill.content()[0], SolidFillContent::Color(_)));
    assert_eq!(fill.color().unwrap().hex(&doc.interner), Some("00FF00"));
    assert_round_trips(&fill, doc, fragment.as_bytes());
}

#[test]
fn builds_solid_fill_and_empty_fill_round_trips() {
    let mut interner = Interner::new();
    let color = Color::srgb(&mut interner, "00FF00");
    let fill = SolidFill::new(&mut interner, Some(color));
    assert_eq!(
        serialize_built(interner, &fill),
        r#"<a:solidFill><a:srgbClr val="00FF00"/></a:solidFill>"#
    );

    // An empty (color-less) solidFill is legal and round-trips; color() is None.
    let fragment = format!(r#"<a:solidFill xmlns:a="{A}"/>"#);
    let (fill, doc): (SolidFill, _) = parse_typed(fragment.as_bytes());
    assert!(fill.color().is_none());
    assert_round_trips(&fill, doc, fragment.as_bytes());
}
