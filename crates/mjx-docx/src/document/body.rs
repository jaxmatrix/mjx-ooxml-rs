//! `w:body` (`CT_Body`) and the block content model it holds: paragraphs (`w:p`, `CT_P`), runs
//! (`w:r`, `CT_R`), text (`w:t`, `CT_Text`) and the rest of `EG_RunInnerContent`'s 33 members.
//!
//! # The three content groups, as three enums
//!
//! WordprocessingML nests three wire groups inside one another, and this module models each with
//! one enum:
//!
//! - [`BlockContent`] — `EG_ContentBlockContent` (plus `w:sectPr`, the one child `CT_Body` adds after
//!   it, and `w:tcPr`, the one child a table cell adds *before* it — see [`BlockContent::Properties`]):
//!   what [`Body`], a header/footer (`headers.rs`, MJXOFF-113) and, now, a table cell
//!   (`tables.rs::Cell`, MJXOFF-116) all hold. [`Paragraph`] and [`BlockContent::Table`] are typed;
//!   `w:customXml` and `w:sdt` stay [`Unmodeled`] (nobody has claimed them yet).
//! - [`ParagraphContent`] — `EG_PContent`: what [`Paragraph`] and, recursively, [`Hyperlink`] hold.
//!   [`Run`] is the one typed member with real reach; [`Hyperlink`] is typed too, but only enough to
//!   recurse back into this same enum — its own attributes (`r:id`, `anchor`, `tooltip`, …) are
//!   MJXOFF-121's semantics, not this child's. `w:customXml`, `w:smartTag`, `w:sdt`, `w:dir`,
//!   `w:bdo` and `w:fldSimple` stay [`Unmodeled`]: each of them *also* wraps `EG_PContent` per the
//!   schema, so a run three of them deep is schema-legal, but nothing today asks this crate to reach
//!   one — the ticket's own reachability requirement names `w:hyperlink` specifically, and giving
//!   every wrapper the same treatment "for symmetry" would be five recursive types this child was
//!   not asked to test. A later child that needs one flips it from `Unmodeled` to a typed struct
//!   the same way [`Hyperlink`] already is, without touching this enum's shape.
//! - [`RunInnerContent`] — `EG_RunInnerContent`, `CT_R`'s own content: **all 33 members**, every one
//!   with a variant (see the module-level list below). Nine are fully typed ([`Break`], [`Text`]
//!   reused for four of them, [`RelationshipReference`], [`Symbol`], [`PositionalTab`],
//!   [`PhoneticGuide`]); the rest are [`Unmodeled`] — sixteen because `CT_Empty` truly has no content
//!   to type, seven because a later child owns the payload (named per variant below).
//!
//! # `w:t` and `xml:space`
//!
//! [`Text`] never trims on read — significant whitespace an untouched file already carries survives
//! regardless of `xml:space`, because reading never normalizes anything in this codebase.
//! [`Text::set_text`] is the one place `xml:space="preserve"` is written or removed: it inspects the
//! *new* string only, writing `preserve` when the text starts or ends with ASCII whitespace and
//! removing the attribute otherwise, so a caller who never calls it never sees this element's
//! attributes change, and a caller who does gets the one rule applied both ways — see [`Text`]'s own
//! doc comment for the two tests this rule needs.
//!
//! # `EG_RunInnerContent`'s 33 members
//!
//! `br`, `t`, `contentPart`, `delText`, `instrText`, `delInstrText`, `noBreakHyphen`, `softHyphen`,
//! `dayShort`, `monthShort`, `yearShort`, `dayLong`, `monthLong`, `yearLong`, `annotationRef`,
//! `footnoteRef`, `endnoteRef`, `separator`, `continuationSeparator`, `sym`, `pgNum`, `cr`, `tab`,
//! `object`, `pict`, `fldChar`, `ruby`, `footnoteReference`, `endnoteReference`, `commentReference`,
//! `drawing`, `ptab`, `lastRenderedPageBreak` — thirty-three, matching the schema and the dispatch
//! brief's own recount (the ticket's prose says 32; its enumerated list and `wml.xsd` both say 33).

use mjx_ooxml_core::{
    AttributeCodec, Enumeration, FromXml, FromXmlError, Interner, InvalidAttributeValue, Number,
    RawAttribute, RawElement, RawName, RawNode, Text as TextCodec, ToXml,
};
use mjx_ooxml_types::namespaces::WML;
use mjx_ooxml_types::support::OnOff;
use mjx_ooxml_types::wordprocessingml::{
    BreakTextWrappingRestart, BreakType, DecimalNumber, DisplacedByCustomXml, EditingGroup,
    FourDigitHexadecimalNumber, PhoneticGuideAlignment, PositionalTabAlignment, PositionalTabBase,
    PositionalTabLeader, ProofingErrorType,
};

use crate::address::{BlockPath, RunPath};

/// Builds a `w:local` qualified name — literal prefix `w` plus the resolved transitional
/// WordprocessingML namespace, matching `mjx-dml::build::dml_name`'s pattern for `a:`.
///
/// `pub(crate)` so `run_properties.rs` (MJXOFF-94) reuses it rather than declaring a second copy.
pub(crate) fn wml_name(interner: &mut Interner, local: &str) -> RawName {
    RawName {
        prefix: Some(interner.intern("w")),
        local: interner.intern(local),
        namespace: Some(interner.intern(WML.transitional)),
    }
}

// ---------------------------------------------------------------------------------------------
// Body (CT_Body) and its block content (EG_ContentBlockContent + w:sectPr)
// ---------------------------------------------------------------------------------------------

/// `CT_Body` — a document's or glossary document's body: block-level content (paragraphs, tables —
/// `EG_BlockLevelElts`), then the last section's properties (`w:sectPr`).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct Body {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "customXml", variant = CustomXml, ty = Unmodeled),
        child(local = "sdt", variant = StructuredDocumentTag, ty = Unmodeled),
        child(local = "p", variant = Paragraph, ty = Paragraph),
        child(local = "tbl", variant = Table, ty = super::tables::Table),
        child(local = "proofErr", variant = ProofingError, ty = ProofingError),
        child(local = "permStart", variant = PermissionRangeStart, ty = PermissionRangeStart),
        child(local = "permEnd", variant = PermissionRangeEnd, ty = PermissionRangeEnd),
        child(local = "sectPr", variant = SectionProperties, ty = super::sections::SectionProperties),
        child(local = "tcPr", variant = Properties, ty = super::tables::CellProperties)
    )]
    content: Vec<BlockContent>,
}

/// One ordered child of a [`Body`], a header/footer, or a table cell: `EG_ContentBlockContent` plus
/// the `w:sectPr` `CT_Body` appends after it and the `w:tcPr` a table cell (`tables.rs::Cell`)
/// prepends before it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockContent {
    /// `w:customXml` (`CT_CustomXmlBlock`) — unowned; opaque.
    CustomXml(Unmodeled),
    /// `w:sdt` (`CT_SdtBlock`) — unowned; opaque.
    StructuredDocumentTag(Unmodeled),
    /// `w:p` (`CT_P`) — this child's own type.
    Paragraph(Paragraph),
    /// `w:tbl` (`CT_Tbl`) — MJXOFF-116's own type; a table's own `EG_BlockLevelElts` cells reuse
    /// this exact enum for their content, which is what lets a table nest inside a table cell.
    Table(super::tables::Table),
    /// `w:proofErr` (`CT_ProofErr`), folded in from `EG_RunLevelElts`.
    ProofingError(ProofingError),
    /// `w:permStart` (`CT_PermStart`), folded in from `EG_RunLevelElts`.
    PermissionRangeStart(PermissionRangeStart),
    /// `w:permEnd` (`CT_Perm`), folded in from `EG_RunLevelElts`.
    PermissionRangeEnd(PermissionRangeEnd),
    /// `w:sectPr` (`CT_SectPr`) — the section this body ends with (MJXOFF-109's own type, see
    /// `sections.rs`); the document's last section. **Never constructed by [`Body`]'s own API on a
    /// header/footer or a table cell** — `CT_HdrFtr` and `CT_Tc` are not `CT_Body`, so neither ever
    /// carries one — but the derive macro's `#[xml(children, …)]` list generates one exhaustive
    /// match over *every* variant of this enum for *every* struct that holds a `Vec<BlockContent>`,
    /// so `HdrFtr` and `Cell` must still map it (see `headers.rs`'s own doc comment, which this one
    /// follows).
    SectionProperties(super::sections::SectionProperties),
    /// `w:tcPr` (`CT_TcPr`) — a table cell's own properties (`tables.rs::CellProperties`). **Never
    /// constructed by [`Body`] or `HdrFtr`'s own API** — `CT_Body` and `CT_HdrFtr` carry no `tcPr`
    /// (only `CT_Tc` does) — mapped here for the same exhaustive-match reason
    /// [`BlockContent::SectionProperties`] is.
    Properties(super::tables::CellProperties),
    /// Any other child — whitespace or an unknown element — preserved verbatim.
    Raw(RawNode),
}

// -------------------------------------------------------------------------------------------
// Shared paragraph-vec addressing (MJXOFF-113's own generalization of MJXOFF-92's addressing).
//
// `EG_BlockLevelElts` is not only `CT_Body`'s content: `CT_HdrFtr` (`headers.rs`, MJXOFF-113) is the
// *same* repeatable-choice group of paragraphs, tables and range markup, minus the trailing
// `w:sectPr` only `CT_Body` appends. Before this child, every `BlockPath`-addressed operation below
// was an inherent `Body` method reaching directly into `self.content`; a header-only copy of the same
// eleven lines per method would have been exactly the "header-only duplicate of the paragraph API"
// this workspace's own process rules call out as the failure this generalization exists to prevent.
// These free functions take the `Vec<BlockContent>` itself rather than `&Body`, so both `Body`
// (below) and `HdrFtr` (`headers.rs`) share one implementation; `Body`'s own trailing-`w:sectPr`
// bookkeeping is the one place the two containers' shapes differ, so it stays a closure the caller
// supplies rather than something these functions know about.
// -------------------------------------------------------------------------------------------

