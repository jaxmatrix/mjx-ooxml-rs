//! MCE resolve/preserve tests over hand-crafted trees (parsed via mjx-xml's fidelity reader).

use mjx_mce::{resolve, ResolveError, ResolvedElement, ResolvedNode, UnderstoodNamespaces};
use mjx_ooxml_core::RawDocument;
use mjx_xml::fidelity;

const MC: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";

fn parse(xml: &str) -> RawDocument {
    fidelity::parse(xml.as_bytes()).expect("parse")
}

/// Local names of the element children of a resolved element, in order.
fn element_locals<'a>(el: &'a ResolvedElement<'a>, doc: &'a RawDocument) -> Vec<&'a str> {
    el.children
        .iter()
        .filter_map(|n| match n {
            ResolvedNode::Element(e) => Some(doc.interner.resolve(e.name().local)),
            _ => None,
        })
        .collect()
}

#[test]
fn choice_selected_when_requires_understood() {
    let xml = format!(
        r#"<root xmlns:mc="{MC}" xmlns:w14="urn:w14" xmlns:v="urn:v"><mc:AlternateContent><mc:Choice Requires="w14"><w14:new/></mc:Choice><mc:Fallback><v:old/></mc:Fallback></mc:AlternateContent></root>"#
    );
    let doc = parse(&xml);
    let understood = UnderstoodNamespaces::from_uris(["urn:w14"]);
    let root = resolve(&doc, &understood).unwrap();
    assert_eq!(element_locals(&root, &doc), ["new"]);
}

#[test]
fn fallback_selected_when_no_choice_matches() {
    let xml = format!(
        r#"<root xmlns:mc="{MC}" xmlns:w14="urn:w14" xmlns:v="urn:v"><mc:AlternateContent><mc:Choice Requires="w14"><w14:new/></mc:Choice><mc:Fallback><v:old/></mc:Fallback></mc:AlternateContent></root>"#
    );
    let doc = parse(&xml);
    let root = resolve(&doc, &UnderstoodNamespaces::new()).unwrap();
    assert_eq!(element_locals(&root, &doc), ["old"]);
}

#[test]
fn first_matching_choice_wins() {
    let xml = format!(
        r#"<root xmlns:mc="{MC}" xmlns:a="urn:a" xmlns:b="urn:b"><mc:AlternateContent><mc:Choice Requires="a"><a:first/></mc:Choice><mc:Choice Requires="b"><b:second/></mc:Choice></mc:AlternateContent></root>"#
    );
    let doc = parse(&xml);
    let understood = UnderstoodNamespaces::from_uris(["urn:a", "urn:b"]);
    let root = resolve(&doc, &understood).unwrap();
    assert_eq!(element_locals(&root, &doc), ["first"]);
}

#[test]
fn no_choice_no_fallback_resolves_to_nothing() {
    let xml = format!(
        r#"<root xmlns:mc="{MC}" xmlns:w14="urn:w14"><mc:AlternateContent><mc:Choice Requires="w14"><w14:new/></mc:Choice></mc:AlternateContent></root>"#
    );
    let doc = parse(&xml);
    let root = resolve(&doc, &UnderstoodNamespaces::new()).unwrap();
    assert!(element_locals(&root, &doc).is_empty());
}

#[test]
fn ignorable_drops_unknown_element_and_attribute() {
    let xml = format!(
        r#"<root xmlns:mc="{MC}" xmlns:x="urn:x" mc:Ignorable="x"><keep/><x:drop/><known x:attr="v" plain="p"/></root>"#
    );
    let doc = parse(&xml);

    // x is ignorable and not understood → x:drop removed, known/@x:attr removed.
    let root = resolve(&doc, &UnderstoodNamespaces::new()).unwrap();
    assert_eq!(element_locals(&root, &doc), ["keep", "known"]);
    let known = match &root.children[1] {
        ResolvedNode::Element(e) => e,
        other => panic!("expected element, got {other:?}"),
    };
    let attrs: Vec<&str> = known
        .attributes
        .iter()
        .map(|a| doc.interner.resolve(a.name.local))
        .collect();
    assert_eq!(attrs, ["plain"], "x:attr should be dropped, plain kept");

    // When x IS understood, nothing is ignored.
    let understood = UnderstoodNamespaces::from_uris(["urn:x"]);
    let root = resolve(&doc, &understood).unwrap();
    assert_eq!(element_locals(&root, &doc), ["keep", "drop", "known"]);
}

#[test]
fn process_content_hoists_children() {
    let xml = format!(
        r#"<root xmlns:mc="{MC}" xmlns:x="urn:x" mc:Ignorable="x" mc:ProcessContent="x"><x:wrapper><inner/></x:wrapper></root>"#
    );
    let doc = parse(&xml);
    let root = resolve(&doc, &UnderstoodNamespaces::new()).unwrap();
    // wrapper dropped, its <inner/> hoisted up.
    assert_eq!(element_locals(&root, &doc), ["inner"]);
}

#[test]
fn must_understand_errors_when_unknown() {
    let xml = format!(r#"<root xmlns:mc="{MC}" xmlns:x="urn:x" mc:MustUnderstand="x"><a/></root>"#);
    let doc = parse(&xml);
    match resolve(&doc, &UnderstoodNamespaces::new()) {
        Err(ResolveError::MustUnderstand(uri)) => assert_eq!(uri, "urn:x"),
        other => panic!("expected MustUnderstand error, got {other:?}"),
    }
    // Understood → ok.
    let understood = UnderstoodNamespaces::from_uris(["urn:x"]);
    assert!(resolve(&doc, &understood).is_ok());
}

#[test]
fn preserve_mode_round_trips_and_resolve_does_not_mutate() {
    let xml = format!(
        r#"<root xmlns:mc="{MC}" xmlns:w14="urn:w14" xmlns:v="urn:v"><mc:AlternateContent><mc:Choice Requires="w14"><w14:new/></mc:Choice><mc:Fallback><v:old/></mc:Fallback></mc:AlternateContent></root>"#
    );
    let doc = parse(&xml);

    // Preserve: the untouched tree serializes back byte-identically, mc:* intact.
    assert_eq!(fidelity::serialize_to_vec(&doc), xml.as_bytes());

    // Resolve is non-mutating: after resolving, the source still serializes byte-identically.
    let understood = UnderstoodNamespaces::from_uris(["urn:w14"]);
    let _ = resolve(&doc, &understood).unwrap();
    assert_eq!(fidelity::serialize_to_vec(&doc), xml.as_bytes());
}
