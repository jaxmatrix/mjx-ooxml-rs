//! Unit tests for the DrawingML preset-geometry model, driven through the public API only.
//!
//! Fragments are parsed with the `mjx-xml` fidelity reader (declaring `xmlns:a` inline), turned into
//! typed values with `FromXml`, inspected, and rebuilt with `ToXml`. Every round-trip assertion is
//! paired with a **structural** assertion so byte-identity cannot pass by the model silently dumping
//! everything into the opaque `Raw` bucket. Builder tests construct values from scratch and check both
//! the serialized bytes and a re-parse.

use mjx_dml::{
    GeometryGuide, GeometryGuideList, GeometryGuideListContent, PresetGeometry,
    PresetGeometryContent,
};
use mjx_ooxml_core::{FromXml, Interner, RawDocument, ToXml};
use mjx_ooxml_types::drawingml::PresetShapeType;
use mjx_xml::fidelity;

/// Parses a fragment and turns its root element into a typed `T`, returning it with the still-usable
/// document (whose interner the value's `Symbol`s belong to).
fn parse_typed<T: FromXml>(fragment: &[u8]) -> (T, RawDocument) {
    let doc = fidelity::parse(fragment).expect("fragment parses");
    let typed = T::from_xml(&doc.root, &doc.interner).expect("from_xml succeeds");
    (typed, doc)
}

/// Rebuilds the document root from `typed` (reusing the part's interner) and asserts the serialized
/// bytes equal `expected`.
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

/// Serializes a value built against a fresh `interner` (which the value's `Symbol`s belong to).
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

const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
const A_STRICT: &str = "http://purl.oclc.org/ooxml/drawingml/main";

const PRSTGEOM: &[u8] = br#"<a:prstGeom xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" prst="roundRect"><a:avLst><a:gd name="adj" fmla="val 25000"/><a:gd name="adj2" fmla="val 12500"/></a:avLst></a:prstGeom>"#;

#[test]
fn parses_typed_structure() {
    let (geom, doc): (PresetGeometry, _) = parse_typed(PRSTGEOM);

    assert_eq!(
        geom.preset(&doc.interner),
        Some(PresetShapeType::RoundedRectangle)
    );
    assert_eq!(geom.preset_token(&doc.interner), Some("roundRect"));

    // The only typed child is the avLst.
    assert_eq!(geom.content().len(), 1);
    assert!(matches!(
        geom.content()[0],
        PresetGeometryContent::AdjustValues(_)
    ));

    let list = geom.adjust_values().expect("has an avLst");
    let guides: Vec<_> = list.guides().collect();
    assert_eq!(guides.len(), 2);
    assert_eq!(guides[0].name(&doc.interner), Some("adj"));
    assert_eq!(guides[0].formula(&doc.interner), Some("val 25000"));
    assert_eq!(guides[1].name(&doc.interner), Some("adj2"));
    assert_eq!(guides[1].formula(&doc.interner), Some("val 12500"));
}

#[test]
fn round_trips_prstgeom_with_avlst_byte_identical() {
    let (geom, doc): (PresetGeometry, _) = parse_typed(PRSTGEOM);
    assert_eq!(geom.adjust_values().unwrap().guides().count(), 2); // structural pair
    assert_round_trips(&geom, doc, PRSTGEOM);
}

