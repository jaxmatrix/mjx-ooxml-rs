//! A representative single-attribute edit, chosen generically rather than by format-specific
//! structural knowledge (MJXOFF-147) — used by the peak-RSS checkpoints in [`super::run_membench`].
//!
//! Each format crate's own criterion benchmarks carry an **independent copy** of this same logic
//! (`crates/mjx-pptx/benches/`, `.../mjx-docx/benches/`, `.../mjx-xlsx/benches/`): a bench target
//! cannot depend on this dev-only binary without a dependency cycle back onto the format crate
//! `xtask` itself depends on downward for the pptx corpus. The two copies are kept in step by
//! being this small and this generic — neither knows anything about `p:sp` vs `w:p` vs `<row>`.
//!
//! # Why the search is read-only
//!
//! `RawElement::DerefMut` drops that element's verbatim source span unconditionally, on the
//! reasoning that "what is about to happen to it may be anything" (see
//! `crates/mjx-ooxml-core/src/raw/element.rs`) — so a search that borrows `&mut` on every element
//! it merely *visits*, not just the one it edits, quietly invalidates every visited element's span,
//! whether or not it ends up being the target. On a 20,000-paragraph document, "find element
//! 10,000" pre-order visits the first ~10,000 — a search like that would turn a one-attribute edit
//! into one that (for span-preservation purposes) touched half the document, which is exactly the
//! averaging-away MJXOFF-147 warns against: a "lightly edited" save benchmark would silently become
//! a second "fully materialized" one. So [`find_nth_attributed_path`] is `&RawElement` throughout —
//! plain [`Deref`](std::ops::Deref) reads never touch the span — and only [`mutate_at`] borrows
//! `&mut`, and only on the handful of elements the returned path actually names.

use anyhow::{Context, Result};
use mjx_ooxml_core::{RawAttribute, RawDocument, RawElement, RawNode};

/// The path (child indices, root to parent of the target) to the element roughly in the middle of
/// the document's pre-order walk that carries at least one attribute — deep in the file and away
/// from both ends, the same target A7d's `mjx248_measure` harness picks by hand (the shape in the
/// middle of the part, not the first or the last). Computed once, read-only; see the module docs
/// for why a mutable search would be a different, wrong benchmark.
///
/// # Errors
/// Returns an error if no element in the document carries an attribute.
pub fn representative_path(doc: &RawDocument) -> Result<Vec<usize>> {
    let total = count_attributed(&doc.root);
    let mut remaining = total / 2;
    find_nth_attributed_path(&doc.root, &mut remaining)
        .context("no attributed element found for the representative edit")
}

/// Descends `path` (as returned by [`representative_path`]) and mutates that element's first
/// attribute. The only elements this clears the span of are the ones on `path` itself.
///
/// # Errors
/// Returns an error if `path` does not resolve (wrong document) or the resolved element carries no
/// attribute.
pub fn mutate_at(doc: &mut RawDocument, path: &[usize]) -> Result<()> {
    let mut element = &mut doc.root;
    for &index in path {
        let child = element
            .children
            .get_mut(index)
            .context("edit path index out of range")?;
        let RawNode::Element(next) = child else {
            anyhow::bail!("edit path index {index} does not name an element");
        };
        element = next;
    }
    mutate_first_attribute(
        element
            .attributes
            .first_mut()
            .context("the path's element carries no attribute")?,
    );
    Ok(())
}

/// The combination the peak-RSS checkpoints want in one call: find the path fresh, then edit it.
/// Criterion benchmarks that time repeated edits use [`representative_path`] once and
/// [`mutate_at`] per iteration instead, so the search cost is not folded into "the cost of an
/// edit" — see the module docs.
///
/// # Errors
/// Propagates [`representative_path`]'s and [`mutate_at`]'s.
pub fn representative_edit(doc: &mut RawDocument) -> Result<()> {
    let path = representative_path(doc)?;
    mutate_at(doc, &path)
}

fn mutate_first_attribute(attribute: &mut RawAttribute) {
    let mut value = attribute.value.to_vec();
    value.push(b'x');
    attribute.value = value.into_boxed_slice();
}

/// How many elements in this subtree carry at least one attribute. Read-only.
fn count_attributed(element: &RawElement) -> usize {
    let mut count = usize::from(!element.attributes.is_empty());
    for child in &element.children {
        if let RawNode::Element(child) = child {
            count += count_attributed(child);
        }
    }
    count
}

/// The child-index path (root to parent) to the `n`th element (pre-order, 0-based) that carries at
/// least one attribute. Read-only — see the module docs for why that matters here.
fn find_nth_attributed_path(element: &RawElement, n: &mut usize) -> Option<Vec<usize>> {
    if !element.attributes.is_empty() {
        if *n == 0 {
            return Some(Vec::new());
        }
        *n -= 1;
    }
    for (index, child) in element.children.iter().enumerate() {
        if let RawNode::Element(child) = child {
            if let Some(mut path) = find_nth_attributed_path(child, n) {
                path.insert(0, index);
                return Some(path);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use mjx_xml::fidelity;

    #[test]
    fn the_search_is_read_only_up_to_the_target() {
        // Fifteen siblings under the root, only the middle one carrying an attribute the search
        // must find by walking past several attribute-less ones first. Asserted directly on
        // `source_span()`, not inferred from re-serialized bytes: re-serializing an unchanged
        // element from the model produces byte-identical output to copying its span verbatim, so a
        // byte comparison cannot tell "span preserved" from "span cleared but rebuilt the same" —
        // exactly the gap that let the old, mutable-walk version of this search look correct while
        // clearing every visited sibling's span (this is the regression that test would have missed
        // too, so it asserts the field the fidelity engine itself keys on instead).
        let mut source = String::from("<a>");
        for i in 0..15 {
            if i == 7 {
                source.push_str("<b y=\"middle\"/>");
            } else {
                source.push_str("<b/>");
            }
        }
        source.push_str("</a>");
        let source = source.into_bytes();

        let mut doc = fidelity::parse(&source).expect("well-formed");
        let path = representative_path(&doc).expect("the middle `b` carries an attribute");
        assert_eq!(
            path,
            vec![7],
            "the search must land on sibling 7, not miscount"
        );
        mutate_at(&mut doc, &path).expect("the path resolves");

        for (index, node) in doc.root.children.iter().enumerate() {
            let RawNode::Element(element) = node else {
                panic!("every child here is a `<b>` element");
            };
            if index == 7 {
                assert!(
                    element.source_span().is_none(),
                    "the edited element's span must be cleared"
                );
            } else {
                assert!(
                    element.source_span().is_some(),
                    "sibling {index}, merely walked past on the way to sibling 7, must keep its \
                     verbatim span — a mutable search would have cleared it too"
                );
            }
        }
    }
}