/// Every paragraph in `content`, in document order.
pub(crate) fn block_paragraphs(content: &[BlockContent]) -> impl Iterator<Item = &Paragraph> {
    content.iter().filter_map(|item| match item {
        BlockContent::Paragraph(paragraph) => Some(paragraph),
        _ => None,
    })
}

/// The paragraph at `path` within `content`, or `None` if the address is out of range.
///
/// Only a top-level [`BlockPath`] resolves anything today — no fixture can construct a nested one
/// until `w:tbl` is modeled (see [`BlockPath`]'s own doc comment).
pub(crate) fn block_paragraph<'a>(
    content: &'a [BlockContent],
    path: &BlockPath,
) -> Option<&'a Paragraph> {
    let [index] = path.indices() else {
        return None;
    };
    block_paragraphs(content).nth(*index)
}

/// [`block_paragraph`], mutably.
pub(crate) fn block_paragraph_mut<'a>(
    content: &'a mut [BlockContent],
    path: &BlockPath,
) -> Option<&'a mut Paragraph> {
    let [index] = path.indices() else {
        return None;
    };
    content
        .iter_mut()
        .filter_map(|item| match item {
            BlockContent::Paragraph(paragraph) => Some(paragraph),
            _ => None,
        })
        .nth(*index)
}

/// The `content` index of the `index`th paragraph, or `None` if there is no such paragraph.
fn block_nth_paragraph_slot(content: &[BlockContent], index: usize) -> Option<usize> {
    content
        .iter()
        .enumerate()
        .filter(|(_, item)| matches!(item, BlockContent::Paragraph(_)))
        .nth(index)
        .map(|(at, _)| at)
}

/// The `content` index at which the `index`th paragraph belongs — an existing paragraph's own slot,
/// or `end_slot()` (evaluated only when no such paragraph exists yet) when `index` is one past the
/// last. `end_slot` is `Body`'s "before the trailing `w:sectPr`, if any" rule or `HdrFtr`'s plain
/// "the end of `content`" — the one place the two containers' insertion position differs.
fn block_nth_paragraph_slot_or(
    content: &[BlockContent],
    index: usize,
    end_slot: impl FnOnce() -> usize,
) -> usize {
    block_nth_paragraph_slot(content, index).unwrap_or_else(end_slot)
}

/// Inserts `paragraph` so it becomes the paragraph at `path` in `content`, shifting every paragraph
/// at or after that position one place later. `path` must address an existing paragraph slot or the
/// one past the last (i.e. `0..=`[`block_paragraphs`]`(content).count()`); anything else is rejected.
/// `end_slot` is [`block_nth_paragraph_slot_or`]'s own parameter.
///
/// Returns `false`, leaving `content` untouched, if `path` is out of range.
#[must_use]
pub(crate) fn block_insert_paragraph(
    content: &mut Vec<BlockContent>,
    path: &BlockPath,
    paragraph: Paragraph,
    end_slot: impl FnOnce() -> usize,
) -> bool {
    let [index] = path.indices() else {
        return false;
    };
    let count = block_paragraphs(content).count();
    if *index > count {
        return false;
    }
    let at = block_nth_paragraph_slot_or(content, *index, end_slot);
    content.insert(at, BlockContent::Paragraph(paragraph));
    true
}

/// Removes and returns the paragraph at `path` from `content`, or `None` if the address is out of
/// range.
pub(crate) fn block_remove_paragraph(
    content: &mut Vec<BlockContent>,
    path: &BlockPath,
) -> Option<Paragraph> {
    let [index] = path.indices() else {
        return None;
    };
    let at = block_nth_paragraph_slot(content, *index)?;
    match content.remove(at) {
        BlockContent::Paragraph(paragraph) => Some(paragraph),
        _ => unreachable!("block_nth_paragraph_slot only returns indices of Paragraph items"),
    }
}

// -------------------------------------------------------------------------------------------
// Shared table-vec addressing — [`block_paragraphs`] and friends above, for the other typed
// member of [`BlockContent`]. A table is addressed by its own top-level index, exactly as a
// paragraph is by `block_paragraph`'s — the two index spaces are independent (a table interleaved
// between two paragraphs does not shift either paragraph's index, and vice versa), matching how a
// caller thinks about "the second table" regardless of how many paragraphs sit before it.
// -------------------------------------------------------------------------------------------

/// Every table in `content`, in document order.
pub(crate) fn block_tables(
    content: &[BlockContent],
) -> impl Iterator<Item = &super::tables::Table> {
    content.iter().filter_map(|item| match item {
        BlockContent::Table(table) => Some(table),
        _ => None,
    })
}

/// The table at top-level index `index`, or `None` if there is no such table.
pub(crate) fn block_table(content: &[BlockContent], index: usize) -> Option<&super::tables::Table> {
    block_tables(content).nth(index)
}

/// [`block_table`], mutably.
pub(crate) fn block_table_mut(
    content: &mut [BlockContent],
    index: usize,
) -> Option<&mut super::tables::Table> {
    content
        .iter_mut()
        .filter_map(|item| match item {
            BlockContent::Table(table) => Some(table),
            _ => None,
        })
        .nth(index)
}

/// Appends `table` at `end_slot` — `Body`'s "before the trailing `w:sectPr`, if any" rule or
/// `HdrFtr`'s plain "the end of `content`" — and returns its new top-level index.
pub(crate) fn block_append_table(
    content: &mut Vec<BlockContent>,
    table: super::tables::Table,
    end_slot: usize,
) -> usize {
    let index = block_tables(content).count();
    content.insert(end_slot, BlockContent::Table(table));
    index
}

/// Removes and returns the table at top-level index `index`, or `None` if there is no such table.
pub(crate) fn block_remove_table(
    content: &mut Vec<BlockContent>,
    index: usize,
) -> Option<super::tables::Table> {
    let at = content
        .iter()
        .enumerate()
        .filter(|(_, item)| matches!(item, BlockContent::Table(_)))
        .nth(index)
        .map(|(at, _)| at)?;
    match content.remove(at) {
        BlockContent::Table(table) => Some(table),
        _ => unreachable!("the position found above is a Table item"),
    }
}

impl Body {
    /// How many paragraphs this body holds, in document order. Only `w:p` counts — matching
    /// `Presentation::shape_count`'s "every kind shares one count" would make "the third item" mean
    /// something different every time a `w:customXml` or a table sits between two paragraphs; a
    /// caller asking "how many paragraphs" wants paragraphs.
    #[must_use]
    pub fn paragraph_count(&self) -> usize {
        self.paragraphs().count()
    }

    /// Every paragraph in document order.
    pub fn paragraphs(&self) -> impl Iterator<Item = &Paragraph> {
        block_paragraphs(&self.content)
    }

    /// The paragraph at `path`, or `None` if the address is out of range.
    ///
    /// Only a top-level [`BlockPath`] resolves anything today — no fixture can construct a nested
    /// one until `w:tbl` is modeled (see [`BlockPath`]'s own doc comment).
    #[must_use]
    pub fn paragraph(&self, path: impl Into<BlockPath>) -> Option<&Paragraph> {
        block_paragraph(&self.content, &path.into())
    }

    /// The paragraph at `path`, mutably.
    pub fn paragraph_mut(&mut self, path: impl Into<BlockPath>) -> Option<&mut Paragraph> {
        block_paragraph_mut(&mut self.content, &path.into())
    }

    /// Inserts `paragraph` so it becomes the paragraph at `path`, shifting every paragraph at or
    /// after that position one place later. `path` must address an existing paragraph slot or the
    /// one past the last (i.e. `0..=paragraph_count()`); anything else is rejected.
    ///
    /// # Errors
    /// Returns `false`, leaving `self` untouched, if `path` is out of range.
    #[must_use]
    pub fn insert_paragraph(&mut self, path: impl Into<BlockPath>, paragraph: Paragraph) -> bool {
        let path = path.into();
        let end = self.trailing_section_properties_slot();
        block_insert_paragraph(&mut self.content, &path, paragraph, || end)
    }

    /// Appends `paragraph` as this body's new last paragraph — **before** `w:sectPr` when one is
    /// present, since `CT_Body`'s `xsd:sequence` puts every block-level child ahead of it.
    pub fn append_paragraph(&mut self, paragraph: Paragraph) {
        let at = self.trailing_section_properties_slot();
        self.content.insert(at, BlockContent::Paragraph(paragraph));
    }

    /// Removes and returns the paragraph at `path`, or `None` if the address is out of range.
    pub fn remove_paragraph(&mut self, path: impl Into<BlockPath>) -> Option<Paragraph> {
        block_remove_paragraph(&mut self.content, &path.into())
    }

    /// How many tables this body holds at its own top level, in document order.
    #[must_use]
    pub fn table_count(&self) -> usize {
        self.tables().count()
    }

    /// Every top-level table in document order.
    pub fn tables(&self) -> impl Iterator<Item = &super::tables::Table> {
        block_tables(&self.content)
    }

    /// The top-level table at `index`, or `None` if there is no such table.
    #[must_use]
    pub fn table(&self, index: usize) -> Option<&super::tables::Table> {
        block_table(&self.content, index)
    }

    /// [`Body::table`], mutably.
    pub fn table_mut(&mut self, index: usize) -> Option<&mut super::tables::Table> {
        block_table_mut(&mut self.content, index)
    }

    /// Appends `table` as this body's new last top-level table — **before** `w:sectPr` when one is
    /// present, exactly as [`Body::append_paragraph`] does — and returns its new index.
    pub fn append_table(&mut self, table: super::tables::Table) -> usize {
        let at = self.trailing_section_properties_slot();
        self.empty = false;
        block_append_table(&mut self.content, table, at)
    }

    /// Removes and returns the top-level table at `index`, or `None` if there is no such table.
    pub fn remove_table(&mut self, index: usize) -> Option<super::tables::Table> {
        block_remove_table(&mut self.content, index)
    }

