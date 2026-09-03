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
/// Those rules make a span *travel* nowhere by default, which costs one thing: a typed model is a
/// view, so a `FromXml` / `ToXml` pass rebuilds every element it looked at and the whole part
/// re-flows even where nothing changed. [`replace_preserving_verbatim_source`] closes that without
/// loosening any of the above — it restores a range only onto a node that compares equal to the one
/// it replaces, at the position it replaces it, so both the "same markup" and the "same buffer"
/// halves of the invariant are re-established rather than assumed.
///
/// [`replace_preserving_verbatim_source`]: RawElement::replace_preserving_verbatim_source
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

    /// The element a typed model rebuilds when it serializes itself — **the one construction point
    /// every `ToXml` goes through**.
    ///
    /// Identical to [`new`](Self::new) today, and deliberately a separate name: a rebuilt element is
    /// not the same thing as a newly authored one. It is a model re-stating an element that already
    /// existed in a document, so it is the only construction that could ever carry the original's
    /// verbatim [source range](Self::source_span) forward. Nothing does that yet — every rebuild
    /// serializes from the model — but when something does, it changes here, once, rather than at
    /// every `to_xml` in the workspace.
    ///
    /// Use [`new`](Self::new) for an element the program invented, which has no original to carry
    /// anything forward from.
    #[must_use]
    pub fn rebuilt(
        name: RawName,
        attributes: Vec<RawAttribute>,
        children: Vec<RawNode>,
        empty: bool,
    ) -> Self {
        Self::new(name, attributes, children, empty)
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

    /// Overwrites this element with `rebuilt` — what a typed model made of it — and gives back the
    /// verbatim [source range](Self::source_span) of every node the rebuild reproduced unchanged.
    ///
    /// This is the whole-subtree counterpart of [`DerefMut`](RawElement) dropping a span: a typed
    /// model is a *view*, so a `FromXml` / `ToXml` pass rebuilds every element it looked at, and
    /// nearly all of them come back byte-for-byte the same. Assigning the rebuild over the original
    /// (`*slot = value.to_xml(interner)`) throws away the range each of those could still have been
    /// copied from, and the part re-flows. Doing it through here does not.
    ///
    /// # Why this may set a range, when nothing else outside [`parsed`](Self::parsed) may
    ///
    /// A range is a claim that a stretch of some buffer *is* an element's markup, and a wrong claim
    /// is the one way this design writes the wrong bytes. Two facts discharge that burden here, and
    /// both are structural rather than remembered:
    ///
    /// * **The bytes still describe the element.** A range is moved onto a rebuilt node only where
    ///   that node compares [equal](PartialEq) to the original it replaces — same name, same
    ///   self-closing style, same attributes in the same order with the same quoting, and, all the
    ///   way down, the same children. That is precisely the set of properties an element's markup
    ///   determines, so "equal" *is* "these bytes spell this element". Where anything differs, the
    ///   node keeps the `None` a rebuild is born with and serializes from the model, and so does
    ///   every ancestor of it.
    /// * **The buffer is the right one.** The range comes from the element being overwritten, and
    ///   lands on its replacement at that same position — so the destination document is, by
    ///   construction, the one that measured it. A caller cannot pass an original from one document
    ///   and a rebuild destined for another, because the original *is* the destination.
    ///
    /// The serializer re-checks what it can regardless (that the range fits, opens with `<` and this
    /// element's qualified name, and closes the way `empty` says it closes), so even a range that
    /// reached here wrongly degrades to a reflow rather than to wrong bytes.
    ///
    /// Costs one structural comparison of the two subtrees — a single pass, each node visited once,
    /// which is the same order as the rebuild that produced `rebuilt`.
    pub fn replace_preserving_verbatim_source(&mut self, mut rebuilt: Self) {
        restore_verbatim_source(self, &mut rebuilt);
        *self = rebuilt;
    }
}

