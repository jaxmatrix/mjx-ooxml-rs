//! PR 2b.1 exit proof: a real, in-context `p:txBody` from a slide round-trips byte-identically through
//! the typed model, and its text is readable.
//!
//! Opens `sample.pptx`, extracts `ppt/slides/slide1.xml`, replaces its `p:txBody` subtree with the
//! result of `TextBody::from_xml` → `to_xml`, and asserts the whole slide re-serializes to the exact
//! original bytes — a stronger check than a synthetic fragment because it exercises the real `p:`
//! wrapper prefix and the opaque `a:bodyPr`/`a:lstStyle` in place.

use std::path::PathBuf;

use mjx_dml::{TextBody, TextBodyContent};
use mjx_ooxml_core::{FromXml, RawDocument, RawElement, RawNode, ToXml};
use mjx_opc::{Package, PartName};
use mjx_xml::fidelity;

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()))
}

/// Depth-first search for the first element satisfying `predicate`, returning a mutable slot.
fn find_element_mut<'a>(
    element: &'a mut RawElement,
    predicate: &impl Fn(&RawElement) -> bool,
) -> Option<&'a mut RawElement> {
    if predicate(element) {
        return Some(element);
    }
    for child in &mut element.children {
        if let RawNode::Element(child_element) = child {
            if let Some(found) = find_element_mut(child_element, predicate) {
                return Some(found);
            }
        }
    }
    None
}

#[test]
fn txbody_round_trips_byte_identical_in_context() {
    let package = Package::open(&fixture("sample.pptx")).expect("open pptx");
    let slide = PartName::new("/ppt/slides/slide1.xml").expect("valid part name");
    let original = package
        .part_bytes(&slide)
        .expect("slide1.xml present")
        .to_vec();

    let mut doc = fidelity::parse(&original).expect("parse slide");

    // Split-borrow: `interner` (shared, for name resolution) vs `root` (mutable, to locate + replace).
    let RawDocument { interner, root, .. } = &mut doc;

    let is_txbody = |e: &RawElement| interner.resolve(e.name.local) == "txBody";
    let slot = find_element_mut(root, &is_txbody).expect("slide has a p:txBody");

    let body = TextBody::from_xml(slot, interner).expect("from_xml");

    // Structural assertions BEFORE replacing (anti-tautology): the model really parsed the body.
    assert_eq!(body.text(), "Hello OOXML");
    assert_eq!(body.paragraphs().count(), 1);
    assert_eq!(body.paragraphs().next().unwrap().runs().count(), 1);
    // content order: opaque bodyPr, opaque lstStyle, then the typed paragraph.
    assert_eq!(body.content().len(), 3);
    assert!(matches!(body.content()[0], TextBodyContent::Raw(_)));
    assert!(matches!(body.content()[2], TextBodyContent::Paragraph(_)));

    // Round-trip: rebuild the txBody in place, then re-serialize the WHOLE slide.
    *slot = body.to_xml(interner);
    let out = fidelity::serialize_to_vec(&doc);
    assert_eq!(
        String::from_utf8_lossy(&out),
        String::from_utf8_lossy(&original),
        "slide is not byte-identical after typed round-trip of its txBody"
    );
}
