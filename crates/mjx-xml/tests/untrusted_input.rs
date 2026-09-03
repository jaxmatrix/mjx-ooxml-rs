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
