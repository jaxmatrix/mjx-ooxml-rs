//! The part frame the three non-worksheet sheet kinds share.
//!
//! # Why this exists, and why [`WorksheetPart`](crate::WorksheetPart) does not use it
//!
//! `CT_Chartsheet`, `CT_Dialogsheet` and `CT_Macrosheet` are three different complex types with
//! three different `xsd:sequence`s — but they are the *same part shape*: an XML declaration, a root
//! element in the SpreadsheetML namespace, a list of children each of which is either modelled or
//! preserved verbatim, and the slot-level copy-on-write `crates/mjx-sml/src/worksheet/frame.rs`
//! documents in full. Written three times, that is three copies of one `write_into`, one
//! `insert_index` and one `bind_relationship_prefix`, free to drift apart one at a time. Written
//! once here, it is one.
//!
//! [`WorksheetPart`](crate::WorksheetPart) is **deliberately not** retrofitted onto it, and the
//! reason is a property of `CT_Worksheet` rather than a preference:
//!
//! * its `sheetData` slot is [`SheetData`](crate::SheetData), a packed store that writes its **own**
//!   bytes at three further granularities and never holds a `RawElement` at all, so it is not a slot
//!   this frame's `verbatim`/`as_raw_element` pair describes;
//! * it carries thirty-nine slots and a curated surface (cells, dimensions, merges, conditional
//!   rules, hyperlinks) that six children have built on, and moving its storage would touch every
//!   one of them.
//!
//! That is the same call MJXOFF-131 made about `mjx-pptx`'s `spPr` navigation: a new caller gets the
//! shared mechanism, and migrating the old one is a separate refactor with its own risk. It is
//! recorded here rather than left implicit, because "why are there two frames" is the first question
//! a reader of this file will have.
//!
//! # What the frame guarantees
//!
//! Exactly what the worksheet frame guarantees, restated once so it is not re-derived per kind:
//!
//! * **The whole part.** Until anything is edited, [`SheetFrame::write_into`] is one
//!   `extend_from_slice` of the buffer the part was parsed from — prologue, root start tag, every
//!   slot, whitespace between them, unexamined.
//! * **One slot.** After an edit somewhere, every *other* slot still writes from its own bytes: an
//!   unmodelled child is a [`RawNode`] that kept its source range, and a modelled one keeps the
//!   [`RawElement`] it was read from beside the model.
//! * **Exactly one door.** A modelled slot's verbatim element is dropped by the `_mut` accessor that
//!   hands out `&mut`, and by the setter that replaces it. A slot whose bytes are still claimed is a
//!   slot nothing has been able to change.
//! * **Placement is generated.** A new child goes in at its rank in the type's own
//!   [`ChildOrder`](mjx_ooxml_types::child_order::ChildOrder), and an unmodelled child is ranked too
//!   — through [`ChildOrder::rank_of_node`](mjx_ooxml_types::child_order::ChildOrder::rank_of_node),
//!   so a modelled child can never be placed on the wrong side of a held one.

use std::sync::Arc;

use mjx_ooxml_core::{
    Interner, RawAttribute, RawDocument, RawElement, RawElementContent, RawName, RawNode,
};
use mjx_ooxml_types::child_order::ChildOrder;
use mjx_ooxml_types::namespaces::SML;

use crate::error::SmlError;

/// One sheet part's content vocabulary: how a child is read, ranked and written.
///
/// Implemented once per sheet kind, over that kind's own `…Content` enum. Everything else about the
/// part is [`SheetFrame`]'s.
pub(crate) trait SheetContent: Sized {
    /// The generated `xsd:sequence` table for the complex type this content belongs to.
    const ORDER: &'static ChildOrder;

    /// Reads one SpreadsheetML child into a modelled variant, or answers `None` when this type does
    /// not model an element by that name — in which case the frame holds it verbatim.
    ///
    /// # Errors
    /// [`SmlError`] when the element *is* modelled but does not match its complex type.
    fn read(element: &RawElement, interner: &Interner) -> Result<Option<Self>, SmlError>;

