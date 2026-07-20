//! Unit tests for the DrawingML text model, driven through the public API only.
//!
//! Fragments are parsed with the `mjx-xml` fidelity reader (declaring `xmlns:a` inline so namespaces
//! resolve), turned into typed values with `FromXml`, inspected, and rebuilt with `ToXml`. Every
//! round-trip assertion is paired with a **structural** assertion so byte-identity cannot pass by the
//! model silently dumping everything into the opaque `Raw` bucket.

use mjx_dml::{Paragraph, ParagraphContent, RunContent, Text, TextBody, TextBodyContent, TextRun};
use mjx_ooxml_core::{FromXml, FromXmlError, RawDocument, ToXml};
use mjx_xml::fidelity;

/// Parses a fragment and turns its root element into a typed `T`. Returns the value (which owns its
/// data — no borrow of the doc survives) alongside the still-usable document.
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

/// Rebuilds the root from `typed` and returns the serialized bytes as a lossy string (for inspecting
/// mutations, where the output is expected to differ from the input).
fn serialize_to_string<T: ToXml>(typed: &T, mut doc: RawDocument) -> String {
    doc.root = typed.to_xml(&mut doc.interner);
    String::from_utf8_lossy(&fidelity::serialize_to_vec(&doc)).into_owned()
}

const TXBODY: &[u8] = br#"<a:txBody xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>Hi</a:t></a:r></a:p></a:txBody>"#;

const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";

#[test]
fn parses_typed_structure() {
    let (body, _doc): (TextBody, _) = parse_typed(TXBODY);
    // bodyPr stays opaque; the list style and the paragraph are typed — and order is preserved.
    assert_eq!(body.content().len(), 3);
    assert!(matches!(body.content()[0], TextBodyContent::Raw(_)));
    assert!(matches!(body.content()[1], TextBodyContent::ListStyle(_)));
    assert!(body.list_style().is_some());
    let TextBodyContent::Paragraph(paragraph) = &body.content()[2] else {
        panic!("third child should be a typed paragraph");
    };
    assert_eq!(body.paragraphs().count(), 1);
    assert_eq!(paragraph.runs().count(), 1);
    assert_eq!(paragraph.runs().next().unwrap().text(), "Hi");
    assert_eq!(body.text(), "Hi");
}

#[test]
fn round_trips_fragment_byte_identical() {
    let (body, doc): (TextBody, _) = parse_typed(TXBODY);
    assert_eq!(body.text(), "Hi"); // structural pair
    assert_round_trips(&body, doc, TXBODY);
}

#[test]
fn preserves_self_closing_children_as_raw() {
    let (body, doc): (TextBody, _) = parse_typed(TXBODY);
    doc_serialize_contains(&body, doc, "<a:bodyPr/>");
    let (body, doc): (TextBody, _) = parse_typed(TXBODY);
    doc_serialize_contains(&body, doc, "<a:lstStyle/>");
}

#[track_caller]
fn doc_serialize_contains<T: ToXml>(typed: &T, mut doc: RawDocument, needle: &str) {
    doc.root = typed.to_xml(&mut doc.interner);
    let out = fidelity::serialize_to_vec(&doc);
    assert!(
        String::from_utf8_lossy(&out).contains(needle),
        "serialized output missing {needle:?}"
    );
}

#[test]
fn preserves_rpr_and_reads_text() {
    const RUN: &[u8] = br#"<a:r xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:rPr b="1"/><a:t>Bold</a:t></a:r>"#;
    let (run, doc): (TextRun, _) = parse_typed(RUN);
    assert_eq!(run.content().len(), 2);
    assert!(matches!(run.content()[0], RunContent::Properties(_))); // a:rPr is typed
    assert!(matches!(run.content()[1], RunContent::Text(_)));
    assert_eq!(run.text(), "Bold"); // text read past the rPr
    assert_eq!(
        run.properties()
            .expect("the run has properties")
            .is_bold(&doc.interner),
        Some(true)
    );
    assert_round_trips(&run, doc, RUN);
}

#[test]
fn amp_entity_byte_identical_and_decoded() {
    const T: &[u8] =
        br#"<a:t xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">a &amp; b</a:t>"#;
    let (text, doc): (Text, _) = parse_typed(T);
    assert_eq!(text.text(), "a & b"); // decoded
    assert_round_trips(&text, doc, T); // canonical `&amp;` survives byte-for-byte
}