    /// The `content` index of this body's own `w:sectPr`, or one past the end of `content` if it
    /// carries none — where a new trailing paragraph belongs either way, since `CT_Body`'s
    /// `xsd:sequence` puts every block-level child ahead of `w:sectPr`.
    fn trailing_section_properties_slot(&self) -> usize {
        self.content
            .iter()
            .position(|item| matches!(item, BlockContent::SectionProperties(_)))
            .unwrap_or(self.content.len())
    }

    /// The section this body ends with (`w:sectPr`) — the document's last section — or `None` if it
    /// carries none (schema-legal: `CT_Body/sectPr` is `minOccurs="0"`, though no fixture in this
    /// workspace's corpus omits it — real Word output always writes one).
    #[must_use]
    pub fn section_properties(&self) -> Option<&super::sections::SectionProperties> {
        self.content.iter().find_map(|item| match item {
            BlockContent::SectionProperties(section) => Some(section),
            _ => None,
        })
    }

    /// [`Body::section_properties`], mutably.
    pub fn section_properties_mut(&mut self) -> Option<&mut super::sections::SectionProperties> {
        self.content.iter_mut().find_map(|item| match item {
            BlockContent::SectionProperties(section) => Some(section),
            _ => None,
        })
    }

    /// Sets (replaces) or removes the body-level `w:sectPr`. `CT_Body`'s own `xsd:sequence` puts it
    /// after every block-level child, so inserting one when none exists always appends.
    pub fn set_section_properties(&mut self, value: Option<super::sections::SectionProperties>) {
        let at = self
            .content
            .iter()
            .position(|item| matches!(item, BlockContent::SectionProperties(_)));
        match (at, value) {
            (Some(at), Some(value)) => self.content[at] = BlockContent::SectionProperties(value),
            (Some(at), None) => {
                self.content.remove(at);
            }
            (None, Some(value)) => self.content.push(BlockContent::SectionProperties(value)),
            (None, None) => {}
        }
    }

    /// [`Body::section_properties_mut`], creating an empty `w:sectPr` at the body's own trailing
    /// position if it does not already carry one.
    pub fn section_properties_or_insert(
        &mut self,
        interner: &mut Interner,
    ) -> &mut super::sections::SectionProperties {
        if self.section_properties().is_none() {
            self.set_section_properties(Some(super::sections::SectionProperties::new(interner)));
            self.empty = false;
        }
        match self.section_properties_mut() {
            Some(properties) => properties,
            None => unreachable!("just inserted above"),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Paragraph (CT_P) and its content (EG_PContent)
// ---------------------------------------------------------------------------------------------

/// `w:p` (`CT_P`) — a paragraph: optional properties (`w:pPr`, out of scope — MJXOFF-96), then
/// `EG_PContent*`.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct Paragraph {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "pPr", variant = Properties, ty = super::paragraph_properties::ParagraphProperties),
        child(local = "customXml", variant = CustomXml, ty = Unmodeled),
        child(local = "smartTag", variant = SmartTag, ty = Unmodeled),
        child(local = "sdt", variant = StructuredDocumentTag, ty = Unmodeled),
        child(local = "dir", variant = BidirectionalEmbedding, ty = Unmodeled),
        child(local = "bdo", variant = BidirectionalOverride, ty = Unmodeled),
        child(local = "r", variant = Run, ty = Run),
        child(local = "proofErr", variant = ProofingError, ty = ProofingError),
        child(local = "permStart", variant = PermissionRangeStart, ty = PermissionRangeStart),
        child(local = "permEnd", variant = PermissionRangeEnd, ty = PermissionRangeEnd),
        child(local = "fldSimple", variant = SimpleField, ty = super::fields::SimpleField),
        child(local = "hyperlink", variant = Hyperlink, ty = Hyperlink),
        child(local = "subDoc", variant = SubDocument, ty = RelationshipReference),
        // `w:fldData` (`CT_Text`) is `CT_SimpleField`'s own leading child, never a legal direct
        // child of `w:p`/`w:hyperlink` — mapped here only so this struct's exhaustive `ToXml` match
        // (shared with every other `Vec<ParagraphContent>` holder, see `FieldData`'s own doc
        // comment) compiles; a non-conformant file that nests one directly under `w:p` is still
        // typed rather than falling to `Raw`, which is harmless.
        child(local = "fldData", variant = FieldData, ty = Text)
    )]
    content: Vec<ParagraphContent>,
}

/// `w:hyperlink` (`CT_Hyperlink`) — typed both for reach (it recurses back into
/// [`ParagraphContent`]) and, since MJXOFF-121, for its own attributes: an external relationship
/// (`r:id`) or an internal bookmark (`anchor`) plus `tgtFrame`, `tooltip`, `docLocation` and
/// `history`. `crate::document::hyperlinks` is where a [`crate::Document`] adds/removes one
/// together with the relationship it names; this struct only reads and writes the element itself.
///
/// **`r:id` wins over `anchor` when a file carries both** — ECMA-376 Part 1 §17.16.22 states it
/// explicitly: *"If a hyperlink target is also specified using the r:id attribute, then this
/// \[`anchor`\] attribute shall be ignored."* Neither this struct nor `crate::document::hyperlinks`
/// enforces that precedence on write (both attributes round-trip verbatim, whichever a file
/// carries); a reader resolving *the* target should check `relationship_id` first.
///
/// `anchor` naming a bookmark is not resolved against a bookmark index here — that index is
/// MJXOFF-124's own (bookmarks are out of this child's scope, per its own ticket) — so
/// [`Hyperlink::anchor`] hands back the raw bookmark name, unresolved. This is the seam MJXOFF-121's
/// own ticket asks to leave and say so.
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = WML)]
#[xml(attribute(local = "id", prefix = "r", codec = TextCodec, accessor = relationship_id))]
#[xml(attribute(local = "anchor", prefix = "w", codec = TextCodec, accessor = anchor))]
#[xml(attribute(local = "tgtFrame", prefix = "w", codec = TextCodec, accessor = target_frame))]
#[xml(attribute(local = "tooltip", prefix = "w", codec = TextCodec, accessor = tooltip))]
#[xml(attribute(local = "docLocation", prefix = "w", codec = TextCodec, accessor = doc_location))]
// ECMA-376 Part 1 §17.16.22: "If this attribute is omitted, then its value shall be assumed to be
// false."
#[xml(attribute(local = "history", prefix = "w", codec = OnOff, accessor = history, default = false))]
pub struct Hyperlink {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    // `CT_Hyperlink`'s own content model has no `w:pPr` — this entry exists only so this struct's
    // `#[xml(children)]` list declares every `ParagraphContent` variant `Paragraph` also declares.
    // The generated `ToXml` match must be exhaustive over the whole enum, not just the subset a
    // given struct expects to parse, so the two lists have to agree; a `w:pPr` a non-conformant
    // file nested inside a `w:hyperlink` is typed rather than falling to `Raw`, which is harmless.
    #[xml(
        children,
        child(local = "pPr", variant = Properties, ty = super::paragraph_properties::ParagraphProperties),
        child(local = "customXml", variant = CustomXml, ty = Unmodeled),
        child(local = "smartTag", variant = SmartTag, ty = Unmodeled),
        child(local = "sdt", variant = StructuredDocumentTag, ty = Unmodeled),
        child(local = "dir", variant = BidirectionalEmbedding, ty = Unmodeled),
        child(local = "bdo", variant = BidirectionalOverride, ty = Unmodeled),
        child(local = "r", variant = Run, ty = Run),
        child(local = "proofErr", variant = ProofingError, ty = ProofingError),
        child(local = "permStart", variant = PermissionRangeStart, ty = PermissionRangeStart),
        child(local = "permEnd", variant = PermissionRangeEnd, ty = PermissionRangeEnd),
        child(local = "fldSimple", variant = SimpleField, ty = super::fields::SimpleField),
        child(local = "hyperlink", variant = Hyperlink, ty = Hyperlink),
        child(local = "subDoc", variant = SubDocument, ty = RelationshipReference),
        child(local = "fldData", variant = FieldData, ty = Text)
    )]
    content: Vec<ParagraphContent>,
}

impl Hyperlink {
    /// Builds a new, empty hyperlink — no attributes, no content — ready for
    /// `crate::document::hyperlinks` to set a `r:id`/`anchor` and append runs into before inserting
    /// it into a [`Paragraph`].
    #[must_use]
    pub(crate) fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "hyperlink"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// The hyperlink's attributes, verbatim — `r:id`, `anchor`, `tooltip`, `history`, `tgtFrame`,
    /// `docLocation`, whichever it carries. Prefer the typed accessors the
    /// [`mjx_derive::XmlAttributes`] derive above generates (`relationship_id`, `anchor`,
    /// `target_frame`, `tooltip`, `doc_location`, `history`) — this is the raw escape hatch for
    /// anything a file carries that this crate does not otherwise name.
    #[must_use]
    pub fn attributes(&self) -> &[RawAttribute] {
        &self.attributes
    }

    /// Appends `run` as this hyperlink's new last top-level run — `crate::document::hyperlinks`'
    /// own counterpart to [`Paragraph::append_run`], used when building a freshly inserted
    /// hyperlink's wrapped text.
    pub(crate) fn append_run(&mut self, run: Run) {
        self.content.push(ParagraphContent::Run(run));
        self.empty = false;
    }
}