/// Copies `original`'s source range onto `rebuilt` wherever the two spell the same markup, and
/// reports whether they do.
///
/// Bottom-up and single-pass: each node of each tree is visited once, so a deep subtree costs no
/// more than a shallow one of the same size. The `&=` is deliberate — it must not short-circuit,
/// because a child whose sibling already differs can still be restored.
fn restore_verbatim_source(original: &RawElement, rebuilt: &mut RawElement) -> bool {
    // Named field access throughout, never `Deref`/`DerefMut`: reaching the content mutably would
    // clear the very range this function exists to restore.
    let mut same = original.name == rebuilt.name
        && original.empty == rebuilt.empty
        && original.content.attributes == rebuilt.content.attributes;

    let were = &original.content.children;
    let are = &mut rebuilt.content.children;
    if were.len() == are.len() {
        for (was, is) in were.iter().zip(are.iter_mut()) {
            same &= restore_verbatim_source_node(was, is);
        }
    } else {
        // A child was inserted or removed, so positions no longer line up. Match what still does —
        // the unbroken run at each end — and leave the middle to serialize from the model. The
        // element itself differs either way, so it keeps no range.
        same = false;
        let (was_len, is_len) = (were.len(), are.len());
        let common = was_len.min(is_len);
        let mut front = 0;
        while front < common && restore_verbatim_source_node(&were[front], &mut are[front]) {
            front += 1;
        }
        let mut back = 0;
        while back < common - front
            && restore_verbatim_source_node(&were[was_len - 1 - back], &mut are[is_len - 1 - back])
        {
            back += 1;
        }
    }

    if same {
        rebuilt.source = original.source;
    }
    same
}

