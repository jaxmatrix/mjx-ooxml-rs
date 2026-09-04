//! Comments, footnotes and endnotes (`w:comments`/`w:footnotes`/`w:endnotes`, `CT_Comments`/
//! `CT_Footnotes`/`CT_Endnotes`) — three of `wml.xsd`'s fourteen global elements — and the section-
//! level `w:footnotePr`/`w:endnotePr` (`CT_FtnProps`/`CT_EdnProps`) C9 (MJXOFF-109) left structurally
//! opaque on [`super::sections::SectionProperties`] for this child to give real content.
//!
//! All three parts share one shape with bookmarks (`ranges.rs`): **content lives in another part,
//! referenced by id from the body**. Comments use `ranges.rs`'s own range mechanism (a
//! `commentRangeStart`/`commentRangeEnd` pair marks the extent, `w:commentReference` — one
//! `CT_Markup`, a point, not a range — names which [`Comment`] it is). Footnotes and endnotes are
//! simpler: `w:footnoteReference`/`w:endnoteReference` (`CT_FtnEdnRef`, [`FootnoteEndnoteReference`])
//! is a single point in the run stream, naming the [`FootnoteEndnote`] by id directly — ECMA-376 gives
//! a footnote/endnote no *start*/*end* pair to range-resolve, only the one mark where the reference
//! number renders.
//!
//! # The reserved `separator`/`continuationSeparator`/`continuationNotice` entries
//!
//! A footnotes or endnotes part carries, alongside every user-visible footnote, one or more entries
//! whose `w:type` is `separator`, `continuationSeparator` or `continuationNotice` (`ST_FtnEdn`) rather
//! than `normal` (the default when `w:type` is omitted — ECMA-376 Part 1 §17.11.10's own "id"
//! attribute table entry: *"If this attribute \[`type`\] is omitted, then it shall be considered to be
//! of style normal"*). §17.11.10 also states directly: *"If a footnote or endnote is not of style
//! normal, then it shall not be referenced by a footnoteReference or endnoteReference element within
//! the main document story."* These entries draw the horizontal separator line Word renders above a
//! page's footnotes — they are never a user's own footnote, and Word repairs (silently rewrites) a
//! `footnotes.xml`/`endnotes.xml` that lacks them.
//!
//! **`w:type` is the authoritative discriminant — not `w:id`.** The ticket's own dispatch note reads
//! "conventionally ids `0` and `-1`" — deliberately hedged, and checking the prose confirms why:
//! §17.11.1's own worked example writes `<w:footnote w:type="continuationSeparator" w:id="1">` and
//! §17.11.23's writes `<w:footnote w:type="separator" w:id="0">` — the *specification's own examples*
//! use `0`/`1`, not `-1`/`0`. Real Word output commonly uses `-1` for the separator and `0` for the
//! continuation separator, but nothing in the schema or the prose pins those specific values down;
//! only `w:type` does. [`FootnoteEndnote::is_user_visible`] reads `w:type`, never `w:id`, and
//! [`Footnotes::user_footnotes`]/[`Endnotes::user_endnotes`] filter on it — excluding the reserved
//! entries from the user-visible list is a `w:type` check, not an id range check. On authoring a fresh
//! part ([`Footnotes::blank`]/[`Endnotes::blank`]), this crate follows the common real-world
//! convention (`-1` separator, `0` continuation separator) since it is what a reader most likely to
//! expect a hand-authored file to look like, but a `w:type` check is what makes that choice
//! non-load-bearing.
//!
//! # Footnote/endnote ids are never reused, and reserved ids are skipped when assigning a fresh one
//!
//! [`Footnotes::next_user_id`]/[`Endnotes::next_user_id`] look at *every* entry's id (reserved ones
//! included, since a document is free to spend `-1`/`0` on them) and hand back one strictly greater
//! than the maximum found — the same "never renumber, never collide" rule
//! `Document::next_rid_for`/`next_header_footer_part` already apply to relationship ids and header/
//! footer part names.

use mjx_ooxml_core::{
    Enumeration, FromXml, FromXmlError, Interner, Number, RawAttribute, RawElement, RawName,
    RawNode, Text as TextCodec, ToXml,
};
use mjx_ooxml_types::support::OnOff;
use mjx_ooxml_types::wordprocessingml::{
    DecimalNumber, EndnotePosition, FootnoteEndnoteType, FootnotePosition, NumberFormat,
    NumberingRestartLocation,
};

use super::body::{
    block_paragraph, block_paragraph_mut, block_paragraphs, block_tables, wml_name, BlockContent,
    Paragraph, Run, RunInnerContent, Unmodeled,
};
use super::paragraph_properties::DecimalNumberValue;
use crate::address::BlockPath;

// =================================================================================================
// w:comments (CT_Comments) and w:comment (CT_Comment)
// =================================================================================================

/// `w:comments` (`CT_Comments`) — `word/comments.xml`'s own root: every [`Comment`] in the part, in
/// document order (which is *not* reading order — a comment's position in this part is independent of
/// where its `w:commentRangeStart`/`w:commentReference` sit in the body; see
/// [`crate::Document::add_comment`]).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct Comments {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "comment", variant = Comment, ty = Comment))]
    content: Vec<CommentsContent>,
}

/// One ordered child of [`Comments`]: `CT_Comments`'s own single repeatable element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommentsContent {
    /// `w:comment` (`CT_Comment`).
    Comment(Comment),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

