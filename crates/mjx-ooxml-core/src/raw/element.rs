//! Element-level nodes of the preservation tree.
//!
//! All names are interned; attribute values and text are stored as **raw, escaped bytes** exactly as
//! they appeared in the source — never unescaped on read nor re-escaped on write. This is what makes
//! byte-identical round-trips possible.
//!
//! [`RawElement`] documents the other half — subtree copy-on-write, and how a recorded byte range
//! stays sound while the tree around it is edited.

use std::num::NonZeroU32;
use std::ops::{Deref, DerefMut, Range};

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
///
/// Keeping them as ordinary attributes is what makes the namespace half of subtree copy-on-write
/// work: an element whose start tag is rewritten re-emits every attribute it holds, declarations
/// included, so a verbatim descendant never loses the binding its prefixes depend on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawAttribute {
    /// The attribute name.
    pub name: RawName,
    /// The raw, escaped value bytes exactly as they appeared between the quotes.
    pub value: Box<[u8]>,
    /// The quote character used.
    pub quote: QuoteStyle,
}

/// An element's attribute and child lists — everything about it a mutation changes.
///
/// [`RawElement`] dereferences to this, so `element.attributes` and `element.children` read exactly
/// as if they were its own fields. Reaching either of them **mutably** goes through
/// [`DerefMut`](RawElement), and that is what drops the element's verbatim
/// [source span](RawElement::source_span) — which is why they live here rather than on the element.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RawElementContent {
    /// Attributes in document order (including `xmlns` declarations).
    pub attributes: Vec<RawAttribute>,
    /// Child nodes in document order.
    pub children: Vec<RawNode>,
}

/// The byte range an element was parsed from, in its document's source buffer.
///
/// Stored as a start plus a [`NonZeroU32`] end rather than a [`Range<u32>`] so that
/// `Option<SourceSpan>` fits in 8 bytes: an element's range always ends past its `<`, so zero is
/// free to serve as `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourceSpan {
    start: u32,
    end: NonZeroU32,
}

/// An element and its ordered children.
///
/// Build one with [`RawElement::new`]. A byte-faithful reader uses [`RawElement::parsed`] instead,
/// recording the byte range the element came from so a serializer can copy it verbatim.
///
/// Its attribute and child lists live in a [`RawElementContent`] this type dereferences to, so
/// `element.attributes` and `element.children` work unchanged and taking either mutably drops the
/// recorded range.
///
/// # Subtree copy-on-write
///
/// A decomposed tree can only reproduce the properties it decided to record. It records names,
/// values, quote style, prefixes and self-closing style — but not the whitespace *between*
/// attributes, so a start tag Office wrapped across four lines re-emits on one. Recording that
/// whitespace would buy exactly one property, and the next one (entity spelling, comment placement)
/// would cost the same again.
///
/// Instead, an element parsed from a buffer remembers the byte range it came from, and a serializer
/// that still holds that buffer copies the range verbatim rather than descending into it. One field
/// subsumes the whole family, and it makes a lightly-edited part mostly `memcpy` — the copy-on-write
/// `mjx-opc` already does per part, at subtree granularity.
///
/// The span is only sound while the element still *is* what was parsed, so the invariant is enforced
/// rather than remembered:
///
/// * The attribute and child lists do not live on [`RawElement`] directly; they live in a
///   [`RawElementContent`] it [`Deref`]s to. Shared access is transparent, so `element.children` and
///   `element.attributes` read exactly as before — but reaching either of them *mutably* goes
///   through [`DerefMut`], which drops the element's span. Because mutable descent into a child goes
///   through every ancestor's child list, this drops the span on the whole path from the root, which
///   is what "a mutation clears the span on that node and every ancestor" means in practice.
/// * [`RawElement::name`] and [`RawElement::empty`] stay direct fields — they are read on every
///   navigation and must not cost a deref — and the serializer checks them against the recorded
///   bytes instead (the range must open with `<` + this element's qualified name and close the way
///   this element says it closes), which is exact and costs only the name's length.
/// * [`Clone`] drops the span, on the node and — because the clone recurses through this same impl —
///   on every descendant. A span means nothing against a different document's buffer, and a clone is
///   how a subtree travels between documents.
/// * The range is untrusted on the way *out* as well as in: a serializer must slice fallibly and
///   reconstruct if the range does not fit its buffer.
///
/// The whole mechanism costs **8 bytes per element**: the span packs into two `u32`s whose second
/// half is a [`NonZeroU32`], so `Option` needs no discriminant, and the lists moving behind a
/// `Deref` costs nothing at all.
pub struct RawElement {
    /// The element name.
    pub name: RawName,
    /// Whether the element was written self-closing (`<a/>`). Invariant: if `true`, `children` is
    /// empty. A childless element with `empty == false` re-emits as `<a></a>`.
    pub empty: bool,
    /// The attribute and child lists, reached through [`Deref`] / [`DerefMut`].
    content: RawElementContent,
    /// Where this element's bytes are, while it is still unmodified.
    ///
    /// Private because a wrong value here is the one way this design writes the wrong bytes: it is
    /// set only by [`Self::parsed`], read only through [`Self::source_span`], and dropped by
    /// [`Self::clear_source_span`], by [`DerefMut`] and by [`Clone`].
    source: Option<SourceSpan>,
}

