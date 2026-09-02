//! The raw preservation tree: a lossless DOM that can reproduce a part's source bytes.
//!
//! Every distinct string (namespace URI, prefix, local name) is interned; attribute values, text,
//! and other content are stored as raw escaped bytes. A `Vec<RawNode>` is also the "unknown content
//! bucket" that future typed complex types carry to survive round-trips.
//!
//! A document parsed from bytes also *keeps* those bytes, so any subtree still in the state it was
//! parsed in can be written by copying its byte range instead of being rebuilt from the model. See
//! [`RawElement`] for how that stays sound.

mod element;

pub use element::{QuoteStyle, RawAttribute, RawElement, RawElementContent, RawName};

use std::sync::Arc;

use crate::intern::Interner;

/// A node in the preservation tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RawNode {
    /// An element subtree.
    Element(RawElement),
    /// Character data (raw, escaped, verbatim — includes significant whitespace).
    Text(Box<[u8]>),
    /// The inner bytes of a `<![CDATA[ … ]]>` section.
    CData(Box<[u8]>),
    /// The inner bytes of a `<!-- … -->` comment.
    Comment(Box<[u8]>),
    /// The inner bytes of a `<? … ?>` processing instruction.
    ProcessingInstruction(Box<[u8]>),
    /// The inner bytes of the `<?xml … ?>` declaration (prologue only).
    Declaration(Box<[u8]>),
    /// The inner bytes of a `<!DOCTYPE … >` (rare in OOXML).
    DocType(Box<[u8]>),
}

/// A fully parsed part: everything needed to reproduce the source byte-for-byte.
///
/// Owns its [`Interner`]; every [`RawName`] in the tree refers to this interner — and, when the
/// document was parsed rather than built, its source buffer, which every unmodified element's
/// [`RawElement::source_span`] indexes into. Both are document-relative: a node moved between
/// documents resolves its names against the wrong interner and its span against the wrong buffer,
/// which is why [`RawElement`]'s [`Clone`] drops the span.
#[derive(Debug)]
pub struct RawDocument {
    /// The string interner backing every name in this document.
    pub interner: Interner,
    /// Whether the source began with a UTF-8 byte-order mark.
    pub bom: bool,
    /// Nodes before the root element (declaration, whitespace, comments, PIs, doctype), in order.
    pub prologue: Vec<RawNode>,
    /// The document's root element.
    pub root: RawElement,
    /// Nodes after the root element (trailing whitespace, comments, PIs), in order.
    pub epilogue: Vec<RawNode>,
    /// The bytes this document was parsed from, shared with whoever else retains them.
    ///
    /// Private: a document handed a buffer its spans were not measured against would write another
    /// document's markup. It is set only by [`Self::parsed`] and dropped by [`Self::release_source`].
    source: Option<Arc<[u8]>>,
}

impl RawDocument {
    /// A document built from the model, with no source bytes behind it. Every element serializes
    /// from the model.
    #[must_use]
    pub fn new(
        interner: Interner,
        bom: bool,
        prologue: Vec<RawNode>,
        root: RawElement,
        epilogue: Vec<RawNode>,
    ) -> Self {
        Self {
            interner,
            bom,
            prologue,
            root,
            epilogue,
            source: None,
        }
    }

    /// A document a byte-faithful reader parsed out of `source`, retaining that buffer so
    /// unmodified subtrees can be copied from it.
    ///
    /// Every [`RawElement::parsed`] span in `root` must have been measured against exactly these
    /// bytes, offsets included — a byte-order mark is part of the buffer, not stripped from it.
    #[must_use]
    pub fn parsed(
        interner: Interner,
        bom: bool,
        prologue: Vec<RawNode>,
        root: RawElement,
        epilogue: Vec<RawNode>,
        source: Arc<[u8]>,
    ) -> Self {
        Self {
            interner,
            bom,
            prologue,
            root,
            epilogue,
            source: Some(source),
        }
    }

    /// The bytes this document was parsed from, if it still holds them.
    #[must_use]
    pub fn source(&self) -> Option<&[u8]> {
        self.source.as_deref()
    }

    /// The source buffer itself, for a caller that wants to retain it without a second copy.
    #[must_use]
    pub fn shared_source(&self) -> Option<&Arc<[u8]>> {
        self.source.as_ref()
    }

    /// Drops the source buffer. Every element then serializes from the model, which is correct but
    /// reflows whatever whitespace only the bytes remembered.
    pub fn release_source(&mut self) {
        self.source = None;
    }

    /// Drops the source buffer **if no element can still be copied from it**, and reports whether it
    /// did.
    ///
    /// This is the memory half of subtree copy-on-write: a part every element of which has now been
    /// rewritten keeps its bytes alive for nothing. Walks the tree, so call it where a walk is
    /// already being paid for — after serializing, not after every edit.
    pub fn release_unused_source(&mut self) -> bool {
        if self.source.is_none() {
            return false;
        }
        if has_live_span(&self.root) {
            return false;
        }
        self.source = None;
        true
    }
}