    /// Wraps a node this type does not model.
    fn raw(node: RawNode) -> Self;

    /// This child's wire local name, or `None` for an unmodelled node.
    fn local(&self) -> Option<&'static str>;

    /// The node an unmodelled child holds, or `None` for a modelled one.
    fn raw_node(&self) -> Option<&RawNode>;

    /// This child rebuilt as an element, for a slot whose verbatim bytes are no longer claimed.
    ///
    /// `None` for an unmodelled child, which [`raw_node`](Self::raw_node) answers for instead.
    fn as_raw_element(&self) -> Option<RawElement>;
}

/// One child of a sheet part, and the bytes it may still be written from.
#[derive(Debug)]
pub(crate) struct FrameSlot<C> {
    /// The element as it was **moved** out of the parsed tree, with its verbatim source range
    /// intact — or `None` once the slot has been authored, replaced, or reached mutably.
    verbatim: Option<RawElement>,
    value: C,
}

impl<C: SheetContent> FrameSlot<C> {
    /// A slot whose value must be written from the model.
    fn authored(value: C) -> Self {
        Self {
            verbatim: None,
            value,
        }
    }

    /// This child's rank in the type's `xsd:sequence`, from the generated table.
    ///
    /// An unmodelled child is ranked too, through the same table: see the module documentation.
    fn rank(&self, interner: &Interner) -> Option<u16> {
        match self.value.raw_node() {
            Some(node) => C::ORDER.rank_of_node(node, interner),
            None => C::ORDER.rank_of(None, self.value.local()?),
        }
    }

    /// Gives up the claim on this slot's original bytes, because it is about to be changed.
    fn dirty(&mut self) {
        self.verbatim = None;
    }

    fn write_into(&self, interner: &Interner, source: Option<&[u8]>, out: &mut Vec<u8>) {
        if let Some(node) = self.value.raw_node() {
            mjx_xml::fidelity::serialize_node(node, interner, source, out);
            return;
        }
        if let Some(original) = &self.verbatim {
            mjx_xml::fidelity::serialize_element(original, interner, source, out);
            return;
        }
        if let Some(element) = self.value.as_raw_element() {
            mjx_xml::fidelity::serialize_element(&element, interner, source, out);
        }
    }
}

/// The whole of one sheet part: everything outside the root element, the root element itself, and
/// its children in document order.
///
/// **Consumes** the document it was read from, as [`WorksheetPart`](crate::WorksheetPart) does and
/// for the same reason: the part's slots keep the byte ranges they were parsed at, so the tree can
/// be dropped and the buffer shared.
#[derive(Debug)]
pub(crate) struct SheetFrame<C> {
    /// The interner every [`RawName`] below was interned in.
    interner: Interner,
    /// The part's own bytes, shared with whoever else holds them. `None` for an authored part.
    source: Option<Arc<[u8]>>,
    /// Whether the part began with a UTF-8 byte-order mark.
    bom: bool,
    /// Nodes before the root element: the XML declaration, and any comment or PI beside it.
    prologue: Vec<RawNode>,
    /// Nodes after the root element.
    epilogue: Vec<RawNode>,
    /// The root element's qualified name, as the file wrote it.
    name: RawName,
    /// The root element's attributes, in order — every `xmlns:` declaration among them.
    attributes: Vec<RawAttribute>,
    /// Whether the root was written self-closing.
    empty: bool,
    /// Every child, in document order.
    content: Vec<FrameSlot<C>>,
    /// Whether anything at all has been changed since the part was read.
    edited: bool,
}