impl RawElement {
    /// A newly authored element. It carries no source range, so it always serializes from the model.
    #[must_use]
    pub fn new(
        name: RawName,
        attributes: Vec<RawAttribute>,
        children: Vec<RawNode>,
        empty: bool,
    ) -> Self {
        Self {
            name,
            empty,
            content: RawElementContent {
                attributes,
                children,
            },
            source: None,
        }
    }

    /// An element a byte-faithful reader parsed from `source`, the byte range — `<` of the start tag
    /// through `>` of the end tag — it occupied in the document's source buffer.
    ///
    /// The range must be exactly that element's extent in exactly that buffer; an empty or inverted
    /// range is simply not recorded. A serializer checks what it can before trusting a range (that
    /// it fits, opens with `<` and this element's qualified name, and closes the way `empty` says it
    /// closes) and reconstructs the element otherwise, so a wrong range degrades to a reflow rather
    /// than to wrong bytes — but only a reader that measured the range itself should call this.
    #[must_use]
    pub fn parsed(
        name: RawName,
        attributes: Vec<RawAttribute>,
        children: Vec<RawNode>,
        empty: bool,
        source: Range<u32>,
    ) -> Self {
        let mut element = Self::new(name, attributes, children, empty);
        if source.start < source.end {
            if let Some(end) = NonZeroU32::new(source.end) {
                element.source = Some(SourceSpan {
                    start: source.start,
                    end,
                });
            }
        }
        element
    }

    /// The byte range this element may still be copied verbatim from, or `None` if it was authored,
    /// cloned, or has been reached mutably since it was parsed.
    ///
    /// The range is relative to [`RawDocument::source`](super::RawDocument::source) of the document
    /// this element belongs to, and means nothing against any other buffer.
    #[must_use]
    pub fn source_span(&self) -> Option<Range<u32>> {
        self.source.map(|span| span.start..span.end.get())
    }

    /// Drops this element's source range, so it serializes from the model.
    ///
    /// Only needed after mutating [`Self::name`] or [`Self::empty`], the two fields that are not
    /// behind the [`DerefMut`]; the serializer detects both anyway, so this is a tidiness
    /// affordance rather than a requirement.
    pub fn clear_source_span(&mut self) {
        self.source = None;
    }

    /// Consumes the element, yielding its attribute and child lists.
    #[must_use]
    pub fn into_content(self) -> RawElementContent {
        self.content
    }
}

impl Deref for RawElement {
    type Target = RawElementContent;

    fn deref(&self) -> &RawElementContent {
        &self.content
    }
}

impl DerefMut for RawElement {
    /// Hands out the attribute and child lists mutably — and drops the verbatim source range,
    /// because what is about to happen to them may be anything.
    fn deref_mut(&mut self) -> &mut RawElementContent {
        self.source = None;
        &mut self.content
    }
}

impl Clone for RawElement {
    /// Clones the element **without** its source range, here and — by recursing through this same
    /// impl for every child element — throughout the subtree.
    ///
    /// A range is meaningful only against the buffer it was measured from, and cloning is how a
    /// subtree leaves the document that owns that buffer.
    fn clone(&self) -> Self {
        Self {
            name: self.name,
            empty: self.empty,
            content: self.content.clone(),
            source: None,
        }
    }
}

impl PartialEq for RawElement {
    /// Compares markup, not provenance: two elements with the same name, attributes, children and
    /// self-closing style are equal whether or not either still remembers where it was parsed from.
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.empty == other.empty && self.content == other.content
    }
}

impl Eq for RawElement {}

impl std::fmt::Debug for RawElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RawElement")
            .field("name", &self.name)
            .field("attributes", &self.content.attributes)
            .field("children", &self.content.children)
            .field("empty", &self.empty)
            .field("source", &self.source_span())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The budget the design was chosen against: one span, eight bytes, nothing else.
    #[test]
    fn subtree_copy_on_write_costs_eight_bytes_per_element() {
        struct WithoutTheSpan {
            _name: RawName,
            _empty: bool,
            _content: RawElementContent,
        }
        assert_eq!(std::mem::size_of::<Option<SourceSpan>>(), 8);
        assert_eq!(
            std::mem::size_of::<RawElement>() - std::mem::size_of::<WithoutTheSpan>(),
            8
        );
    }

    #[test]
    fn an_empty_or_inverted_range_is_not_recorded() {
        let mut interner = crate::Interner::new();
        let name = RawName {
            prefix: None,
            local: interner.intern("a"),
            namespace: None,
        };
        assert_eq!(
            RawElement::parsed(name, Vec::new(), Vec::new(), true, 4..4).source_span(),
            None
        );
        // An inverted range: `#[allow]` because writing it down is the point of the test.
        #[allow(clippy::reversed_empty_ranges)]
        let inverted = 9..4;
        assert_eq!(
            RawElement::parsed(name, Vec::new(), Vec::new(), true, inverted).source_span(),
            None
        );
        assert_eq!(
            RawElement::parsed(name, Vec::new(), Vec::new(), true, 0..4).source_span(),
            Some(0..4)
        );
    }
}