impl Comments {
    /// Every comment in the part, in document order (see this type's own doc comment for why that is
    /// not reading order).
    pub fn comments(&self) -> impl Iterator<Item = &Comment> {
        self.content.iter().filter_map(|item| match item {
            CommentsContent::Comment(comment) => Some(comment),
            _ => None,
        })
    }

    /// The comment with this `id`, or `None` if none has it.
    #[must_use]
    pub fn comment(&self, interner: &Interner, id: i64) -> Option<&Comment> {
        self.comments()
            .find(|comment| comment.id(interner) == Ok(id))
    }

    /// Appends `comment` as the part's new last entry.
    pub(crate) fn push(&mut self, comment: Comment) {
        self.content.push(CommentsContent::Comment(comment));
        self.empty = false;
    }

    /// Removes and returns the comment with this `id`, or `None` if none has it.
    pub(crate) fn remove(&mut self, interner: &Interner, id: i64) -> Option<Comment> {
        let at = self
            .content
            .iter()
            .position(|item| matches!(item, CommentsContent::Comment(comment) if comment.id(interner) == Ok(id)))?;
        match self.content.remove(at) {
            CommentsContent::Comment(comment) => Some(comment),
            _ => unreachable!("position matched only a Comment item"),
        }
    }

    /// One past the highest `id` any comment in the part already carries — `0` for an empty part
    /// (comment ids are conventionally 1-based; nothing in the schema requires it, but nothing this
    /// crate authors collides with an existing one either way).
    #[must_use]
    pub(crate) fn next_id(&self, interner: &Interner) -> i64 {
        self.comments()
            .filter_map(|comment| comment.id(interner).ok())
            .max()
            .map_or(0, |max| max + 1)
    }
}

/// `w:comment` (`CT_Comment`, extends `CT_TrackChange` extends `CT_Markup`) — one comment's own
/// metadata (id, author, date, initials) and block content (any paragraphs/tables it holds — the same
/// container mechanism [`super::body::Body`]/[`super::headers::HdrFtr`]/[`super::tables::Cell`] share,
/// a fourth consumer of it rather than a copy). `w15:` threaded-reply extensions a real Word file
/// carries live in `extra` — never modeled, never dropped.
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = WML)]
#[xml(attribute(local = "id", prefix = "w", codec = Number<DecimalNumber>, accessor = id, required))]
#[xml(attribute(local = "author", prefix = "w", codec = TextCodec, accessor = raw_author, required))]
#[xml(attribute(local = "date", prefix = "w", codec = TextCodec, accessor = raw_date))]
#[xml(attribute(local = "initials", prefix = "w", codec = TextCodec, accessor = raw_initials))]
pub struct Comment {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "customXml", variant = CustomXml, ty = super::body::Unmodeled),
        child(local = "sdt", variant = StructuredDocumentTag, ty = super::body::Unmodeled),
        child(local = "p", variant = Paragraph, ty = Paragraph),
        child(local = "tbl", variant = Table, ty = super::tables::Table),
        child(local = "proofErr", variant = ProofingError, ty = super::body::ProofingError),
        child(local = "permStart", variant = PermissionRangeStart, ty = super::body::PermissionRangeStart),
        child(local = "permEnd", variant = PermissionRangeEnd, ty = super::body::PermissionRangeEnd),
        child(local = "sectPr", variant = SectionProperties, ty = super::sections::SectionProperties),
        child(local = "tcPr", variant = Properties, ty = super::tables::CellProperties)
    )]
    content: Vec<BlockContent>,
}

impl Comment {
    /// Builds a new comment: `id`, `author`, optional `initials`, one empty paragraph ready for
    /// [`Comment::append_paragraph`]/[`crate::Document::set_run_text`] to fill in.
    #[must_use]
    pub(crate) fn new(interner: &mut Interner, id: i64, author: &str) -> Self {
        let mut value = Self {
            name: wml_name(interner, "comment"),
            attributes: Vec::new(),
            empty: false,
            content: vec![BlockContent::Paragraph(Paragraph::new(interner))],
        };
        value.set_id(interner, id);
        value.set_raw_author(interner, author);
        value
    }

    /// The comment's author (`w:author`), or `None` if malformed.
    #[must_use]
    pub fn author(&self, interner: &Interner) -> Option<String> {
        self.raw_author(interner).ok().map(|cow| cow.into_owned())
    }

    /// The comment's own date/time stamp (`w:date`, `ST_DateTime` — an opaque ISO-8601-ish wire
    /// string; this crate does not parse or validate it), or `None` if absent/malformed.
    #[must_use]
    pub fn date(&self, interner: &Interner) -> Option<String> {
        self.raw_date(interner)
            .ok()
            .flatten()
            .map(|cow| cow.into_owned())
    }

    /// The comment author's initials (`w:initials`), or `None` if absent/malformed.
    #[must_use]
    pub fn initials(&self, interner: &Interner) -> Option<String> {
        self.raw_initials(interner)
            .ok()
            .flatten()
            .map(|cow| cow.into_owned())
    }

    /// Every paragraph directly in this comment's content, in document order.
    pub fn paragraphs(&self) -> impl Iterator<Item = &Paragraph> {
        block_paragraphs(&self.content)
    }

    /// How many paragraphs this comment holds directly.
    #[must_use]
    pub fn paragraph_count(&self) -> usize {
        self.paragraphs().count()
    }

    /// The paragraph at `path`, or `None` if the address is out of range.
    #[must_use]
    pub fn paragraph(&self, path: impl Into<BlockPath>) -> Option<&Paragraph> {
        block_paragraph(&self.content, &path.into())
    }