/// Whether `element` or any descendant can still be written from the document's source buffer.
fn has_live_span(element: &RawElement) -> bool {
    if element.source_span().is_some() {
        return true;
    }
    element.children.iter().any(|node| match node {
        RawNode::Element(child) => has_live_span(child),
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(interner: &mut Interner, local: &str) -> RawName {
        RawName {
            prefix: None,
            local: interner.intern(local),
            namespace: None,
        }
    }

    #[test]
    fn builds_a_small_tree() {
        let mut interner = Interner::new();
        let name = name(&mut interner, "root");
        let root = RawElement::new(
            name,
            Vec::new(),
            vec![RawNode::Text(Box::from(&b"hi"[..]))],
            false,
        );
        let doc = RawDocument::new(interner, false, Vec::new(), root, Vec::new());
        assert_eq!(doc.interner.resolve(doc.root.name.local), "root");
        assert_eq!(doc.root.children.len(), 1);
        assert!(doc.source().is_none());
        assert!(doc.root.source_span().is_none());
    }

    #[test]
    fn mutating_a_child_list_invalidates_the_span() {
        let mut interner = Interner::new();
        let name = name(&mut interner, "a");
        let mut root = RawElement::parsed(name, Vec::new(), Vec::new(), true, 0..3);
        assert_eq!(root.source_span(), Some(0..3));
        root.children.push(RawNode::Text(Box::from(&b"x"[..])));
        assert_eq!(root.source_span(), None);
    }

    #[test]
    fn mutating_an_attribute_list_invalidates_the_span() {
        let mut interner = Interner::new();
        let name = name(&mut interner, "a");
        let mut root = RawElement::parsed(name, Vec::new(), Vec::new(), true, 0..3);
        root.attributes.push(RawAttribute {
            name,
            value: Box::from(&b"1"[..]),
            quote: QuoteStyle::Double,
        });
        assert_eq!(root.source_span(), None);
    }

    #[test]
    fn descending_mutably_invalidates_every_ancestor() {
        let mut interner = Interner::new();
        let leaf_name = name(&mut interner, "leaf");
        let mid_name = name(&mut interner, "mid");
        let root_name = name(&mut interner, "root");
        let leaf = RawElement::parsed(leaf_name, Vec::new(), Vec::new(), true, 12..19);
        let mid = RawElement::parsed(
            mid_name,
            Vec::new(),
            vec![RawNode::Element(leaf)],
            false,
            6..30,
        );
        let mut root = RawElement::parsed(
            root_name,
            Vec::new(),
            vec![RawNode::Element(mid)],
            false,
            0..40,
        );

        // Descend the way a caller does: through each ancestor's child list.
        let RawNode::Element(mid) = &mut root.children[0] else {
            panic!("expected an element");
        };
        let RawNode::Element(leaf) = &mut mid.children[0] else {
            panic!("expected an element");
        };
        leaf.attributes.push(RawAttribute {
            name: leaf_name,
            value: Box::from(&b"1"[..]),
            quote: QuoteStyle::Double,
        });

        assert_eq!(root.source_span(), None, "the root is on the mutated path");
        let RawNode::Element(mid) = &root.children[0] else {
            panic!("expected an element");
        };
        assert_eq!(mid.source_span(), None, "the interior node is on the path");
        let RawNode::Element(leaf) = &mid.children[0] else {
            panic!("expected an element");
        };
        assert_eq!(leaf.source_span(), None, "the mutated node itself");
    }

    #[test]
    fn a_verbatim_sibling_survives_a_mutated_one() {
        let mut interner = Interner::new();
        let kept_name = name(&mut interner, "kept");
        let edited_name = name(&mut interner, "edited");
        let root_name = name(&mut interner, "root");
        let mut root = RawElement::parsed(
            root_name,
            Vec::new(),
            vec![
                RawNode::Element(RawElement::parsed(
                    kept_name,
                    Vec::new(),
                    Vec::new(),
                    true,
                    6..15,
                )),
                RawNode::Element(RawElement::parsed(
                    edited_name,
                    Vec::new(),
                    Vec::new(),
                    true,
                    15..26,
                )),
            ],
            false,
            0..35,
        );
        let RawNode::Element(edited) = &mut root.children[1] else {
            panic!("expected an element");
        };
        edited.empty = false;
        edited.clear_source_span();
        let RawNode::Element(kept) = &root.children[0] else {
            panic!("expected an element");
        };
        assert_eq!(
            kept.source_span(),
            Some(6..15),
            "the untouched sibling kept its span"
        );
    }

    #[test]
    fn release_unused_source_keeps_a_buffer_that_is_still_in_use() {
        let mut interner = Interner::new();
        let root_name = name(&mut interner, "a");
        let root = RawElement::parsed(root_name, Vec::new(), Vec::new(), true, 0..4);
        let mut doc = RawDocument::parsed(
            interner,
            false,
            Vec::new(),
            root,
            Vec::new(),
            Arc::from(&b"<a/>"[..]),
        );
        assert!(!doc.release_unused_source());
        assert!(doc.source().is_some());

        doc.root.clear_source_span();
        assert!(doc.release_unused_source());
        assert!(doc.source().is_none());
        assert!(!doc.release_unused_source(), "releasing twice is a no-op");
    }

    #[test]
    fn release_unused_source_looks_at_descendants_too() {
        let mut interner = Interner::new();
        let kid_name = name(&mut interner, "b");
        let root_name = name(&mut interner, "a");
        let kid = RawElement::parsed(kid_name, Vec::new(), Vec::new(), true, 3..7);
        let mut root = RawElement::parsed(
            root_name,
            Vec::new(),
            vec![RawNode::Element(kid)],
            false,
            0..11,
        );
        root.clear_source_span();
        let mut doc = RawDocument::parsed(
            interner,
            false,
            Vec::new(),
            root,
            Vec::new(),
            Arc::from(&b"<a><b/></a>"[..]),
        );
        assert!(
            !doc.release_unused_source(),
            "a descendant can still be copied from the buffer"
        );
    }
}