impl<C: SheetContent> SheetFrame<C> {
    /// Reads a whole part out of the document it was parsed from, **consuming** it.
    ///
    /// `Ok(None)` when the document's root is not an element named `root_local` in the SpreadsheetML
    /// namespace — the caller handed over a different part, which is a question rather than an
    /// error.
    ///
    /// # Errors
    /// [`SmlError::Model`] if a modelled element does not match its complex type.
    pub(crate) fn read_document(
        document: RawDocument,
        root_local: &str,
    ) -> Result<Option<Self>, SmlError> {
        if !is_spreadsheetml(&document.root.name, &document.interner)
            || document.interner.resolve(document.root.name.local) != root_local
        {
            return Ok(None);
        }
        let source = document.shared_source().cloned();
        let RawDocument {
            interner,
            bom,
            prologue,
            root,
            epilogue,
            ..
        } = document;
        let name = root.name;
        let empty = root.empty;
        let RawElementContent {
            attributes,
            children,
        } = root.into_content();

        let mut content = Vec::with_capacity(children.len());
        for node in children {
            content.push(read_slot::<C>(node, &interner)?);
        }
        Ok(Some(Self {
            interner,
            source,
            bom,
            prologue,
            epilogue,
            name,
            attributes,
            empty,
            content,
            edited: false,
        }))
    }

    /// An empty part rooted at `root_local`, authored rather than read, bound to `prefix` or to the
    /// default namespace.
    ///
    /// Declares no namespaces of its own: a part written from this has to bind at least the
    /// SpreadsheetML namespace.
    pub(crate) fn authored(prefix: Option<&str>, root_local: &str) -> Self {
        let mut interner = Interner::default();
        let name = crate::leaf::sml_name(&mut interner, prefix, root_local);
        Self {
            interner,
            source: None,
            bom: false,
            prologue: Vec::new(),
            epilogue: Vec::new(),
            name,
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
            edited: true,
        }
    }

    /// The interner every name in this part was interned in.
    pub(crate) fn interner(&self) -> &Interner {
        &self.interner
    }

    /// The interner, mutably — for a caller building a child to insert.
    ///
    /// Interning a string does not change the part, so this does not mark it edited; the setters
    /// that take the resulting value do.
    pub(crate) fn interner_mut(&mut self) -> &mut Interner {
        &mut self.interner
    }

    /// The prefix this part's root element is bound to — `None` when it binds SpreadsheetML as the
    /// default namespace.
    pub(crate) fn element_prefix(&self) -> Option<&str> {
        self.name.prefix.map(|symbol| self.interner.resolve(symbol))
    }

    /// The prefix this part binds to the relationship-reference namespace, from its own root-element
    /// `xmlns:` declarations — `r` in every file this project has read.
    pub(crate) fn relationship_prefix(&self) -> Option<&str> {
        crate::leaf::namespace_prefix(
            &self.attributes,
            &self.interner,
            crate::leaf::RELATIONSHIP_REFERENCE,
        )
    }

    /// The prefix this part binds to the relationship-reference namespace, **declaring one on the
    /// root when the part binds none**, and marking the part edited if it had to.
    ///
    /// The same rule [`WorksheetPart::bind_relationship_prefix`](crate::WorksheetPart::bind_relationship_prefix)
    /// states: `r`, or the first of `r2`, `r3`, … the root does not already bind to something else,
    /// because overwriting a binding the file made would change what every existing `r:`-prefixed
    /// attribute in the part means.
    pub(crate) fn bind_relationship_prefix(&mut self) -> String {
        if let Some(prefix) = self.relationship_prefix() {
            return prefix.to_owned();
        }
        let prefix = self.free_namespace_prefix();
        mjx_xml::attribute::set(
            &mut self.attributes,
            &mut self.interner,
            Some("xmlns"),
            &prefix,
            crate::leaf::RELATIONSHIP_REFERENCE.transitional,
        );
        self.edited = true;
        prefix
    }