    /// [`Comment::paragraph`], mutably.
    pub fn paragraph_mut(&mut self, path: impl Into<BlockPath>) -> Option<&mut Paragraph> {
        block_paragraph_mut(&mut self.content, &path.into())
    }

    /// Appends `paragraph` as this comment's new last paragraph.
    pub fn append_paragraph(&mut self, paragraph: Paragraph) {
        self.content.push(BlockContent::Paragraph(paragraph));
        self.empty = false;
    }

    /// Every table directly in this comment's content.
    pub fn tables(&self) -> impl Iterator<Item = &super::tables::Table> {
        block_tables(&self.content)
    }

    /// This comment's own text: every paragraph joined by `\n`, matching
    /// [`super::tables::Cell::text`]'s own convention.
    #[must_use]
    pub fn text(&self) -> String {
        self.paragraphs()
            .map(Paragraph::text)
            .collect::<Vec<_>>()
            .join("\n")
    }
}

// =================================================================================================
// w:footnotes / w:endnotes (CT_Footnotes / CT_Endnotes) and w:footnote / w:endnote (CT_FtnEdn)
// =================================================================================================

/// `w:footnotes` (`CT_Footnotes`) — `word/footnotes.xml`'s own root.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct Footnotes {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "footnote", variant = Footnote, ty = FootnoteEndnote))]
    content: Vec<FootnotesContent>,
}

/// One ordered child of [`Footnotes`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FootnotesContent {
    /// `w:footnote` (`CT_FtnEdn`).
    Footnote(FootnoteEndnote),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

/// `w:endnotes` (`CT_Endnotes`) — `word/endnotes.xml`'s own root. A distinct Rust type from
/// [`Footnotes`] even though `wml.xsd` gives the two an identical shape (`CT_Endnotes` mirrors
/// `CT_Footnotes` element for element): which document.xml relationship this part answers to (and so
/// which `w:footnoteReference`-vs-`w:endnoteReference` ids it resolves) is a fact about the *part*,
/// not the content model, and keeping the two apart at the type level is what stops a
/// [`Document::add_footnote`](crate::Document::add_footnote) caller from ever handing one back to
/// [`Document::edit_endnotes`](crate::Document::edit_endnotes) by accident.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct Endnotes {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "endnote", variant = Endnote, ty = FootnoteEndnote))]
    content: Vec<EndnotesContent>,
}

/// One ordered child of [`Endnotes`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EndnotesContent {
    /// `w:endnote` (`CT_FtnEdn`).
    Endnote(FootnoteEndnote),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

/// The reserved separator id this crate writes when authoring a fresh footnotes/endnotes part — see
/// this module's own doc comment for why the specific value is a convention, not a schema constraint.
const RESERVED_SEPARATOR_ID: i64 = -1;
/// The reserved continuation-separator id this crate writes when authoring a fresh part.
const RESERVED_CONTINUATION_SEPARATOR_ID: i64 = 0;

impl Footnotes {
    /// Builds a brand-new `word/footnotes.xml` root carrying **only** the two reserved entries (`-1`
    /// separator, `0` continuationSeparator) every footnotes part needs — Word repairs a file that
    /// lacks them (this module's own doc comment). No user footnote yet; [`Document::add_footnote`]
    /// appends those.
    #[must_use]
    pub(crate) fn blank(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "footnotes"),
            attributes: Vec::new(),
            empty: false,
            content: vec![
                FootnotesContent::Footnote(FootnoteEndnote::reserved(
                    interner,
                    "footnote",
                    RESERVED_SEPARATOR_ID,
                    FootnoteEndnoteType::Separator,
                )),
                FootnotesContent::Footnote(FootnoteEndnote::reserved(
                    interner,
                    "footnote",
                    RESERVED_CONTINUATION_SEPARATOR_ID,
                    FootnoteEndnoteType::ContinuationSeparator,
                )),
            ],
        }
    }

    /// Every entry in the part, reserved separator entries included, in document order. Prefer
    /// [`Footnotes::user_footnotes`] for anything a person would recognize as "the footnotes" — this
    /// is the raw list, for completeness (round-tripping, counting the reserved entries themselves).
    pub fn footnotes(&self) -> impl Iterator<Item = &FootnoteEndnote> {
        self.content.iter().filter_map(|item| match item {
            FootnotesContent::Footnote(footnote) => Some(footnote),
            _ => None,
        })
    }

    /// Every **user-visible** footnote — `w:type` absent or `normal` — excluding the reserved
    /// `separator`/`continuationSeparator`/`continuationNotice` entries. This is "the footnote count"
    /// a caller almost always means; see this module's own doc comment for why a count that includes
    /// the reserved entries is simply wrong.
    pub fn user_footnotes<'a>(
        &'a self,
        interner: &'a Interner,
    ) -> impl Iterator<Item = &'a FootnoteEndnote> + 'a {
        self.footnotes()
            .filter(|footnote| footnote.is_user_visible(interner))
    }

    /// The footnote with this `id` (reserved entries included — a caller who already has the id from
    /// a `w:footnoteReference` wants exactly that footnote, whichever kind it turns out to be), or
    /// `None` if none has it.
    #[must_use]
    pub fn footnote(&self, interner: &Interner, id: i64) -> Option<&FootnoteEndnote> {
        self.footnotes()
            .find(|footnote| footnote.id(interner) == Ok(id))
    }

    pub(crate) fn push(&mut self, footnote: FootnoteEndnote) {
        self.content.push(FootnotesContent::Footnote(footnote));
        self.empty = false;
    }

    pub(crate) fn remove(&mut self, interner: &Interner, id: i64) -> Option<FootnoteEndnote> {
        let at = self.content.iter().position(
            |item| matches!(item, FootnotesContent::Footnote(footnote) if footnote.id(interner) == Ok(id)),
        )?;
        match self.content.remove(at) {
            FootnotesContent::Footnote(footnote) => Some(footnote),
            _ => unreachable!("position matched only a Footnote item"),
        }
    }

    /// One past the highest id **any** entry (reserved ones included) already carries — see this
    /// module's own doc comment for why reserved ids are not skipped over here, only never reused.
    #[must_use]
    pub(crate) fn next_user_id(&self, interner: &Interner) -> i64 {
        self.footnotes()
            .filter_map(|footnote| footnote.id(interner).ok())
            .max()
            .map_or(1, |max| max + 1)
    }
}

