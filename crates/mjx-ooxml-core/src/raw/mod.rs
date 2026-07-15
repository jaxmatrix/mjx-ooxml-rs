//! The raw preservation tree: a lossless DOM that can reproduce a part's source bytes.
//!
//! Every distinct string (namespace URI, prefix, local name) is interned; attribute values, text,
//! and other content are stored as raw escaped bytes. A `Vec<RawNode>` is also the "unknown content
//! bucket" that future typed complex types carry to survive round-trips.

mod element;

pub use element::{QuoteStyle, RawAttribute, RawElement, RawName};

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
/// Owns its [`Interner`]; every [`RawName`] in the tree refers to this interner.
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_a_small_tree() {
        let mut interner = Interner::new();
        let name = RawName {
            prefix: None,
            local: interner.intern("root"),
            namespace: None,
        };
        let root = RawElement {
            name,
            attributes: Vec::new(),
            children: vec![RawNode::Text(Box::from(&b"hi"[..]))],
            empty: false,
        };
        let doc = RawDocument {
            interner,
            bom: false,
            prologue: Vec::new(),
            root,
            epilogue: Vec::new(),
        };
        assert_eq!(doc.interner.resolve(doc.root.name.local), "root");
        assert_eq!(doc.root.children.len(), 1);
    }
}