/// One ordered child of a [`Paragraph`] or a [`Hyperlink`]: `EG_PContent`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParagraphContent {
    /// `w:pPr` (`CT_PPr`) — a paragraph's own properties: [`Paragraph`] only (a `w:pPr` a
    /// non-conformant file nested inside a [`Hyperlink`] is still typed rather than falling to
    /// `Raw`, for the exhaustive-match reason this enum's own container structs document). MJXOFF-96
    /// gives it real content; reach it through [`Paragraph::properties`], never by conflating it with
    /// a run's own `w:rPr`.
    Properties(super::paragraph_properties::ParagraphProperties),
    /// `w:customXml` (`CT_CustomXmlRun`) — unowned; opaque.
    CustomXml(Unmodeled),
    /// `w:smartTag` (`CT_SmartTagRun`) — unowned; opaque.
    SmartTag(Unmodeled),
    /// `w:sdt` (`CT_SdtRun`) — unowned; opaque.
    StructuredDocumentTag(Unmodeled),
    /// `w:dir` (`CT_DirContentRun`, "Bidirectional Embedding Level", ECMA-376 Part 1 §17.3.2.8) —
    /// unowned; opaque.
    BidirectionalEmbedding(Unmodeled),
    /// `w:bdo` (`CT_BdoContentRun`, "Bidirectional Override", ECMA-376 Part 1 §17.3.2.3) — unowned;
    /// opaque.
    BidirectionalOverride(Unmodeled),
    /// `w:r` (`CT_R`) — this child's own type.
    Run(Run),
    /// `w:proofErr` (`CT_ProofErr`), folded in from `EG_RunLevelElts`.
    ProofingError(ProofingError),
    /// `w:permStart` (`CT_PermStart`), folded in from `EG_RunLevelElts`.
    PermissionRangeStart(PermissionRangeStart),
    /// `w:permEnd` (`CT_Perm`), folded in from `EG_RunLevelElts`.
    PermissionRangeEnd(PermissionRangeEnd),
    /// `w:fldSimple` (`CT_SimpleField`) — MJXOFF-121's own type; see
    /// [`super::fields::SimpleField`].
    SimpleField(super::fields::SimpleField),
    /// `w:hyperlink` (`CT_Hyperlink`) — this child's own type, typed for reach and, since
    /// MJXOFF-121, for its own attributes too.
    Hyperlink(Hyperlink),
    /// `w:subDoc` (`CT_Rel`) — a master-document subdocument reference.
    SubDocument(RelationshipReference),
    /// `w:fldData` (`CT_Text`) — `CT_SimpleField`'s own optional legacy leading child, carrying a
    /// pre-computed field result for readers that do not evaluate fields. **Never constructed by
    /// this crate's own API on a [`Paragraph`] or [`Hyperlink`]** — `CT_P`/`CT_Hyperlink` are not
    /// `CT_SimpleField`, so neither ever legally carries one — but the derive macro's
    /// `#[xml(children, …)]` list generates one exhaustive match over *every* variant of this enum
    /// for *every* struct that holds a `Vec<ParagraphContent>` (see [`BlockContent::SectionProperties`]'s
    /// own doc comment for the identical reason on the sibling enum), so [`Paragraph`] and
    /// [`Hyperlink`] must still map it even though only [`super::fields::SimpleField`] ever produces
    /// one through this crate's own writers.
    FieldData(Text),
    /// Any other child — whitespace or an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// The visible text `content` renders: every [`Run`] it holds, descending into every
/// [`ParagraphContent::Hyperlink`] and [`ParagraphContent::SimpleField`] (a field's own cached
/// result *is* its displayed content — see `fields.rs`'s own doc comment for why instruction text,
/// carried in `w:instrText`/`RunInnerContent::FieldCode`, is never included here, exactly as
/// [`Run::text`] already excludes it). `pub(crate)` — [`Paragraph::text`] and
/// `fields.rs`'s [`super::fields::SimpleField::cached_result_text`] both call this rather than each
/// walking the tree its own way.
pub(crate) fn paragraph_content_text(content: &[ParagraphContent], out: &mut String) {
    for item in content {
        match item {
            ParagraphContent::Run(run) => out.push_str(&run.text()),
            ParagraphContent::Hyperlink(hyperlink) => {
                paragraph_content_text(&hyperlink.content, out);
            }
            ParagraphContent::SimpleField(field) => {
                paragraph_content_text(field.content(), out);
            }
            _ => {}
        }
    }
}

/// One [`ParagraphContent`] item that is, or can lead to, a run: what
/// [`Paragraph::run_count`]/[`Paragraph::run`] address.
enum RunSlot<'a> {
    Run(&'a Run),
    Hyperlink(&'a Hyperlink),
}

/// The [`ParagraphContent`] items that occupy a run-addressing slot — a [`Run`] itself, or a
/// [`Hyperlink`] a [`RunPath`] can descend into — in document order. Everything else
/// (`w:pPr`, `w:proofErr`, an unmodeled wrapper, …) is skipped: it has no run to be, so it does not
/// consume a slot.
fn run_slots(content: &[ParagraphContent]) -> impl Iterator<Item = RunSlot<'_>> {
    content.iter().filter_map(|item| match item {
        ParagraphContent::Run(run) => Some(RunSlot::Run(run)),
        ParagraphContent::Hyperlink(hyperlink) => Some(RunSlot::Hyperlink(hyperlink)),
        _ => None,
    })
}

/// Resolves a [`RunPath`]'s indices against `content`, descending into a [`Hyperlink`] for every
/// index but the last, which must land on an actual [`Run`].
fn resolve_run<'a>(content: &'a [ParagraphContent], indices: &[usize]) -> Option<&'a Run> {
    let (&first, rest) = indices.split_first()?;
    match (run_slots(content).nth(first)?, rest.is_empty()) {
        (RunSlot::Run(run), true) => Some(run),
        (RunSlot::Hyperlink(hyperlink), false) => resolve_run(&hyperlink.content, rest),
        _ => None,
    }
}

/// [`resolve_run`], mutably.
fn resolve_run_mut<'a>(
    content: &'a mut [ParagraphContent],
    indices: &[usize],
) -> Option<&'a mut Run> {
    let (&first, rest) = indices.split_first()?;
    let slot = content.iter_mut().filter_map(|item| match item {
        ParagraphContent::Run(run) => Some(RunSlotMut::Run(run)),
        ParagraphContent::Hyperlink(hyperlink) => Some(RunSlotMut::Hyperlink(hyperlink)),
        _ => None,
    });
    match (slot.into_iter().nth(first)?, rest.is_empty()) {
        (RunSlotMut::Run(run), true) => Some(run),
        (RunSlotMut::Hyperlink(hyperlink), false) => resolve_run_mut(&mut hyperlink.content, rest),
        _ => None,
    }
}