impl Endnotes {
    /// As [`Footnotes::blank`], for endnotes.
    #[must_use]
    pub(crate) fn blank(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "endnotes"),
            attributes: Vec::new(),
            empty: false,
            content: vec![
                EndnotesContent::Endnote(FootnoteEndnote::reserved(
                    interner,
                    "endnote",
                    RESERVED_SEPARATOR_ID,
                    FootnoteEndnoteType::Separator,
                )),
                EndnotesContent::Endnote(FootnoteEndnote::reserved(
                    interner,
                    "endnote",
                    RESERVED_CONTINUATION_SEPARATOR_ID,
                    FootnoteEndnoteType::ContinuationSeparator,
                )),
            ],
        }
    }

    /// As [`Footnotes::footnotes`], for endnotes.
    pub fn endnotes(&self) -> impl Iterator<Item = &FootnoteEndnote> {
        self.content.iter().filter_map(|item| match item {
            EndnotesContent::Endnote(endnote) => Some(endnote),
            _ => None,
        })
    }

    /// As [`Footnotes::user_footnotes`], for endnotes.
    pub fn user_endnotes<'a>(
        &'a self,
        interner: &'a Interner,
    ) -> impl Iterator<Item = &'a FootnoteEndnote> + 'a {
        self.endnotes()
            .filter(|endnote| endnote.is_user_visible(interner))
    }

    /// As [`Footnotes::footnote`], for endnotes.
    #[must_use]
    pub fn endnote(&self, interner: &Interner, id: i64) -> Option<&FootnoteEndnote> {
        self.endnotes()
            .find(|endnote| endnote.id(interner) == Ok(id))
    }

    pub(crate) fn push(&mut self, endnote: FootnoteEndnote) {
        self.content.push(EndnotesContent::Endnote(endnote));
        self.empty = false;
    }

    pub(crate) fn remove(&mut self, interner: &Interner, id: i64) -> Option<FootnoteEndnote> {
        let at = self.content.iter().position(
            |item| matches!(item, EndnotesContent::Endnote(endnote) if endnote.id(interner) == Ok(id)),
        )?;
        match self.content.remove(at) {
            EndnotesContent::Endnote(endnote) => Some(endnote),
            _ => unreachable!("position matched only an Endnote item"),
        }
    }

    /// As [`Footnotes::next_user_id`], for endnotes.
    #[must_use]
    pub(crate) fn next_user_id(&self, interner: &Interner) -> i64 {
        self.endnotes()
            .filter_map(|endnote| endnote.id(interner).ok())
            .max()
            .map_or(1, |max| max + 1)
    }
}

/// `w:footnote`/`w:endnote` (`CT_FtnEdn`) — one footnote or endnote's own id, kind (`w:type`,
/// `ST_FtnEdn` — `normal`/`separator`/`continuationSeparator`/`continuationNotice`), and block
/// content. The same Rust type serves both wire elements (`wml.xsd` gives them the identical
/// `CT_FtnEdn` type), exactly as [`super::body::Text`] serves four `EG_RunInnerContent` members.
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = WML)]
#[xml(attribute(local = "id", prefix = "w", codec = Number<DecimalNumber>, accessor = id, required))]
#[xml(attribute(local = "type", prefix = "w", codec = Enumeration<FootnoteEndnoteType>, accessor = kind))]
pub struct FootnoteEndnote {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "customXml", variant = CustomXml, ty = super::body::Unmodeled),
        child(local = "sdt", variant = StructuredDocumentTag, ty = super::body::Unmodeled),
        child(local = "p", variant = Paragraph, ty = Paragraph),
        child(local = "tbl", variant = Table, ty = super::tables::Table),
        child(local = "proofErr", variant = ProofingError, ty = super::body::ProofingError),
        child(local = "permStart", variant = PermissionRangeStart, ty = super::body::PermissionRangeStart),
        child(local = "permEnd", variant = PermissionRangeEnd, ty = super::body::PermissionRangeEnd),
        child(local = "sectPr", variant = SectionProperties, ty = super::sections::SectionProperties),
        child(local = "tcPr", variant = Properties, ty = super::tables::CellProperties)
    )]
    content: Vec<BlockContent>,
}

impl FootnoteEndnote {
    /// Builds a new **user-visible** footnote/endnote (`local` `"footnote"`/`"endnote"`): `id`, no
    /// `w:type` stated (schema default `normal`), one empty paragraph.
    #[must_use]
    pub(crate) fn new(interner: &mut Interner, local: &str, id: i64) -> Self {
        let mut value = Self {
            name: wml_name(interner, local),
            attributes: Vec::new(),
            empty: false,
            content: vec![BlockContent::Paragraph(Paragraph::new(interner))],
        };
        value.set_id(interner, id);
        value
    }