#[test]
fn gt_entity_is_canonical_not_byte_identical() {
    const T: &[u8] =
        br#"<a:t xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">a &gt; b</a:t>"#;
    let (text, mut doc): (Text, _) = parse_typed(T);
    assert_eq!(text.text(), "a > b"); // decoded
    doc.root = text.to_xml(&mut doc.interner);
    let out = fidelity::serialize_to_vec(&doc);
    // minimal escaping leaves `>` literal, so `&gt;` does NOT round-trip byte-identically.
    assert_ne!(out.as_slice(), T);
    assert!(String::from_utf8_lossy(&out).contains("a > b"));
}

#[test]
fn lt_is_always_escaped() {
    const T: &[u8] =
        br#"<a:t xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">a &lt; b</a:t>"#;
    let (text, doc): (Text, _) = parse_typed(T);
    assert_eq!(text.text(), "a < b");
    // `<` must be re-escaped, so the canonical form round-trips exactly.
    assert_round_trips(&text, doc, T);
}

#[test]
fn empty_text_both_spellings() {
    const SELF_CLOSING: &[u8] =
        br#"<a:t xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"/>"#;
    const OPEN_CLOSE: &[u8] =
        br#"<a:t xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"></a:t>"#;
    let (text, doc): (Text, _) = parse_typed(SELF_CLOSING);
    assert_eq!(text.text(), "");
    assert_round_trips(&text, doc, SELF_CLOSING);
    let (text, doc): (Text, _) = parse_typed(OPEN_CLOSE);
    assert_eq!(text.text(), "");
    assert_round_trips(&text, doc, OPEN_CLOSE);
}

#[test]
fn multiple_paragraphs_and_runs() {
    const BODY: &[u8] = br#"<a:txBody xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:p><a:r><a:t>A</a:t></a:r><a:r><a:t>B</a:t></a:r></a:p><a:p><a:r><a:t>C</a:t></a:r></a:p></a:txBody>"#;
    let (body, doc): (TextBody, _) = parse_typed(BODY);
    assert_eq!(body.paragraphs().count(), 2);
    assert_eq!(body.paragraphs().next().unwrap().runs().count(), 2);
    assert_eq!(body.text(), "AB\nC"); // runs joined directly, paragraphs by newline
    assert_round_trips(&body, doc, BODY);
}

#[test]
fn strict_namespace_is_recognized() {
    // Same shape but under the STRICT DrawingML URI — typed nodes must still be built.
    const BODY: &[u8] = br#"<a:txBody xmlns:a="http://purl.oclc.org/ooxml/drawingml/main"><a:p><a:r><a:t>X</a:t></a:r></a:p></a:txBody>"#;
    let (body, doc): (TextBody, _) = parse_typed(BODY);
    assert_eq!(body.paragraphs().count(), 1);
    assert_eq!(body.text(), "X");
    assert_round_trips(&body, doc, BODY);
}

#[test]
fn unknown_child_preserved_as_raw() {
    const P: &[u8] = br#"<a:p xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:z="urn:z"><z:custom foo="1">x</z:custom><a:r><a:t>Y</a:t></a:r></a:p>"#;
    let (paragraph, doc): (Paragraph, _) = parse_typed(P);
    assert_eq!(paragraph.content().len(), 2);
    assert!(matches!(paragraph.content()[0], ParagraphContent::Raw(_))); // foreign element
    assert_eq!(paragraph.runs().count(), 1);
    assert_eq!(paragraph.text(), "Y");
    assert_round_trips(&paragraph, doc, P); // z:custom + foo + inner "x" all preserved
}

#[test]
fn xml_space_preserve_and_no_trimming() {
    const T: &[u8] = br#"<a:t xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xml:space="preserve">  spaced  </a:t>"#;
    let (text, doc): (Text, _) = parse_typed(T);
    assert_eq!(text.text(), "  spaced  "); // significant whitespace not trimmed
    assert!(
        text.attributes()
            .iter()
            .any(|attr| doc.interner.resolve(attr.name.local) == "space"),
        "xml:space attribute not preserved"
    );
    assert_round_trips(&text, doc, T);
}

#[test]
fn line_break_is_opaque_and_excluded_from_text() {
    const P: &[u8] = br#"<a:p xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:r><a:t>A</a:t></a:r><a:br/><a:r><a:t>B</a:t></a:r></a:p>"#;
    let (paragraph, doc): (Paragraph, _) = parse_typed(P);
    assert_eq!(paragraph.runs().count(), 2);
    assert_eq!(paragraph.text(), "AB"); // a:br yields no newline (it is opaque)
    assert_round_trips(&paragraph, doc, P);
}

