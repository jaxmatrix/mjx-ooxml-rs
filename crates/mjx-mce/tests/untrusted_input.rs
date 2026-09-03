//! Regression tests for the defect the fuzz campaign found in MCE resolution (MJXOFF-146).
//!
//! Run the campaign that produced them with `cargo run -p xtask -- fuzz --target mce-resolve`.

use mjx_mce::{resolve, UnderstoodNamespaces};
use mjx_xml::fidelity::{self, MAXIMUM_DEPTH};

/// `<a>` × `depth` then `</a>` × `depth`.
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

/// **The invariant: no byte string can make resolution recurse past the reader's nesting bound.**
///
/// This is the defect that mattered most. `resolve_element` descends the tree, the tree came from
/// untrusted bytes, and nothing bounded it: about 140 KB of `<a>` — a trivial size for a document
/// part — overflowed the stack and aborted the process. Not a panic a caller could catch; an abort.
///
/// The fix is in `mjx-xml`, not here, because the tree is where the depth is decided and every walk
/// over it — this one, `Clone`, `Drop`, the serializer, and every model Phase C and D will add —
/// inherits the bound for free. So the property this file asserts is the one that protects *this*
/// crate: whatever the bytes say, `resolve` is never handed a tree deeper than it can walk.
///
/// The loop runs on a **2 MiB thread**, the default for a spawned thread, because that is the
/// configuration the bound was measured against; the main thread's 8 MiB would hide a regression in
/// the constant.
#[test]
fn resolution_survives_the_deepest_document_any_input_can_produce() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let understood = UnderstoodNamespaces::from_uris(["urn:a"]);
            for depth in [1, 16, MAXIMUM_DEPTH - 1, MAXIMUM_DEPTH] {
                let document = fidelity::parse(&nested(depth))
                    .unwrap_or_else(|e| panic!("depth {depth} is inside the limit: {e}"));
                let resolved = resolve(&document, &understood)
                    .unwrap_or_else(|e| panic!("depth {depth} must resolve: {e}"));
                assert_eq!(
                    resolved.children.len(),
                    usize::from(depth > 1),
                    "the resolved view lost the shape of a {depth}-deep document"
                );
            }
            // And the input that used to abort never reaches this crate at all.
            for depth in [MAXIMUM_DEPTH + 1, 20_000] {
                assert!(
                    fidelity::parse(&nested(depth)).is_err(),
                    "depth {depth} produced a tree; resolving it is a stack overflow"
                );
            }
        })
        .expect("spawn")
        .join()
        .expect("resolution must not overflow a 2 MiB stack at the reader's own limit");
}

/// Nested `mc:AlternateContent` is bounded by the same reader limit, and the deepest nesting the
/// reader will build is one resolution walks without trouble.
///
/// `AlternateContent` recursion goes through a second path — `resolve_alternate_content` calls back
/// into `resolve_child` — so a bound proved on plain elements is not automatically a bound on this.
#[test]
fn deeply_nested_alternate_content_resolves_rather_than_overflowing() {
    const MC: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";
    // Each level is three elements, so this reaches the reader's limit rather than a round number.
    let levels = (MAXIMUM_DEPTH - 2) / 3;
    let mut xml = format!(r#"<r xmlns:mc="{MC}" xmlns:n="urn:n">"#).into_bytes();
    for _ in 0..levels {
        xml.extend_from_slice(br#"<mc:AlternateContent><mc:Fallback>"#);
    }
    xml.extend_from_slice(b"<n:leaf/>");
    for _ in 0..levels {
        xml.extend_from_slice(br#"</mc:Fallback></mc:AlternateContent>"#);
    }
    xml.extend_from_slice(b"</r>");

    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || {
            let document = fidelity::parse(&xml).expect("inside the reader's limit");
            let resolved = resolve(&document, &UnderstoodNamespaces::new())
                .expect("a fallback chain resolves");
            assert_eq!(
                resolved.children.len(),
                1,
                "every fallback should have been flattened down to the one leaf"
            );
        })
        .expect("spawn")
        .join()
        .expect("nested AlternateContent must not overflow a 2 MiB stack");
}
