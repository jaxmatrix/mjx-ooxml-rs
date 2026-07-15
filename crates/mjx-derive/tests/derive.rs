//! Isolated tests for `#[derive(FromXml, ToXml)]`, decoupled from `mjx-dml`.
//!
//! Tiny local types (non-`pub`, so `missing_docs` does not apply) exercise every codegen path — the
//! container match/recurse/Raw-fallthrough, the text-leaf decode/escape, the self-closing invariant,
//! both-URI matching, and error propagation. Fragments are parsed with the fidelity reader; because
//! `from_xml` never validates the element's own name, a local wrapper tag (`<demo>` / `<t>`) works.

// `FromXml`/`ToXml` are both a derive macro (macro namespace) and a trait (type namespace); importing
// both names from the two crates is the standard derive pattern and does not collide.
use mjx_derive::{FromXml, ToXml};
use mjx_ooxml_core::{FromXml, FromXmlError, RawAttribute, RawDocument, RawName, RawNode, ToXml};
use mjx_xml::fidelity;

const DML_TRANSITIONAL: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
const DML_STRICT: &str = "http://purl.oclc.org/ooxml/drawingml/main";

/// A container type: framework fields + an ordered content vec whose only typed child is `Leaf`.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_MAIN)]
struct Demo {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "p", variant = Item, ty = Leaf))]
    content: Vec<DemoContent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DemoContent {
    Item(Leaf),
    Raw(RawNode),
}

/// A text-leaf type.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
struct Leaf {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(text)]
    text: String,
}

fn parse_typed<T: FromXml>(fragment: &[u8]) -> (T, RawDocument) {
    let doc = fidelity::parse(fragment).expect("fragment parses");
    let typed = T::from_xml(&doc.root, &doc.interner).expect("from_xml succeeds");
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

#[test]
fn container_round_trips_typed_child_and_raw() {
    // `a:p` matches (DrawingML, local "p") → Item; the foreign `<other>` → Raw.
    const FRAG: &[u8] = br#"<demo xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:p>hi</a:p><other>x</other></demo>"#;
    let (demo, doc): (Demo, _) = parse_typed(FRAG);
    assert_eq!(demo.content.len(), 2);
    let DemoContent::Item(leaf) = &demo.content[0] else {
        panic!("first child should be a typed Item");
    };
    assert_eq!(leaf.text, "hi");
    assert!(matches!(demo.content[1], DemoContent::Raw(_)));
    assert_round_trips(&demo, doc, FRAG);
}

#[test]
fn text_leaf_decodes_and_reescapes() {
    const FRAG: &[u8] =
        br#"<t xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">a &amp; b</t>"#;
    let (leaf, doc): (Leaf, _) = parse_typed(FRAG);
    assert_eq!(leaf.text, "a & b"); // decoded
    assert_round_trips(&leaf, doc, FRAG); // canonical `&amp;` survives byte-for-byte
}

#[test]
fn text_leaf_empty_both_spellings() {
    const SELF_CLOSING: &[u8] =
        br#"<t xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"/>"#;
    const OPEN_CLOSE: &[u8] =
        br#"<t xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"></t>"#;
    let (leaf, doc): (Leaf, _) = parse_typed(SELF_CLOSING);
    assert_eq!(leaf.text, "");
    assert_round_trips(&leaf, doc, SELF_CLOSING); // <t/> stays self-closing
    let (leaf, doc): (Leaf, _) = parse_typed(OPEN_CLOSE);
    assert_eq!(leaf.text, "");
    assert_round_trips(&leaf, doc, OPEN_CLOSE); // <t></t> stays open/close
}

#[test]
fn both_strict_and_transitional_uris_match() {
    for uri in [DML_TRANSITIONAL, DML_STRICT] {
        let frag = format!(r#"<demo xmlns:a="{uri}"><a:p>y</a:p></demo>"#).into_bytes();
        let (demo, doc): (Demo, _) = parse_typed(&frag);
        assert_eq!(demo.content.len(), 1);
        assert!(
            matches!(demo.content[0], DemoContent::Item(_)),
            "child not typed under {uri}"
        );
        assert_round_trips(&demo, doc, &frag);
    }
}

#[test]
fn unknown_namespaced_child_preserved_as_raw() {
    const FRAG: &[u8] = br#"<demo xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:z="urn:z"><z:custom foo="1">x</z:custom><a:p>y</a:p></demo>"#;
    let (demo, doc): (Demo, _) = parse_typed(FRAG);
    assert_eq!(demo.content.len(), 2);
    assert!(matches!(demo.content[0], DemoContent::Raw(_))); // z:custom is foreign
    assert!(matches!(demo.content[1], DemoContent::Item(_)));
    assert_round_trips(&demo, doc, FRAG); // z:custom + foo + inner "x" all preserved
}

#[test]
fn invalid_entity_is_error() {
    const FRAG: &[u8] =
        br#"<t xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">a &bogus; b</t>"#;
    let doc = fidelity::parse(FRAG).expect("fidelity parse tolerates unknown entities");
    let result = Leaf::from_xml(&doc.root, &doc.interner);
    assert!(matches!(result, Err(FromXmlError::InvalidEntity(_))));
}
