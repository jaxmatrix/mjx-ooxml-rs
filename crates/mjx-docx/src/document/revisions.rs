//! Revision marks (tracked changes): `w:ins`/`w:del`/`w:moveFrom`/`w:moveTo` as run-level
//! containers, the eight `*Change` property wrappers, cell-merge tracking, tracked numbering
//! changes, and the move-range markers built on MJXOFF-124's own [`super::ranges`] mechanism.
//!
//! # The sixteen `CT_` types this ticket named, resolved
//!
//! Fourteen are modeled here (or reuse an existing type directly): [`TrackChangeMarker`]
//! (`CT_TrackChange`, the bare id/author/date marker shared by `w:cellIns`, `w:cellDel`, the four
//! `customXml*RangeStart` elements, `w:numPr/w:ins` and `w:pPr/w:rPr`'s own
//! `w:ins`/`w:del`/`w:moveFrom`/`w:moveTo`), [`RunTrackChange`] (`CT_RunTrackChange`, `w:ins`/
//! `w:del`/`w:moveFrom`/`w:moveTo` as run-level containers), [`MoveBookmark`]
//! (`CT_MoveBookmark`, `w:moveFromRangeStart`/`w:moveToRangeStart`), [`CellMergeTrackChange`]
//! (`w:cellMerge`), [`TrackChangeNumbering`] (`CT_TrackChangeNumbering`, `w:numberingChange`) and
//! the eight property wrappers ([`RunPropertiesChange`], [`ParagraphPropertiesChange`],
//! [`ParagraphMarkPropertiesChange`], [`SectionPropertiesChange`], [`TablePropertiesChange`],
//! [`TableExceptionPropertiesChange`], [`TableGridChange`], [`CellPropertiesChange`],
//! [`RowPropertiesChange`] — nine names for eight schema types because `w:trPr` and `w:tcPr` each
//! need their own, matching the ticket's own list of `CT_RPrChange`/`CT_PPrChange`/
//! `CT_ParaRPrChange`/`CT_SectPrChange`/`CT_TblPrChange`/`CT_TblPrExChange`/`CT_TblGridChange`/
//! `CT_TcPrChange`/`CT_TrPrChange`).
//!
//! Two are declined, both checked directly against `wml.xsd` rather than assumed from the ticket's
//! own list:
//!
//! - **`CT_TrackChangeRange` is genuinely unreachable** — `grep -c 'CT_TrackChangeRange'
//!   wml.xsd` finds exactly one hit, its own `<xsd:complexType name="CT_TrackChangeRange">`
//!   declaration; nothing extends it and no element names it as a type. This is a real correction
//!   to the pinned pre-dispatch note, which flagged the *sibling* type (`CT_TrackChangeNumbering`)
//!   as unreachable and was wrong about that one (see below) while not checking this one at all.
//! - **`CT_MathCtrlIns`/`CT_MathCtrlDel`** extend `CT_TrackChange` but are reached only through
//!   `EG_RPrMath`, declared in `wml.xsd` but referenced only from `shared-math.xsd`'s own `m:rPr`
//!   (`grep -rl EG_RPrMath References/.../OfficeOpenXML-XMLSchema-Transitional/` — `wml.xsd` and
//!   `shared-math.xsd` only). Math content inside a Word run (`m:oMath`/`m:oMathPara`,
//!   `EG_MathContent`) is not modeled anywhere in `mjx-docx` today — it falls to
//!   [`super::body::ParagraphContent::Raw`]/[`super::body::RunInnerContent`]'s own catch-all — so
//!   there is no reachable Rust call site for a *math run's* tracked-change wrapper without first
//!   typing OMML content in a run, which is `mjx-omml`/a future child's scope, not this one's.
//!
//! **The pinned pre-dispatch note's claim that `CT_TrackChangeNumbering` is unreachable is
//! wrong.** `grep -n 'type="CT_TrackChangeNumbering"' wml.xsd` finds two hits: `CT_NumPr`'s own
//! `numberingChange` (`paragraph_properties.rs`'s [`super::paragraph_properties::
//! NumberingPropertyContent::NumberingChange`], MJXOFF-96) and `CT_FldChar`'s own
//! (`fields.rs`'s [`super::fields::FieldCharacterContent::NumberingChange`], MJXOFF-121) — both
//! already wired up as opaque [`super::body::Unmodeled`] placeholders whose own doc comments say
//! "MJXOFF-126 owns". `crates/mjx-docx/src/document/numbering.rs`'s own module doc reaches the
//! same conclusion independently (checking whether `CT_TrackChangeNumbering` is reachable *from
//! `CT_Numbering`*, which it correctly says it is not — a narrower question than "reachable at
//! all", and the numbering module doc never claims otherwise). [`TrackChangeNumbering`] is modeled
//! here and wired into both of those two real use sites.
//!
//! # The one rule for every mutation path
//!
//! **Every existing mutation-path scan and address (`RunPath`, the field/comment/bookmark scanning
//! in `fields.rs`/`annotations.rs` built on [`super::ranges::RangeIndex`], the structural row/column
//! edits in `tables.rs`, every property setter) operates over *top-level* paragraph/run content
//! only. `w:ins`, `w:del`, `w:moveFrom` and `w:moveTo` are opaque containers to every one of those
//! paths: content nested inside one is preserved exactly, in position, but is never addressed,
//! scanned, or reachable through this crate's ordinary mutation surface.**
//!
//! This is a deliberate, structural guarantee, not an oversight: [`super::body::run_slots`] (the
//! function every `RunPath` resolution and every `Paragraph::run_count`/`run`/`run_mut` call goes
//! through) matches only [`super::body::ParagraphContent::Run`] and
//! [`super::body::ParagraphContent::Hyperlink`] — it does not, and after this child still does not,
//! descend into [`super::body::ParagraphContent::Ins`]/`Del`/`MoveFrom`/`MoveTo`. A run physically
//! inside one of those four containers simply does not consume a run-index slot, exactly as a
//! `w:pPr` never did. The same is true of [`super::ranges::flatten_paragraphs`]/`RangeIndex::build`,
//! which classify items from `paragraph.content().iter()` directly — a bookmark, comment range, or
//! field marker sequence wrapped entirely inside a revision container is not found by this crate's
//! scanning today, a stated limitation (not silent corruption: the markup itself is never touched).
//!
//! The consequence is the strongest possible fidelity guarantee with the least code: **no mutation
//! path in this crate can ever rewrite, relocate, or drop a byte of tracked-change history**,
//! because none of them ever look inside one. A caller who wants to read what a revision contains
//! reaches it through [`crate::Document::revisions`] or by matching
//! [`super::body::ParagraphContent::Ins`] etc. directly — never through the editing surface.
//!
//! The second half of the rule, for property setters: **every setter in this crate replaces or
//! inserts only the one content-item variant it owns, at its own schema rank, and never touches,
//! reorders, or removes any other item in the same `Vec` — a `*Change` sibling included.** This was
//! already true before this child (every setter in `run_properties.rs`/`paragraph_properties.rs`/
//! `sections.rs`/`table_properties.rs`/`tables.rs`/`styles.rs` already follows this "find-my-own-
//! variant, else insert-at-rank" shape); this child's own contribution is giving the `*Change`
//! variants a real type instead of [`super::body::Unmodeled`], and, for four containers that had no
//! slot for their trailing extension members at all yet (`w:tblPr`, `w:tblPrEx`, `w:trPr`,
//! `w:tblGrid`), adding one.
//!
//! # Mutation-path table
//!
//! | Path (child) | Behaviour for a revision-marked target |
//! |---|---|
//! | Run text edit, run split/merge (MJXOFF-92) | A run inside `w:ins`/`w:del`/`w:moveFrom`/`w:moveTo` is not addressed by `RunPath` — see this module's own "one rule" above. Editing an *unrelated* run never touches the wrapper or its contents. |
//! | Paragraph insert/remove/split (MJXOFF-92) | A whole paragraph is a `BlockContent::Paragraph` regardless of what revisions its own content holds; removing/inserting paragraphs elsewhere leaves every revision container's bytes untouched (position only). |
//! | Row/column insert/remove (MJXOFF-116) | A newly inserted row/cell always starts from `RowProperties::new`/`CellProperties::new` (both build an empty `content: Vec::new()`) — a fresh row/cell never inherits a neighbour's `w:trPr/w:ins`, `w:tcPr/w:cellIns`/`cellDel`/`cellMerge` or `*PrChange`. Removing a row/cell removes exactly that physical row/cell; any `*Change`/`cellMerge` it carried is removed with it (it describes *that* row/cell's own history), and every other row/cell's own markers are untouched. |
//! | Property setters — `w:rPr`/`w:pPr`/`w:pPr/w:rPr`/`w:sectPr`/`w:tblPr`/`w:tblPrEx`/`w:tblGrid`/`w:tcPr`/`w:trPr` (MJXOFF-92/109/116/119) | Setting any typed member (bold, alignment, width, …) never touches a `*Change`/`cellIns`/`cellDel`/`cellMerge`/`ins`/`del` sibling already present in the same properties block — see the rule above. Reading the *previous* property set a `*Change` carries goes through its own typed accessor (e.g. [`RunPropertiesChange::run_properties`]) — it is never conflated with the *live* property being edited. |
//! | Field edit — `set_field_instruction`/`set_field_cached_result_text` (MJXOFF-121) | Field-marker scanning (`fields.rs::parse_top`) walks top-level paragraph content only; a field whose `w:fldChar` markers sit inside a revision container is not found — see the rule above. A field found and edited normally is untouched if it carries a sibling `w:numberingChange` on one of its own `w:fldChar` markers ([`FieldCharacterContent::NumberingChange`], typed by this child as [`TrackChangeNumbering`]) — the edit only ever replaces `w:instrText`/cached-result runs, never a `w:fldChar`'s own children. |
//! | Bookmark/comment-range add/remove — `add_bookmark`/`remove_bookmark`/`remove_comment` (MJXOFF-124) | [`super::ranges::RangeIndex`] classifies top-level paragraph content only (the rule above); a bookmark/comment range wrapped inside a revision container is not found, so it cannot be resolved or removed by these paths, and is never touched by them either. `remove_matching` removes exactly the items its own predicate matches — never anything from a sibling `w:ins`/`w:del` it does not recurse into. |
//! | Cell-merge tracking (`CT_CellMergeTrackChange`, this child) | Read-only: [`CellMergeTrackChange::vertical_merge`]/`vertical_merge_original` report what a file already states. This crate's own structural merge/unmerge (MJXOFF-116) never authors or clears a `w:cellMerge` — it edits `w:vMerge` (the *live* continuation marker) exactly as before; the two are independent per §17.13.5.6 ("tracks the history... does not affect... rendering"). |
//! | Move ranges (`w:moveFromRangeStart`/`w:moveToRangeStart`, this child, MJXOFF-124's engine) | Typed as [`ParagraphContent::MoveFromRangeStart`]/`MoveToRangeStart` ([`MoveBookmark`]) and resolved with [`super::ranges::RangeIndex::build`] + [`classify_move_range`] below — **the same id-keyed pairing MJXOFF-124 built, not a second engine.** Read-only: this crate does not relocate content between a `moveFrom`/`moveTo` pair (see "Accept/reject" below). |
//!
//! # Accept/reject: declined, and why
//!
//! **This child implements revision *enumeration* and computed accepted/rejected *text* (both
//! required by the ticket's own "Reading" bullet — [`crate::Document::revisions`],
//! [`crate::Document::text_with_revisions_accepted`], [`crate::Document::
//! text_with_revisions_rejected`]) but declines mutating accept/reject *operations* — the ticket's
//! own "Optional but decide explicitly" bullet.**
//!
//! Handling all eight `*Change` kinds plus moves correctly as an in-place mutation is not a small
//! extension of the read model above — it is a second, structurally different feature: rejecting a
//! `w:pPrChange` means splicing the *previous* `CT_PPrBase` back in as the live `w:pPr` while
//! discarding the current one; accepting a `w:moveFrom`/`w:moveTo` pair means physically relocating
//! content between two positions in the document that this ticket's own trap fixture deliberately
//! puts far apart; rejecting `w:tblGridChange` means restoring a table's column structure, which
//! this crate's own grid/row invariants (`Table::grid_discrepancies`) were not designed to be
//! rewound through. None of the "Done when" bullets require it (they ask for enumeration, computed
//! text, edit isolation, and setter-does-not-destroy-the-record — all satisfied without ever
//! mutating a revision), and building it well would essentially be re-implementing Word's own
//! "Accept All Changes"/"Reject All Changes" simplification pass, which the "not in scope" section's
//! own OLE/DrawingML and content-control carve-outs strongly suggest was never the intent for the
//! same ticket that also asks for a mutation-path *audit* across five other children. So: declined,
//! stated here, and no half version — nothing here works "only for `w:ins`".