    /// Builds a reserved entry (`kind` `Separator`/`ContinuationSeparator`) — the content Word itself
    /// writes: one paragraph holding one run holding the matching `w:separator`/
    /// `w:continuationSeparator` mark.
    fn reserved(interner: &mut Interner, local: &str, id: i64, kind: FootnoteEndnoteType) -> Self {
        let mark_local = match kind {
            FootnoteEndnoteType::Separator => "separator",
            FootnoteEndnoteType::ContinuationSeparator => "continuationSeparator",
            FootnoteEndnoteType::Normal | FootnoteEndnoteType::ContinuationNotice => {
                unreachable!("reserved() is only called with Separator/ContinuationSeparator")
            }
        };
        let mark = Unmodeled::new(interner, mark_local);
        let mark_variant = match kind {
            FootnoteEndnoteType::Separator => RunInnerContent::FootnoteEndnoteSeparatorMark(mark),
            FootnoteEndnoteType::ContinuationSeparator => {
                RunInnerContent::ContinuationSeparatorMark(mark)
            }
            _ => unreachable!("checked above"),
        };
        let mut paragraph = Paragraph::new(interner);
        paragraph.append_run(Run::with_inner_content(interner, mark_variant));
        let mut value = Self {
            name: wml_name(interner, local),
            attributes: Vec::new(),
            empty: false,
            content: vec![BlockContent::Paragraph(paragraph)],
        };
        value.set_id(interner, id);
        value.set_kind(interner, Some(kind));
        value
    }

    /// Whether this entry is a footnote/endnote a caller should treat as one of "the document's own
    /// footnotes/endnotes" — `w:type` absent, or explicitly `normal`. `false` for `separator`,
    /// `continuationSeparator` and `continuationNotice` — see this module's own doc comment. A
    /// malformed `w:type` (present but not one of the four `ST_FtnEdn` values) is treated as visible,
    /// the same leniency this crate gives every other malformed-but-present attribute: an untrusted
    /// file's own violation of its own schema is never silently reclassified as "not a footnote".
    /// [`FootnoteEndnote::kind`] (the derive-generated accessor for `w:type`) is the one to reach for
    /// the raw value; this is the "does it belong in the user-visible list" question this module's
    /// own `user_footnotes`/`user_endnotes` need answered.
    #[must_use]
    pub fn is_user_visible(&self, interner: &Interner) -> bool {
        !matches!(
            self.kind(interner).ok().flatten(),
            Some(FootnoteEndnoteType::Separator)
                | Some(FootnoteEndnoteType::ContinuationSeparator)
                | Some(FootnoteEndnoteType::ContinuationNotice)
        )
    }

    /// Every paragraph directly in this entry's content, in document order.
    pub fn paragraphs(&self) -> impl Iterator<Item = &Paragraph> {
        block_paragraphs(&self.content)
    }

    /// How many paragraphs this entry holds directly.
    #[must_use]
    pub fn paragraph_count(&self) -> usize {
        self.paragraphs().count()
    }

    /// The paragraph at `path`, or `None` if the address is out of range.
    #[must_use]
    pub fn paragraph(&self, path: impl Into<BlockPath>) -> Option<&Paragraph> {
        block_paragraph(&self.content, &path.into())
    }

    /// [`FootnoteEndnote::paragraph`], mutably.
    pub fn paragraph_mut(&mut self, path: impl Into<BlockPath>) -> Option<&mut Paragraph> {
        block_paragraph_mut(&mut self.content, &path.into())
    }

    /// Appends `paragraph` as this entry's new last paragraph.
    pub fn append_paragraph(&mut self, paragraph: Paragraph) {
        self.content.push(BlockContent::Paragraph(paragraph));
    }

    /// Every table directly in this entry's content.
    pub fn tables(&self) -> impl Iterator<Item = &super::tables::Table> {
        block_tables(&self.content)
    }

    /// This entry's own text: every paragraph joined by `\n` — for a reserved separator entry this is
    /// simply empty (its content is a bare mark, no text run).
    #[must_use]
    pub fn text(&self) -> String {
        self.paragraphs()
            .map(Paragraph::text)
            .collect::<Vec<_>>()
            .join("\n")
    }
}

// =================================================================================================
// w:footnoteReference / w:endnoteReference (CT_FtnEdnRef)
// =================================================================================================