    /// `r`, or the first of `r2`, `r3`, … the root does not already bind to something else.
    fn free_namespace_prefix(&self) -> String {
        let bound = |candidate: &str| {
            self.attributes.iter().any(|attribute| {
                attribute
                    .name
                    .prefix
                    .is_some_and(|prefix| self.interner.resolve(prefix) == "xmlns")
                    && self.interner.resolve(attribute.name.local) == candidate
            })
        };
        if !bound("r") {
            return "r".to_owned();
        }
        for suffix in 2..=u32::MAX {
            let candidate = format!("r{suffix}");
            if !bound(&candidate) {
                return candidate;
            }
        }
        // Unreachable: the loop runs to four billion and an element cannot carry that many
        // declarations.
        "r".to_owned()
    }

    /// Whether the whole part can still be written straight out of the bytes it was read from.
    pub(crate) fn is_verbatim(&self) -> bool {
        !self.edited && self.source.is_some()
    }

    /// Every child, in document order, including the slots this type does not model.
    pub(crate) fn children(&self) -> impl ExactSizeIterator<Item = &C> + '_ {
        self.content.iter().map(|slot| &slot.value)
    }

    /// The local name of every **element** child, in document order — the unmodelled ones included.
    ///
    /// What an ordering assertion is written against: it says what the part *will emit*, which is
    /// the thing schema order is a property of.
    pub(crate) fn child_element_locals(&self) -> impl Iterator<Item = &str> + '_ {
        self.content
            .iter()
            .filter_map(|slot| match (slot.value.raw_node(), slot.value.local()) {
                (Some(RawNode::Element(element)), _) => {
                    Some(self.interner.resolve(element.name.local))
                }
                (Some(_), _) => None,
                (None, local) => local,
            })
    }

    /// The first child `is_target` accepts.
    pub(crate) fn find<'a, T>(&'a self, pick: impl Fn(&'a C) -> Option<&'a T>) -> Option<&'a T> {
        self.content.iter().find_map(|slot| pick(&slot.value))
    }

    /// The first child `pick` accepts, mutably — **the one door**: reaching it gives up the slot's
    /// verbatim bytes and marks the part edited.
    pub(crate) fn find_mut<T>(
        &mut self,
        is_target: impl Fn(&C) -> bool,
        pick: impl for<'a> Fn(&'a mut C) -> Option<&'a mut T>,
    ) -> Option<&mut T> {
        let slot = self
            .content
            .iter_mut()
            .find(|slot| is_target(&slot.value))?;
        slot.dirty();
        self.edited = true;
        pick(&mut slot.value)
    }

    /// Replaces the first child `is_target` accepts, keeping its position; inserts at the schema
    /// rank when there is none; removes it when `value` is `None`.
    pub(crate) fn replace_or_insert(
        &mut self,
        local: &str,
        is_target: impl Fn(&C) -> bool,
        value: Option<C>,
    ) {
        let existing = self.content.iter().position(|slot| is_target(&slot.value));
        match (existing, value) {
            (Some(at), Some(value)) => self.content[at] = FrameSlot::authored(value),
            (Some(at), None) => {
                self.content.remove(at);
            }
            (None, Some(value)) => {
                let at = C::ORDER.insert_index_of_names(
                    self.content.iter().map(|slot| slot.rank(&self.interner)),
                    local,
                );
                self.content.insert(at, FrameSlot::authored(value));
                self.empty = false;
            }
            (None, None) => return,
        }
        self.edited = true;
    }

    /// Appends the whole part — declaration, root element and all — to `out`.
    ///
    /// A part nobody edited is one copy of its own buffer.
    pub(crate) fn write_into(&self, out: &mut Vec<u8>) {
        if let (false, Some(source)) = (self.edited, self.source.as_deref()) {
            out.extend_from_slice(source);
            return;
        }
        let source = self.source.as_deref();
        if self.bom {
            out.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
        }
        for node in &self.prologue {
            mjx_xml::fidelity::serialize_node(node, &self.interner, source, out);
        }
        let self_closing = self.empty && self.content.is_empty();
        mjx_xml::fidelity::serialize_start_tag(
            &self.name,
            &self.attributes,
            self_closing,
            &self.interner,
            out,
        );
        if !self_closing {
            for slot in &self.content {
                slot.write_into(&self.interner, source, out);
            }
            mjx_xml::fidelity::serialize_end_tag(&self.name, &self.interner, out);
        }
        for node in &self.epilogue {
            mjx_xml::fidelity::serialize_node(node, &self.interner, source, out);
        }
    }

    /// The whole part as bytes.
    pub(crate) fn to_markup(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.source.as_ref().map_or(1024, |bytes| bytes.len()));
        self.write_into(&mut out);
        out
    }
}

