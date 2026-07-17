//! Unit tests for the shape-style reference (`a:fillRef`) parse and the theme color map resolution,
//! through the public API only.

use mjx_dml::{ColorKind, ColorMap, ColorSchemeSlot, SchemeColor, StyleMatrixReference};
use mjx_ooxml_core::{FromXml, RawDocument};
use mjx_xml::fidelity;

const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";

fn parse_ref(fragment: &[u8]) -> (StyleMatrixReference, RawDocument) {
    let doc = fidelity::parse(fragment).expect("fragment parses");
    let reference = StyleMatrixReference::from_xml(&doc.root, &doc.interner).expect("from_xml");
    (reference, doc)
}

#[test]
fn fill_ref_reads_index_and_color() {
    let fragment =
        format!(r#"<a:fillRef xmlns:a="{A}" idx="1"><a:schemeClr val="accent1"/></a:fillRef>"#);
    let (reference, doc) = parse_ref(fragment.as_bytes());
    assert_eq!(reference.idx(), Some(1));
    let color = reference.color().expect("fillRef color");
    assert_eq!(color.kind(&doc.interner), ColorKind::Scheme);
    assert_eq!(
        color.scheme_color(&doc.interner),
        Some(SchemeColor::Accent1)
    );
}

#[test]
fn fill_ref_without_color_and_zero_index() {
    // idx 0 is the schema's "no reference"; a bare fillRef carries no color.
    let fragment = format!(r#"<a:fillRef xmlns:a="{A}" idx="0"/>"#);
    let (reference, _doc) = parse_ref(fragment.as_bytes());
    assert_eq!(reference.idx(), Some(0));
    assert!(reference.color().is_none());
}

#[test]
fn fill_ref_missing_index_is_none() {
    let fragment = format!(r#"<a:fillRef xmlns:a="{A}"><a:srgbClr val="FF0000"/></a:fillRef>"#);
    let (reference, doc) = parse_ref(fragment.as_bytes());
    assert_eq!(reference.idx(), None);
    assert_eq!(
        reference.color().unwrap().hex(&doc.interner),
        Some("FF0000")
    );
}

#[test]
fn color_map_resolves_logical_direct_and_placeholder() {
    // A custom map that remaps the logical background/text names.
    let map = ColorMap {
        background1: ColorSchemeSlot::Dark2,
        text1: ColorSchemeSlot::Light1,
        ..ColorMap::identity()
    };
    // Logical names go through the map.
    assert_eq!(
        map.resolve(SchemeColor::Background1),
        Some(ColorSchemeSlot::Dark2)
    );
    assert_eq!(
        map.resolve(SchemeColor::Text1),
        Some(ColorSchemeSlot::Light1)
    );
    // Accents pass through (identity here).
    assert_eq!(
        map.resolve(SchemeColor::Accent3),
        Some(ColorSchemeSlot::Accent3)
    );
    // dk/lt reference a slot directly, bypassing the map.
    assert_eq!(
        map.resolve(SchemeColor::Dark1),
        Some(ColorSchemeSlot::Dark1)
    );
    assert_eq!(
        map.resolve(SchemeColor::Light2),
        Some(ColorSchemeSlot::Light2)
    );
    // phClr is not a scheme color.
    assert_eq!(map.resolve(SchemeColor::PlaceholderColor), None);
}

#[test]
fn identity_map_matches_the_standard_office_mapping() {
    let map = ColorMap::identity();
    assert_eq!(
        map.resolve(SchemeColor::Background1),
        Some(ColorSchemeSlot::Light1)
    );
    assert_eq!(
        map.resolve(SchemeColor::Text1),
        Some(ColorSchemeSlot::Dark1)
    );
    assert_eq!(
        map.resolve(SchemeColor::Background2),
        Some(ColorSchemeSlot::Light2)
    );
    assert_eq!(
        map.resolve(SchemeColor::Text2),
        Some(ColorSchemeSlot::Dark2)
    );
    assert_eq!(
        map.resolve(SchemeColor::FollowedHyperlink),
        Some(ColorSchemeSlot::FollowedHyperlink)
    );
}