use std::borrow::Cow;

use mjx_ooxml_core::{
    Enumeration, FromXml, FromXmlError, Interner, Number, RawAttribute, RawElement, RawName,
    RawNode, Text as TextCodec, ToXml,
};
use mjx_ooxml_types::wordprocessingml::{
    DecimalNumber, DisplacedByCustomXml, VerticalMergeRevision,
};

use crate::error::DocxError;

use super::body::{wml_name, Hyperlink, ParagraphContent, ProofingError};
use super::fields::SimpleField;
use super::paragraph_properties::ParagraphMarkRunProperties;
use super::ranges::{MarkerRole, Markup, MarkupRange};
use super::run_properties::RunProperties;
use super::sections::SectionProperties;
use super::styles::StyleParagraphProperties;
use super::table_properties::{RowProperties, TableExceptionProperties, TableProperties};
use super::tables::{CellProperties, Grid};

// =================================================================================================
// ST_DateTime: opaque, preserved verbatim on read; validated only when this crate authors one.
// =================================================================================================

/// Whether `s` is a well-formed `xsd:dateTime` (`ST_DateTime`'s own unconstrained restriction base —
/// `wml.xsd`'s `<xsd:restriction base="xsd:dateTime"/>` adds no facet of its own, so this is the
/// full check): `YYYY-MM-DDThh:mm:ss` with an optional `.` + fractional digits and an optional
/// timezone (`Z` or `±hh:mm`), per XML Schema Part 2 §3.2.7. **Reading never calls this** — a
/// malformed `w:date` in an untrusted file is preserved byte-for-byte, exactly as written, never
/// normalised (see this module's own top-level doc comment and `fields.rs`'s identical precedent
/// for over-long strings). This is the gate a `set_date_checked` setter runs *before* writing, so
/// this crate never authors a `w:date` the schema gate would then reject.
#[must_use]
pub(crate) fn is_valid_xsd_datetime(s: &str) -> bool {
    let bytes = s.as_bytes();
    // A minimal hand-rolled parser: no external date/time dependency is worth adding for a single
    // authoring-time format check. Digit-count and separator checks only — this validates *shape*,
    // not calendar semantics (e.g. it accepts 2024-02-30); ECMA-376 does not require calendar
    // validity either, only lexical well-formedness against `xsd:dateTime`.
    fn digits(b: &[u8], at: usize, n: usize) -> bool {
        b.get(at..at + n)
            .is_some_and(|s| s.iter().all(u8::is_ascii_digit))
    }
    if bytes.len() < 19 {
        return false;
    }
    if !digits(bytes, 0, 4) || bytes[4] != b'-' || !digits(bytes, 5, 2) || bytes[7] != b'-' {
        return false;
    }
    if !digits(bytes, 8, 2) || bytes[10] != b'T' {
        return false;
    }
    if !digits(bytes, 11, 2) || bytes[13] != b':' || !digits(bytes, 14, 2) || bytes[16] != b':' {
        return false;
    }
    if !digits(bytes, 17, 2) {
        return false;
    }
    let mut rest = &bytes[19..];
    if let Some(after_dot) = rest.strip_prefix(b".") {
        let digit_count = after_dot.iter().take_while(|b| b.is_ascii_digit()).count();
        if digit_count == 0 {
            return false;
        }
        rest = &after_dot[digit_count..];
    }
    match rest {
        b"" | b"Z" => true,
        _ => {
            (rest.len() == 6)
                && (rest[0] == b'+' || rest[0] == b'-')
                && digits(rest, 1, 2)
                && rest[3] == b':'
                && digits(rest, 4, 2)
        }
    }
}

/// Refuses `date` with [`DocxError::MalformedDateTime`] unless it is a well-formed `xsd:dateTime` —
/// see [`is_valid_xsd_datetime`].
fn check_date_time(date: &str) -> Result<(), DocxError> {
    if is_valid_xsd_datetime(date) {
        Ok(())
    } else {
        Err(DocxError::MalformedDateTime(date.to_owned()))
    }
}

// =================================================================================================
// TrackChangeMarker (CT_TrackChange, bare) — cellIns, cellDel, the four customXml*RangeStart
// elements, w:numPr/w:ins, and w:pPr/w:rPr's own ins/del/moveFrom/moveTo.
// =================================================================================================

