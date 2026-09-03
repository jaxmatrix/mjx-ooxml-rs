//! Regression tests for defects the fuzz campaign found in the fidelity reader (MJXOFF-146).
//!
//! Each asserts the **property**, not the behaviour that happened to be there when the defect was
//! fixed. "This input returns `Err`" would pass just as well against a reader that returned the
//! wrong error, or that returned it for the wrong reason; the assertions below say what must be true
//! of every input, and name the input that proved it was not.

use mjx_xml::fidelity::{self, MAXIMUM_DEPTH};
use mjx_xml::XmlError;

/// `<a>` × `depth` then `</a>` × `depth`: legal, well-formed XML of any nesting.
fn nested(depth: usize) -> Vec<u8> {
    let mut xml = Vec::with_capacity(depth * 7);
    for _ in 0..depth {
        xml.extend_from_slice(b"<a>");
    }
    for _ in 0..depth {
        xml.extend_from_slice(b"</a>");
    }
    xml
}

/// **The invariant: no byte string produces a tree deeper than [`MAXIMUM_DEPTH`].**
///
/// Found by the campaign as a hard stack overflow — not a panic, an abort — because every walk over
/// the tree is recursive while the reader that builds it is not. `mjx_mce::resolve` died first, at a
/// depth reachable in about 140 KB of input, which is nothing for a document part.
///
/// The assertion is over the whole range rather than one input, because a limit that held at exactly
/// 1,000 and not at 1,001 would satisfy any single-input test and none of the property.
#[test]
fn no_input_can_nest_more_deeply_than_the_reader_permits() {
    for depth in [1, 2, 17, MAXIMUM_DEPTH - 1, MAXIMUM_DEPTH] {
        let document = fidelity::parse(&nested(depth))
            .unwrap_or_else(|e| panic!("depth {depth} is within the limit and must parse: {e}"));
        assert_eq!(
            measured_depth(&document),
            depth,
            "the reader built a different shape than the input describes at depth {depth}"
        );
    }
    for depth in [MAXIMUM_DEPTH + 1, MAXIMUM_DEPTH + 2, 20_000, 200_000] {
        match fidelity::parse(&nested(depth)) {
            Err(XmlError::DepthLimit { limit }) => assert_eq!(limit, MAXIMUM_DEPTH),
            Err(other) => panic!("depth {depth} was refused for the wrong reason: {other}"),
            Ok(_) => panic!(
                "depth {depth} built a tree; every walk over it recurses, so this is a stack \
                 overflow waiting for a consumer"
            ),
        }
    }
}

/// The limit refuses; it does not silently truncate. A reader that stopped at the limit and returned
/// the shallow prefix would pass a does-not-crash test and would have lost the document.
#[test]
fn what_the_reader_accepts_at_the_limit_still_round_trips_byte_for_byte() {
    let input = nested(MAXIMUM_DEPTH);
    let document = fidelity::parse(&input).expect("the limit itself is accepted");
    assert_eq!(
        fidelity::serialize_to_vec(&document),
        input,
        "a document at the depth limit must still come back byte-for-byte"
    );
}

/// The limit is a resource bound, not a well-formedness one, and says so with its own variant.
///
/// Folding it into [`XmlError::Syntax`] would tell a caller that a legal document was malformed,
/// which is the wrong thing to tell them and the wrong thing for them to log.
#[test]
fn the_depth_limit_is_its_own_error_and_not_a_syntax_error() {
    let error = fidelity::parse(&nested(MAXIMUM_DEPTH + 1)).expect_err("refused");
    assert!(
        matches!(error, XmlError::DepthLimit { .. }),
        "expected a depth-limit error, got {error}"
    );
    assert!(
        error.to_string().contains(&MAXIMUM_DEPTH.to_string()),
        "the message must name the limit so a caller can act on it: {error}"
    );
}

/// The maximum element depth of a document, measured iteratively — a recursive measurement would
/// overflow on exactly the inputs this file is about.
fn measured_depth(document: &mjx_ooxml_core::RawDocument) -> usize {
    let mut deepest = 0usize;
    let mut stack = vec![(&document.root, 1usize)];
    while let Some((element, depth)) = stack.pop() {
        deepest = deepest.max(depth);
        for child in element.children.iter() {
            if let mjx_ooxml_core::RawNode::Element(child) = child {
                stack.push((child, depth + 1));
            }
        }
    }
    deepest
}