/// `CT_FtnEdnRef` — `w:footnoteReference`/`w:endnoteReference` (`EG_RunInnerContent`): the id of the
/// [`FootnoteEndnote`] this point reference names, and whether the run supplies its own custom
/// reference mark (`customMarkFollows`) rather than the auto-numbered one.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "customMarkFollows", prefix = "w", codec = OnOff, accessor = custom_mark_follows))]
#[xml(attribute(local = "id", prefix = "w", codec = Number<DecimalNumber>, accessor = id, required))]
pub struct FootnoteEndnoteReference {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl FootnoteEndnoteReference {
    /// Builds a new `local` reference (`"footnoteReference"`/`"endnoteReference"`) naming `id`, no
    /// custom mark.
    #[must_use]
    pub(crate) fn new(interner: &mut Interner, local: &str, id: i64) -> Self {
        let mut value = Self {
            name: wml_name(interner, local),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        value.set_id(interner, id);
        value
    }
}

impl FromXml for FootnoteEndnoteReference {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for FootnoteEndnoteReference {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// =================================================================================================
// w:footnotePr / w:endnotePr (CT_FtnProps / CT_EdnProps) — C9's own opaque slots, given real content.
// =================================================================================================

/// `w:pos` inside `w:footnotePr` (`CT_FtnPos`) — one required [`FootnotePosition`].
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<FootnotePosition>, accessor = value, required))]
pub struct FootnotePositionElement {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl FootnotePositionElement {
    #[must_use]
    fn new(interner: &mut Interner, value: FootnotePosition) -> Self {
        let mut element = Self {
            name: wml_name(interner, "pos"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        element.set_value(interner, value);
        element
    }
}

impl FromXml for FootnotePositionElement {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for FootnotePositionElement {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:pos` inside `w:endnotePr` (`CT_EdnPos`) — one required [`EndnotePosition`]. A distinct type from
/// [`FootnotePositionElement`] because `CT_FtnPos`/`CT_EdnPos` restrict `w:val` to two different
/// (though overlapping) `ST_*` enumerations — `ST_FtnPos` has four values, `ST_EdnPos` two.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<EndnotePosition>, accessor = value, required))]
pub struct EndnotePositionElement {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl EndnotePositionElement {
    #[must_use]
    fn new(interner: &mut Interner, value: EndnotePosition) -> Self {
        let mut element = Self {
            name: wml_name(interner, "pos"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        element.set_value(interner, value);
        element
    }
}

impl FromXml for EndnotePositionElement {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for EndnotePositionElement {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:numFmt` (`CT_NumFmt`) — shared by `w:footnotePr` and `w:endnotePr`: a required number format
/// (`ST_NumberFormat`, [`NumberFormat`]'s own 63 variants) and an optional custom format string.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<NumberFormat>, accessor = value, required))]
#[xml(attribute(local = "format", prefix = "w", codec = TextCodec, accessor = raw_format))]
pub struct NumberFormatElement {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl NumberFormatElement {
    #[must_use]
    fn new(interner: &mut Interner, value: NumberFormat) -> Self {
        let mut element = Self {
            name: wml_name(interner, "numFmt"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        element.set_value(interner, value);
        element
    }

    /// The custom format string (`@format`), or `None` if absent/malformed.
    #[must_use]
    pub fn format(&self, interner: &Interner) -> Option<String> {
        self.raw_format(interner)
            .ok()
            .flatten()
            .map(|cow| cow.into_owned())
    }
}

impl FromXml for NumberFormatElement {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for NumberFormatElement {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:numRestart` (`CT_NumRestart`) — one required [`NumberingRestartLocation`]
/// (`continuous`/`eachSect`/`eachPage`). Reachable at exactly one place in `wml.xsd`:
/// `EG_FtnEdnNumProps`, inside `CT_FtnProps`/`CT_EdnProps` — footnote/endnote numbering restart,
/// nothing to do with list numbering (`numbering.rs`'s own `CT_Num`/`CT_Lvl`).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<NumberingRestartLocation>, accessor = value, required))]
pub struct NumberRestartElement {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl NumberRestartElement {
    #[must_use]
    fn new(interner: &mut Interner, value: NumberingRestartLocation) -> Self {
        let mut element = Self {
            name: wml_name(interner, "numRestart"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        element.set_value(interner, value);
        element
    }
}

impl FromXml for NumberRestartElement {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for NumberRestartElement {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:footnotePr` (`CT_FtnProps`) — a section's own footnote settings: position, number format, start
/// number and restart rule (`EG_FtnEdnNumProps`).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct FootnoteProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "pos", variant = Position, ty = FootnotePositionElement),
        child(local = "numFmt", variant = NumberFormat, ty = NumberFormatElement),
        child(local = "numStart", variant = NumberStart, ty = DecimalNumberValue),
        child(local = "numRestart", variant = NumberRestart, ty = NumberRestartElement)
    )]
    content: Vec<FootnotePropertyContent>,
}

/// One ordered child of [`FootnoteProperties`]: `CT_FtnProps`'s own `pos?, numFmt?, (numStart?,
/// numRestart?)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FootnotePropertyContent {
    /// `w:pos` (`CT_FtnPos`).
    Position(FootnotePositionElement),
    /// `w:numFmt` (`CT_NumFmt`).
    NumberFormat(NumberFormatElement),
    /// `w:numStart` (`CT_DecimalNumber`).
    NumberStart(DecimalNumberValue),
    /// `w:numRestart` (`CT_NumRestart`).
    NumberRestart(NumberRestartElement),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

/// `w:endnotePr` (`CT_EdnProps`) — as [`FootnoteProperties`], for endnotes; a distinct type only
/// because `w:pos`'s own value type differs (`CT_EdnPos`, two values, vs. `CT_FtnPos`'s four).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct EndnoteProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "pos", variant = Position, ty = EndnotePositionElement),
        child(local = "numFmt", variant = NumberFormat, ty = NumberFormatElement),
        child(local = "numStart", variant = NumberStart, ty = DecimalNumberValue),
        child(local = "numRestart", variant = NumberRestart, ty = NumberRestartElement)
    )]
    content: Vec<EndnotePropertyContent>,
}

/// One ordered child of [`EndnoteProperties`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EndnotePropertyContent {
    /// `w:pos` (`CT_EdnPos`).
    Position(EndnotePositionElement),
    /// `w:numFmt` (`CT_NumFmt`).
    NumberFormat(NumberFormatElement),
    /// `w:numStart` (`CT_DecimalNumber`).
    NumberStart(DecimalNumberValue),
    /// `w:numRestart` (`CT_NumRestart`).
    NumberRestart(NumberRestartElement),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

/// Places `value` into `content` at `local`'s own schema rank in `order` (`FOOTNOTE_PROPERTIES`/
/// `ENDNOTE_PROPERTIES` — both `pos(0), numFmt(1), numStart(2), numRestart(3)`), replacing the
/// existing member `is_target` matches in place, or inserting at the position the rank table requires
/// when there is none — the same [`mjx_ooxml_types::child_order::ChildOrder::insert_index_of_names`]
/// pattern `fields.rs`'s own `place_at_rank` uses for the sibling ordered-sequence case.
fn place_at_rank<T>(
    content: &mut Vec<T>,
    order: &'static mjx_ooxml_types::child_order::ChildOrder,
    is_target: impl Fn(&T) -> bool,
    local_of: impl Fn(&T) -> &'static str,
    new_local: &'static str,
    value: T,
) {
    if let Some(at) = content.iter().position(is_target) {
        content[at] = value;
        return;
    }
    let ranks: Vec<Option<u16>> = content
        .iter()
        .map(|item| order.rank_of(None, local_of(item)))
        .collect();
    let at = order.insert_index_of_names(ranks.into_iter(), new_local);
    content.insert(at, value);
}

fn footnote_property_local(item: &FootnotePropertyContent) -> &'static str {
    match item {
        FootnotePropertyContent::Position(_) => "pos",
        FootnotePropertyContent::NumberFormat(_) => "numFmt",
        FootnotePropertyContent::NumberStart(_) => "numStart",
        FootnotePropertyContent::NumberRestart(_) => "numRestart",
        FootnotePropertyContent::Raw(_) => "",
    }
}

fn endnote_property_local(item: &EndnotePropertyContent) -> &'static str {
    match item {
        EndnotePropertyContent::Position(_) => "pos",
        EndnotePropertyContent::NumberFormat(_) => "numFmt",
        EndnotePropertyContent::NumberStart(_) => "numStart",
        EndnotePropertyContent::NumberRestart(_) => "numRestart",
        EndnotePropertyContent::Raw(_) => "",
    }
}

impl FootnoteProperties {
    /// Builds a new, empty `w:footnotePr`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "footnotePr"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// This section's own footnote position (`w:pos`), or `None` if it states none.
    #[must_use]
    pub fn position(&self, interner: &Interner) -> Option<FootnotePosition> {
        self.content.iter().find_map(|item| match item {
            FootnotePropertyContent::Position(element) => element.value(interner).ok(),
            _ => None,
        })
    }

