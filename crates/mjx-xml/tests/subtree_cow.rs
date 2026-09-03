//! Subtree-granular copy-on-write: a parsed element remembers the byte range it came from, and the
//! serializer copies that range verbatim instead of reconstructing it.
//!
//! # Why the first test in this file is the namespace test
//!
//! A verbatim subtree is bytes, not markup — it carries prefixes (`a:off`) but not the `xmlns:a`
//! declaration that binds them, because that declaration lives on an ancestor. If an ancestor is
//! *rewritten* (it was edited, so its start tag is reconstructed from the model) and the
//! reconstruction drops or rewrites its namespace declarations, every verbatim descendant beneath it
//! silently loses its bindings: the file still looks plausible and every prefix in it is now
//! unbound. That is the one way subtree copy-on-write can corrupt a document that part-level
//! copy-on-write could not, so it is pinned before anything else here.
//!
//! The rule it pins: **a dirty element re-emits its namespace declarations, always.**

use std::sync::Arc;

use mjx_ooxml_core::{RawDocument, RawElement, RawNode};
use mjx_xml::fidelity;

/// A deliberately awkward part. Every property here exists to discriminate: an input whose start
/// tags are already one-line, single-quoted and comment-free would round-trip identically with or
/// without spans and would prove nothing.
///
/// * `p:sld`'s attributes are **wrapped across four lines** — the reflow this child exists to fix.
/// * `xmlns:a` and `xmlns:r` are declared **only on the root**, which the tests below then dirty, so
///   every descendant's prefix depends on a rewritten ancestor re-emitting them.
/// * `xmlns:r` uses **single quotes** and `a:off/@y` uses single quotes — quote style is a second
///   property reconstruction would have to carry separately.
/// * A **comment** and a **processing instruction** sit inside `p:cSld`; both must survive inside a
///   verbatim sibling subtree.
/// * `a:t` carries a **numeric character reference** (`&#38;`) where the writer would otherwise have
///   no reason to prefer it over `&amp;` — entity spelling is preserved only because the bytes are.
/// * `a:ext` is written `<a:ext …></a:ext>` while `a:off` is self-closing, and `a:off` has **two
///   spaces after its name** and a space before `/>`.
const SOURCE: &[u8] = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:p="urn:p"
       xmlns:a="urn:a"
       xmlns:r='urn:r'
       p:tag="keep">
  <p:cSld>
    <!-- a comment that must survive byte-for-byte -->
    <?office-hint value="1"?>
    <a:off  x="1"
            y='2' />
    <a:ext cx="3" cy="4"></a:ext>
    <a:t r:id='rId1'>Q &#38; A</a:t>
  </p:cSld>
  <p:clrMapOvr/>
</p:sld>
"#;

/// The exact bytes of the `<p:cSld>…</p:cSld>` subtree in [`SOURCE`], start tag through end tag.
fn source_slice(open: &str, close: &str) -> &'static [u8] {
    let text = std::str::from_utf8(SOURCE).expect("fixture is UTF-8");
    let start = text.find(open).expect("open tag in fixture");
    let end = text.find(close).expect("close tag in fixture") + close.len();
    &SOURCE[start..end]
}

fn parse() -> RawDocument {
    fidelity::parse(SOURCE).expect("fixture parses")
}