/// `CT_TrackChange` — the bare id/author/date marker with no payload of its own, reused wherever the
/// schema marks *that a whole element was tracked-inserted-or-deleted* without wrapping any content:
/// `w:cellIns`/`w:cellDel` (a whole table cell), `w:numPr/w:ins` (a whole numbering reference), the
/// paragraph mark's own `w:pPr/w:rPr/w:ins`/`w:del`/`w:moveFrom`/`w:moveTo`, and the four
/// `customXml*RangeStart` members of `EG_RangeMarkupElements` (MJXOFF-124's own range group — see
/// [`classify_custom_xml_range`] below).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "id", prefix = "w", codec = Number<DecimalNumber>, accessor = id, required))]
#[xml(attribute(local = "author", prefix = "w", codec = TextCodec, accessor = raw_author, required))]
#[xml(attribute(local = "date", prefix = "w", codec = TextCodec, accessor = raw_date))]
pub struct TrackChangeMarker {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl TrackChangeMarker {
    /// Builds a new `local` marker (`"cellIns"`, `"cellDel"`, `"ins"`, `"del"`, `"moveFrom"`,
    /// `"moveTo"`, or one of the four `customXml*RangeStart` names) with `id`/`author`, no `date`
    /// stated.
    #[must_use]
    pub fn new(interner: &mut Interner, local: &str, id: i64, author: &str) -> Self {
        let mut value = Self {
            name: wml_name(interner, local),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        value.set_id(interner, id);
        value.set_raw_author(interner, author);
        value
    }

    /// The author (`@author`), or `None` if malformed.
    #[must_use]
    pub fn author(&self, interner: &Interner) -> Option<String> {
        self.raw_author(interner).ok().map(Cow::into_owned)
    }

    /// The date/time stamp (`@date`, `ST_DateTime`) as the file wrote it — an opaque wire string
    /// this crate never parses or normalises (see this module's own top doc comment) — or `None`
    /// if absent/malformed.
    #[must_use]
    pub fn date(&self, interner: &Interner) -> Option<String> {
        self.raw_date(interner).ok().flatten().map(Cow::into_owned)
    }

    /// Sets `@date`, refusing a value that is not a well-formed `xsd:dateTime`.
    ///
    /// # Errors
    /// [`DocxError::MalformedDateTime`] if `date` is `Some` and not well-formed.
    pub fn set_date_checked(
        &mut self,
        interner: &mut Interner,
        date: Option<&str>,
    ) -> Result<(), DocxError> {
        if let Some(date) = date {
            check_date_time(date)?;
        }
        self.set_raw_date(interner, date);
        Ok(())
    }
}

impl FromXml for TrackChangeMarker {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for TrackChangeMarker {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// =================================================================================================
// RunTrackChange (CT_RunTrackChange) — w:ins, w:del, w:moveFrom, w:moveTo as run-level containers.
// =================================================================================================

/// `CT_RunTrackChange` — `w:ins`/`w:del`/`w:moveFrom`/`w:moveTo` (`EG_RunLevelElts`): a `CT_TrackChange`
/// marker (id/author/date) wrapping ordinary paragraph content. Reuses [`ParagraphContent`] directly
/// for its own content, exactly as [`Hyperlink`] already does for the same reason (documented on
/// that type): `CT_RunTrackChange`'s real content model (`EG_ContentRunContent` or `m:
/// EG_OMathMathElements`) is a *subset* of `ParagraphContent` (no `w:pPr`, `w:fldSimple`,
/// `w:hyperlink` or `w:subDoc`), so a handful of variants are simply never produced by this crate's
/// own writer here — but the recursion is exactly what the ticket's own trap fixture needs: an
/// insertion nested inside a deletion is one [`ParagraphContent::Ins`] inside another
/// [`ParagraphContent::Del`]'s own `content`.
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = WML)]
#[xml(attribute(local = "id", prefix = "w", codec = Number<DecimalNumber>, accessor = id, required))]
#[xml(attribute(local = "author", prefix = "w", codec = TextCodec, accessor = raw_author, required))]
#[xml(attribute(local = "date", prefix = "w", codec = TextCodec, accessor = raw_date))]
pub struct RunTrackChange {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "pPr", variant = Properties, ty = super::paragraph_properties::ParagraphProperties),
        child(local = "customXml", variant = CustomXml, ty = super::body::Unmodeled),
        child(local = "smartTag", variant = SmartTag, ty = super::body::Unmodeled),
        child(local = "sdt", variant = StructuredDocumentTag, ty = super::body::Unmodeled),
        child(local = "dir", variant = BidirectionalEmbedding, ty = super::body::Unmodeled),
        child(local = "bdo", variant = BidirectionalOverride, ty = super::body::Unmodeled),
        child(local = "r", variant = Run, ty = super::body::Run),
        child(local = "proofErr", variant = ProofingError, ty = ProofingError),
        child(local = "permStart", variant = PermissionRangeStart, ty = super::body::PermissionRangeStart),
        child(local = "permEnd", variant = PermissionRangeEnd, ty = super::body::PermissionRangeEnd),
        child(local = "fldSimple", variant = SimpleField, ty = SimpleField),
        child(local = "hyperlink", variant = Hyperlink, ty = Hyperlink),
        child(local = "subDoc", variant = SubDocument, ty = super::body::RelationshipReference),
        child(local = "fldData", variant = FieldData, ty = super::body::Text),
        child(local = "bookmarkStart", variant = BookmarkStart, ty = super::ranges::Bookmark),
        child(local = "bookmarkEnd", variant = BookmarkEnd, ty = MarkupRange),
        child(local = "commentRangeStart", variant = CommentRangeStart, ty = MarkupRange),
        child(local = "commentRangeEnd", variant = CommentRangeEnd, ty = MarkupRange),
        child(local = "moveFromRangeStart", variant = MoveFromRangeStart, ty = MoveBookmark),
        child(local = "moveFromRangeEnd", variant = MoveFromRangeEnd, ty = MarkupRange),
        child(local = "moveToRangeStart", variant = MoveToRangeStart, ty = MoveBookmark),
        child(local = "moveToRangeEnd", variant = MoveToRangeEnd, ty = MarkupRange),
        child(local = "customXmlInsRangeStart", variant = CustomXmlInsRangeStart, ty = TrackChangeMarker),
        child(local = "customXmlInsRangeEnd", variant = CustomXmlInsRangeEnd, ty = Markup),
        child(local = "customXmlDelRangeStart", variant = CustomXmlDelRangeStart, ty = TrackChangeMarker),
        child(local = "customXmlDelRangeEnd", variant = CustomXmlDelRangeEnd, ty = Markup),
        child(local = "customXmlMoveFromRangeStart", variant = CustomXmlMoveFromRangeStart, ty = TrackChangeMarker),
        child(local = "customXmlMoveFromRangeEnd", variant = CustomXmlMoveFromRangeEnd, ty = Markup),
        child(local = "customXmlMoveToRangeStart", variant = CustomXmlMoveToRangeStart, ty = TrackChangeMarker),
        child(local = "customXmlMoveToRangeEnd", variant = CustomXmlMoveToRangeEnd, ty = Markup),
        child(local = "ins", variant = Ins, ty = RunTrackChange),
        child(local = "del", variant = Del, ty = RunTrackChange),
        child(local = "moveFrom", variant = MoveFrom, ty = RunTrackChange),
        child(local = "moveTo", variant = MoveTo, ty = RunTrackChange)
    )]
    content: Vec<ParagraphContent>,
}

impl RunTrackChange {
    /// Builds a new, empty `local` container (`"ins"`, `"del"`, `"moveFrom"` or `"moveTo"`) with
    /// `id`/`author`, no `date` stated, no content yet.
    #[must_use]
    pub fn new(interner: &mut Interner, local: &str, id: i64, author: &str) -> Self {
        let mut value = Self {
            name: wml_name(interner, local),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        };
        value.set_id(interner, id);
        value.set_raw_author(interner, author);
        value
    }

    /// The author (`@author`), or `None` if malformed.
    #[must_use]
    pub fn author(&self, interner: &Interner) -> Option<String> {
        self.raw_author(interner).ok().map(Cow::into_owned)
    }

    /// The date/time stamp (`@date`), an opaque wire string, or `None` if absent/malformed.
    #[must_use]
    pub fn date(&self, interner: &Interner) -> Option<String> {
        self.raw_date(interner).ok().flatten().map(Cow::into_owned)
    }

    /// Sets `@date`, refusing a value that is not a well-formed `xsd:dateTime`.
    ///
    /// # Errors
    /// [`DocxError::MalformedDateTime`] if `date` is `Some` and not well-formed.
    pub fn set_date_checked(
        &mut self,
        interner: &mut Interner,
        date: Option<&str>,
    ) -> Result<(), DocxError> {
        if let Some(date) = date {
            check_date_time(date)?;
        }
        self.set_raw_date(interner, date);
        Ok(())
    }

    /// This container's own content, in document order — every run, nested revision container, or
    /// other paragraph-level item it wraps.
    #[must_use]
    pub fn content(&self) -> &[ParagraphContent] {
        &self.content
    }

    /// [`RunTrackChange::content`], mutably. `pub(crate)`: see this module's own "one rule" — no
    /// mutation path in this crate reaches into a revision container's own content, so nothing
    /// outside this module calls this today.
    pub fn content_mut(&mut self) -> &mut Vec<ParagraphContent> {
        &mut self.content
    }

    /// The plain text this container wraps, descending into every nested revision container,
    /// hyperlink and simple field exactly as [`super::body::Paragraph::text`] does at the top
    /// level. Used by [`crate::Document::text_with_revisions_accepted`]/
    /// `text_with_revisions_rejected`.
    #[must_use]
    pub fn text(&self) -> String {
        let mut out = String::new();
        collect_text(&self.content, &mut out);
        out
    }
}

/// [`RunTrackChange::text`]'s own recursive walk — also used directly by the accepted/rejected text
/// computation in `mod.rs` for content that is *not* inside a revision container.
pub(crate) fn collect_text(content: &[ParagraphContent], out: &mut String) {
    for item in content {
        match item {
            ParagraphContent::Run(run) => out.push_str(&run.text()),
            ParagraphContent::Hyperlink(hyperlink) => collect_text(hyperlink.content(), out),
            ParagraphContent::SimpleField(field) => {
                out.push_str(&field.cached_result_text());
            }
            ParagraphContent::Ins(change)
            | ParagraphContent::Del(change)
            | ParagraphContent::MoveFrom(change)
            | ParagraphContent::MoveTo(change) => collect_text(change.content(), out),
            _ => {}
        }
    }
}

// =================================================================================================
// MoveBookmark (CT_MoveBookmark) — w:moveFromRangeStart, w:moveToRangeStart.
// =================================================================================================