#[test]
fn round_trips_self_closing_prstgeom_without_avlst() {
    let fragment = format!(r#"<a:prstGeom xmlns:a="{A}" prst="rect"/>"#);
    let (geom, doc): (PresetGeometry, _) = parse_typed(fragment.as_bytes());
    // Structural: a bare preset, no adjust values.
    assert_eq!(geom.preset(&doc.interner), Some(PresetShapeType::Rectangle));
    assert!(geom.adjust_values().is_none());
    assert!(geom.content().is_empty());
    assert_round_trips(&geom, doc, fragment.as_bytes());
}

#[test]
fn preserves_unknown_child_as_raw() {
    // An unexpected child element must survive as `Raw`, ahead of the typed avLst.
    let fragment =
        format!(r#"<a:prstGeom xmlns:a="{A}" prst="ellipse"><a:custom/><a:avLst/></a:prstGeom>"#);
    let (geom, doc): (PresetGeometry, _) = parse_typed(fragment.as_bytes());
    assert_eq!(geom.preset(&doc.interner), Some(PresetShapeType::Ellipse));
    assert_eq!(geom.content().len(), 2);
    assert!(matches!(geom.content()[0], PresetGeometryContent::Raw(_)));
    assert!(matches!(
        geom.content()[1],
        PresetGeometryContent::AdjustValues(_)
    ));
    assert_round_trips(&geom, doc, fragment.as_bytes());
}

#[test]
fn matches_children_under_strict_namespace_uri() {
    // The same model reads a shape whose `a` prefix is bound to the strict URI.
    let fragment = format!(
        r#"<a:prstGeom xmlns:a="{A_STRICT}" prst="roundRect"><a:avLst><a:gd name="adj" fmla="val 16667"/></a:avLst></a:prstGeom>"#
    );
    let (geom, doc): (PresetGeometry, _) = parse_typed(fragment.as_bytes());
    assert_eq!(geom.preset_token(&doc.interner), Some("roundRect"));
    let guides: Vec<_> = geom.adjust_values().unwrap().guides().collect();
    assert_eq!(guides.len(), 1);
    assert_eq!(guides[0].formula(&doc.interner), Some("val 16667"));
    assert_round_trips(&geom, doc, fragment.as_bytes());
}

#[test]
fn preserves_significant_whitespace() {
    let fragment = format!(
        "<a:prstGeom xmlns:a=\"{A}\" prst=\"roundRect\">\n  <a:avLst>\n    <a:gd name=\"adj\" fmla=\"val 25000\"/>\n  </a:avLst>\n</a:prstGeom>"
    );
    let (geom, doc): (PresetGeometry, _) = parse_typed(fragment.as_bytes());
    // Whitespace between the prstGeom's children is preserved as Raw, interleaved with the avLst.
    assert!(geom
        .content()
        .iter()
        .any(|item| matches!(item, PresetGeometryContent::Raw(_))));
    let list = geom.adjust_values().expect("avLst");
    assert_eq!(list.guides().count(), 1);
    // Whitespace inside the avLst is Raw too, surrounding the single typed guide.
    assert!(list
        .content()
        .iter()
        .any(|item| matches!(item, GeometryGuideListContent::Raw(_))));
    assert_round_trips(&geom, doc, fragment.as_bytes());
}

#[test]
fn unknown_prst_token_round_trips_and_is_readable() {
    // A token this build does not know still round-trips and is readable as its raw spelling.
    let fragment = format!(r#"<a:prstGeom xmlns:a="{A}" prst="notAShape"/>"#);
    let (geom, doc): (PresetGeometry, _) = parse_typed(fragment.as_bytes());
    assert_eq!(geom.preset(&doc.interner), None);
    assert_eq!(geom.preset_token(&doc.interner), Some("notAShape"));
    assert_round_trips(&geom, doc, fragment.as_bytes());
}

#[test]
fn builds_prstgeom_with_one_guide() {
    let mut interner = Interner::new();
    let guide = GeometryGuide::new(&mut interner, "adj", "val 25000");
    let list = GeometryGuideList::new(&mut interner, vec![guide]);
    let geom = PresetGeometry::new(&mut interner, PresetShapeType::RoundedRectangle, Some(list));

    assert_eq!(
        serialize_built(interner, &geom),
        r#"<a:prstGeom prst="roundRect"><a:avLst><a:gd name="adj" fmla="val 25000"/></a:avLst></a:prstGeom>"#
    );
}

#[test]
fn built_prstgeom_reparses_to_the_same_values() {
    let mut interner = Interner::new();
    let guide = GeometryGuide::new(&mut interner, "adj", "val 20000");
    let list = GeometryGuideList::new(&mut interner, vec![guide]);
    let geom = PresetGeometry::new(&mut interner, PresetShapeType::Teardrop, Some(list));
    let serialized = serialize_built(interner, &geom);

    // Re-parse the built bytes (declaring the a-prefix so it resolves) and read back.
    let fragment = serialized.replacen("<a:prstGeom", &format!(r#"<a:prstGeom xmlns:a="{A}""#), 1);
    let (reparsed, doc): (PresetGeometry, _) = parse_typed(fragment.as_bytes());
    assert_eq!(
        reparsed.preset(&doc.interner),
        Some(PresetShapeType::Teardrop)
    );
    let guides: Vec<_> = reparsed.adjust_values().unwrap().guides().collect();
    assert_eq!(guides.len(), 1);
    assert_eq!(guides[0].name(&doc.interner), Some("adj"));
    assert_eq!(guides[0].formula(&doc.interner), Some("val 20000"));
}

#[test]
fn builds_self_closing_prstgeom_without_adjust_values() {
    let mut interner = Interner::new();
    let geom = PresetGeometry::new(&mut interner, PresetShapeType::Rectangle, None);
    assert_eq!(
        serialize_built(interner, &geom),
        r#"<a:prstGeom prst="rect"/>"#
    );
}

#[test]
fn guide_new_builds_a_self_closing_gd() {
    let mut interner = Interner::new();
    let guide = GeometryGuide::new(&mut interner, "adj1", "val 50000");
    assert_eq!(guide.name(&interner), Some("adj1"));
    assert_eq!(guide.formula(&interner), Some("val 50000"));
    assert_eq!(
        serialize_built(interner, &guide),
        r#"<a:gd name="adj1" fmla="val 50000"/>"#
    );
}

#[test]
fn set_preset_rewrites_only_the_prst_value() {
    let (mut geom, mut doc): (PresetGeometry, _) = parse_typed(PRSTGEOM);
    geom.set_preset(&mut doc.interner, PresetShapeType::Teardrop);
    // The preset changed; the avLst (and everything else) is untouched.
    assert_eq!(geom.preset(&doc.interner), Some(PresetShapeType::Teardrop));
    assert_eq!(geom.adjust_values().unwrap().guides().count(), 2);
    let expected =
        String::from_utf8_lossy(PRSTGEOM).replace(r#"prst="roundRect""#, r#"prst="teardrop""#);
    assert_round_trips(&geom, doc, expected.as_bytes());
}