    /// Sets (or, given `None`, removes) `w:pos`.
    pub fn set_position(&mut self, interner: &mut Interner, value: Option<FootnotePosition>) {
        let is_target =
            |item: &FootnotePropertyContent| matches!(item, FootnotePropertyContent::Position(_));
        match value {
            None => self.content.retain(|item| !is_target(item)),
            Some(value) => {
                let element = FootnotePositionElement::new(interner, value);
                place_at_rank(
                    &mut self.content,
                    mjx_ooxml_types::child_order::FOOTNOTE_PROPERTIES,
                    is_target,
                    footnote_property_local,
                    "pos",
                    FootnotePropertyContent::Position(element),
                );
            }
        }
        self.empty = false;
    }

    /// This section's own footnote number format (`w:numFmt`), or `None` if it states none.
    #[must_use]
    pub fn number_format(&self) -> Option<&NumberFormatElement> {
        self.content.iter().find_map(|item| match item {
            FootnotePropertyContent::NumberFormat(element) => Some(element),
            _ => None,
        })
    }

    /// Sets (or, given `None`, removes) `w:numFmt`.
    pub fn set_number_format(&mut self, interner: &mut Interner, value: Option<NumberFormat>) {
        let is_target = |item: &FootnotePropertyContent| {
            matches!(item, FootnotePropertyContent::NumberFormat(_))
        };
        match value {
            None => self.content.retain(|item| !is_target(item)),
            Some(value) => {
                let element = NumberFormatElement::new(interner, value);
                place_at_rank(
                    &mut self.content,
                    mjx_ooxml_types::child_order::FOOTNOTE_PROPERTIES,
                    is_target,
                    footnote_property_local,
                    "numFmt",
                    FootnotePropertyContent::NumberFormat(element),
                );
            }
        }
        self.empty = false;
    }

    /// This section's own footnote start number (`w:numStart`), or `None` if it states none.
    #[must_use]
    pub fn number_start(&self, interner: &Interner) -> Option<i64> {
        self.content.iter().find_map(|item| match item {
            FootnotePropertyContent::NumberStart(value) => value.value(interner).ok(),
            _ => None,
        })
    }

    /// Sets (or, given `None`, removes) `w:numStart`.
    pub fn set_number_start(&mut self, interner: &mut Interner, value: Option<i64>) {
        let is_target = |item: &FootnotePropertyContent| {
            matches!(item, FootnotePropertyContent::NumberStart(_))
        };
        match value {
            None => self.content.retain(|item| !is_target(item)),
            Some(value) => {
                let element = DecimalNumberValue::new(interner, "numStart", value);
                place_at_rank(
                    &mut self.content,
                    mjx_ooxml_types::child_order::FOOTNOTE_PROPERTIES,
                    is_target,
                    footnote_property_local,
                    "numStart",
                    FootnotePropertyContent::NumberStart(element),
                );
            }
        }
        self.empty = false;
    }