enum RunSlotMut<'a> {
    Run(&'a mut Run),
    Hyperlink(&'a mut Hyperlink),
}

/// The `content` index of the `index`th run-or-hyperlink slot at the top level (not descending),
/// or `None` if there is no such slot.
fn nth_slot_index(content: &[ParagraphContent], index: usize) -> Option<usize> {
    content
        .iter()
        .enumerate()
        .filter(|(_, item)| {
            matches!(
                item,
                ParagraphContent::Run(_) | ParagraphContent::Hyperlink(_)
            )
        })
        .nth(index)
        .map(|(at, _)| at)
}

impl Paragraph {
    /// Builds a new, empty paragraph — no `w:pPr`, no content — ready to insert or append into a
    /// [`Body`] and then hold runs added with [`Paragraph::append_run`]/[`Paragraph::insert_run`].
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "p"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// This paragraph's own properties (`w:pPr`, MJXOFF-96), or `None` if it carries none.
    ///
    /// Closes the reachability gap MJXOFF-152 found: every leaf type it fixed was correct but
    /// unreachable through `Document`/`Body`/`Paragraph`/`Run`'s own public surface. `w:pPr` is
    /// reachable from the moment this method exists.
    #[must_use]
    pub fn properties(&self) -> Option<&super::paragraph_properties::ParagraphProperties> {
        self.content.iter().find_map(|item| match item {
            ParagraphContent::Properties(properties) => Some(properties),
            _ => None,
        })
    }

    /// This paragraph's own properties, mutably, or `None` if it carries none — see
    /// [`Paragraph::properties_or_insert`] to create one.
    pub fn properties_mut(
        &mut self,
    ) -> Option<&mut super::paragraph_properties::ParagraphProperties> {
        self.content.iter_mut().find_map(|item| match item {
            ParagraphContent::Properties(properties) => Some(properties),
            _ => None,
        })
    }

    /// This paragraph's own properties, mutably — creating an empty `w:pPr` at its schema rank
    /// (always the first child of `w:p`, per `CT_P`'s `xsd:sequence`) if this paragraph does not
    /// already carry one.
    pub fn properties_or_insert(
        &mut self,
        interner: &mut Interner,
    ) -> &mut super::paragraph_properties::ParagraphProperties {
        let at = match self
            .content
            .iter()
            .position(|item| matches!(item, ParagraphContent::Properties(_)))
        {
            Some(at) => at,
            None => {
                // `pPr` is `CT_P`'s unique rank-0 child; every `EG_PContent` member shares rank 1
                // (confirmed against `mjx_ooxml_types::child_order::PARAGRAPH`), so a rank-0
                // insertion always belongs at index 0 — computed via the generated table rather than
                // hard-coded, exactly as `Run::run_properties_or_insert` does for `w:rPr`.
                let at = mjx_ooxml_types::child_order::PARAGRAPH.insert_index_of_names(
                    self.content.iter().map(|item| match item {
                        ParagraphContent::Properties(_) => Some(0),
                        ParagraphContent::Raw(_) => None,
                        _ => Some(1),
                    }),
                    "pPr",
                );
                self.content.insert(
                    at,
                    ParagraphContent::Properties(
                        super::paragraph_properties::ParagraphProperties::new(interner),
                    ),
                );
                self.empty = false;
                at
            }
        };
        match &mut self.content[at] {
            ParagraphContent::Properties(properties) => properties,
            _ => unreachable!("`at` was just found or inserted as a `Properties` item"),
        }
    }

    /// The paragraph's whole text: every run reachable from it, in document order — descending into
    /// every `w:hyperlink` and `w:fldSimple` it holds — concatenated with no separator (matching how
    /// the runs themselves concatenate on the page).
    #[must_use]
    pub fn text(&self) -> String {
        let mut text = String::new();
        paragraph_content_text(&self.content, &mut text);
        text
    }

    /// This paragraph's own content, immutably. `pub(crate)`: `fields.rs` (MJXOFF-121) walks this
    /// the same way [`Run::content`] lets it walk a run's own inner content — see that method's own
    /// doc comment.
    pub(crate) fn content(&self) -> &[ParagraphContent] {
        &self.content
    }

    /// [`Paragraph::content`], mutably.
    pub(crate) fn content_mut(&mut self) -> &mut Vec<ParagraphContent> {
        &mut self.content
    }

    /// How many run-or-hyperlink slots this paragraph holds at its **top level** — a `w:hyperlink`
    /// counts as one slot, its own runs are not included (mirrors
    /// `Presentation::shape_count`: "a group counts as one shape here; its own members ... are not
    /// included in this count"). Use a nested [`RunPath`] to reach inside one, or
    /// [`Paragraph::text`] to read every run's text regardless of nesting.
    #[must_use]
    pub fn run_count(&self) -> usize {
        run_slots(&self.content).count()
    }

    /// The run at `path`, or `None` if the address is out of range or lands on something that is
    /// not, in the end, a run (e.g. a bare top-level path to a `w:hyperlink`).
    #[must_use]
    pub fn run(&self, path: impl Into<RunPath>) -> Option<&Run> {
        resolve_run(&self.content, path.into().indices())
    }

    /// The run at `path`, mutably.
    pub fn run_mut(&mut self, path: impl Into<RunPath>) -> Option<&mut Run> {
        resolve_run_mut(&mut self.content, path.into().indices())
    }

    /// Inserts `run` at top-level slot `path`, shifting every run-or-hyperlink slot at or after that
    /// position one place later. Only a top-level (depth-1) `path` is accepted — inserting a new run
    /// *inside* an existing `w:hyperlink` is not this method's job (open it with
    /// [`Paragraph::run_mut`] on the hyperlink's own members once MJXOFF-121 gives `Hyperlink` an
    /// editing surface, or build the hyperlink's content before wrapping it).
    ///
    /// `path` must address an existing slot or the one past the last (`0..=run_count()`).
    ///
    /// Returns `false`, leaving `self` untouched, if `path` is out of range or not top-level.
    #[must_use]
    pub fn insert_run(&mut self, path: impl Into<RunPath>, run: Run) -> bool {
        let path = path.into();
        let [index] = path.indices() else {
            return false;
        };
        let count = self.run_count();
        if *index > count {
            return false;
        }
        let at = nth_slot_index(&self.content, *index).unwrap_or(self.content.len());
        self.content.insert(at, ParagraphContent::Run(run));
        true
    }

    /// Appends `run` as this paragraph's new last top-level run.
    pub fn append_run(&mut self, run: Run) {
        self.content.push(ParagraphContent::Run(run));
    }

    /// Removes and returns the run at `path`, or `None` if the address is out of range or does not,
    /// in the end, land on a run.
    pub fn remove_run(&mut self, path: impl Into<RunPath>) -> Option<Run> {
        Self::remove_run_at(&mut self.content, path.into().indices())
    }

    fn remove_run_at(content: &mut Vec<ParagraphContent>, indices: &[usize]) -> Option<Run> {
        let (&first, rest) = indices.split_first()?;
        let at = nth_slot_index(content, first)?;
        if rest.is_empty() {
            match content.get(at)? {
                ParagraphContent::Run(_) => match content.remove(at) {
                    ParagraphContent::Run(run) => Some(run),
                    _ => unreachable!("checked above"),
                },
                _ => None,
            }
        } else {
            match content.get_mut(at)? {
                ParagraphContent::Hyperlink(hyperlink) => {
                    Self::remove_run_at(&mut hyperlink.content, rest)
                }
                _ => None,
            }
        }
    }

    /// The **top-level** hyperlink at run-or-hyperlink slot `path`, or `None` if `path` is out of
    /// range, not top-level, or does not land on a hyperlink.
    #[must_use]
    pub(crate) fn hyperlink_at(&self, path: impl Into<RunPath>) -> Option<&Hyperlink> {
        let path = path.into();
        let [index] = path.indices() else {
            return None;
        };
        let at = nth_slot_index(&self.content, *index)?;
        match self.content.get(at)? {
            ParagraphContent::Hyperlink(hyperlink) => Some(hyperlink),
            _ => None,
        }
    }

    /// Inserts `hyperlink` at top-level run-or-hyperlink slot `path`, shifting every slot at or
    /// after that position one place later — [`Paragraph::insert_run`]'s own counterpart, for
    /// `crate::document::hyperlinks` (MJXOFF-121). Only a top-level path is accepted, for the same
    /// reason `insert_run` restricts itself to one.
    ///
    /// `path` must address an existing slot or the one past the last (`0..=run_count()`).
    ///
    /// Returns `false`, leaving `self` untouched, if `path` is out of range or not top-level.
    #[must_use]
    pub(crate) fn insert_hyperlink(
        &mut self,
        path: impl Into<RunPath>,
        hyperlink: Hyperlink,
    ) -> bool {
        let path = path.into();
        let [index] = path.indices() else {
            return false;
        };
        let count = self.run_count();
        if *index > count {
            return false;
        }
        let at = nth_slot_index(&self.content, *index).unwrap_or(self.content.len());
        self.content
            .insert(at, ParagraphContent::Hyperlink(hyperlink));
        true
    }

    /// Removes and returns the **top-level** hyperlink at run-or-hyperlink slot `path` — together
    /// with every run it wraps; a caller that wants to keep the wrapped text un-linked reads
    /// [`Hyperlink::content`] first and reinserts plain runs. `None` if `path` is out of range, not
    /// top-level, or does not land on a hyperlink (a bare run, say).
    pub(crate) fn remove_hyperlink(&mut self, path: impl Into<RunPath>) -> Option<Hyperlink> {
        let path = path.into();
        let [index] = path.indices() else {
            return None;
        };
        let at = nth_slot_index(&self.content, *index)?;
        match self.content.get(at)? {
            ParagraphContent::Hyperlink(_) => match self.content.remove(at) {
                ParagraphContent::Hyperlink(hyperlink) => Some(hyperlink),
                _ => unreachable!("checked above"),
            },
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Run (CT_R) and its content (EG_RunInnerContent, all 33 members)
// ---------------------------------------------------------------------------------------------

/// `w:r` (`CT_R`) — a run: optional run properties (`w:rPr`, `CT_RPr` — MJXOFF-94's own type), then
/// `EG_RunInnerContent*`.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct Run {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "rPr", variant = RunProperties, ty = super::run_properties::RunProperties),
        child(local = "br", variant = Break, ty = Break),
        child(local = "t", variant = Text, ty = Text),
        child(local = "contentPart", variant = ContentPart, ty = RelationshipReference),
        child(local = "delText", variant = DeletedText, ty = Text),
        child(local = "instrText", variant = FieldCode, ty = Text),
        child(local = "delInstrText", variant = DeletedFieldCode, ty = Text),
        child(local = "noBreakHyphen", variant = NonBreakingHyphen, ty = Unmodeled),
        child(local = "softHyphen", variant = OptionalHyphen, ty = Unmodeled),
        child(local = "dayShort", variant = ShortDay, ty = Unmodeled),
        child(local = "monthShort", variant = ShortMonth, ty = Unmodeled),
        child(local = "yearShort", variant = ShortYear, ty = Unmodeled),
        child(local = "dayLong", variant = LongDay, ty = Unmodeled),
        child(local = "monthLong", variant = LongMonth, ty = Unmodeled),
        child(local = "yearLong", variant = LongYear, ty = Unmodeled),
        child(local = "annotationRef", variant = CommentInformationBlock, ty = Unmodeled),
        child(local = "footnoteRef", variant = FootnoteReferenceMark, ty = Unmodeled),
        child(local = "endnoteRef", variant = EndnoteReferenceMark, ty = Unmodeled),
        child(local = "separator", variant = FootnoteEndnoteSeparatorMark, ty = Unmodeled),
        child(local = "continuationSeparator", variant = ContinuationSeparatorMark, ty = Unmodeled),
        child(local = "sym", variant = Symbol, ty = Symbol),
        child(local = "pgNum", variant = PageNumberBlock, ty = Unmodeled),
        child(local = "cr", variant = CarriageReturn, ty = Unmodeled),
        child(local = "tab", variant = TabCharacter, ty = Unmodeled),
        child(local = "object", variant = EmbeddedObject, ty = Unmodeled),
        child(local = "pict", variant = LegacyPicture, ty = Unmodeled),
        child(local = "fldChar", variant = ComplexFieldCharacter, ty = super::fields::FieldCharacter),
        child(local = "ruby", variant = PhoneticGuideRun, ty = PhoneticGuide),
        child(local = "footnoteReference", variant = FootnoteReference, ty = Unmodeled),
        child(local = "endnoteReference", variant = EndnoteReference, ty = Unmodeled),
        child(local = "commentReference", variant = CommentReference, ty = Unmodeled),
        child(local = "drawing", variant = Drawing, ty = Unmodeled),
        child(local = "ptab", variant = PositionalTabRun, ty = PositionalTab),
        child(local = "lastRenderedPageBreak", variant = LastRenderedPageBreak, ty = Unmodeled)
    )]
    content: Vec<RunInnerContent>,
}

/// One ordered child of a [`Run`]: `EG_RunInnerContent`'s 33 members, plus `w:rPr` itself —
/// `CT_R`'s content is `rPr?, EG_RunInnerContent*`, so `rPr` is not technically a member of the
/// group, but it is `CT_R`'s own first child and MJXOFF-94 types it here rather than adding a second
/// vector to [`Run`] for one optional element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunInnerContent {
    /// `w:rPr` (`CT_RPr`, "Run Properties", §17.3.2.28) — MJXOFF-94's own type; see
    /// [`RunProperties`](crate::RunProperties).
    RunProperties(super::run_properties::RunProperties),
    /// `w:br` (`CT_Br`, "Break", §17.3.3.1) — this child's own type.
    Break(Break),
    /// `w:t` (`CT_Text`, "Text", §17.3.3.31) — this child's own type. The run's visible text.
    Text(Text),
    /// `w:contentPart` (`CT_Rel`, "Content Part", §17.3.3.2) — a reference to an external content
    /// part (e.g. ink); typed only far enough to read its relationship id.
    ContentPart(RelationshipReference),
    /// `w:delText` (`CT_Text`, "Deleted Text", §17.3.3.7) — text a tracked deletion removed. The
    /// revision semantics that mark it deleted are MJXOFF-126; the text itself is [`Text`].
    DeletedText(Text),
    /// `w:instrText` (`CT_Text`, "Field Code", §17.16.23) — a field's instruction source.
    FieldCode(Text),
    /// `w:delInstrText` (`CT_Text`, "Deleted Field Code", §17.16.13) — a deleted field's instruction
    /// source.
    DeletedFieldCode(Text),
    /// `w:noBreakHyphen` (`CT_Empty`, "Non Breaking Hyphen Character", §17.3.3.18) — `CT_Empty` has
    /// no content to type.
    NonBreakingHyphen(Unmodeled),
    /// `w:softHyphen` (`CT_Empty`, "Optional Hyphen Character", §17.3.3.29).
    OptionalHyphen(Unmodeled),
    /// `w:dayShort` (`CT_Empty`, "Date Block - Short Day Format", §17.3.3.6).
    ShortDay(Unmodeled),
    /// `w:monthShort` (`CT_Empty`, "Date Block - Short Month Format", §17.3.3.16).
    ShortMonth(Unmodeled),
    /// `w:yearShort` (`CT_Empty`, "Date Block - Short Year Format", §17.3.3.34).
    ShortYear(Unmodeled),
    /// `w:dayLong` (`CT_Empty`, "Date Block - Long Day Format", §17.3.3.5).
    LongDay(Unmodeled),
    /// `w:monthLong` (`CT_Empty`, "Date Block - Long Month Format", §17.3.3.15).
    LongMonth(Unmodeled),
    /// `w:yearLong` (`CT_Empty`, "Date Block - Long Year Format", §17.3.3.33).
    LongYear(Unmodeled),
    /// `w:annotationRef` (`CT_Empty`, "Comment Information Block", §17.13.4.1).
    CommentInformationBlock(Unmodeled),
    /// `w:footnoteRef` (`CT_Empty`, "Footnote Reference Mark", §17.11.13).
    FootnoteReferenceMark(Unmodeled),
    /// `w:endnoteRef` (`CT_Empty`, "Endnote Reference Mark", §17.11.6).
    EndnoteReferenceMark(Unmodeled),
    /// `w:separator` (`CT_Empty`, "Footnote/Endnote Separator Mark", §17.11.23).
    FootnoteEndnoteSeparatorMark(Unmodeled),
    /// `w:continuationSeparator` (`CT_Empty`, "Continuation Separator Mark", §17.11.1).
    ContinuationSeparatorMark(Unmodeled),
    /// `w:sym` (`CT_Sym`, "Symbol Character", §17.3.3.30) — this child's own type.
    Symbol(Symbol),
    /// `w:pgNum` (`CT_Empty`, "Page Number Block", §17.3.3.22).
    PageNumberBlock(Unmodeled),
    /// `w:cr` (`CT_Empty`, "Carriage Return", §17.3.3.4).
    CarriageReturn(Unmodeled),
    /// `w:tab` (`CT_Empty`, "Tab Character", §17.3.3.32).
    TabCharacter(Unmodeled),
    /// `w:object` (`CT_Object`, "Embedded Object", §17.3.3.19) — MJXOFF-131 (C16) owns embedded
    /// object payloads.
    EmbeddedObject(Unmodeled),
    /// `w:pict` (`CT_Picture`) — a legacy VML picture. ECMA-376 Part 1 does not caption this element
    /// in its own numbered list (unlike its 32 siblings); named for the VML/legacy picture content
    /// it wraps rather than sourced from prose that does not exist. MJXOFF-131 (C16) owns it.
    LegacyPicture(Unmodeled),
    /// `w:fldChar` (`CT_FldChar`, "Complex Field Character", §17.16.18) — MJXOFF-121's own type; see
    /// [`super::fields::FieldCharacter`].
    ComplexFieldCharacter(super::fields::FieldCharacter),
    /// `w:ruby` (`CT_Ruby`, "Phonetic Guide", §17.3.3.25) — this child's own type.
    PhoneticGuideRun(PhoneticGuide),
    /// `w:footnoteReference` (`CT_FtnEdnRef`, "Footnote Reference", §17.11.14) — MJXOFF-090's Phase C
    /// plan names footnotes/endnotes their own later child; kept opaque here.
    FootnoteReference(Unmodeled),
    /// `w:endnoteReference` (`CT_FtnEdnRef`, "Endnote Reference", §17.11.7) — see
    /// [`FootnoteReference`](Self::FootnoteReference).
    EndnoteReference(Unmodeled),
    /// `w:commentReference` (`CT_Markup`, "Comment Content Reference Mark", §17.13.4.5) — comments
    /// are a later child's ("annotations.rs") subject; kept opaque here.
    CommentReference(Unmodeled),
    /// `w:drawing` (`CT_Drawing`, "DrawingML Object", §17.3.3.9) — MJXOFF-131 (C16) owns DrawingML
    /// hosted in Word.
    Drawing(Unmodeled),
    /// `w:ptab` (`CT_PTab`, "Absolute Position Tab Character", §17.3.3.23) — this child's own type.
    PositionalTabRun(PositionalTab),
    /// `w:lastRenderedPageBreak` (`CT_Empty`, "Position of Last Calculated Page Break", §17.3.3.13).
    LastRenderedPageBreak(Unmodeled),
    /// Any other child — an unknown element, or `w:rPr` from a non-conformant file that repeats it —
    /// preserved verbatim.
    Raw(RawNode),
}