/// [`restore_verbatim_source`] for a node: recurse into a pair of elements, compare anything else.
///
/// Only an element carries a range; text, CDATA, comments and processing instructions are stored as
/// their verbatim bytes already, so for them "reproduced unchanged" is just equality.
fn restore_verbatim_source_node(original: &RawNode, rebuilt: &mut RawNode) -> bool {
    match (original, rebuilt) {
        (RawNode::Element(was), RawNode::Element(is)) => restore_verbatim_source(was, is),
        (was, is) => was == &*is,
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
    ///
    /// The guarantee is deliberately **relative**. What matters is that the span costs eight bytes
    /// and nothing else costs anything — that a later field, or a wider span, would show up here.
    /// `RawElement`'s absolute size is not pinned because it is not a property of this design: it
    /// follows from `Vec`, `Box<[u8]>` and `Symbol`, and so differs by target. An absolute figure
    /// would fail on a 32-bit build while the budget it claims to guard was still met.
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

    /// A tree shaped like what a typed model rebuilds: a root, three element children, some text.
    fn sample_tree(interner: &mut crate::Interner) -> RawElement {
        let named = |interner: &mut crate::Interner, local: &str| RawName {
            prefix: None,
            local: interner.intern(local),
            namespace: None,
        };
        let attribute = |interner: &mut crate::Interner, local: &str, value: &str| RawAttribute {
            name: named(interner, local),
            value: value.as_bytes().into(),
            quote: QuoteStyle::Double,
        };
        let first = RawElement::parsed(
            named(interner, "first"),
            vec![attribute(interner, "val", "1")],
            Vec::new(),
            true,
            10..30,
        );
        let second = RawElement::parsed(
            named(interner, "second"),
            vec![attribute(interner, "val", "2")],
            Vec::new(),
            true,
            30..50,
        );
        let third = RawElement::parsed(
            named(interner, "third"),
            vec![attribute(interner, "val", "3")],
            Vec::new(),
            true,
            50..70,
        );
        RawElement::parsed(
            named(interner, "root"),
            Vec::new(),
            vec![
                RawNode::Element(first),
                RawNode::Text(b"\n".as_slice().into()),
                RawNode::Element(second),
                RawNode::Element(third),
            ],
            false,
            0..80,
        )
    }

    /// What a `from_xml` / `to_xml` pass produces: the same markup with every span gone.
    fn as_a_model_rebuilds_it(element: &RawElement) -> RawElement {
        let rebuilt = element.clone();
        assert_eq!(rebuilt.source_span(), None, "Clone must drop the span");
        rebuilt
    }

    #[test]
    fn a_rebuild_that_changed_nothing_gets_every_span_back() {
        let mut interner = crate::Interner::new();
        let mut original = sample_tree(&mut interner);
        let rebuilt = as_a_model_rebuilds_it(&original);
        let expected = original.clone();

        original.replace_preserving_verbatim_source(rebuilt);

        assert_eq!(original, expected, "the markup is untouched");
        assert_eq!(
            original.source_span(),
            Some(0..80),
            "the root's span is back"
        );
        let spans: Vec<_> = original
            .children
            .iter()
            .filter_map(|child| match child {
                RawNode::Element(element) => Some(element.source_span()),
                _ => None,
            })
            .collect();
        assert_eq!(spans, vec![Some(10..30), Some(30..50), Some(50..70)]);
    }

    #[test]
    fn a_changed_child_loses_its_span_and_so_does_every_ancestor_while_its_siblings_keep_theirs() {
        let mut interner = crate::Interner::new();
        let mut original = sample_tree(&mut interner);
        let mut rebuilt = as_a_model_rebuilds_it(&original);
        // Exactly the shape of a typed setter: one attribute value, three levels down from nothing.
        match &mut rebuilt.children[2] {
            RawNode::Element(second) => second.attributes[0].value = b"changed".as_slice().into(),
            other => panic!("expected the second element, found {other:?}"),
        }

        original.replace_preserving_verbatim_source(rebuilt);

        assert_eq!(
            original.source_span(),
            None,
            "the root contains a changed descendant, so it must serialize from the model"
        );
        let spans: Vec<_> = original
            .children
            .iter()
            .filter_map(|child| match child {
                RawNode::Element(element) => Some(element.source_span()),
                _ => None,
            })
            .collect();
        assert_eq!(
            spans,
            vec![Some(10..30), None, Some(50..70)],
            "only the element that changed reflows"
        );
    }

    #[test]
    fn a_removed_child_costs_no_sibling_its_span() {
        let mut interner = crate::Interner::new();
        let mut original = sample_tree(&mut interner);
        let mut rebuilt = as_a_model_rebuilds_it(&original);
        rebuilt.children.remove(2); // the second element

        original.replace_preserving_verbatim_source(rebuilt);

        assert_eq!(original.source_span(), None, "the root's children changed");
        let spans: Vec<_> = original
            .children
            .iter()
            .filter_map(|child| match child {
                RawNode::Element(element) => Some(element.source_span()),
                _ => None,
            })
            .collect();
        assert_eq!(
            spans,
            vec![Some(10..30), Some(50..70)],
            "the run before the removal and the run after it both still match"
        );
    }

    /// The soundness argument in one assertion: a span is restored **iff** the two elements are
    /// equal, and `PartialEq` here compares exactly the properties an element's markup determines.
    #[test]
    fn a_span_is_restored_exactly_where_the_rebuild_compares_equal() {
        let mut interner = crate::Interner::new();
        let original = sample_tree(&mut interner);
        type Mutation = Box<dyn Fn(&mut RawElement)>;
        let mut mutations: Vec<Mutation> = Vec::new();
        mutations.push(Box::new(|_| {}));
        mutations.push(Box::new(|element: &mut RawElement| {
            let name = element.name;
            element.attributes.push(RawAttribute {
                name,
                value: b"x".as_slice().into(),
                quote: QuoteStyle::Single,
            });
        }));
        mutations.push(Box::new(|element: &mut RawElement| {
            element.children.clear();
        }));
        mutations.push(Box::new(|element: &mut RawElement| {
            element.empty = !element.empty;
        }));
        mutations.push(Box::new(|element: &mut RawElement| {
            if let Some(RawNode::Element(first)) = element.children.first_mut() {
                first.attributes[0].quote = QuoteStyle::Single;
            }
        }));

        for mutate in &mutations {
            let mut rebuilt = as_a_model_rebuilds_it(&original);
            mutate(&mut rebuilt);
            let equal = rebuilt == original;
            // A second parse of the same tree, so the destination carries the real spans.
            let mut destination = sample_tree(&mut interner);
            destination.replace_preserving_verbatim_source(rebuilt);
            assert_eq!(
                destination.source_span().is_some(),
                equal,
                "a span must be restored exactly when the rebuild reproduced the element"
            );
        }
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