    /// This section's own footnote restart rule (`w:numRestart`), or `None` if it states none.
    #[must_use]
    pub fn number_restart(&self, interner: &Interner) -> Option<NumberingRestartLocation> {
        self.content.iter().find_map(|item| match item {
            FootnotePropertyContent::NumberRestart(element) => element.value(interner).ok(),
            _ => None,
        })
    }

    /// Sets (or, given `None`, removes) `w:numRestart`.
    pub fn set_number_restart(
        &mut self,
        interner: &mut Interner,
        value: Option<NumberingRestartLocation>,
    ) {
        let is_target = |item: &FootnotePropertyContent| {
            matches!(item, FootnotePropertyContent::NumberRestart(_))
        };
        match value {
            None => self.content.retain(|item| !is_target(item)),
            Some(value) => {
                let element = NumberRestartElement::new(interner, value);
                place_at_rank(
                    &mut self.content,
                    mjx_ooxml_types::child_order::FOOTNOTE_PROPERTIES,
                    is_target,
                    footnote_property_local,
                    "numRestart",
                    FootnotePropertyContent::NumberRestart(element),
                );
            }
        }
        self.empty = false;
    }
}

impl EndnoteProperties {
    /// Builds a new, empty `w:endnotePr`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "endnotePr"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// This section's own endnote position (`w:pos`), or `None` if it states none.
    #[must_use]
    pub fn position(&self, interner: &Interner) -> Option<EndnotePosition> {
        self.content.iter().find_map(|item| match item {
            EndnotePropertyContent::Position(element) => element.value(interner).ok(),
            _ => None,
        })
    }

    /// Sets (or, given `None`, removes) `w:pos`.
    pub fn set_position(&mut self, interner: &mut Interner, value: Option<EndnotePosition>) {
        let is_target =
            |item: &EndnotePropertyContent| matches!(item, EndnotePropertyContent::Position(_));
        match value {
            None => self.content.retain(|item| !is_target(item)),
            Some(value) => {
                let element = EndnotePositionElement::new(interner, value);
                place_at_rank(
                    &mut self.content,
                    mjx_ooxml_types::child_order::ENDNOTE_PROPERTIES,
                    is_target,
                    endnote_property_local,
                    "pos",
                    EndnotePropertyContent::Position(element),
                );
            }
        }
        self.empty = false;
    }

    /// This section's own endnote number format (`w:numFmt`), or `None` if it states none.
    #[must_use]
    pub fn number_format(&self) -> Option<&NumberFormatElement> {
        self.content.iter().find_map(|item| match item {
            EndnotePropertyContent::NumberFormat(element) => Some(element),
            _ => None,
        })
    }

    /// Sets (or, given `None`, removes) `w:numFmt`.
    pub fn set_number_format(&mut self, interner: &mut Interner, value: Option<NumberFormat>) {
        let is_target =
            |item: &EndnotePropertyContent| matches!(item, EndnotePropertyContent::NumberFormat(_));
        match value {
            None => self.content.retain(|item| !is_target(item)),
            Some(value) => {
                let element = NumberFormatElement::new(interner, value);
                place_at_rank(
                    &mut self.content,
                    mjx_ooxml_types::child_order::ENDNOTE_PROPERTIES,
                    is_target,
                    endnote_property_local,
                    "numFmt",
                    EndnotePropertyContent::NumberFormat(element),
                );
            }
        }
        self.empty = false;
    }

    /// This section's own endnote start number (`w:numStart`), or `None` if it states none.
    #[must_use]
    pub fn number_start(&self, interner: &Interner) -> Option<i64> {
        self.content.iter().find_map(|item| match item {
            EndnotePropertyContent::NumberStart(value) => value.value(interner).ok(),
            _ => None,
        })
    }

    /// Sets (or, given `None`, removes) `w:numStart`.
    pub fn set_number_start(&mut self, interner: &mut Interner, value: Option<i64>) {
        let is_target =
            |item: &EndnotePropertyContent| matches!(item, EndnotePropertyContent::NumberStart(_));
        match value {
            None => self.content.retain(|item| !is_target(item)),
            Some(value) => {
                let element = DecimalNumberValue::new(interner, "numStart", value);
                place_at_rank(
                    &mut self.content,
                    mjx_ooxml_types::child_order::ENDNOTE_PROPERTIES,
                    is_target,
                    endnote_property_local,
                    "numStart",
                    EndnotePropertyContent::NumberStart(element),
                );
            }
        }
        self.empty = false;
    }

    /// This section's own endnote restart rule (`w:numRestart`), or `None` if it states none.
    #[must_use]
    pub fn number_restart(&self, interner: &Interner) -> Option<NumberingRestartLocation> {
        self.content.iter().find_map(|item| match item {
            EndnotePropertyContent::NumberRestart(element) => element.value(interner).ok(),
            _ => None,
        })
    }

    /// Sets (or, given `None`, removes) `w:numRestart`.
    pub fn set_number_restart(
        &mut self,
        interner: &mut Interner,
        value: Option<NumberingRestartLocation>,
    ) {
        let is_target = |item: &EndnotePropertyContent| {
            matches!(item, EndnotePropertyContent::NumberRestart(_))
        };
        match value {
            None => self.content.retain(|item| !is_target(item)),
            Some(value) => {
                let element = NumberRestartElement::new(interner, value);
                place_at_rank(
                    &mut self.content,
                    mjx_ooxml_types::child_order::ENDNOTE_PROPERTIES,
                    is_target,
                    endnote_property_local,
                    "numRestart",
                    EndnotePropertyContent::NumberRestart(element),
                );
            }
        }
        self.empty = false;
    }
}