impl Run {
    /// This run's own inner content (`EG_RunInnerContent`, all 33 members plus `w:rPr` itself, see
    /// [`RunInnerContent`]'s own doc comment), immutably, in document order. `pub(crate)`:
    /// `fields.rs` (MJXOFF-121) scans this for `w:fldChar`/`w:instrText`/`w:t` items when pairing a
    /// field's markers, the same way `table_regions.rs` (MJXOFF-119) reaches into a cell's own
    /// properties through an accessor this module grew for exactly that purpose.
    pub(crate) fn content(&self) -> &[RunInnerContent] {
        &self.content
    }

    /// [`Run::content`], mutably — `fields.rs` uses this to rewrite a field's instruction or cached
    /// result in place without reshaping [`RunInnerContent`] itself.
    pub(crate) fn content_mut(&mut self) -> &mut Vec<RunInnerContent> {
        &mut self.content
    }

    /// This run's properties (`w:rPr`), or `None` if it carries none.
    #[must_use]
    pub fn run_properties(&self) -> Option<&super::run_properties::RunProperties> {
        self.content.iter().find_map(|item| match item {
            RunInnerContent::RunProperties(properties) => Some(properties),
            _ => None,
        })
    }

    /// This run's properties, mutably, or `None` if it carries none — see
    /// [`Run::run_properties_or_insert`] to create one.
    pub fn run_properties_mut(&mut self) -> Option<&mut super::run_properties::RunProperties> {
        self.content.iter_mut().find_map(|item| match item {
            RunInnerContent::RunProperties(properties) => Some(properties),
            _ => None,
        })
    }

    /// This run's properties, mutably — creating an empty `w:rPr` at its schema rank (always the
    /// first child of `w:r`, per `CT_R`'s `xsd:sequence`) if this run does not already carry one.
    pub fn run_properties_or_insert(
        &mut self,
        interner: &mut Interner,
    ) -> &mut super::run_properties::RunProperties {
        let at = match self
            .content
            .iter()
            .position(|item| matches!(item, RunInnerContent::RunProperties(_)))
        {
            Some(at) => at,
            None => {
                // `rPr` is `CT_R`'s unique rank-0 child; every `EG_RunInnerContent` member shares
                // rank 1 (confirmed against `mjx_ooxml_types::child_order::RUN`), so a rank-0
                // insertion always belongs at index 0 — computed via the generated table rather than
                // hard-coded, so a schema change would move this insertion point too.
                let at = mjx_ooxml_types::child_order::RUN.insert_index_of_names(
                    self.content.iter().map(|item| match item {
                        RunInnerContent::RunProperties(_) => Some(0),
                        RunInnerContent::Raw(_) => None,
                        _ => Some(1),
                    }),
                    "rPr",
                );
                self.content.insert(
                    at,
                    RunInnerContent::RunProperties(super::run_properties::RunProperties::new(
                        interner,
                    )),
                );
                self.empty = false;
                at
            }
        };
        match &mut self.content[at] {
            RunInnerContent::RunProperties(properties) => properties,
            _ => unreachable!("`at` was just found or inserted as a `RunProperties` item"),
        }
    }

    /// The run's text: every [`RunInnerContent::Text`] (`w:t`) it holds, concatenated in document
    /// order — `""` if it holds none. Deliberately **not** `w:delText`/`w:instrText`: those are
    /// different content (deleted text, a field's source), not what the run displays.
    #[must_use]
    pub fn text(&self) -> String {
        let mut text = String::new();
        for item in &self.content {
            if let RunInnerContent::Text(t) = item {
                text.push_str(t.text());
            }
        }
        text
    }

    /// Sets the run's text: replaces the first `w:t` this run holds, or appends a new one (after
    /// every existing child — `EG_RunInnerContent` is an `xsd:choice`, so nothing here has an
    /// ordering constraint to respect) when it holds none.
    pub fn set_text(&mut self, interner: &mut Interner, text: &str) {
        for item in &mut self.content {
            if let RunInnerContent::Text(t) = item {
                t.set_text(interner, text);
                return;
            }
        }
        let mut t = Text::new(interner);
        t.set_text(interner, text);
        self.content.push(RunInnerContent::Text(t));
    }

