//! Shared helpers for `pptx_operations` (MJXOFF-147).
//!
//! **Independent copy, on purpose.** `xtask/src/corpus/edit.rs` carries the same
//! `representative_path` / `mutate_at` logic; a bench target here cannot depend on `xtask` (a
//! dev-only binary that itself depends downward on the format crates) without creating a
//! dependency cycle back onto this crate. Both copies are kept in step by being this small and
//! this generic — neither knows anything about `w:p` vs `<row>` vs `p:sp`.
//!
//! # Why the search is read-only, and the edit is computed once
//!
//! `RawElement::DerefMut` drops that element's verbatim source span unconditionally (see
//! `crates/mjx-ooxml-core/src/raw/element.rs`), so a search that borrows `&mut` on every element it
//! merely *visits* — not just the one it edits — quietly invalidates every visited element's span.
//! Each slide here is its own part (its own `RawDocument`, ~57 elements), so the effect is smaller
//! than on `mjx-docx`/`mjx-xlsx`'s one giant flat part — but a mutable search would still clear
//! several of that one slide's elements it walked past, and would still fold an O(n) search into
//! what is supposed to
//! [`representative_path`] is `&RawDocument` throughout (never touches a span) and is computed
//! **once**, outside any timed closure; [`mutate_at`] then does the one `&mut` descent the timed
//! routine actually needs, and only clears the span of elements on that path.
use std::path::PathBuf;

use mjx_ooxml_core::{RawAttribute, RawDocument, RawElement, RawNode};
use mjx_opc::{OpcError, PartName};

/// The number of slides `cargo run -p xtask -- corpus` builds `deck_large.pptx` with
/// (`xtask/src/corpus/pptx.rs::SLIDE_COUNT`) — kept in step by both being "300", not by sharing
/// code (see the module doc comment for why they cannot share code).
pub(crate) const SLIDE_COUNT: usize = 300;

/// The part name of the slide roughly in the middle of the generated deck. `next_slide_part`
/// numbers slides `1..=SLIDE_COUNT` in insertion order with no gaps (a blank deck starts with
/// none), so this is exactly `slide{index + 1}.xml` without needing to rebuild the deck to look it
/// up — see `xtask/src/corpus/pptx.rs::representative_slide_part` for the identical reasoning.
///
/// # Errors
/// Never, for this crate's `SLIDE_COUNT` — [`PartName::new`] only rejects a malformed name.
pub(crate) fn representative_slide_part() -> Result<PartName, OpcError> {
    let number = SLIDE_COUNT / 2 + 1;
    PartName::new(&format!("/ppt/slides/slide{number}.xml"))
}

/// The file `cargo run --release -p xtask -- corpus` writes for this format.
pub(crate) fn corpus_path(file_name: &str) -> PathBuf {
    mjx_fixtures::workspace_root()
        .join("target")
        .join("corpus")
        .join(file_name)
}

/// Reads a corpus file, panicking with the one command that fixes it if it is not there yet.
///
/// A benchmark is not `cargo test --workspace`: nothing here runs (or generates hundreds of
/// megabytes) on a normal test run, only on `cargo bench`, which is what makes an on-demand
/// generation step — rather than a lazily-generated one baked into every bench iteration — the
/// right shape (MJXOFF-147's "must not slow `cargo test --workspace`" constraint).
pub(crate) fn load_corpus(file_name: &str) -> Vec<u8> {
    let path = corpus_path(file_name);
    std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "reading {}: {e}\n\nGenerate the MJXOFF-147 corpus first:\n  \
             cargo run --release -p xtask -- corpus",
            path.display()
        )
    })
}

/// The path (child indices, root to parent of the target) to the element roughly in the middle of
/// the document's pre-order walk that carries at least one attribute — deep in the file and away
/// from both ends, the same target A7d's `mjx248_measure` harness picks by hand. Read-only; see the
/// module docs for why that matters.
///
/// # Panics
/// If no element in the document carries an attribute (never true for this crate's generated
/// corpus).
pub(crate) fn representative_path(doc: &RawDocument) -> Vec<usize> {
    let total = count_attributed(&doc.root);
    let mut remaining = total / 2;
    find_nth_attributed_path(&doc.root, &mut remaining)
        .expect("the generated corpus always has an attributed element")
}

/// Descends `path` (from [`representative_path`]) and mutates that element's first attribute. The
/// only elements this clears the span of are the ones on `path` itself.
///
/// # Panics
/// If `path` does not resolve, or the resolved element carries no attribute.
pub(crate) fn mutate_at(doc: &mut RawDocument, path: &[usize]) {
    let mut element = &mut doc.root;
    for &index in path {
        let RawNode::Element(next) = &mut element.children[index] else {
            panic!("edit path index {index} does not name an element");
        };
        element = next;
    }
    mutate_first_attribute(
        element
            .attributes
            .first_mut()
            .expect("the path's element carries an attribute"),
    );
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
/// least one attribute. Read-only.
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
