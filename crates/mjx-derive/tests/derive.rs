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

// =================================================================================================
// Child order (MJXOFF-265). The census in `xtask/tests/child_order_census.rs` establishes that no
// hand-written serialization body in the workspace moves a child out of the order its file put it
// in; the derive is the mechanism the *rest* of the workspace reads through, and this is where its
// half of that claim is made.
//
// `unknown_namespaced_child_preserved_as_raw` above already places a foreign child before a typed
// one. What it does not do is present two *typed* children in an order the schema would not put
// them in, which is the shape MJXOFF-251 was: a writer that emits from the model in the order the
// model declares rather than the order the file held.
// =================================================================================================

/// A container with **two** typed children, so a fragment can be written whose element order is
/// deliberately the reverse of the one this type declares.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_MAIN)]
struct Ordered {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "a", variant = First, ty = Leaf),
        child(local = "b", variant = Second, ty = Leaf)
    )]
    content: Vec<OrderedContent>,
}

/// One child of [`Ordered`].
#[derive(Debug, Clone, PartialEq, Eq)]
enum OrderedContent {
    First(Leaf),
    Second(Leaf),
    Raw(RawNode),
}

/// Two typed children written in the reverse of the order the type declares them come back in the
/// order the *file* had, not the order the type declares.
#[test]
fn typed_children_come_back_in_the_files_order_not_the_types_order() {
    const FRAG: &[u8] = br#"<demo xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:b>second</a:b><a:a>first</a:a></demo>"#;
    let (ordered, doc): (Ordered, _) = parse_typed(FRAG);
    assert!(
        matches!(ordered.content[0], OrderedContent::Second(_))
            && matches!(ordered.content[1], OrderedContent::First(_)),
        "the content vector must hold the file's order, not the declaration's"
    );
    assert_round_trips(&ordered, doc, FRAG);
}

/// The half MJXOFF-251's own report did not reach, at the derive: **indentation is made of text
/// nodes, and a text node is a child.** A pretty-printed container whose elements are already in
/// declaration order still round-trips only if the whitespace between them keeps its place too.
#[test]
fn indentation_between_typed_children_keeps_its_place() {
    const FRAG: &[u8] = b"<demo xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\">\n  <a:a>first</a:a>\n  <a:b>second</a:b>\n</demo>";
    let (ordered, doc): (Ordered, _) = parse_typed(FRAG);
    assert_eq!(
        ordered.content.len(),
        5,
        "three whitespace text nodes and two elements — a text node is a child"
    );
    assert_round_trips(&ordered, doc, FRAG);
}

/// A foreign child *between* two typed ones stays between them, rather than being swept to either
/// end of the content vector.
#[test]
fn a_foreign_child_between_two_typed_ones_stays_between_them() {
    const FRAG: &[u8] = br#"<demo xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:z="urn:z"><a:a>first</a:a><z:custom/><a:b>second</a:b></demo>"#;
    let (ordered, doc): (Ordered, _) = parse_typed(FRAG);
    assert!(matches!(ordered.content[1], OrderedContent::Raw(_)));
    assert_round_trips(&ordered, doc, FRAG);
}