    /// Builds a new run holding one `w:t` with `text`, ready to insert or append into a
    /// [`Paragraph`].
    #[must_use]
    pub fn with_text(interner: &mut Interner, text: &str) -> Self {
        let name = wml_name(interner, "r");
        let mut t = Text::new(interner);
        t.set_text(interner, text);
        Self {
            name,
            attributes: Vec::new(),
            empty: false,
            content: vec![RunInnerContent::Text(t)],
        }
    }

    /// Builds a new run holding one `w:instrText` with `text` — `fields.rs`'s (MJXOFF-121) own
    /// counterpart to [`Run::with_text`], used when collapsing a complex field's instruction to a
    /// single run.
    pub(crate) fn with_field_code(interner: &mut Interner, text: &str) -> Self {
        let name = wml_name(interner, "r");
        let mut t = Text::with_local(interner, "instrText");
        t.set_text(interner, text);
        Self {
            name,
            attributes: Vec::new(),
            empty: false,
            content: vec![RunInnerContent::FieldCode(t)],
        }
    }
}

// ---------------------------------------------------------------------------------------------
// w:t (CT_Text) — reused for t / delText / instrText / delInstrText
// ---------------------------------------------------------------------------------------------

/// `xml:space` as read/written on [`Text`]: the two values W3C's XML 1.0 §2.10 defines for it.
/// [`AttributeCodec::decode`] never rejects a spelling that is not one of the two — an attribute
/// value comes from an untrusted file — and [`AttributeCodec::encode`] writes exactly `preserve` or
/// `default`, the only two spellings that exist.
///
/// `pub` (and re-exported), not merely visible inside this module: the generated accessor
/// ([`Text::preserve_whitespace`]) names this type in its return type through
/// [`AttributeCodec::Value`], and a private type there is a hard compile error (`E0446`, "private
/// type in public interface") for any caller outside this crate — the same defect MJXOFF-94 found
/// and fixed for `run_properties.rs`'s own seven tag types, confirmed here by
/// `crates/mjx-docx/tests/leaf_attributes.rs`, a separate compilation unit.
#[derive(Debug)]
pub struct WhitespacePreservation;

impl AttributeCodec for WhitespacePreservation {
    type Value<'a> = bool;
    type Input<'a> = bool;

    fn decode<'a>(raw: std::borrow::Cow<'a, str>) -> Result<bool, InvalidAttributeValue> {
        match raw.as_ref() {
            "preserve" => Ok(true),
            "default" => Ok(false),
            other => Err(InvalidAttributeValue::new(format!(
                "expected \"preserve\" or \"default\", found {other:?}"
            ))),
        }
    }

    fn encode<'a>(value: bool) -> std::borrow::Cow<'a, str> {
        std::borrow::Cow::Borrowed(if value { "preserve" } else { "default" })
    }
}

/// `CT_Text` — an `xsd:string` with optional `xml:space`: `w:t` ("Text"), `w:delText` ("Deleted
/// Text"), `w:instrText` ("Field Code") and `w:delInstrText` ("Deleted Field Code") all share this
/// shape, so one type serves all four `EG_RunInnerContent` members — each keeps its own wire name
/// (captured in `name` on read, or set explicitly by [`Text::new`]) and reads back under whichever
/// [`RunInnerContent`] variant its parent matched it into.
///
/// # The `xml:space` rule
///
/// **Read never trims.** [`Text::text`] returns exactly the decoded character data, regardless of
/// whether `xml:space="preserve"` is present — an untouched file's significant whitespace survives
/// because nothing here ever normalizes on read (the same contract every typed attribute in this
/// workspace keeps).
///
/// **[`Text::set_text`] is the one write path, and it manages `xml:space` for the caller.** After
/// setting the text, it writes `xml:space="preserve"` when the new string starts or ends with ASCII
/// whitespace, and removes `xml:space` entirely otherwise. Two failure modes are what this guards
/// against, symmetrically: writing whitespace-bearing text without the attribute silently loses it
/// the next time something whitespace-collapses the file (the W3C rule `xml:space`'s own prose in
/// ECMA-376 §17.3.3.31 names — "that whitespace ... is subject to the space preservation rules
/// currently specified in that run's scope"); and leaving the attribute on text that no longer needs
/// it churns markup a caller did not ask to touch. Both directions are exercised in
/// `crates/mjx-docx/tests/roundtrip.rs`.
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(attribute(local = "space", prefix = "xml", codec = WhitespacePreservation, accessor = preserve_whitespace))]
pub struct Text {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(text)]
    text: String,
}

impl Text {
    /// Builds an empty `w:t`, ready for [`Text::set_text`].
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self::with_local(interner, "t")
    }

    /// [`Text::new`], for any of the other three `EG_RunInnerContent` members this type serves
    /// (`"delText"`, `"instrText"`, `"delInstrText"`) — the wire name is whichever `RunInnerContent`
    /// variant the caller means to build one for, since [`ToXml`] writes exactly the `name` stored
    /// here regardless of which enum variant wraps the value. `pub(crate)`: `fields.rs` (MJXOFF-121)
    /// is the first caller that ever constructs a fresh `w:instrText`/`w:delInstrText` rather than
    /// only reading one an existing file already carried.
    #[must_use]
    pub(crate) fn with_local(interner: &mut Interner, local: &str) -> Self {
        Self {
            name: wml_name(interner, local),
            attributes: Vec::new(),
            empty: true,
            text: String::new(),
        }
    }

    /// The decoded text content — never trimmed; see the type's own doc comment.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Replaces the text content and applies the `xml:space` rule — see the type's own doc comment.
    pub fn set_text(&mut self, interner: &mut Interner, text: &str) {
        self.text = text.to_owned();
        self.empty = self.text.is_empty();
        let needs_preserve = text.starts_with(|c: char| c.is_ascii_whitespace())
            || text.ends_with(|c: char| c.is_ascii_whitespace());
        self.set_preserve_whitespace(interner, needs_preserve.then_some(true));
    }
}

// ---------------------------------------------------------------------------------------------
// Small attribute-only leaves: CT_Br, CT_PTab, CT_Sym, CT_ProofErr, CT_Perm, CT_PermStart, CT_Rel
// ---------------------------------------------------------------------------------------------