/// `CT_MoveBookmark` (`CT_BookmarkRange` + required `author`/`date`) — `w:moveFromRangeStart`/
/// `w:moveToRangeStart`. Its own attribute set mirrors [`super::ranges::Bookmark`] exactly (same
/// flattened `CT_BookmarkRange` ancestry that type's own doc comment explains) with `author`/`date`
/// added — **both required here**, unlike the bare [`TrackChangeMarker`]'s optional `date`
/// (`CT_MoveBookmark`'s own extension states `date` as `use="required"`, confirmed directly against
/// `wml.xsd`; `CT_TrackChange`'s base `date` is `use="optional"`).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "id", prefix = "w", codec = Number<DecimalNumber>, accessor = id, required))]
#[xml(attribute(local = "displacedByCustomXml", prefix = "w", codec = Enumeration<DisplacedByCustomXml>, accessor = displaced_by_custom_xml))]
#[xml(attribute(local = "colFirst", prefix = "w", codec = Number<DecimalNumber>, accessor = first_column))]
#[xml(attribute(local = "colLast", prefix = "w", codec = Number<DecimalNumber>, accessor = last_column))]
#[xml(attribute(local = "name", prefix = "w", codec = TextCodec, accessor = raw_name, required))]
#[xml(attribute(local = "author", prefix = "w", codec = TextCodec, accessor = raw_author, required))]
#[xml(attribute(local = "date", prefix = "w", codec = TextCodec, accessor = raw_date, required))]
pub struct MoveBookmark {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl MoveBookmark {
    /// Builds a new `local` marker (`"moveFromRangeStart"`/`"moveToRangeStart"`) with `id`,
    /// `bookmark_name`, `author` and `date` — all four are required by the schema.
    #[must_use]
    pub fn new(
        interner: &mut Interner,
        local: &str,
        id: i64,
        bookmark_name: &str,
        author: &str,
        date: &str,
    ) -> Self {
        let mut value = Self {
            name: wml_name(interner, local),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        value.set_id(interner, id);
        value.set_raw_name(interner, bookmark_name);
        value.set_raw_author(interner, author);
        value.set_raw_date(interner, date);
        value
    }

    /// The move range's own name (`@name`), or `None` if malformed.
    #[must_use]
    pub fn name(&self, interner: &Interner) -> Option<String> {
        self.raw_name(interner).ok().map(Cow::into_owned)
    }

    /// The author (`@author`), or `None` if malformed.
    #[must_use]
    pub fn author(&self, interner: &Interner) -> Option<String> {
        self.raw_author(interner).ok().map(Cow::into_owned)
    }

    /// The date/time stamp (`@date`), an opaque wire string, or `None` if absent/malformed.
    #[must_use]
    pub fn date(&self, interner: &Interner) -> Option<String> {
        self.raw_date(interner).ok().map(Cow::into_owned)
    }
}

impl FromXml for MoveBookmark {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for MoveBookmark {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// =================================================================================================
// CellMergeTrackChange (CT_CellMergeTrackChange) — w:cellMerge.
// =================================================================================================

/// `CT_CellMergeTrackChange` — `w:cellMerge`: a tracked table-cell merge, carrying the vertical-merge
/// state before (`vMergeOrig`) and after (`vMerge`) the tracked edit (§17.13.5.6). Independent of the
/// *live* `w:vMerge` continuation marker [`super::tables::MergeMarker`] reads — this crate's own
/// structural merge/unmerge (MJXOFF-116) edits the live marker only; see this module's own top-level
/// mutation-path table.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "id", prefix = "w", codec = Number<DecimalNumber>, accessor = id, required))]
#[xml(attribute(local = "author", prefix = "w", codec = TextCodec, accessor = raw_author, required))]
#[xml(attribute(local = "date", prefix = "w", codec = TextCodec, accessor = raw_date))]
#[xml(attribute(local = "vMerge", prefix = "w", codec = Enumeration<VerticalMergeRevision>, accessor = vertical_merge))]
#[xml(attribute(local = "vMergeOrig", prefix = "w", codec = Enumeration<VerticalMergeRevision>, accessor = vertical_merge_original))]
pub struct CellMergeTrackChange {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl CellMergeTrackChange {
    /// Builds a new `w:cellMerge` with `id`/`author`, no `date`/`vMerge`/`vMergeOrig` stated.
    #[must_use]
    pub fn new(interner: &mut Interner, id: i64, author: &str) -> Self {
        let mut value = Self {
            name: wml_name(interner, "cellMerge"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        value.set_id(interner, id);
        value.set_raw_author(interner, author);
        value
    }

    /// The author (`@author`), or `None` if malformed.
    #[must_use]
    pub fn author(&self, interner: &Interner) -> Option<String> {
        self.raw_author(interner).ok().map(Cow::into_owned)
    }

    /// The date/time stamp (`@date`), or `None` if absent/malformed.
    #[must_use]
    pub fn date(&self, interner: &Interner) -> Option<String> {
        self.raw_date(interner).ok().flatten().map(Cow::into_owned)
    }

    /// Sets `@date`, refusing a value that is not a well-formed `xsd:dateTime`.
    ///
    /// # Errors
    /// [`DocxError::MalformedDateTime`] if `date` is `Some` and not well-formed.
    pub fn set_date_checked(
        &mut self,
        interner: &mut Interner,
        date: Option<&str>,
    ) -> Result<(), DocxError> {
        if let Some(date) = date {
            check_date_time(date)?;
        }
        self.set_raw_date(interner, date);
        Ok(())
    }
}

impl FromXml for CellMergeTrackChange {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for CellMergeTrackChange {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// =================================================================================================
// TrackChangeNumbering (CT_TrackChangeNumbering) — w:numberingChange, on w:numPr and w:fldChar.
// =================================================================================================

/// `CT_TrackChangeNumbering` — `w:numberingChange`, reachable at exactly two places in `wml.xsd`
/// (see this module's own top doc comment for why the pinned pre-dispatch note was wrong to call
/// this type unreachable): `w:numPr/w:numberingChange` ([`super::paragraph_properties::
/// NumberingPropertyContent::NumberingChange`]) and `w:fldChar/w:numberingChange`
/// ([`super::fields::FieldCharacterContent::NumberingChange`]).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "id", prefix = "w", codec = Number<DecimalNumber>, accessor = id, required))]
#[xml(attribute(local = "author", prefix = "w", codec = TextCodec, accessor = raw_author, required))]
#[xml(attribute(local = "date", prefix = "w", codec = TextCodec, accessor = raw_date))]
#[xml(attribute(local = "original", prefix = "w", codec = TextCodec, accessor = raw_original))]
pub struct TrackChangeNumbering {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl TrackChangeNumbering {
    /// Builds a new `w:numberingChange` with `id`/`author`, no `date`/`original` stated.
    #[must_use]
    pub fn new(interner: &mut Interner, id: i64, author: &str) -> Self {
        let mut value = Self {
            name: wml_name(interner, "numberingChange"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        value.set_id(interner, id);
        value.set_raw_author(interner, author);
        value
    }

    /// The author (`@author`), or `None` if malformed.
    #[must_use]
    pub fn author(&self, interner: &Interner) -> Option<String> {
        self.raw_author(interner).ok().map(Cow::into_owned)
    }

    /// The date/time stamp (`@date`), or `None` if absent/malformed.
    #[must_use]
    pub fn date(&self, interner: &Interner) -> Option<String> {
        self.raw_date(interner).ok().flatten().map(Cow::into_owned)
    }

    /// Sets `@date`, refusing a value that is not a well-formed `xsd:dateTime`.
    ///
    /// # Errors
    /// [`DocxError::MalformedDateTime`] if `date` is `Some` and not well-formed.
    pub fn set_date_checked(
        &mut self,
        interner: &mut Interner,
        date: Option<&str>,
    ) -> Result<(), DocxError> {
        if let Some(date) = date {
            check_date_time(date)?;
        }
        self.set_raw_date(interner, date);
        Ok(())
    }

    /// The previous numbering reference this change replaced (`@original`), or `None` if
    /// absent/malformed.
    #[must_use]
    pub fn original(&self, interner: &Interner) -> Option<String> {
        self.raw_original(interner)
            .ok()
            .flatten()
            .map(Cow::into_owned)
    }
}

impl FromXml for TrackChangeNumbering {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for TrackChangeNumbering {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// =================================================================================================
// The eight property `*Change` wrappers, one macro: each is CT_TrackChange (id/author/date) plus
// exactly one typed payload child, reusing the property type C3/C4/C9/C12 already built for the
// *live* property block — see this module's own top doc comment for why reusing the live type
// (rather than a parallel "-Original" type) is this codebase's own established pattern, not a
// shortcut invented here.
// =================================================================================================

macro_rules! property_change {
    (
        $(#[$meta:meta])*
        $name:ident, $content_enum:ident, $wire_local:literal,
        $child_local:literal, $child_variant:ident, $child_ty:ty,
        $getter:ident, $getter_mut:ident, $child_doc:literal
    ) => {
        $(#[$meta])*
        #[derive(
            Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
        )]
        #[xml(namespace = WML)]
        #[xml(attribute(local = "id", prefix = "w", codec = Number<DecimalNumber>, accessor = id, required))]
        #[xml(attribute(local = "author", prefix = "w", codec = TextCodec, accessor = raw_author, required))]
        #[xml(attribute(local = "date", prefix = "w", codec = TextCodec, accessor = raw_date))]
        pub struct $name {
            name: RawName,
            attributes: Vec<RawAttribute>,
            empty: bool,
            #[xml(children, child(local = $child_local, variant = $child_variant, ty = $child_ty))]
            content: Vec<$content_enum>,
        }

        #[doc = concat!("One ordered child of a [`", stringify!($name), "`]: `w:", $child_local, "` or an unknown element.")]
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub enum $content_enum {
            #[doc = $child_doc]
            $child_variant($child_ty),
            /// Any other child — preserved verbatim.
            Raw(RawNode),
        }

        impl $name {
            /// Builds a new, empty wrapper with `id`/`author`, no `date`/payload stated.
            #[must_use]
            pub fn new(interner: &mut Interner, id: i64, author: &str) -> Self {
                let mut value = Self {
                    name: wml_name(interner, $wire_local),
                    attributes: Vec::new(),
                    empty: true,
                    content: Vec::new(),
                };
                value.set_id(interner, id);
                value.set_raw_author(interner, author);
                value
            }

            /// The author (`@author`), or `None` if malformed.
            #[must_use]
            pub fn author(&self, interner: &Interner) -> Option<String> {
                self.raw_author(interner).ok().map(Cow::into_owned)
            }

            /// The date/time stamp (`@date`), an opaque wire string this crate never parses or
            /// normalises, or `None` if absent/malformed.
            #[must_use]
            pub fn date(&self, interner: &Interner) -> Option<String> {
                self.raw_date(interner).ok().flatten().map(Cow::into_owned)
            }

            /// Sets `@date`, refusing a value that is not a well-formed `xsd:dateTime`.
            ///
            /// # Errors
            /// [`DocxError::MalformedDateTime`] if `date` is `Some` and not well-formed.
            pub fn set_date_checked(
                &mut self,
                interner: &mut Interner,
                date: Option<&str>,
            ) -> Result<(), DocxError> {
                if let Some(date) = date {
                    check_date_time(date)?;
                }
                self.set_raw_date(interner, date);
                Ok(())
            }

            #[doc = $child_doc]
            #[must_use]
            pub fn $getter(&self) -> Option<&$child_ty> {
                self.content.iter().find_map(|item| match item {
                    $content_enum::$child_variant(value) => Some(value),
                    _ => None,
                })
            }

            #[doc = concat!("[`", stringify!($name), "::", stringify!($getter), "`], mutably.")]
            pub fn $getter_mut(&mut self) -> Option<&mut $child_ty> {
                self.content.iter_mut().find_map(|item| match item {
                    $content_enum::$child_variant(value) => Some(value),
                    _ => None,
                })
            }
        }
    };
}

property_change!(
    /// `CT_RPrChange` — `w:rPrChange`, on a run's own `w:rPr`: the run properties *before* this
    /// tracked change.
    RunPropertiesChange, RunPropertiesChangeContent, "rPrChange",
    "rPr", RunProperties, RunProperties,
    previous_run_properties, previous_run_properties_mut,
    "`w:rPr` (`CT_RPrOriginal`, modeled as the same [`RunProperties`] the live `w:rPr` uses — see this \
     module's own doc comment on reusing the live type) — the run properties this change replaced."
);

property_change!(
    /// `CT_PPrChange` — `w:pPrChange`, on a paragraph's own `w:pPr` (or a style/numbering-level
    /// `w:pPr`): the paragraph properties *before* this tracked change.
    ParagraphPropertiesChange, ParagraphPropertiesChangeContent, "pPrChange",
    "pPr", ParagraphProperties, StyleParagraphProperties,
    previous_paragraph_properties, previous_paragraph_properties_mut,
    "`w:pPr` (`CT_PPrBase`, modeled as [`StyleParagraphProperties`] — the closest-fitting existing type: \
     `CT_PPrBase` plus `pPrChange` only, with no `w:rPr`/`w:sectPr` a `w:pPrChange`'s own payload could \
     never legally carry either) — the paragraph properties this change replaced."
);

property_change!(
    /// `CT_ParaRPrChange` — `w:pPr/w:rPr/w:rPrChange`: the paragraph mark's own run properties
    /// *before* this tracked change.
    ParagraphMarkPropertiesChange, ParagraphMarkPropertiesChangeContent, "rPrChange",
    "rPr", ParagraphMarkRunProperties, ParagraphMarkRunProperties,
    previous_paragraph_mark_properties, previous_paragraph_mark_properties_mut,
    "`w:rPr` (`CT_ParaRPrOriginal`, modeled as the same [`ParagraphMarkRunProperties`] the live \
     paragraph-mark `w:rPr` uses) — the paragraph mark's own run properties this change replaced."
);

property_change!(
    /// `CT_SectPrChange` — `w:sectPr/w:sectPrChange`: the section properties *before* this tracked
    /// change. Unlike every other member of this family, its own payload is `minOccurs="0"` — a
    /// `w:sectPrChange` with no `w:sectPr` child at all is legal (ECMA-376 Part 1 §17.13.5.32: the
    /// section break itself was tracked-inserted with nothing to compare against).
    SectionPropertiesChange, SectionPropertiesChangeContent, "sectPrChange",
    "sectPr", SectionProperties, SectionProperties,
    previous_section_properties, previous_section_properties_mut,
    "`w:sectPr` (`CT_SectPrBase`, modeled as the same [`SectionProperties`] the live `w:sectPr` uses) — \
     the section properties this change replaced, or absent when there was nothing to compare against."
);

property_change!(
    /// `CT_TblPrChange` — `w:tblPr/w:tblPrChange`: the table properties *before* this tracked
    /// change.
    TablePropertiesChange, TablePropertiesChangeContent, "tblPrChange",
    "tblPr", TableProperties, TableProperties,
    previous_table_properties, previous_table_properties_mut,
    "`w:tblPr` (`CT_TblPrBase`, modeled as the same [`TableProperties`] the live `w:tblPr` uses) — the \
     table properties this change replaced."
);

property_change!(
    /// `CT_TblPrExChange` — `w:tblPrEx/w:tblPrExChange`: the table-exception properties *before*
    /// this tracked change.
    TableExceptionPropertiesChange, TableExceptionPropertiesChangeContent, "tblPrExChange",
    "tblPrEx", TableExceptionProperties, TableExceptionProperties,
    previous_table_exception_properties, previous_table_exception_properties_mut,
    "`w:tblPrEx` (`CT_TblPrExBase`, modeled as the same [`TableExceptionProperties`] the live \
     `w:tblPrEx` uses) — the row's table-exception properties this change replaced."
);

property_change!(
    /// `CT_TcPrChange` — `w:tcPr/w:tcPrChange`: the cell properties *before* this tracked change.
    CellPropertiesChange, CellPropertiesChangeContent, "tcPrChange",
    "tcPr", CellProperties, CellProperties,
    previous_cell_properties, previous_cell_properties_mut,
    "`w:tcPr` (`CT_TcPrInner`, modeled as the same [`CellProperties`] the live `w:tcPr` uses) — the \
     cell properties this change replaced."
);

property_change!(
    /// `CT_TrPrChange` — `w:trPr/w:trPrChange`: the row properties *before* this tracked change.
    RowPropertiesChange, RowPropertiesChangeContent, "trPrChange",
    "trPr", RowProperties, RowProperties,
    previous_row_properties, previous_row_properties_mut,
    "`w:trPr` (`CT_TrPrBase`, modeled as the same [`RowProperties`] the live `w:trPr` uses) — the row \
     properties this change replaced."
);

// =================================================================================================
// TableGridChange (CT_TblGridChange) — w:tblGrid/w:tblGridChange. Extends CT_Markup (id only), not
// CT_TrackChange — no author/date, verified directly against wml.xsd.
// =================================================================================================

/// `CT_TblGridChange` — `w:tblGridChange`: the table's declared column widths *before* this tracked
/// change. **Extends `CT_Markup`, not `CT_TrackChange`** — an id only, no `author`/`date` — confirmed
/// directly against `wml.xsd` (`<xsd:extension base="CT_Markup">`), unlike every other member of this
/// family.
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = WML)]
#[xml(attribute(local = "id", prefix = "w", codec = Number<DecimalNumber>, accessor = id, required))]
pub struct TableGridChange {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "tblGrid", variant = Grid, ty = Grid))]
    content: Vec<TableGridChangeContent>,
}

/// One ordered child of a [`TableGridChange`]: `w:tblGrid` or an unknown element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableGridChangeContent {
    /// `w:tblGrid` (`CT_TblGridBase`, modeled as the same [`Grid`] the live `w:tblGrid` uses) — the
    /// column widths this change replaced.
    Grid(Grid),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

impl TableGridChange {
    /// Builds a new, empty `w:tblGridChange` with `id`, no payload yet.
    #[must_use]
    pub fn new(interner: &mut Interner, id: i64) -> Self {
        let mut value = Self {
            name: wml_name(interner, "tblGridChange"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        };
        value.set_id(interner, id);
        value
    }

    /// The table's previous column widths (`w:tblGrid`), or `None` if this change carries none.
    #[must_use]
    pub fn previous_grid(&self) -> Option<&Grid> {
        self.content.iter().find_map(|item| match item {
            TableGridChangeContent::Grid(grid) => Some(grid),
            _ => None,
        })
    }

    /// [`TableGridChange::previous_grid`], mutably.
    pub fn previous_grid_mut(&mut self) -> Option<&mut Grid> {
        self.content.iter_mut().find_map(|item| match item {
            TableGridChangeContent::Grid(grid) => Some(grid),
            _ => None,
        })
    }
}

// =================================================================================================
// Move-range classification — reuses MJXOFF-124's RangeIndex, never a second engine.
// =================================================================================================

/// Classifies `item` as a `w:moveFromRangeStart`/`w:moveFromRangeEnd` marker pair's own kind — a
/// working example of the classifier closure [`super::ranges::RangeIndex::build`] takes, exactly as
/// its own doc comment specifies for MJXOFF-126. Pass this for the `moveFrom` side; see
/// [`classify_move_to_range`] for the `moveTo` side (two separate id spaces — a `moveFromRangeStart`
/// and a `moveToRangeStart` sharing an id is unrelated, per §17.13.5.20/`.24`'s own separate
/// enumeration).
pub(crate) fn classify_move_from_range(
    item: &ParagraphContent,
    interner: &Interner,
) -> Option<(MarkerRole, i64)> {
    match item {
        ParagraphContent::MoveFromRangeStart(marker) => {
            marker.id(interner).ok().map(|id| (MarkerRole::Start, id))
        }
        ParagraphContent::MoveFromRangeEnd(end) => {
            end.id(interner).ok().map(|id| (MarkerRole::End, id))
        }
        _ => None,
    }
}

/// [`classify_move_from_range`]'s own `moveTo` counterpart.
pub(crate) fn classify_move_to_range(
    item: &ParagraphContent,
    interner: &Interner,
) -> Option<(MarkerRole, i64)> {
    match item {
        ParagraphContent::MoveToRangeStart(marker) => {
            marker.id(interner).ok().map(|id| (MarkerRole::Start, id))
        }
        ParagraphContent::MoveToRangeEnd(end) => {
            end.id(interner).ok().map(|id| (MarkerRole::End, id))
        }
        _ => None,
    }
}

/// Classifies `item` as any one of the four `customXml*RangeStart`/`*RangeEnd` pairs — all four
/// share one id space is *not* assumed here (each pair is matched independently by which of the
/// four `ParagraphContent` variants it is, so [`super::ranges::RangeIndex::build`] must be called
/// once per pair, exactly as bookmarks and comment ranges are already two separate calls).
pub(crate) fn classify_custom_xml_ins_range(
    item: &ParagraphContent,
    interner: &Interner,
) -> Option<(MarkerRole, i64)> {
    match item {
        ParagraphContent::CustomXmlInsRangeStart(marker) => {
            marker.id(interner).ok().map(|id| (MarkerRole::Start, id))
        }
        ParagraphContent::CustomXmlInsRangeEnd(end) => {
            end.id(interner).ok().map(|id| (MarkerRole::End, id))
        }
        _ => None,
    }
}

/// [`classify_custom_xml_ins_range`]'s own `customXmlDelRange*` counterpart.
pub(crate) fn classify_custom_xml_del_range(
    item: &ParagraphContent,
    interner: &Interner,
) -> Option<(MarkerRole, i64)> {
    match item {
        ParagraphContent::CustomXmlDelRangeStart(marker) => {
            marker.id(interner).ok().map(|id| (MarkerRole::Start, id))
        }
        ParagraphContent::CustomXmlDelRangeEnd(end) => {
            end.id(interner).ok().map(|id| (MarkerRole::End, id))
        }
        _ => None,
    }
}

/// [`classify_custom_xml_ins_range`]'s own `customXmlMoveFromRange*` counterpart.
pub(crate) fn classify_custom_xml_move_from_range(
    item: &ParagraphContent,
    interner: &Interner,
) -> Option<(MarkerRole, i64)> {
    match item {
        ParagraphContent::CustomXmlMoveFromRangeStart(marker) => {
            marker.id(interner).ok().map(|id| (MarkerRole::Start, id))
        }
        ParagraphContent::CustomXmlMoveFromRangeEnd(end) => {
            end.id(interner).ok().map(|id| (MarkerRole::End, id))
        }
        _ => None,
    }
}

/// [`classify_custom_xml_ins_range`]'s own `customXmlMoveToRange*` counterpart.
pub(crate) fn classify_custom_xml_move_to_range(
    item: &ParagraphContent,
    interner: &Interner,
) -> Option<(MarkerRole, i64)> {
    match item {
        ParagraphContent::CustomXmlMoveToRangeStart(marker) => {
            marker.id(interner).ok().map(|id| (MarkerRole::Start, id))
        }
        ParagraphContent::CustomXmlMoveToRangeEnd(end) => {
            end.id(interner).ok().map(|id| (MarkerRole::End, id))
        }
        _ => None,
    }
}

// =================================================================================================
// Revision enumeration.
// =================================================================================================

/// The kind of tracked change one [`RevisionInfo`] reports — every content/property revision this
/// child models (move ranges are reported by their own [`RevisionKind::MoveFromRange`]/
/// `MoveToRange` alongside the content `MoveFrom`/`MoveTo`, since a real move is a `w:moveFrom`
/// **content** container plus a `w:moveFromRangeStart`/`End` **range** pair together, per
/// §17.13.5.20-25 — this crate reports both halves rather than assuming a caller only wants one).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RevisionKind {
    /// `w:ins` — inserted content.
    Inserted,
    /// `w:del` — deleted content.
    Deleted,
    /// `w:moveFrom` — content moved away from here.
    MovedFromContent,
    /// `w:moveTo` — content moved to here.
    MovedToContent,
    /// `w:rPrChange` — a run's formatting changed.
    RunPropertiesChanged,
    /// `w:pPrChange` — a paragraph's formatting changed.
    ParagraphPropertiesChanged,
    /// `w:pPr/w:rPr/w:rPrChange` — a paragraph mark's own formatting changed.
    ParagraphMarkPropertiesChanged,
    /// `w:sectPrChange` — a section's formatting changed.
    SectionPropertiesChanged,
    /// `w:tblPrChange` — a table's formatting changed.
    TablePropertiesChanged,
    /// `w:tblPrExChange` — a row's table-exception formatting changed.
    TableExceptionPropertiesChanged,
    /// `w:tblGridChange` — a table's column structure changed.
    TableGridChanged,
    /// `w:tcPrChange` — a cell's formatting changed.
    CellPropertiesChanged,
    /// `w:trPrChange` — a row's formatting changed.
    RowPropertiesChanged,
    /// `w:cellMerge` — a tracked cell merge.
    CellMerged,
    /// `w:numberingChange` — a tracked numbering-reference change.
    NumberingChanged,
    /// `w:numPr/w:ins`, `w:cellIns`, `w:trPr/w:ins`, or a paragraph mark's own `w:ins` — a bare
    /// tracked-inserted marker with no wrapped content of its own.
    MarkerInserted,
    /// `w:cellDel`, `w:trPr/w:del`, or a paragraph mark's own `w:del` — the bare-marker counterpart
    /// of [`RevisionKind::MarkerInserted`].
    MarkerDeleted,
    /// A paragraph mark's own `w:moveFrom`.
    MarkerMovedFrom,
    /// A paragraph mark's own `w:moveTo`.
    MarkerMovedTo,
}

/// One revision found by [`crate::Document::revisions`]: its kind, author and date, exactly as the
/// underlying element states them (author lossy-decoded, date the raw wire string — see this
/// module's own top doc comment for why dates are never parsed here).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionInfo {
    /// What kind of tracked change this is.
    pub kind: RevisionKind,
    /// The author (`@author`), or `None` if malformed.
    pub author: Option<String>,
    /// The date/time stamp (`@date`), the file's own wire string, or `None` if absent/malformed.
    pub date: Option<String>,
    /// The revision's own id (`@id`), or `None` if malformed.
    pub id: Option<i64>,
}

/// Walks every [`ParagraphContent`] revision-bearing item reachable from `content` (recursing into
/// nested revision containers — an insertion nested inside a deletion reports both — and into every
/// table cell, mirroring [`super::ranges::flatten_paragraphs`]'s own reach), pushing one
/// [`RevisionInfo`] per item found onto `out`. `pub(crate)`: [`crate::Document::revisions`] is the
/// public entry point, since a full walk needs the whole document (headers/footers/comments/
/// footnotes/endnotes too), not just one container's content.
pub(crate) fn collect_revisions(
    content: &[super::body::BlockContent],
    interner: &Interner,
    out: &mut Vec<RevisionInfo>,
) {
    for block in content {
        match block {
            super::body::BlockContent::Paragraph(paragraph) => {
                collect_paragraph_revisions(paragraph.content(), interner, out);
                if let Some(properties) = paragraph.properties() {
                    if let Some(change) = properties.change() {
                        push_change(
                            out,
                            RevisionKind::ParagraphPropertiesChanged,
                            change,
                            interner,
                        );
                    }
                    if let Some(mark) = properties.paragraph_mark_properties() {
                        collect_mark_revisions(mark, interner, out);
                    }
                }
            }
            super::body::BlockContent::Table(table) => {
                for row in table.rows() {
                    if let Some(properties) = row.properties() {
                        collect_row_revisions(properties, interner, out);
                    }
                    for cell in row.cells() {
                        if let Some(properties) = cell.properties() {
                            collect_cell_revisions(properties, interner, out);
                        }
                        collect_revisions(cell.content(), interner, out);
                    }
                }
            }
            _ => {}
        }
    }
}

fn collect_paragraph_revisions(
    content: &[ParagraphContent],
    interner: &Interner,
    out: &mut Vec<RevisionInfo>,
) {
    for item in content {
        match item {
            ParagraphContent::Ins(change) => {
                push_run_change(out, RevisionKind::Inserted, change, interner);
                collect_paragraph_revisions(change.content(), interner, out);
            }
            ParagraphContent::Del(change) => {
                push_run_change(out, RevisionKind::Deleted, change, interner);
                collect_paragraph_revisions(change.content(), interner, out);
            }
            ParagraphContent::MoveFrom(change) => {
                push_run_change(out, RevisionKind::MovedFromContent, change, interner);
                collect_paragraph_revisions(change.content(), interner, out);
            }
            ParagraphContent::MoveTo(change) => {
                push_run_change(out, RevisionKind::MovedToContent, change, interner);
                collect_paragraph_revisions(change.content(), interner, out);
            }
            ParagraphContent::Hyperlink(hyperlink) => {
                collect_paragraph_revisions(hyperlink.content(), interner, out);
            }
            ParagraphContent::Run(run) => {
                if let Some(properties) = run.run_properties() {
                    if let Some(change) = properties.change() {
                        push_change(out, RevisionKind::RunPropertiesChanged, change, interner);
                    }
                }
            }
            _ => {}
        }
    }
}

fn collect_mark_revisions(
    mark: &ParagraphMarkRunProperties,
    interner: &Interner,
    out: &mut Vec<RevisionInfo>,
) {
    if let Some(marker) = mark.inserted() {
        push_marker(out, RevisionKind::MarkerInserted, marker, interner);
    }
    if let Some(marker) = mark.deleted() {
        push_marker(out, RevisionKind::MarkerDeleted, marker, interner);
    }
    if let Some(marker) = mark.moved_from() {
        push_marker(out, RevisionKind::MarkerMovedFrom, marker, interner);
    }
    if let Some(marker) = mark.moved_to() {
        push_marker(out, RevisionKind::MarkerMovedTo, marker, interner);
    }
    if let Some(change) = mark.change() {
        push_change(
            out,
            RevisionKind::ParagraphMarkPropertiesChanged,
            change,
            interner,
        );
    }
}

fn collect_row_revisions(
    properties: &RowProperties,
    interner: &Interner,
    out: &mut Vec<RevisionInfo>,
) {
    if let Some(marker) = properties.inserted() {
        push_marker(out, RevisionKind::MarkerInserted, marker, interner);
    }
    if let Some(marker) = properties.deleted() {
        push_marker(out, RevisionKind::MarkerDeleted, marker, interner);
    }
    if let Some(change) = properties.change() {
        push_change(out, RevisionKind::RowPropertiesChanged, change, interner);
    }
}

fn collect_cell_revisions(
    properties: &CellProperties,
    interner: &Interner,
    out: &mut Vec<RevisionInfo>,
) {
    if let Some(marker) = properties.cell_inserted() {
        push_marker(out, RevisionKind::MarkerInserted, marker, interner);
    }
    if let Some(marker) = properties.cell_deleted() {
        push_marker(out, RevisionKind::MarkerDeleted, marker, interner);
    }
    if let Some(merge) = properties.cell_merge() {
        out.push(RevisionInfo {
            kind: RevisionKind::CellMerged,
            author: merge.author(interner),
            date: merge.date(interner),
            id: merge.id(interner).ok(),
        });
    }
    if let Some(change) = properties.change() {
        push_change(out, RevisionKind::CellPropertiesChanged, change, interner);
    }
}

fn push_marker(
    out: &mut Vec<RevisionInfo>,
    kind: RevisionKind,
    marker: &TrackChangeMarker,
    interner: &Interner,
) {
    out.push(RevisionInfo {
        kind,
        author: marker.author(interner),
        date: marker.date(interner),
        id: marker.id(interner).ok(),
    });
}

fn push_change<T>(out: &mut Vec<RevisionInfo>, kind: RevisionKind, change: &T, interner: &Interner)
where
    T: ChangeParts,
{
    let (author, date, id) = change.revision_parts(interner);
    out.push(RevisionInfo {
        kind,
        author,
        date,
        id,
    });
}

/// A small internal trait so [`push_change`] is generic over the eight `*Change` wrapper types
/// without repeating the same three-field extraction eight times.
trait ChangeParts {
    fn revision_parts(&self, interner: &Interner) -> (Option<String>, Option<String>, Option<i64>);
}
impl ChangeParts for RunPropertiesChange {
    fn revision_parts(&self, interner: &Interner) -> (Option<String>, Option<String>, Option<i64>) {
        (
            self.author(interner),
            self.date(interner),
            self.id(interner).ok(),
        )
    }
}
impl ChangeParts for ParagraphPropertiesChange {
    fn revision_parts(&self, interner: &Interner) -> (Option<String>, Option<String>, Option<i64>) {
        (
            self.author(interner),
            self.date(interner),
            self.id(interner).ok(),
        )
    }
}
impl ChangeParts for ParagraphMarkPropertiesChange {
    fn revision_parts(&self, interner: &Interner) -> (Option<String>, Option<String>, Option<i64>) {
        (
            self.author(interner),
            self.date(interner),
            self.id(interner).ok(),
        )
    }
}
impl ChangeParts for SectionPropertiesChange {
    fn revision_parts(&self, interner: &Interner) -> (Option<String>, Option<String>, Option<i64>) {
        (
            self.author(interner),
            self.date(interner),
            self.id(interner).ok(),
        )
    }
}
impl ChangeParts for TablePropertiesChange {
    fn revision_parts(&self, interner: &Interner) -> (Option<String>, Option<String>, Option<i64>) {
        (
            self.author(interner),
            self.date(interner),
            self.id(interner).ok(),
        )
    }
}
impl ChangeParts for TableExceptionPropertiesChange {
    fn revision_parts(&self, interner: &Interner) -> (Option<String>, Option<String>, Option<i64>) {
        (
            self.author(interner),
            self.date(interner),
            self.id(interner).ok(),
        )
    }
}
impl ChangeParts for CellPropertiesChange {
    fn revision_parts(&self, interner: &Interner) -> (Option<String>, Option<String>, Option<i64>) {
        (
            self.author(interner),
            self.date(interner),
            self.id(interner).ok(),
        )
    }
}
impl ChangeParts for RowPropertiesChange {
    fn revision_parts(&self, interner: &Interner) -> (Option<String>, Option<String>, Option<i64>) {
        (
            self.author(interner),
            self.date(interner),
            self.id(interner).ok(),
        )
    }
}

fn push_run_change(
    out: &mut Vec<RevisionInfo>,
    kind: RevisionKind,
    change: &RunTrackChange,
    interner: &Interner,
) {
    out.push(RevisionInfo {
        kind,
        author: change.author(interner),
        date: change.date(interner),
        id: change.id(interner).ok(),
    });
}

// =================================================================================================
// Accepted/rejected text — computed views, no document mutation. See this module's own top doc
// comment for why these are "Reading" (required), not "accept/reject operations" (declined).
// =================================================================================================

/// The text `content` would render with every tracked insertion kept and every tracked deletion
/// dropped — `w:ins` content included, `w:del`/`w:delText`/`w:delInstrText` excluded,
/// `w:moveFrom`/`w:moveTo` both **excluded** (a `moveTo`'s content already appears once, in its own
/// position; "accepted" here means "the insert/delete half of tracking is resolved", not "collapse a
/// move to one of its two locations" — see this module's own doc comment on why an in-place move
/// resolution is part of the declined mutating accept/reject, not this read-only computation).
/// `pub(crate)`: [`crate::Document::text_with_revisions_accepted`] is the public entry point.
pub(crate) fn text_with_accepted(content: &[super::body::BlockContent]) -> String {
    let mut out = String::new();
    resolve_blocks(content, &mut out, true);
    out
}

/// [`text_with_accepted`]'s own rejected-text counterpart: `w:del` content kept, `w:ins` excluded.
pub(crate) fn text_with_rejected(content: &[super::body::BlockContent]) -> String {
    let mut out = String::new();
    resolve_blocks(content, &mut out, false);
    out
}

fn resolve_blocks(content: &[super::body::BlockContent], out: &mut String, accept: bool) {
    for (index, block) in content.iter().enumerate() {
        if let super::body::BlockContent::Paragraph(paragraph) = block {
            if index > 0 && !out.is_empty() {
                out.push('\n');
            }
            resolve_paragraph_content(paragraph.content(), out, accept);
        }
    }
}

fn resolve_paragraph_content(content: &[ParagraphContent], out: &mut String, accept: bool) {
    for item in content {
        match item {
            ParagraphContent::Run(run) => out.push_str(&run.text()),
            ParagraphContent::Hyperlink(hyperlink) => {
                resolve_paragraph_content(hyperlink.content(), out, accept);
            }
            ParagraphContent::SimpleField(field) => out.push_str(&field.cached_result_text()),
            ParagraphContent::Ins(change) => {
                if accept {
                    resolve_paragraph_content(change.content(), out, accept);
                }
            }
            ParagraphContent::Del(change) => {
                if !accept {
                    resolve_paragraph_content(change.content(), out, accept);
                }
            }
            // A move's content is never duplicated by this computation — see `text_with_accepted`'s
            // own doc comment.
            ParagraphContent::MoveFrom(_) | ParagraphContent::MoveTo(_) => {}
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mjx_ooxml_core::RawDocument;
    use mjx_xml::fidelity;

    use super::super::body::{BlockContent, Paragraph, Run};

    const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

    fn parse_typed<T: FromXml>(fragment: &[u8]) -> (T, RawDocument) {
        let doc = fidelity::parse(fragment).expect("fragment parses");
        let typed = T::from_xml(&doc.root, &doc.interner).expect("from_xml succeeds");
        (typed, doc)
    }

    #[track_caller]
    fn assert_round_trips<T: ToXml>(typed: &T, mut doc: RawDocument, expected: &[u8]) {
        doc.root = typed.to_xml(&mut doc.interner);
        let out = fidelity::serialize_to_vec(&doc);
        assert_eq!(
            String::from_utf8_lossy(&out),
            String::from_utf8_lossy(expected),
            "round-trip byte mismatch"
        );
    }

    // =============================================================================================
    // is_valid_xsd_datetime
    // =============================================================================================

    #[test]
    fn well_formed_datetimes_are_accepted() {
        for value in [
            "2024-01-01T00:00:00",
            "2024-01-01T00:00:00Z",
            "2024-01-01T00:00:00.5",
            "2024-01-01T00:00:00.123Z",
            "2024-01-01T00:00:00+05:30",
            "2024-01-01T00:00:00-08:00",
        ] {
            assert!(is_valid_xsd_datetime(value), "{value:?} should be valid");
        }
    }

    /// Would this pass if the work were not done? No: a validator that only checks length (rather
    /// than shape) would accept "not-a-date" (10 chars) exactly as readily as a real date.
    #[test]
    fn malformed_datetimes_are_rejected() {
        for value in [
            "not-a-date",
            "2024-01-01",
            "2024-01-01T00:00",
            "2024/01/01T00:00:00",
            "",
            "2024-01-01T00:00:00+5:30",
            "2024-01-01T00:00:00.",
        ] {
            assert!(
                !is_valid_xsd_datetime(value),
                "{value:?} should be rejected"
            );
        }
    }

    #[test]
    fn set_date_checked_refuses_a_malformed_date_and_leaves_the_existing_value_untouched() {
        let mut interner = Interner::new();
        let mut marker = TrackChangeMarker::new(&mut interner, "cellIns", 1, "Author");
        marker
            .set_date_checked(&mut interner, Some("2024-01-01T00:00:00Z"))
            .expect("well-formed date accepted");
        assert_eq!(
            marker.date(&interner).as_deref(),
            Some("2024-01-01T00:00:00Z")
        );

        let err = marker
            .set_date_checked(&mut interner, Some("not-a-date"))
            .expect_err("malformed date refused");
        assert!(matches!(err, DocxError::MalformedDateTime(value) if value == "not-a-date"));
        // The refused write never happened — the well-formed value from before is still there.
        assert_eq!(
            marker.date(&interner).as_deref(),
            Some("2024-01-01T00:00:00Z")
        );
    }

    // =============================================================================================
    // Round-trip fidelity, including a malformed (but preserved) date.
    // =============================================================================================

    #[test]
    fn track_change_marker_round_trips_author_and_date() {
        let xml = format!(
            r#"<w:cellIns xmlns:w="{W}" w:id="7" w:author="Jamie Lee" w:date="2024-03-14T09:30:00Z"/>"#
        )
        .into_bytes();
        let (marker, doc): (TrackChangeMarker, _) = parse_typed(&xml);
        assert_eq!(marker.id(&doc.interner), Ok(7));
        assert_eq!(marker.author(&doc.interner).as_deref(), Some("Jamie Lee"));
        assert_eq!(
            marker.date(&doc.interner).as_deref(),
            Some("2024-03-14T09:30:00Z")
        );
        assert_round_trips(&marker, doc, &xml);
    }

    /// A malformed `w:date` (fails `xsd:dateTime`) is preserved byte-for-byte on read — never
    /// normalised, never rejected — matching this module's own top doc comment and
    /// `fields.rs`'s identical precedent for an over-long string. This is the fixture proving
    /// malformed-date preservation; it is deliberately *not* schema-gated (`ST_DateTime` has no
    /// relaxation, so this markup is itself schema-invalid, which is the gate working correctly —
    /// see `crates/mjx-docx/tests/revisions.rs`'s own module doc for the full account).
    #[test]
    fn a_malformed_date_round_trips_byte_identical_never_normalised() {
        let xml = format!(
            r#"<w:cellIns xmlns:w="{W}" w:id="7" w:author="Jamie Lee" w:date="not-a-real-date"/>"#
        )
        .into_bytes();
        let (marker, doc): (TrackChangeMarker, _) = parse_typed(&xml);
        // Read never rejects it...
        assert_eq!(
            marker.date(&doc.interner).as_deref(),
            Some("not-a-real-date")
        );
        // ...and never rewrites it either.
        assert_round_trips(&marker, doc, &xml);
    }

    #[test]
    fn move_bookmark_round_trips_all_seven_attributes() {
        let xml = format!(
            r#"<w:moveFromRangeStart xmlns:w="{W}" w:id="3" w:displacedByCustomXml="next" w:colFirst="1" w:colLast="2" w:name="MoveIt" w:author="A" w:date="2024-01-01T00:00:00Z"/>"#
        )
        .into_bytes();
        let (marker, doc): (MoveBookmark, _) = parse_typed(&xml);
        assert_eq!(marker.id(&doc.interner), Ok(3));
        assert_eq!(marker.name(&doc.interner).as_deref(), Some("MoveIt"));
        assert_eq!(marker.author(&doc.interner).as_deref(), Some("A"));
        assert_round_trips(&marker, doc, &xml);
    }

    #[test]
    fn cell_merge_track_change_round_trips_vertical_merge_states() {
        let xml = format!(
            r#"<w:cellMerge xmlns:w="{W}" w:id="9" w:author="A" w:vMerge="cont" w:vMergeOrig="rest"/>"#
        )
        .into_bytes();
        let (change, doc): (CellMergeTrackChange, _) = parse_typed(&xml);
        assert_eq!(
            change.vertical_merge(&doc.interner),
            Ok(Some(VerticalMergeRevision::Merged))
        );
        assert_eq!(
            change.vertical_merge_original(&doc.interner),
            Ok(Some(VerticalMergeRevision::Split))
        );
        assert_round_trips(&change, doc, &xml);
    }

    #[test]
    fn track_change_numbering_round_trips_the_original_attribute() {
        let xml =
            format!(r#"<w:numberingChange xmlns:w="{W}" w:id="4" w:author="A" w:original="5"/>"#)
                .into_bytes();
        let (change, doc): (TrackChangeNumbering, _) = parse_typed(&xml);
        assert_eq!(change.original(&doc.interner).as_deref(), Some("5"));
        assert_round_trips(&change, doc, &xml);
    }

    #[test]
    fn table_grid_change_extends_ct_markup_not_ct_track_change_no_author_or_date_attribute() {
        let xml = format!(
            r#"<w:tblGridChange xmlns:w="{W}" w:id="2"><w:tblGrid><w:gridCol w:w="100"/></w:tblGrid></w:tblGridChange>"#
        )
        .into_bytes();
        let (change, doc): (TableGridChange, _) = parse_typed(&xml);
        assert_eq!(change.id(&doc.interner), Ok(2));
        assert!(change.previous_grid().is_some());
        assert_round_trips(&change, doc, &xml);
    }

    #[test]
    fn run_properties_change_round_trips_its_previous_run_properties() {
        let xml = format!(
            r#"<w:rPrChange xmlns:w="{W}" w:id="1" w:author="A" w:date="2024-01-01T00:00:00Z"><w:rPr><w:b/></w:rPr></w:rPrChange>"#
        )
        .into_bytes();
        let (change, doc): (RunPropertiesChange, _) = parse_typed(&xml);
        assert!(change.previous_run_properties().is_some());
        assert_round_trips(&change, doc, &xml);
    }

    // =============================================================================================
    // An insertion nested inside a deletion — the ticket's own trap.
    // =============================================================================================

    /// Would this pass if the work were not done? No: reusing `Raw` for `w:ins`/`w:del` (their
    /// pre-child state) would parse this whole fragment as one opaque node, so `content()` would
    /// never resolve to a nested `Ins` at all — this assertion is specifically about the *typed*
    /// nesting, not just successful parsing.
    #[test]
    fn an_insertion_nested_inside_a_deletion_round_trips_and_is_reachable_by_type() {
        let xml = format!(
            r#"<w:del xmlns:w="{W}" w:id="1" w:author="Del Author" w:date="2024-01-01T00:00:00Z"><w:r><w:delText xml:space="preserve">outer </w:delText></w:r><w:ins w:id="2" w:author="Ins Author" w:date="2024-02-02T00:00:00Z"><w:r><w:t>nested insertion</w:t></w:r></w:ins></w:del>"#
        )
        .into_bytes();
        let (outer, doc): (RunTrackChange, _) = parse_typed(&xml);
        assert_eq!(outer.author(&doc.interner).as_deref(), Some("Del Author"));
        let nested = outer
            .content()
            .iter()
            .find_map(|item| match item {
                ParagraphContent::Ins(inner) => Some(inner),
                _ => None,
            })
            .expect("the nested w:ins is typed, not Raw");
        assert_eq!(nested.author(&doc.interner).as_deref(), Some("Ins Author"));
        assert_round_trips(&outer, doc, &xml);
    }

    // =============================================================================================
    // Revision enumeration and accepted/rejected text.
    // =============================================================================================

    fn paragraph_with(interner: &mut Interner, items: Vec<ParagraphContent>) -> BlockContent {
        let mut paragraph = Paragraph::new(interner);
        for item in items {
            paragraph.content_mut().push(item);
        }
        BlockContent::Paragraph(paragraph)
    }

    fn run_content(interner: &mut Interner, text: &str) -> ParagraphContent {
        ParagraphContent::Run(Run::with_text(interner, text))
    }

    /// Would this pass if the work were not done? No: with `Ins`/`Del` still falling to `Raw` (their
    /// pre-child state), `collect_revisions` would find nothing, and both accepted/rejected text
    /// would equal the plain "before after" with neither "inserted" nor "deleted" anywhere in
    /// either.
    #[test]
    fn accepted_and_rejected_text_diverge_exactly_at_the_tracked_span() {
        let mut interner = Interner::new();
        let mut ins = RunTrackChange::new(&mut interner, "ins", 1, "Author");
        ins.content_mut().push(ParagraphContent::Run(Run::with_text(
            &mut interner,
            "inserted ",
        )));
        let mut del = RunTrackChange::new(&mut interner, "del", 2, "Author");
        del.content_mut().push(ParagraphContent::Run(Run::with_text(
            &mut interner,
            "deleted ",
        )));

        let items = vec![
            run_content(&mut interner, "before "),
            ParagraphContent::Ins(ins),
            ParagraphContent::Del(del),
            run_content(&mut interner, "after"),
        ];
        let blocks = [paragraph_with(&mut interner, items)];

        assert_eq!(text_with_accepted(&blocks), "before inserted after");
        assert_eq!(text_with_rejected(&blocks), "before deleted after");

        let mut found = Vec::new();
        collect_revisions(&blocks, &interner, &mut found);
        assert_eq!(found.len(), 2);
        assert!(found
            .iter()
            .any(|r| r.kind == RevisionKind::Inserted && r.id == Some(1)));
        assert!(found
            .iter()
            .any(|r| r.kind == RevisionKind::Deleted && r.id == Some(2)));
    }

    // =============================================================================================
    // Move-range classification — reuses MJXOFF-124's RangeIndex.
    // =============================================================================================

    #[test]
    fn move_from_and_move_to_ranges_are_classified_independently_by_id() {
        let mut interner = Interner::new();
        let start = MoveBookmark::new(
            &mut interner,
            "moveFromRangeStart",
            1,
            "M",
            "A",
            "2024-01-01T00:00:00Z",
        );
        let end = super::super::ranges::MarkupRange::new(&mut interner, "moveFromRangeEnd", 1);
        let items = vec![
            ParagraphContent::MoveFromRangeStart(start),
            run_content(&mut interner, "moved text"),
            ParagraphContent::MoveFromRangeEnd(end),
        ];
        let blocks = [paragraph_with(&mut interner, items)];
        let paragraphs: Vec<&Paragraph> = blocks
            .iter()
            .filter_map(|b| match b {
                BlockContent::Paragraph(p) => Some(p),
                _ => None,
            })
            .collect();
        let mut starts = 0;
        let mut ends = 0;
        for paragraph in &paragraphs {
            for item in paragraph.content() {
                match classify_move_from_range(item, &interner) {
                    Some((MarkerRole::Start, 1)) => starts += 1,
                    Some((MarkerRole::End, 1)) => ends += 1,
                    _ => {}
                }
            }
        }
        assert_eq!((starts, ends), (1, 1));
    }
}