/// Whether `name` is in either conformance world's SpreadsheetML namespace.
fn is_spreadsheetml(name: &RawName, interner: &Interner) -> bool {
    let namespace = name.namespace.map(|symbol| interner.resolve(symbol));
    namespace == Some(SML.transitional) || (namespace.is_some() && namespace == SML.strict)
}

/// Reads one child of the root element into a slot.
///
/// A modelled element is **moved** into the slot's `verbatim` field, not cloned, so its source range
/// survives: a cloned `RawElement` drops it.
fn read_slot<C: SheetContent>(
    node: RawNode,
    interner: &Interner,
) -> Result<FrameSlot<C>, SmlError> {
    let RawNode::Element(element) = node else {
        return Ok(FrameSlot {
            verbatim: None,
            value: C::raw(node),
        });
    };
    if !is_spreadsheetml(&element.name, interner) {
        return Ok(FrameSlot {
            verbatim: None,
            value: C::raw(RawNode::Element(element)),
        });
    }
    match C::read(&element, interner)? {
        Some(value) => Ok(FrameSlot {
            verbatim: Some(element),
            value,
        }),
        None => Ok(FrameSlot {
            verbatim: None,
            value: C::raw(RawNode::Element(element)),
        }),
    }
}

/// Generates the part surface every sheet kind shares: the three readers, the authoring constructor,
/// and the frame delegations.
///
/// One macro rather than three copies, for the reason the frame itself is one type. Every body here
/// is a single call into [`SheetFrame`].
macro_rules! sheet_part_surface {
    ($part:ident, $content:ident, $root_local:literal, $what:literal) => {
        impl $part {
            #[doc = concat!("Reads a whole `x:", $root_local, "` part out of the document it was \
                parsed from, **consuming** it.\n\n\
                `Ok(None)` when the document's root is not an `x:", $root_local, "` — the caller \
                handed over a different part, which is a question rather than an error.\n\n\
                # Errors\n\
                [`SmlError::Model`](crate::SmlError::Model) if a modelled element does not match \
                its complex type.")]
            pub fn read_document(
                document: ::mjx_ooxml_core::RawDocument,
            ) -> ::core::result::Result<::core::option::Option<Self>, $crate::error::SmlError> {
                ::core::result::Result::Ok(
                    $crate::sheets::frame::SheetFrame::<$content>::read_document(
                        document,
                        $root_local,
                    )?
                    .map(|frame| Self { frame }),
                )
            }

            #[doc = concat!("Parses `bytes` and reads the `x:", $root_local, "` part in them, \
                sharing the buffer so that every slot nobody edits re-emits from it.\n\n\
                # Errors\n\
                [`SmlError::Xml`](crate::SmlError::Xml) if the part is not well-formed, otherwise \
                as [`read_document`](Self::read_document).")]
            pub fn read_part(
                bytes: &[u8],
            ) -> ::core::result::Result<::core::option::Option<Self>, $crate::error::SmlError> {
                Self::read_shared(::std::sync::Arc::from(bytes))
            }

            #[doc = concat!("[`read_part`](Self::read_part) for a caller that already holds the \
                bytes in an [`Arc`](std::sync::Arc) — a package, above all.\n\n\
                # Errors\n\
                As [`read_part`](Self::read_part).")]
            pub fn read_shared(
                source: ::std::sync::Arc<[u8]>,
            ) -> ::core::result::Result<::core::option::Option<Self>, $crate::error::SmlError> {
                Self::read_document(::mjx_xml::fidelity::parse_shared(source)?)
            }

            #[doc = concat!("An empty `x:", $root_local, "`, authored rather than read, bound to \
                `prefix` or to the default namespace.\n\n\
                Declares no namespaces of its own: a part written from this has to bind at least \
                the SpreadsheetML namespace.")]
            #[must_use]
            pub fn authored(prefix: ::core::option::Option<&str>) -> Self {
                Self {
                    frame: $crate::sheets::frame::SheetFrame::authored(prefix, $root_local),
                }
            }

            /// The interner every name in this part was interned in.
            ///
            /// Hand it to any accessor that takes one — the attribute getters on every type here do,
            /// because an attribute's value is bytes and its name is a symbol.
            #[must_use]
            pub fn interner(&self) -> &::mjx_ooxml_core::Interner {
                self.frame.interner()
            }

            /// The interner, mutably — for a caller building a child to insert.
            ///
            /// Interning a string does not change the part, so this does **not** mark it edited; the
            /// setters that take the resulting value do.
            pub fn interner_mut(&mut self) -> &mut ::mjx_ooxml_core::Interner {
                self.frame.interner_mut()
            }

            /// The prefix this part's root element is bound to — `None` when it binds SpreadsheetML
            /// as the default namespace, which is what every producer this project has read does.
            #[must_use]
            pub fn element_prefix(&self) -> ::core::option::Option<&str> {
                self.frame.element_prefix()
            }

            /// The prefix this part binds to the relationship-reference namespace, from its own
            /// root-element `xmlns:` declarations.
            ///
            /// `None` means the part binds the namespace nowhere, so no element in it can carry an
            /// `r:id` at all.
            #[must_use]
            pub fn relationship_prefix(&self) -> ::core::option::Option<&str> {
                self.frame.relationship_prefix()
            }

            /// The prefix this part binds to the relationship-reference namespace, **declaring one
            /// on the root when the part binds none**, and marking the part edited if it had to.
            pub fn bind_relationship_prefix(&mut self) -> ::std::string::String {
                self.frame.bind_relationship_prefix()
            }

            /// Whether the whole part can still be written straight out of the bytes it was read
            /// from.
            ///
            /// False for an authored part, for one read without a source buffer, and for one
            /// anything has been changed in — after which the *slots* still answer the same question
            /// one at a time.
            #[must_use]
            pub fn is_verbatim(&self) -> bool {
                self.frame.is_verbatim()
            }

            #[doc = concat!("Every child of the ", $what, ", in document order, including the \
                slots this type does not model.")]
            pub fn children(
                &self,
            ) -> impl ::core::iter::ExactSizeIterator<Item = &$content> + '_ {
                self.frame.children()
            }

            /// The local name of every **element** child, in document order — the unmodelled slots
            /// included.
            ///
            /// This is what an ordering assertion is written against: it says what the part *will
            /// emit*, which is the thing schema order is a property of.
            pub fn child_element_locals(&self) -> impl ::core::iter::Iterator<Item = &str> + '_ {
                self.frame.child_element_locals()
            }

            /// Appends the whole part — declaration, root element and all — to `out`.
            ///
            /// A part nobody edited is one copy of its own buffer.
            pub fn write_into(&self, out: &mut ::std::vec::Vec<u8>) {
                self.frame.write_into(out);
            }

            /// The whole part as bytes.
            #[must_use]
            pub fn to_markup(&self) -> ::std::vec::Vec<u8> {
                self.frame.to_markup()
            }
        }
    };
}