/// `CT_Br` (`w:br`, "Break") — an optional break type and an optional text-wrapping restart value.
/// No children per the schema; any this crate did not expect are preserved in `extra` regardless.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "type", prefix = "w", codec = Enumeration<BreakType>, accessor = kind))]
#[xml(attribute(local = "clear", prefix = "w", codec = Enumeration<BreakTextWrappingRestart>, accessor = clear))]
pub struct Break {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl FromXml for Break {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for Break {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_PTab` (`w:ptab`, "Absolute Position Tab Character") — alignment, base and leader, all
/// required by the schema.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "alignment", prefix = "w", codec = Enumeration<PositionalTabAlignment>, accessor = alignment, required))]
#[xml(attribute(local = "relativeTo", prefix = "w", codec = Enumeration<PositionalTabBase>, accessor = relative_to, required))]
#[xml(attribute(local = "leader", prefix = "w", codec = Enumeration<PositionalTabLeader>, accessor = leader, required))]
pub struct PositionalTab {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl FromXml for PositionalTab {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for PositionalTab {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `ST_ShortHexNumber` as an attribute value: two bytes (four hex digits), read and written as
/// [`FourDigitHexadecimalNumber`] verbatim — like [`WhitespacePreservation`], this never rejects on
/// read (an untrusted file's malformed `char` is still a string this codec can hand back), because
/// `mjx-ooxml-types` generated the wrapper type as a plain wire-string carrier with no validation of
/// its own; a stricter reading is a schema-gate concern (`ST_ShortHexNumber`'s own `xsd:length`
/// restriction), not this accessor's.
///
/// `pub` (and re-exported) for the same reason [`WhitespacePreservation`] is — [`Symbol::character`]
/// names this type in its return type, and a private type there does not compile from outside this
/// crate.
#[derive(Debug)]
pub struct ShortHex;

impl AttributeCodec for ShortHex {
    type Value<'a> = FourDigitHexadecimalNumber;
    type Input<'a> = FourDigitHexadecimalNumber;

    fn decode<'a>(
        raw: std::borrow::Cow<'a, str>,
    ) -> Result<FourDigitHexadecimalNumber, InvalidAttributeValue> {
        Ok(FourDigitHexadecimalNumber::from_wire(&raw))
    }

    fn encode<'a>(value: FourDigitHexadecimalNumber) -> std::borrow::Cow<'a, str> {
        std::borrow::Cow::Owned(value.to_wire().to_owned())
    }
}

/// `CT_Sym` (`w:sym`, "Symbol Character") — an optional font name and an optional character code.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "font", prefix = "w", codec = TextCodec, accessor = font))]
#[xml(attribute(local = "char", prefix = "w", codec = ShortHex, accessor = character))]
pub struct Symbol {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl FromXml for Symbol {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for Symbol {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_ProofErr` (`w:proofErr`, "Proofing Error Anchor") — the one required attribute naming which
/// kind of proofing anchor this is.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "type", prefix = "w", codec = Enumeration<ProofingErrorType>, accessor = error_type, required))]
pub struct ProofingError {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl FromXml for ProofingError {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for ProofingError {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_Perm` (`w:permEnd`, "Range Permission End") — a required id and an optional
/// "displaced by custom XML" marker.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "id", prefix = "w", codec = TextCodec, accessor = id, required))]
#[xml(attribute(local = "displacedByCustomXml", prefix = "w", codec = Enumeration<DisplacedByCustomXml>, accessor = displaced_by_custom_xml))]
pub struct PermissionRangeEnd {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl FromXml for PermissionRangeEnd {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for PermissionRangeEnd {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_PermStart` (`w:permStart`, "Range Permission Start") — `CT_Perm`'s two attributes
/// (`xsd:complexContent`/`xsd:extension`) plus four of its own.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "id", prefix = "w", codec = TextCodec, accessor = id, required))]
#[xml(attribute(local = "displacedByCustomXml", prefix = "w", codec = Enumeration<DisplacedByCustomXml>, accessor = displaced_by_custom_xml))]
#[xml(attribute(local = "edGrp", prefix = "w", codec = Enumeration<EditingGroup>, accessor = editing_group))]
#[xml(attribute(local = "ed", prefix = "w", codec = TextCodec, accessor = editor))]
#[xml(attribute(local = "colFirst", prefix = "w", codec = Number<DecimalNumber>, accessor = first_column))]
#[xml(attribute(local = "colLast", prefix = "w", codec = Number<DecimalNumber>, accessor = last_column))]
pub struct PermissionRangeStart {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl FromXml for PermissionRangeStart {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for PermissionRangeStart {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_Rel` — one required relationship id. Reused for `w:contentPart` and `w:subDoc`, the two
/// `EG_RunInnerContent`/`EG_PContent` members whose whole content model is this attribute.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "id", prefix = "r", codec = TextCodec, accessor = relationship_id, required))]
pub struct RelationshipReference {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl RelationshipReference {
    /// Builds a new `local` reference (`"contentPart"`, `"subDoc"`, `"printerSettings"`, …) pointing
    /// at `relationship_id`. Building the value here never creates the part or the relationship it
    /// names — that is the caller's own job through [`mjx_opc::Package`], exactly as
    /// [`super::sections::SectionProperties::printer_settings`]'s own doc comment says for the one
    /// reuse this constructor exists for.
    #[must_use]
    pub fn new(interner: &mut Interner, local: &str, relationship_id: &str) -> Self {
        let mut value = Self {
            name: wml_name(interner, local),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        value.set_relationship_id(interner, relationship_id);
        value
    }
}

impl FromXml for RelationshipReference {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for RelationshipReference {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// ---------------------------------------------------------------------------------------------
// w:ruby (CT_Ruby, CT_RubyPr, CT_RubyAlign, CT_RubyContent)
// ---------------------------------------------------------------------------------------------

/// `CT_RubyAlign` (`w:rubyAlign`, "Phonetic Guide Text Alignment") — the one required `val`.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<PhoneticGuideAlignment>, accessor = value, required))]
pub struct PhoneticGuideTextAlignment {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl FromXml for PhoneticGuideTextAlignment {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for PhoneticGuideTextAlignment {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// One ordered child of [`PhoneticGuideProperties`]: `CT_RubyPr`'s sequence is `rubyAlign, hps,
/// hpsRaise, hpsBaseText, lid, dirty?` — only `rubyAlign` is named in MJXOFF-92's scope, so it is the
/// one typed member; the font-size and language elements a real ruby always carries stay
/// [`Raw`](PhoneticGuidePropertyContent::Raw).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhoneticGuidePropertyContent {
    /// `w:rubyAlign` (`CT_RubyAlign`).
    Alignment(PhoneticGuideTextAlignment),
    /// `w:hps`/`w:hpsRaise`/`w:hpsBaseText`/`w:lid`/`w:dirty` — preserved verbatim.
    Raw(RawNode),
}

/// `CT_RubyPr` (`w:rubyPr`, "Phonetic Guide Properties").
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct PhoneticGuideProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "rubyAlign", variant = Alignment, ty = PhoneticGuideTextAlignment)
    )]
    content: Vec<PhoneticGuidePropertyContent>,
}

impl PhoneticGuideProperties {
    /// The ruby's text alignment (`w:rubyAlign`), or `None` if this properties element does not
    /// carry one (illegal per the schema — `rubyAlign` is required — but a malformed file is read,
    /// not panicked on).
    #[must_use]
    pub fn alignment(&self) -> Option<&PhoneticGuideTextAlignment> {
        self.content.iter().find_map(|item| match item {
            PhoneticGuidePropertyContent::Alignment(alignment) => Some(alignment),
            PhoneticGuidePropertyContent::Raw(_) => None,
        })
    }
}

/// One ordered child of [`PhoneticGuideContent`]: `EG_RubyContent` is `r | EG_RunLevelElts*`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhoneticGuideContentItem {
    /// `w:r` (`CT_R`).
    Run(Run),
    /// A folded-in `EG_RunLevelElts` member this child does not type, or anything unmatched.
    Raw(RawNode),
}

/// `CT_RubyContent` — the content model shared by `w:rt` ("Phonetic Guide Text") and `w:rubyBase`
/// ("Phonetic Guide Base Text"); which one a given value came from is `name`, not the Rust type.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct PhoneticGuideContent {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "r", variant = Run, ty = Run))]
    content: Vec<PhoneticGuideContentItem>,
}

impl PhoneticGuideContent {
    /// Every run this content holds, in document order.
    pub fn runs(&self) -> impl Iterator<Item = &Run> {
        self.content.iter().filter_map(|item| match item {
            PhoneticGuideContentItem::Run(run) => Some(run),
            PhoneticGuideContentItem::Raw(_) => None,
        })
    }
}

/// One ordered child of a [`PhoneticGuide`]: `CT_Ruby`'s sequence is `rubyPr, rt, rubyBase`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhoneticGuideChild {
    /// `w:rubyPr`.
    Properties(PhoneticGuideProperties),
    /// `w:rt` — the phonetic guide text itself.
    Text(PhoneticGuideContent),
    /// `w:rubyBase` — the text the guide annotates.
    Base(PhoneticGuideContent),
    /// Anything unmatched.
    Raw(RawNode),
}

/// `CT_Ruby` (`w:ruby`, "Phonetic Guide") — properties, the guide text, and the base text it
/// annotates.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct PhoneticGuide {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "rubyPr", variant = Properties, ty = PhoneticGuideProperties),
        child(local = "rt", variant = Text, ty = PhoneticGuideContent),
        child(local = "rubyBase", variant = Base, ty = PhoneticGuideContent)
    )]
    content: Vec<PhoneticGuideChild>,
}

impl PhoneticGuide {
    /// The ruby's properties (`w:rubyPr`), or `None` if it carries none (illegal per the schema, but
    /// a malformed file is read, not panicked on).
    #[must_use]
    pub fn properties(&self) -> Option<&PhoneticGuideProperties> {
        self.content.iter().find_map(|item| match item {
            PhoneticGuideChild::Properties(properties) => Some(properties),
            _ => None,
        })
    }

    /// The phonetic guide text (`w:rt`), or `None` if it carries none.
    #[must_use]
    pub fn guide_text(&self) -> Option<&PhoneticGuideContent> {
        self.content.iter().find_map(|item| match item {
            PhoneticGuideChild::Text(text) => Some(text),
            _ => None,
        })
    }

    /// The base text the guide annotates (`w:rubyBase`), or `None` if it carries none.
    #[must_use]
    pub fn base_text(&self) -> Option<&PhoneticGuideContent> {
        self.content.iter().find_map(|item| match item {
            PhoneticGuideChild::Base(base) => Some(base),
            _ => None,
        })
    }
}

// ---------------------------------------------------------------------------------------------
// Shared opaque leaf
// ---------------------------------------------------------------------------------------------

/// An element this crate does not (yet) model the content of: its name, attributes, self-closing
/// flag and every child are preserved verbatim, exactly as `mjx-dml`'s `GeometryGuide`/`TextBody`
/// document for a leaf or a subtree with no typed content. Used for two different reasons across
/// this module — the schema genuinely gives the element no content to type (the sixteen
/// `CT_Empty`-based `EG_RunInnerContent` members), or a later child owns the payload (`w:fldChar`,
/// `w:object`, `w:pict`, `w:drawing`, the footnote/endnote/comment references, `w:tbl`,
/// `w:customXml`, `w:sdt`, `w:smartTag`, `w:dir`, `w:bdo`, `w:fldSimple`, `w:sectPr`) — which reason
/// applies is documented on each [`RunInnerContent`]/[`ParagraphContent`]/[`BlockContent`] variant
/// that carries one, not on this type itself. `w:sectPr` is no longer one of them (MJXOFF-109 gave
/// it a real type, `super::sections::SectionProperties`) — its own `w:footnotePr`/`w:endnotePr`/
/// `w:sectPrChange` children still are, each documented on [`super::sections::SectionPropertyContent`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unmodeled {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

impl Unmodeled {
    /// Builds a new, empty `local` element — every attribute and child absent until something
    /// writes one directly through [`Unmodeled::attributes_mut`] (the escape hatch a sibling module
    /// that needs to layer typed accessors on top of an otherwise-opaque element uses — see
    /// `sections.rs`'s `SectionProperties::page_size`/`page_margins` for the one caller).
    #[must_use]
    pub(crate) fn new(interner: &mut Interner, local: &str) -> Self {
        Self {
            name: wml_name(interner, local),
            attributes: Vec::new(),
            children: Vec::new(),
            empty: true,
        }
    }

    /// This element's raw attribute list — crate-visible so a sibling module can layer a typed
    /// accessor (`mjx_xml::attribute::read`) on top of an element this type otherwise leaves
    /// opaque, without this type growing accessors of its own for every caller's own concept.
    pub(crate) fn attributes(&self) -> &[RawAttribute] {
        &self.attributes
    }

    /// [`Unmodeled::attributes`], mutably.
    pub(crate) fn attributes_mut(&mut self) -> &mut Vec<RawAttribute> {
        &mut self.attributes
    }
}

impl FromXml for Unmodeled {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            children: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for Unmodeled {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.children.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:background` (`CT_Background`) — a document's or glossary document's page background, the one
/// child `CT_DocumentBase` contributes (`CT_DocumentBase` itself has no serialized form: it is
/// spliced into `CT_Document`/`CT_GlossaryDocument` by `xsd:complexContent`/`xsd:extension`, never a
/// wire element of its own — see `xtask/src/codegen/complex.rs`).
///
/// **Skeleton**, for the same reason [`Unmodeled`] exists: `CT_Background`'s own content (a
/// repeating choice of VML/Office-drawing wildcards, then an optional `w:drawing`) and its four
/// color attributes are real modeling work nobody has claimed yet. `tests/fixtures/sample.docx`'s
/// `w:document` carries no `w:background` at all, so this type is exercised by the schema's own
/// permission for it to be absent, not by that fixture's content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Background {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

impl FromXml for Background {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            children: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for Background {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.children.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}
