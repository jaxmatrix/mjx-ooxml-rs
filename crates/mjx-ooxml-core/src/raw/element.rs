//! Element-level nodes of the preservation tree.
//!
//! All names are interned; attribute values and text are stored as **raw, escaped bytes** exactly as
//! they appeared in the source — never unescaped on read nor re-escaped on write. This is what makes
//! byte-identical round-trips possible.

use super::RawNode;
use crate::intern::Symbol;

/// The quote character an attribute value was written with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuoteStyle {
    /// `"…"` — what Office emits.
    Double,
    /// `'…'`.
    Single,
}

impl QuoteStyle {
    /// The quote byte (`"` or `'`).
    #[must_use]
    pub fn byte(self) -> u8 {
        match self {
            Self::Double => b'"',
            Self::Single => b'\'',
        }
    }
}

/// A qualified name. `prefix` preserves the literal source prefix for byte-fidelity; `namespace`
/// records the resolved URI for semantics (MCE, the future typed model). Both are interned; the
/// prefix→namespace redundancy is intentional and cheap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawName {
    /// The literal prefix as written (`w`, `p`, `mc`, `xmlns`), or `None` if unprefixed.
    pub prefix: Option<Symbol>,
    /// The local (unprefixed) name.
    pub local: Symbol,
    /// The resolved namespace URI, or `None` if the name is in no namespace.
    pub namespace: Option<Symbol>,
}

/// A single attribute, in document order. `xmlns` declarations are represented as attributes too
/// (e.g. `xmlns:w` → `prefix = "xmlns"`, `local = "w"`), preserving their exact position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawAttribute {
    /// The attribute name.
    pub name: RawName,
    /// The raw, escaped value bytes exactly as they appeared between the quotes.
    pub value: Box<[u8]>,
    /// The quote character used.
    pub quote: QuoteStyle,
}

/// An element and its ordered children.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawElement {
    /// The element name.
    pub name: RawName,
    /// Attributes in document order (including `xmlns` declarations).
    pub attributes: Vec<RawAttribute>,
    /// Child nodes in document order.
    pub children: Vec<RawNode>,
    /// Whether the element was written self-closing (`<a/>`). Invariant: if `true`, `children` is
    /// empty. A childless element with `empty == false` re-emits as `<a></a>`.
    pub empty: bool,
}