/// Declares one singleton slot of a sheet part: a borrowing getter, a mutable getter that gives up
/// the slot's verbatim bytes, and a setter that places a new child at its schema rank.
macro_rules! sheet_slot {
    (
        $content:ident, $getter:ident, $getter_mut:ident, $setter:ident,
        $variant:ident, $ty:ty, $local:literal, $doc:literal
    ) => {
        #[doc = $doc]
        #[must_use]
        pub fn $getter(&self) -> ::core::option::Option<&$ty> {
            self.frame.find(|child| match child {
                $content::$variant(value) => ::core::option::Option::Some(value),
                _ => ::core::option::Option::None,
            })
        }

        #[doc = concat!("`x:", $local, "`, mutably — `None` if the part has none.\n\n\
            Reaching a slot through here gives up its verbatim bytes: it and the part are marked \
            edited, so the slot re-emits from the model and every *other* slot still re-emits from \
            the file.")]
        #[must_use]
        pub fn $getter_mut(&mut self) -> ::core::option::Option<&mut $ty> {
            self.frame.find_mut(
                |child| ::core::matches!(child, $content::$variant(_)),
                |child| match child {
                    $content::$variant(value) => ::core::option::Option::Some(value),
                    _ => ::core::option::Option::None,
                },
            )
        }

        #[doc = concat!("Sets `x:", $local, "`: `None` removes it; `Some(value)` replaces the \
            existing element **where it is**, or inserts a new one at its rank in the type's \
            `xsd:sequence`.")]
        pub fn $setter(&mut self, value: ::core::option::Option<$ty>) {
            self.frame.replace_or_insert(
                $local,
                |child| ::core::matches!(child, $content::$variant(_)),
                value.map($content::$variant),
            );
        }
    };
}