#[test]
fn invalid_entity_surfaces_as_error() {
    // The fidelity reader accepts this (it never unescapes); the error surfaces in from_xml.
    const T: &[u8] = br#"<a:t xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">a &bogus; b</a:t>"#;
    let doc = fidelity::parse(T).expect("fidelity parse tolerates unknown entities");
    let result = Text::from_xml(&doc.root, &doc.interner);
    assert!(matches!(result, Err(FromXmlError::InvalidEntity(_))));
}

#[test]
fn wrapper_namespace_is_not_validated() {
    // The real-world case: a slide wraps CT_TextBody as `p:txBody` (presentationml), not `a:txBody`.
    const BODY: &[u8] = br#"<p:txBody xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:p><a:r><a:t>Z</a:t></a:r></a:p></p:txBody>"#;
    let (body, doc): (TextBody, _) = parse_typed(BODY);
    assert_eq!(body.text(), "Z");
    assert_eq!(body.paragraphs().count(), 1);
    assert_round_trips(&body, doc, BODY); // the p: prefix is preserved on the wrapper
}

#[test]
fn set_text_replaces_run_text() {
    let run_xml = format!(r#"<a:r xmlns:a="{A}"><a:t>Hi</a:t></a:r>"#).into_bytes();
    let (mut run, doc): (TextRun, _) = parse_typed(&run_xml);
    assert!(
        run.set_text("Bye"),
        "run has an a:t, so set_text should succeed"
    );
    assert_eq!(run.text(), "Bye"); // structural
    let out = serialize_to_string(&run, doc);
    assert!(out.contains("<a:t>Bye</a:t>"), "new text missing: {out}");
    assert!(!out.contains("Hi"), "old text should be gone: {out}");
}

#[test]
fn set_text_escapes_markup() {
    let run_xml = format!(r#"<a:r xmlns:a="{A}"><a:t>Hi</a:t></a:r>"#).into_bytes();
    let (mut run, doc): (TextRun, _) = parse_typed(&run_xml);
    run.set_text("a<b&c");
    let out = serialize_to_string(&run, doc);
    // `<` and `&` are re-escaped, so the output stays well-formed.
    assert!(out.contains("a&lt;b&amp;c"), "text not escaped: {out}");
}

#[test]
fn set_text_on_run_without_a_t_returns_false() {
    // A run with only an opaque rPr and no a:t.
    let run_xml = format!(r#"<a:r xmlns:a="{A}"><a:rPr b="1"/></a:r>"#).into_bytes();
    let (mut run, doc): (TextRun, _) = parse_typed(&run_xml);
    assert!(!run.set_text("Bye"), "a run with no a:t cannot set text");
    assert_eq!(run.text(), ""); // unchanged
    let out = serialize_to_string(&run, doc);
    assert!(
        !out.contains("Bye"),
        "nothing should have been written: {out}"
    );
    assert!(
        out.contains(r#"<a:rPr b="1"/>"#),
        "the rPr must survive: {out}"
    );
}

#[test]
fn runs_mut_targets_only_the_selected_run() {
    let para_xml =
        format!(r#"<a:p xmlns:a="{A}"><a:r><a:t>A</a:t></a:r><a:r><a:t>B</a:t></a:r></a:p>"#)
            .into_bytes();
    let (mut paragraph, doc): (Paragraph, _) = parse_typed(&para_xml);
    let second = paragraph.runs_mut().nth(1).expect("two runs present");
    assert!(second.set_text("X"));
    let texts: Vec<&str> = paragraph.runs().map(TextRun::text).collect();
    assert_eq!(texts, ["A", "X"]); // only the 2nd run changed
    let out = serialize_to_string(&paragraph, doc);
    assert!(
        out.contains("<a:t>A</a:t>") && out.contains("<a:t>X</a:t>"),
        "{out}"
    );
}

// ---------------------------------------------------------------------------------------------
// Splitting a run — the primitive behind formatting part of a paragraph
// ---------------------------------------------------------------------------------------------

const A_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";

#[test]
fn splitting_a_run_divides_its_text_and_copies_its_formatting() {
    let fragment =
        format!(r#"<a:r xmlns:a="{A_NS}"><a:rPr lang="en-US" b="1"/><a:t>Hello world</a:t></a:r>"#);
    let (mut run, doc): (TextRun, _) = parse_typed(fragment.as_bytes());

    let tail = run.split_at(6).expect("splits inside the text");
    assert_eq!(run.text(), "Hello ");
    assert_eq!(tail.text(), "world");
    assert_eq!(
        format!("{}{}", run.text(), tail.text()),
        "Hello world",
        "the halves must reconstitute the original"
    );

    // The tail carries the head's formatting, so splitting alone changes nothing visually.
    let head_properties = run.properties().expect("head keeps its properties");
    let tail_properties = tail.properties().expect("tail gets a copy");
    assert_eq!(head_properties, tail_properties);
    assert_eq!(tail_properties.is_bold(&doc.interner), Some(true));
    assert_eq!(tail_properties.language(&doc.interner), Some("en-US"));
}

#[test]
fn a_split_that_would_leave_a_side_empty_is_refused() {
    let fragment = format!(r#"<a:r xmlns:a="{A_NS}"><a:t>Hello</a:t></a:r>"#);
    for offset in [0, 5, 6, 99] {
        let (mut run, _doc): (TextRun, _) = parse_typed(fragment.as_bytes());
        assert!(
            run.split_at(offset).is_none(),
            "offset {offset} should be refused"
        );
        assert_eq!(run.text(), "Hello", "a refused split must change nothing");
    }
}

#[test]
fn splitting_counts_scalars_not_bytes() {
    // "café" is 5 bytes but 4 scalars; splitting at 3 must land before the é, not inside it.
    let fragment = format!(r#"<a:r xmlns:a="{A_NS}"><a:t>café au lait</a:t></a:r>"#);
    let (mut run, _doc): (TextRun, _) = parse_typed(fragment.as_bytes());
    let tail = run.split_at(3).expect("splits");
    assert_eq!(run.text(), "caf");
    assert_eq!(tail.text(), "é au lait");
}

#[test]
fn a_run_without_properties_splits_into_two_without_properties() {
    let fragment = format!(r#"<a:r xmlns:a="{A_NS}"><a:t>plain text</a:t></a:r>"#);
    let (mut run, _doc): (TextRun, _) = parse_typed(fragment.as_bytes());
    let tail = run.split_at(5).expect("splits");
    assert!(run.properties().is_none());
    assert!(
        tail.properties().is_none(),
        "nothing should be synthesized by a split"
    );
}

#[test]
fn splitting_a_run_leaves_the_paragraphs_other_children_in_place() {
    // A line break and a field sit between the runs; neither may move.
    let fragment = format!(
        concat!(
            r#"<a:p xmlns:a="{A}">"#,
            r#"<a:r><a:t>one two</a:t></a:r>"#,
            r#"<a:br/>"#,
            r#"<a:fld id="{{GUID}}" type="slidenum"><a:t>3</a:t></a:fld>"#,
            r#"<a:r><a:t>after</a:t></a:r>"#,
            r#"</a:p>"#
        ),
        A = A_NS
    );
    let (mut paragraph, mut doc): (Paragraph, _) = parse_typed(fragment.as_bytes());
    assert!(paragraph.split_run_at(0, 3), "run 0 splits at 3");

    let texts: Vec<String> = paragraph.runs().map(|r| r.text().to_owned()).collect();
    assert_eq!(texts, vec!["one", " two", "after"]);

    doc.root = paragraph.to_xml(&mut doc.interner);
    let out = String::from_utf8(fidelity::serialize_to_vec(&doc)).expect("utf-8");
    // The break and the field are still between the split run and the last one, in that order.
    let position = |needle: &str| {
        out.find(needle)
            .unwrap_or_else(|| panic!("{needle}: {out}"))
    };
    assert!(position("<a:t> two</a:t>") < position("<a:br/>"), "{out}");
    assert!(position("<a:br/>") < position("<a:fld"), "{out}");
    assert!(position("<a:fld") < position("<a:t>after</a:t>"), "{out}");
}

#[test]
fn splitting_a_run_that_does_not_exist_changes_nothing() {
    let fragment = format!(r#"<a:p xmlns:a="{A_NS}"><a:r><a:t>only</a:t></a:r></a:p>"#);
    let (mut paragraph, _doc): (Paragraph, _) = parse_typed(fragment.as_bytes());
    assert!(!paragraph.split_run_at(7, 2), "no such run");
    assert!(!paragraph.split_run_at(0, 0), "an empty head is refused");
    assert_eq!(paragraph.runs().count(), 1);
    assert_eq!(paragraph.text(), "only");
}