/// **The invariant: a name the reader accepts is a name it can write back.**
///
/// `<a="x" b="y">` is not well-formed XML, and quick-xml scans its element name up to the first
/// whitespace — so the reader used to build an element literally named `a="x"`. Untouched it
/// serialized verbatim and nothing showed; *dirtied*, the writer put that name between `<` and `>`
/// and produced markup that will not parse. The campaign's dirtied-root oracle found it inside a
/// mutated `activex.pptx` part; this is the minimised form.
#[test]
fn a_name_that_could_not_be_written_back_is_refused_rather_than_accepted() {
    // `<a" b"c="1"/>` is the campaign's finding minimised from 463 bytes to thirteen. The quote in
    // the element's name is balanced by the one in `b"c`, so the *source* tag scans to its `>` and
    // the document parses. The reconstruction is where it breaks: `</a">` carries that quote alone,
    // so the end tag never finds its `>` and the output will not reparse. That is why the shorter
    // candidates below it are not enough on their own — each is refused by the tokenizer before the
    // reader ever sees it, and only this one reaches the defect.
    for hostile in [
        &b"<a\" b\"c=\"1\"/>"[..],
        b"<a=\"../x\" b=\"1\"/>",
        b"<a<b/>",
        b"<a&b/>",
        b"<a b=\"1\"><c=\"2\"/></a>",
    ] {
        match fidelity::parse(hostile) {
            Ok(document) => {
                // If it is accepted, the property that matters is that rewriting it still produces
                // the document it describes — which is the assertion, not "it returned Err".
                let mut document = document;
                document.root.empty = false;
                document
                    .root
                    .children
                    .push(mjx_ooxml_core::RawNode::Comment(Box::from(&b"x"[..])));
                let written = fidelity::serialize_to_vec(&document);
                fidelity::parse(&written).unwrap_or_else(|e| {
                    panic!(
                        "{:?} was accepted and then rewritten into markup that will not parse: {e}",
                        String::from_utf8_lossy(hostile)
                    )
                });
            }
            Err(XmlError::Syntax(_)) => {}
            Err(other) => panic!("refused for the wrong reason: {other}"),
        }
    }
}

/// A legal name is still accepted. Without this, the check above could be satisfied by refusing
/// everything, and the fix would have traded a crash for an inability to open files.
#[test]
fn the_name_check_refuses_only_what_cannot_be_written_back() {
    for legal in [
        &b"<a/>"[..],
        b"<a-b.c_d/>",
        b"<p:sld xmlns:p='urn:p'/>",
        b"<a x-y.z_1='v'/>",
        b"<\xc3\xa9l\xc3\xa9ment/>",
        b"<a b='c'></a>",
    ] {
        let document = fidelity::parse(legal).unwrap_or_else(|e| {
            panic!(
                "{:?} is legal markup and must still parse: {e}",
                String::from_utf8_lossy(legal)
            )
        });
        assert_eq!(fidelity::serialize_to_vec(&document), legal);
    }
}

/// **The invariant: a document type declaration the reader accepts is one the writer reproduces.**
///
/// The writer wraps a `DocType` node in the constant `<!DOCTYPE` … `>`, so a source that spells the
/// keyword any other way cannot come back. quick-xml accepts `<!DoCTYPE`, which XML 1.0 §2.8 does
/// not; sixteen bytes went in and fifteen came out, with the case silently changed as well. The
/// campaign found it through the round-trip oracle.
#[test]
fn a_doctype_the_writer_could_not_reproduce_is_refused() {
    for spelling in [
        &b"<!DoCTYPE a><a/>"[..],
        b"<!doctype a><a/>",
        b"<!DOCTYPe a><a/>",
    ] {
        let error = fidelity::parse(spelling).expect_err(&format!(
            "{:?} cannot be written back and must not be accepted",
            String::from_utf8_lossy(spelling)
        ));
        assert!(matches!(error, XmlError::Syntax(_)), "got {error}");
    }
    // And the spelling XML actually defines still round-trips, spaces and all.
    for legal in [
        &b"<!DOCTYPE a><a/>"[..],
        b"<!DOCTYPE   a   ><a/>",
        b"<!DOCTYPE a [ <!ENTITY x \"y\"> ]><a/>",
    ] {
        let document = fidelity::parse(legal)
            .unwrap_or_else(|e| panic!("{:?} must parse: {e}", String::from_utf8_lossy(legal)));
        assert_eq!(
            fidelity::serialize_to_vec(&document),
            legal,
            "{:?} did not round-trip",
            String::from_utf8_lossy(legal)
        );
    }
}