pub(crate) use {sheet_part_surface, sheet_slot};

// ===============================================================================================
// The slot ledger for the three sheet kinds — the same class MJXOFF-88 §9 B2 names for a worksheet
// ===============================================================================================

#[cfg(test)]
mod tests {
    use mjx_ooxml_types::child_order::{CHARTSHEET, DIALOGSHEET, MACROSHEET};

    use super::*;
    use crate::prose::{check_counts, module_documentation, NUMBER_WORDS};
    use crate::sheets::{ChartSheetContent, DialogSheetContent, MacroSheetContent};

    /// How one sheet kind's slots divide, **derived from its own read path**.
    struct Split {
        /// Every slot [`SheetContent::read`] gives a typed variant.
        modelled: usize,
        /// Every slot it declines, in rank order — the frame holds each as its kind's `Raw`.
        held: Vec<&'static str>,
    }

    /// Reads a part holding **every** slot the generated table names for `C`, and reports how the
    /// frame classified each one.
    ///
    /// The classification comes from [`read_slot`] — the function a real part goes through — so
    /// modelling a held slot flips a row here on the next build, and a slot added to `sml.xsd` and
    /// regenerated arrives in `held` rather than anywhere silent. This is `CT_Worksheet`'s
    /// derivation (`crates/mjx-sml/src/worksheet/frame.rs`) applied to the kinds that share this
    /// frame, because the figure they state is written the same way and rots the same way.
    fn split_derived_from_the_read_path<C: SheetContent>(root: &str) -> Split {
        let mut markup = String::from("<");
        markup.push_str(root);
        markup.push_str(r#" xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#);
        for slot in C::ORDER.slots {
            markup.push('<');
            markup.push_str(slot.local);
            markup.push_str("/>");
        }
        markup.push_str("</");
        markup.push_str(root);
        markup.push('>');

        let document = mjx_xml::fidelity::parse(markup.as_bytes())
            .expect("a part holding one of every slot parses");
        let frame: SheetFrame<C> = SheetFrame::read_document(document, root)
            .expect("the part reads")
            .expect("the root is the element it was built as");
        assert_eq!(
            frame.content.len(),
            C::ORDER.slots.len(),
            "the frame read back a different number of children than the markup held"
        );

        let mut split = Split {
            modelled: 0,
            held: Vec::new(),
        };
        for (slot, declared) in frame.content.iter().zip(C::ORDER.slots) {
            match slot.value.local() {
                Some(local) => {
                    assert_eq!(local, declared.local, "rank {} is misnamed", declared.rank);
                    split.modelled += 1;
                }
                None => split.held.push(declared.local),
            }
        }
        split
    }

    /// **Every slot of all three sheet kinds is either modelled or named as held, and the two add
    /// up** — and the file that documents each kind states the figure its reader produces.
    ///
    /// At 0.0.138 `dialogsheet.rs` opened with *"Eleven of its sixteen slots are modelled"* over a
    /// reader that types ten, and named *"the five held verbatim"* immediately above a list of six
    /// before calling them *"all six"* two sentences later. Nothing could see it: the split was
    /// prose, and the only assertions in the crate were about round-tripping, which holds whether a
    /// slot is typed or not. That is MJXOFF-88 §9 B2 in the file next door to the one it names.
    #[test]
    fn every_slot_of_every_sheet_kind_is_accounted_for() {
        let chartsheet = split_derived_from_the_read_path::<ChartSheetContent>("chartsheet");
        let dialogsheet = split_derived_from_the_read_path::<DialogSheetContent>("dialogsheet");
        let macrosheet = split_derived_from_the_read_path::<MacroSheetContent>("macrosheet");

        assert_eq!(
            chartsheet.held,
            vec!["legacyDrawing", "legacyDrawingHF", "drawingHF", "extLst"]
        );
        assert_eq!(
            dialogsheet.held,
            vec![
                "legacyDrawing",
                "legacyDrawingHF",
                "drawingHF",
                "oleObjects",
                "controls",
                "extLst"
            ]
        );
        assert_eq!(
            macrosheet.held,
            vec![
                "sheetData",
                "phoneticPr",
                "legacyDrawing",
                "legacyDrawingHF",
                "drawingHF",
                "oleObjects",
                "extLst"
            ]
        );

        let kinds = [
            (
                "chartsheet.rs",
                include_str!("chartsheet.rs"),
                CHARTSHEET,
                chartsheet,
            ),
            (
                "dialogsheet.rs",
                include_str!("dialogsheet.rs"),
                DIALOGSHEET,
                dialogsheet,
            ),
            (
                "macrosheet.rs",
                include_str!("macrosheet.rs"),
                MACROSHEET,
                macrosheet,
            ),
        ];
        let mut checked = 0usize;
        for (file, source, order, split) in kinds {
            assert_eq!(
                split.modelled + split.held.len(),
                order.slots.len(),
                "{file}: a slot of {} was classified as neither",
                order.symbol
            );
            let documentation = module_documentation(source);
            let sentence = format!(
                "{} slots, {} modelled, {} held",
                NUMBER_WORDS[order.slots.len()],
                NUMBER_WORDS[split.modelled],
                NUMBER_WORDS[split.held.len()],
            );
            assert!(
                documentation.to_lowercase().contains(&sentence),
                "{file}'s module documentation does not say `{sentence}`, which is what its read \
                 path says"
            );
            checked += 1;
            checked += check_counts(
                file,
                &documentation,
                &[
                    ("slots", order.slots.len()),
                    ("modelled", split.modelled),
                    ("typed", split.modelled),
                    ("held", split.held.len()),
                    ("held verbatim", split.held.len()),
                ],
            );
            println!(
                "{}: {} slots, {} modelled, {} held",
                order.symbol,
                order.slots.len(),
                split.modelled,
                split.held.len()
            );
        }
        assert!(
            checked >= 3,
            "only {checked} spelled-out counts were found across the three sheet kinds — the \
             phrase scanner has stopped matching, and a scanner that matches nothing passes forever"
        );
        println!("sheet-kind prose: {checked} spelled-out counts checked");
    }
}