/// The first child element of `element` whose local name is `local`.
fn child<'a>(doc: &RawDocument, element: &'a RawElement, local: &str) -> &'a RawElement {
    element
        .children
        .iter()
        .find_map(|node| match node {
            RawNode::Element(e) if doc.interner.resolve(e.name.local) == local => Some(e),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no <{local}> child"))
}

/// Rewrites the value of the attribute with local name `local` on `element`, dirtying it.
fn set_attribute(doc: &mut RawDocument, path: &[&str], local: &str, value: &[u8]) {
    let RawDocument { interner, root, .. } = doc;
    let mut element: &mut RawElement = root;
    for step in path {
        let index = element
            .children
            .iter()
            .position(|node| match node {
                RawNode::Element(e) => interner.resolve(e.name.local) == *step,
                _ => false,
            })
            .unwrap_or_else(|| panic!("no <{step}> child"));
        element = match &mut element.children[index] {
            RawNode::Element(e) => e,
            _ => unreachable!("position matched an element"),
        };
    }
    let attribute = element
        .attributes
        .iter_mut()
        .find(|a| interner.resolve(a.name.local) == local)
        .unwrap_or_else(|| panic!("no @{local}"));
    attribute.value = value.into();
}

/// Walks a serialized document and returns the resolved namespace URI recorded for the first
/// element with the given local name — `None` if its prefix ended up unbound.
fn resolved_namespace(bytes: &[u8], local: &str) -> Option<String> {
    let doc = fidelity::parse(bytes).expect("output re-parses");
    fn walk(
        doc: &RawDocument,
        element: &RawElement,
        local: &str,
    ) -> Option<Option<mjx_ooxml_core::Symbol>> {
        if doc.interner.resolve(element.name.local) == local {
            return Some(element.name.namespace);
        }
        element.children.iter().find_map(|node| match node {
            RawNode::Element(e) => walk(doc, e, local),
            _ => None,
        })
    }
    let found = walk(&doc, &doc.root, local).expect("element present in output");
    found.map(|symbol| doc.interner.resolve(symbol).to_owned())
}

/// **The namespace-declaration hazard.** Dirtying the root rewrites its start tag from the model;
/// the `p:cSld` subtree underneath it is copied verbatim and still says `a:off`, `r:id`. If the
/// rewritten root stopped emitting `xmlns:a` / `xmlns:r`, those prefixes would be unbound and the
/// file would be silently broken. Nothing else in this suite fails if that regresses.
#[test]
fn a_dirty_ancestor_re_emits_the_namespace_declarations_its_verbatim_children_depend_on() {
    let mut doc = parse();
    // Dirty the root and *only* the root: rewrite one of its own attributes.
    set_attribute(&mut doc, &[], "tag", b"edited");
    let out = fidelity::serialize_to_vec(&doc);

    // The subtree beneath the rewritten root came through verbatim...
    let subtree = source_slice("<p:cSld>", "</p:cSld>");
    assert!(
        find(&out, subtree).is_some(),
        "the p:cSld subtree was not copied verbatim under a rewritten root:\n{}",
        String::from_utf8_lossy(&out)
    );

    // ...and every prefix it uses still resolves, because the rewritten root re-emitted the
    // declarations. This is the assertion the hazard is about: a namespace-resolving parse of the
    // OUTPUT, which is exactly what a consumer does.
    assert_eq!(
        resolved_namespace(&out, "off").as_deref(),
        Some("urn:a"),
        "a:off lost its xmlns:a binding when the root was rewritten:\n{}",
        String::from_utf8_lossy(&out)
    );
    assert_eq!(
        resolved_namespace(&out, "cSld").as_deref(),
        Some("urn:p"),
        "p:cSld lost its xmlns:p binding:\n{}",
        String::from_utf8_lossy(&out)
    );
    // The `r:` prefix is used only on an attribute inside the verbatim subtree; a writer that
    // pruned "unused" declarations while rewriting the root would drop exactly this one.
    let text = String::from_utf8_lossy(&out);
    assert!(
        text.contains("xmlns:r='urn:r'"),
        "the rewritten root dropped xmlns:r, which only the verbatim subtree uses:\n{text}"
    );
    // The edit itself landed.
    assert!(
        text.contains(r#"p:tag="edited""#),
        "the edit was lost:\n{text}"
    );
    assert!(
        !text.contains(r#"p:tag="keep""#),
        "the stale original value survived — a stale span was written:\n{text}"
    );
}

/// The same hazard one level down: dirty an *interior* element and its own verbatim children must
/// keep working, while the declarations still live on the (also rewritten) root above it.
#[test]
fn a_dirty_interior_element_keeps_its_verbatim_children_resolvable() {
    let mut doc = parse();
    set_attribute(&mut doc, &["cSld", "ext"], "cx", b"30");
    let out = fidelity::serialize_to_vec(&doc);
    let text = String::from_utf8_lossy(&out);

    // The root was never touched, so the whole document is still one verbatim copy except where the
    // edit forced a rewrite: the root, p:cSld and a:ext.
    assert!(
        find(&out, source_slice("<a:off", "/>")).is_some(),
        "the a:off sibling of the edited element reflowed:\n{text}"
    );
    assert_eq!(resolved_namespace(&out, "off").as_deref(), Some("urn:a"));
    assert_eq!(resolved_namespace(&out, "t").as_deref(), Some("urn:a"));
    assert!(text.contains(r#"cx="30""#), "the edit was lost:\n{text}");
}

/// An untouched document must come back byte-for-byte — wrapped attributes, mixed quotes, the
/// numeric character reference, the comment and the processing instruction included.
#[test]
fn an_untouched_document_round_trips_byte_for_byte() {
    let doc = parse();
    assert_eq!(
        String::from_utf8_lossy(&fidelity::serialize_to_vec(&doc)),
        String::from_utf8_lossy(SOURCE)
    );
}

/// Editing one attribute of one element leaves every *sibling* subtree byte-identical — including
/// the comment and the processing instruction, which are siblings of the edited element.
#[test]
fn editing_one_attribute_leaves_every_sibling_subtree_byte_identical() {
    let mut doc = parse();
    set_attribute(&mut doc, &["cSld", "off"], "x", b"11");
    let out = fidelity::serialize_to_vec(&doc);
    let text = String::from_utf8_lossy(&out);

    for sibling in [
        &b"<!-- a comment that must survive byte-for-byte -->"[..],
        &b"<?office-hint value=\"1\"?>"[..],
        &b"<a:ext cx=\"3\" cy=\"4\"></a:ext>"[..],
        &b"<a:t r:id='rId1'>Q &#38; A</a:t>"[..],
        &b"<p:clrMapOvr/>"[..],
    ] {
        assert!(
            find(&out, sibling).is_some(),
            "sibling {:?} was not byte-identical:\n{text}",
            String::from_utf8_lossy(sibling)
        );
    }
    assert!(text.contains(r#"x="11""#), "the edit was lost:\n{text}");
    // The edited element itself is the only thing reconstructed: its own wrapping is gone.
    assert!(
        find(&out, b"<a:off  x=\"11\"").is_none(),
        "the edited element kept a stale span:\n{text}"
    );
}

/// A span is only ever valid against the buffer it was measured from. Cloning a subtree — the way a
/// caller copies a shape from one part into another — must therefore drop it, or the clone would
/// re-emit bytes from a document it no longer belongs to.
#[test]
fn cloning_a_subtree_drops_its_span() {
    let doc = parse();
    let cloned = child(&doc, &doc.root, "cSld").clone();
    assert!(
        cloned.source_span().is_none(),
        "a cloned subtree kept a span into the document it was cloned out of"
    );
    fn assert_clean(element: &RawElement) {
        assert!(
            element.source_span().is_none(),
            "a cloned descendant kept a span"
        );
        for node in element.children.iter() {
            if let RawNode::Element(e) = node {
                assert_clean(e);
            }
        }
    }
    assert_clean(&cloned);
}

/// Two elements that are semantically equal compare equal whatever their spans, or every existing
/// equality assertion in the workspace would quietly change meaning.
#[test]
fn equality_ignores_the_span() {
    let parsed = parse();
    let from_source = child(&parsed, &parsed.root, "clrMapOvr").clone();
    let same_but_parsed = child(&parsed, &parsed.root, "clrMapOvr");
    assert!(
        same_but_parsed.source_span().is_some(),
        "fixture must have a span here"
    );
    assert!(from_source.source_span().is_none(), "the clone must not");
    assert_eq!(
        &from_source, same_but_parsed,
        "spans must not affect equality"
    );
}

/// A span read back from a document whose source was replaced under it must never be written, and
/// must never index out of bounds.
#[test]
fn a_document_without_its_source_reconstructs_instead_of_copying() {
    let mut doc = parse();
    doc.release_source();
    let out = fidelity::serialize_to_vec(&doc);
    // Reconstruction, not the original bytes: the wrapped start tag collapses onto one line.
    let text = String::from_utf8_lossy(&out);
    assert!(
        text.contains(r#"<p:sld xmlns:p="urn:p" xmlns:a="urn:a" xmlns:r='urn:r' p:tag="keep">"#),
        "expected a reconstruction once the source was released:\n{text}"
    );
    // ...and it is still a well-formed, namespace-correct document.
    assert_eq!(resolved_namespace(&out, "off").as_deref(), Some("urn:a"));
}

/// Spans are measured against the whole source buffer, byte-order mark included, so a part that
/// starts with one must not be off by three.
#[test]
fn spans_are_measured_past_a_byte_order_mark() {
    let mut with_bom = vec![0xEF, 0xBB, 0xBF];
    with_bom.extend_from_slice(SOURCE);
    let doc = fidelity::parse(&with_bom).expect("parses");
    assert_eq!(fidelity::serialize_to_vec(&doc), with_bom);
}

/// The source buffer may be handed in already shared, so a package that keeps the part's bytes and
/// the part's tree pays for one copy, not two.
#[test]
fn a_shared_source_buffer_is_not_copied() {
    let shared: Arc<[u8]> = Arc::from(SOURCE);
    let doc = fidelity::parse_shared(Arc::clone(&shared)).expect("parses");
    assert_eq!(
        Arc::strong_count(&shared),
        2,
        "the document should share the buffer"
    );
    assert_eq!(fidelity::serialize_to_vec(&doc), SOURCE);
}

/// Finds `needle` in `haystack`.
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    (0..=haystack.len() - needle.len()).find(|&i| &haystack[i..i + needle.len()] == needle)
}

// -------------------------------------------------------------------------------------------------
// Adversarial: a byte range is untrusted on the way out, not only on the way in
// -------------------------------------------------------------------------------------------------

/// Builds a one-element document whose root claims `span` into `source`, then serializes it.
///
/// [`RawElement::parsed`] is public because the reader lives in another crate, so a wrong range is
/// reachable — from a hostile file the reader mis-measures, or from a caller that simply gets it
/// wrong. Every case below must degrade to a reconstruction, never to a panic and never to bytes
/// lifted from somewhere else in the buffer.
fn serialize_with_span(
    source: &[u8],
    local: &str,
    prefix: Option<&str>,
    empty: bool,
    span: std::ops::Range<u32>,
) -> Vec<u8> {
    let mut interner = mjx_ooxml_core::Interner::new();
    let name = mjx_ooxml_core::RawName {
        prefix: prefix.map(|p| interner.intern(p)),
        local: interner.intern(local),
        namespace: None,
    };
    let root = RawElement::parsed(name, Vec::new(), Vec::new(), empty, span);
    let doc = RawDocument::parsed(
        interner,
        false,
        Vec::new(),
        root,
        Vec::new(),
        Arc::from(source),
    );
    fidelity::serialize_to_vec(&doc)
}

#[test]
fn a_span_past_the_end_of_the_buffer_reconstructs_instead_of_panicking() {
    assert_eq!(serialize_with_span(b"<a/>", "a", None, true, 0..4), b"<a/>");
    assert_eq!(
        serialize_with_span(b"<a/>", "a", None, true, 0..9_999),
        b"<a/>",
        "an out-of-bounds range must be ignored, not sliced"
    );
    assert_eq!(
        serialize_with_span(b"<a/>", "a", None, true, 9_000..9_001),
        b"<a/>"
    );
    assert_eq!(
        serialize_with_span(b"<a/>", "a", None, true, u32::MAX - 1..u32::MAX),
        b"<a/>"
    );
}

#[test]
fn an_inverted_or_empty_span_reconstructs_instead_of_panicking() {
    #[allow(clippy::reversed_empty_ranges)]
    let inverted = 3..1;
    assert_eq!(
        serialize_with_span(b"<a/>", "a", None, true, inverted),
        b"<a/>"
    );
    assert_eq!(serialize_with_span(b"<a/>", "a", None, true, 2..2), b"<a/>");
}

#[test]
fn a_span_pointing_at_a_different_element_is_refused() {
    // The range is in bounds and is a well-formed element — just not *this* element.
    let source = b"<a/><b/>";
    assert_eq!(
        serialize_with_span(source, "a", None, true, 4..8),
        b"<a/>",
        "a range naming another element must not be copied"
    );
}

/// The one that a naive `starts_with(name)` check would get wrong: `<a` is a prefix of `<abbr`.
#[test]
fn a_span_whose_name_is_a_prefix_of_the_source_name_is_refused() {
    assert_eq!(
        serialize_with_span(b"<abbr/>", "a", None, true, 0..7),
        b"<a/>",
        "<a> must not claim the range of <abbr>"
    );
    // And the genuine article still copies.
    assert_eq!(
        serialize_with_span(b"<abbr  />", "abbr", None, true, 0..9),
        b"<abbr  />"
    );
}

#[test]
fn a_span_whose_prefix_does_not_match_is_refused() {
    assert_eq!(
        serialize_with_span(b"<v:shape/>", "shape", Some("o"), true, 0..10),
        b"<o:shape/>"
    );
    assert_eq!(
        serialize_with_span(b"<v:shape/>", "shape", Some("v"), true, 0..10),
        b"<v:shape/>"
    );
}

#[test]
fn flipping_the_self_closing_flag_is_detected_without_any_mutation_tracking() {
    // `empty` and `name` are plain fields, so the serializer checks them against the bytes.
    assert_eq!(
        serialize_with_span(b"<a/>", "a", None, false, 0..4),
        b"<a></a>",
        "a range that self-closes cannot serve an element that says it does not"
    );
    assert_eq!(
        serialize_with_span(b"<a></a>", "a", None, true, 0..7),
        b"<a/>",
        "a range with an end tag cannot serve a self-closing element"
    );
}

#[test]
fn renaming_an_element_is_detected_without_any_mutation_tracking() {
    let mut doc = parse();
    let RawDocument { interner, root, .. } = &mut doc;
    let renamed = interner.intern("renamed");
    root.name.local = renamed;
    let out = fidelity::serialize_to_vec(&doc);
    let text = String::from_utf8_lossy(&out);
    assert!(
        text.starts_with("<?xml") && text.contains("<p:renamed"),
        "the rename was not written:\n{text}"
    );
    assert!(
        !text.contains("<p:sld "),
        "the stale span rewrote the old name:\n{text}"
    );
}

/// An end tag is allowed to carry whitespace (`</a >`), and a range that ends in one is still this
/// element's range.
#[test]
fn an_end_tag_with_whitespace_is_still_recognised() {
    assert_eq!(
        serialize_with_span(b"<a >x</a >", "a", None, false, 0..10),
        b"<a >x</a >"
    );
}

/// Whatever the input, parse-then-serialize must not panic, and whatever parses must come back
/// byte-for-byte. These are the shapes that stress span measurement specifically: markup inside
/// CDATA and comments, a name that is a prefix of a sibling's, empty text runs, nesting, and bytes
/// that are not XML at all.
#[test]
fn hostile_and_malformed_inputs_never_panic_and_never_lose_bytes() {
    let cases: &[&[u8]] = &[
        b"",
        b"<",
        b"<a",
        b"<a>",
        b"</a>",
        b"<a><b></a>",
        b"not xml at all",
        b"<a b=>",
        b"<a b='c>",
        b"<\xff\xfe/>",
        b"<a><![CDATA[</a><b/>]]></a>",
        b"<a><!-- </a><b/> --></a>",
        b"<a><?pi </a> ?></a>",
        b"<a/><!-- trailing -->",
        b"<a></a><b/>",
        b"<abbr><a/></abbr>",
        b"<a  \t\r\n b = 'c'  />",
        b"<a></a >",
        b"<a>&#38;&amp;&#x26;</a>",
        b"\xEF\xBB\xBF<a  x='1' />",
        b"<a><a><a><a><a/></a></a></a></a>",
        b"<!DOCTYPE a><a/>",
        b"<a xmlns='urn:x'><b:c xmlns:b='urn:b'/></a>",
        b"<a xmlns:b='urn:b'><b:c/></a>",
    ];
    for case in cases {
        // Rejecting malformed input is fine; panicking, or losing bytes, is not.
        if let Ok(doc) = fidelity::parse(case) {
            assert_eq!(
                fidelity::serialize_to_vec(&doc),
                *case,
                "input {:?} did not round-trip",
                String::from_utf8_lossy(case)
            );
        }
    }
}

/// The same corpus, but every document is dirtied at the root first: the serializer then mixes a
/// reconstructed start tag with verbatim children, which is where a bad range would show.
#[test]
fn dirtying_the_root_of_a_hostile_input_still_produces_the_document_it_describes() {
    let cases: &[&[u8]] = &[
        b"<a><![CDATA[</a><b/>]]></a>",
        b"<a><!-- </a><b/> --></a>",
        b"<abbr><a/></abbr>",
        b"<a xmlns:b='urn:b'><b:c d='1'/></a>",
        b"<a><a><a/></a></a>",
    ];
    for case in cases {
        let Ok(mut doc) = fidelity::parse(case) else {
            continue;
        };
        doc.root.empty = false;
        doc.root
            .children
            .push(RawNode::Comment(Box::from(&b"x"[..])));
        let out = fidelity::serialize_to_vec(&doc);
        // Whatever came out must itself be well-formed and describe the same tree.
        let reparsed = fidelity::parse(&out).unwrap_or_else(|e| {
            panic!(
                "{:?} produced unparseable output: {e}",
                String::from_utf8_lossy(case)
            )
        });
        assert_eq!(
            reparsed.root.children.len(),
            doc.root.children.len(),
            "child count changed for {:?}",
            String::from_utf8_lossy(case)
        );
    }
}

// ---------------------------------------------------------------------------------------------
// A typed model's round trip through the tree (MJXOFF-143)
// ---------------------------------------------------------------------------------------------

/// What a `FromXml` / `ToXml` pass leaves behind: the same markup, every source range gone, because
/// a model clones what it preserves and constructs what it models. `Clone` is the whole of it —
/// `mjx-xml` sits below every typed model, so this reproduces the shape rather than importing one.
fn as_a_typed_model_rebuilds_it(element: &RawElement) -> RawElement {
    let rebuilt = element.clone();
    assert_eq!(
        rebuilt.source_span(),
        None,
        "a clone must not carry a range — that is what makes this test worth writing"
    );
    rebuilt
}

/// A whole-part typed model reads a part into a value and writes the part back from it. Assigning
/// that value's element over the root throws away every range in the part, and the part comes back
/// re-flowed; `replace_preserving_verbatim_source` gives back the ranges of everything the rebuild
/// reproduced.
///
/// The fixture wraps `p:sld`'s attributes across four lines and quotes `xmlns:r` and `a:off/@y` with
/// apostrophes, so byte identity here cannot be an accident of reconstruction.
#[test]
fn a_whole_part_rebuild_that_changed_nothing_comes_back_byte_identical() {
    let mut doc = parse();
    let rebuilt = as_a_typed_model_rebuilds_it(&doc.root);

    // What the surfaces used to do.
    let mut reflowed = parse();
    reflowed.root = as_a_typed_model_rebuilds_it(&reflowed.root);
    let reflowed = fidelity::serialize_to_vec(&reflowed);
    assert_ne!(
        reflowed.as_slice(),
        SOURCE,
        "the fixture must be one a plain rebuild cannot reproduce, or this test proves nothing"
    );

    // What they do now.
    doc.root.replace_preserving_verbatim_source(rebuilt);
    let out = fidelity::serialize_to_vec(&doc);
    assert_eq!(
        String::from_utf8_lossy(&out),
        String::from_utf8_lossy(SOURCE),
        "a rebuild that changed nothing must re-emit the source"
    );
}

/// The same pass with one attribute changed: the element that changed and its ancestors are
/// reconstructed, and **nothing else is**.
#[test]
fn a_whole_part_rebuild_reconstructs_only_the_path_to_what_changed() {
    let mut doc = parse();
    let mut rebuilt = as_a_typed_model_rebuilds_it(&doc.root);

    // `p:sld > p:cSld > a:ext@cx`, three rungs down.
    let cs_ld = match &mut rebuilt.children[1] {
        RawNode::Element(element) => element,
        other => panic!("expected p:cSld, found {other:?}"),
    };
    let ext = cs_ld
        .children
        .iter_mut()
        .find_map(|node| match node {
            RawNode::Element(element) if doc.interner.resolve(element.name.local) == "ext" => {
                Some(element)
            }
            _ => None,
        })
        .expect("a:ext");
    ext.attributes[0].value = Box::from(&b"99"[..]);

    doc.root.replace_preserving_verbatim_source(rebuilt);
    let out = fidelity::serialize_to_vec(&doc);

    assert!(
        find(&out, b"cx=\"99\"").is_some(),
        "the edit did not land:\n{}",
        String::from_utf8_lossy(&out)
    );
    // Every sibling of the changed element kept its bytes — the comment, the processing
    // instruction, the two-spaces-after-the-name `a:off`, and the numeric character reference.
    for verbatim in [
        &b"<!-- a comment that must survive byte-for-byte -->"[..],
        &b"<?office-hint value=\"1\"?>"[..],
        &b"<a:off  x=\"1\"\n            y='2' />"[..],
        &b"<a:t r:id='rId1'>Q &#38; A</a:t>"[..],
        &b"<p:clrMapOvr/>"[..],
    ] {
        assert!(
            find(&out, verbatim).is_some(),
            "an untouched node was re-flowed:\n{}\n\nwanted verbatim:\n{}",
            String::from_utf8_lossy(&out),
            String::from_utf8_lossy(verbatim)
        );
    }
    // The two elements on the path to the change *are* rebuilt, so `p:sld`'s wrapped start tag is
    // now on one line — and it still carries every declaration its verbatim descendants need.
    assert!(
        find(
            &out,
            b"<p:sld xmlns:p=\"urn:p\" xmlns:a=\"urn:a\" xmlns:r='urn:r' p:tag=\"keep\">"
        )
        .is_some(),
        "the rewritten root lost its shape or its declarations:\n{}",
        String::from_utf8_lossy(&out)
    );
    // And the output is still the document it describes.
    assert_eq!(resolved_namespace(&out, "off").as_deref(), Some("urn:a"));
}

/// **A range that does not describe its element degrades to a reflow, never to wrong bytes.**
///
/// The restoration only ever moves a range from the element being replaced, so this feeds the writer
/// a wrong one directly: a `p:cSld` claiming the extent of `p:clrMapOvr`. The writer's checks — the
/// range must open with `<` and this element's qualified name — reject it, and the element is
/// reconstructed from the model instead.
#[test]
fn a_range_that_describes_a_different_element_is_refused_by_the_writer() {
    let mut doc = parse();
    let text = std::str::from_utf8(SOURCE).expect("fixture is UTF-8");
    let wrong_start =
        u32::try_from(text.find("<p:clrMapOvr/>").expect("in fixture")).expect("fits");

    let original = child(&doc, &doc.root, "cSld").clone();
    let span = child(&doc, &doc.root, "cSld")
        .source_span()
        .expect("p:cSld was parsed with a range");
    let RawDocument { root, .. } = &mut doc;
    let slot = root
        .children
        .iter_mut()
        .find_map(|node| match node {
            RawNode::Element(element) => Some(element),
            _ => None,
        })
        .expect("p:cSld");
    // The same length as the real range, at the wrong offset: it fits the buffer, so only the
    // qualified-name check can catch it.
    let wrong = RawElement::parsed(
        original.name,
        original.attributes.to_vec(),
        original.children.to_vec(),
        original.empty,
        wrong_start..wrong_start + (span.end - span.start),
    );
    assert!(
        wrong.source_span().is_some(),
        "the wrong range must actually be recorded, or this test proves nothing"
    );
    *slot = wrong;

    let out = fidelity::serialize_to_vec(&doc);
    let reparsed = fidelity::parse(&out).expect("the output is still well-formed XML");
    assert_eq!(
        reparsed.interner.resolve(reparsed.root.name.local),
        "sld",
        "the document must still be the document it was:\n{}",
        String::from_utf8_lossy(&out)
    );
    assert!(
        find(&out, b"<a:t r:id='rId1'>Q &#38; A</a:t>").is_some(),
        "the refused range should have reflowed p:cSld's start tag and no more:\n{}",
        String::from_utf8_lossy(&out)
    );
    assert!(
        find(&out, b"<p:clrMapOvr/></p:cSld>").is_none(),
        "the writer emitted the bytes the wrong range pointed at:\n{}",
        String::from_utf8_lossy(&out)
    );
}
