//! Range markers (`EG_RangeMarkupElements`, ECMA-376 Part 1 §17.13.6): bookmarks
//! (`w:bookmarkStart`/`w:bookmarkEnd`) and comment ranges (`w:commentRangeStart`/`w:commentRangeEnd`)
//! share one shape — a flat marker dropped into the paragraph content stream (`EG_PContent`; nothing
//! wraps the content a range covers), paired with its counterpart purely by a shared `id` attribute
//! (`ST_DecimalNumber`, `CT_Markup`'s own attribute every range-marker type in this group extends).
//!
//! # Pairing is by id, never by a stack
//!
//! ECMA-376 Part 1 §17.13.6.2 (`bookmarkStart`) states the pairing rule directly: *"This start marker
//! is matched with the appropriately paired end marker by matching the value of the id attribute from
//! the associated bookmarkEnd element."* Nothing about document order or nesting depth enters into
//! it. A reader that instead pairs markers with a stack (closing whichever range opened most
//! recently, the way balanced brackets are matched) gets this wrong the moment two ranges overlap
//! rather than nest: `A start (id 1), B start (id 2), A end (id 1), B end (id 2)` is ordinary,
//! legal WordprocessingML — comment and bookmark ranges are never constrained to nest — but a stack
//! pairs the *first* end marker it sees (id 1, closing `A`) with whatever it last pushed (id 2, `B`'s
//! own start), which is simply the wrong range closing. [`RangeIndex::build`] pairs by `id` alone: one
//! linear scan collects every start and every end keyed by its own id, and a range's extent is "the
//! start with this id" paired with "the end with this id" — overlap, or any other interleaving,
//! changes nothing about how either one resolves.
//!
//! `crates/mjx-docx/tests/annotations.rs`'s `overlapping_comment_ranges_resolve_independently_of_
//! nesting` fixture is hand-built for exactly this reason: a writer that only ever emits well-nested
//! ranges cannot produce it, and a stack-based reader turns it red on it.
//!
//! # What MJXOFF-126 should call
//!
//! [`RangeIndex::build`] takes a `classify` closure (`Fn(&ParagraphContent, &Interner) ->
//! Option<(MarkerRole, i64)>`) rather than being hard-coded to bookmarks or comment ranges.
//! `moveFromRangeStart`/`moveToRangeStart` (`CT_MoveBookmark`) and the four `customXml*RangeStart`
//! (`CT_TrackChange`) members of this same `EG_RangeMarkupElements` group are this mechanism's other
//! clients — this child leaves all six of those `ParagraphContent::Raw` (their *semantics* are
//! MJXOFF-126's, not structure this child needs to guess the shape of), but once MJXOFF-126 gives them
//! their own variants, a classifier matching those variants is all `RangeIndex::build` needs: the
//! id-based pairing, the whole-document-order paragraph flattening (`flatten_paragraphs`, which
//! already recurses into every table cell), and [`covered_text`] all come for free. `classify_bookmark`
//! and `classify_comment_range` below are two working examples of exactly the closure shape to write.
//!
//! # `id` is `ST_DecimalNumber` — arbitrary precision, read as `i64`
//!
//! `ST_DecimalNumber` is `<xsd:restriction base="xsd:integer"/>` — unbounded, not `xsd:int` — and the
//! reserved separator ids genuinely go negative, so an unsigned representation would already be wrong.
//! This module reuses `mjx_ooxml_types::wordprocessingml::DecimalNumber` (`i64`) and the
//! `Number<DecimalNumber>` codec every other `ST_DecimalNumber` attribute in this crate already reads
//! through (`PermissionRangeStart::first_column`, `paragraph_properties::DecimalNumberValue`, …)
//! rather than inventing a second representation for this one ticket. An id outside `i64`'s range is
//! vanishingly unlikely in a real document, and reading one is never a panic or a silent truncation
//! either way: `Number`'s own `decode` is `str::parse::<i64>()`, so an out-of-range id simply becomes
//! `AttributeError::InvalidValue` the moment a caller reads `.id(interner)` — exactly like every other
//! malformed required numeric attribute in this crate (see `mjx_ooxml_core::attribute`'s own doc
//! comment). A *setter* can never write one in the first place: its `Input` is a plain `i64`, so an
//! unrepresentable id is simply not a value that type can hold.

use std::collections::HashMap;

use mjx_ooxml_core::{
    Enumeration, FromXml, FromXmlError, Interner, Number, RawAttribute, RawElement, RawName,
    RawNode, Text as TextCodec, ToXml,
};
use mjx_ooxml_types::wordprocessingml::{DecimalNumber, DisplacedByCustomXml};

use super::body::{
    paragraph_content_text, wml_name, BlockContent, Paragraph, ParagraphContent, RunInnerContent,
};

// =================================================================================================
// The marker types themselves: CT_Markup, CT_MarkupRange, CT_Bookmark.
// =================================================================================================

/// `CT_Markup` — the one attribute every `EG_RangeMarkupElements` member shares: a required
/// `ST_DecimalNumber` id. Used directly for `w:commentReference` (`EG_RunInnerContent`).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "id", prefix = "w", codec = Number<DecimalNumber>, accessor = id, required))]
pub struct Markup {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl Markup {
    /// Builds a new `local` marker (`"commentReference"`) with `id`.
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

impl FromXml for Markup {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for Markup {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_MarkupRange` (`CT_Markup` + an optional "displaced by custom XML" marker) — `w:bookmarkEnd`,
/// `w:commentRangeStart`, `w:commentRangeEnd`.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "id", prefix = "w", codec = Number<DecimalNumber>, accessor = id, required))]
#[xml(attribute(local = "displacedByCustomXml", prefix = "w", codec = Enumeration<DisplacedByCustomXml>, accessor = displaced_by_custom_xml))]
pub struct MarkupRange {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl MarkupRange {
    /// Builds a new `local` marker (`"bookmarkEnd"`, `"commentRangeStart"`, `"commentRangeEnd"`) with
    /// `id`.
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

impl FromXml for MarkupRange {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for MarkupRange {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_Bookmark` (`CT_BookmarkRange` — `CT_MarkupRange` + optional `colFirst`/`colLast`, the table
/// column range ECMA-376 Part 1 §17.13.6.2 describes for a bookmark confined to certain columns of a
/// table row — plus a required `name`) — `w:bookmarkStart`. `CT_BookmarkRange` itself is never the
/// type of a wire element on its own (only `CT_Bookmark`/`CT_MoveBookmark` extend it), so its two
/// attributes are flattened directly onto this struct rather than given a separate Rust type with no
/// element of its own to represent — the same flattening [`super::body::Hyperlink`] already applies
/// to its own four-deep attribute-group ancestry.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "id", prefix = "w", codec = Number<DecimalNumber>, accessor = id, required))]
#[xml(attribute(local = "displacedByCustomXml", prefix = "w", codec = Enumeration<DisplacedByCustomXml>, accessor = displaced_by_custom_xml))]
#[xml(attribute(local = "colFirst", prefix = "w", codec = Number<DecimalNumber>, accessor = first_column))]
#[xml(attribute(local = "colLast", prefix = "w", codec = Number<DecimalNumber>, accessor = last_column))]
#[xml(attribute(local = "name", prefix = "w", codec = TextCodec, accessor = raw_name, required))]
pub struct Bookmark {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl Bookmark {
    /// Builds a new `w:bookmarkStart` with `id` and `bookmark_name`.
    #[must_use]
    pub(crate) fn new(interner: &mut Interner, id: i64, bookmark_name: &str) -> Self {
        let mut value = Self {
            name: wml_name(interner, "bookmarkStart"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        value.set_id(interner, id);
        value.set_raw_name(interner, bookmark_name);
        value
    }

    /// The bookmark's own name (`w:name`), or `None` if malformed — never panics on untrusted input.
    #[must_use]
    pub fn name(&self, interner: &Interner) -> Option<String> {
        self.raw_name(interner).ok().map(|cow| cow.into_owned())
    }
}

impl FromXml for Bookmark {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for Bookmark {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// =================================================================================================
// The generic range-resolution engine.
// =================================================================================================

/// Whether a located marker opens or closes its range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MarkerRole {
    /// `w:bookmarkStart`/`w:commentRangeStart`.
    Start,
    /// `w:bookmarkEnd`/`w:commentRangeEnd`.
    End,
}

/// Where one marker sits: the 0-based index of its paragraph in **document order**, recursing into
/// every nested table cell (`flatten_paragraphs` — not [`crate::BlockPath`], whose own indices are
/// local to one container and cannot name a position that crosses a table boundary, exactly the
/// "bookmark starts inside a cell and ends outside the table" fixture this ticket's own trap
/// describes), and the index of the marker's own slot within that paragraph's own content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarkerLocation {
    pub(crate) paragraph: usize,
    pub(crate) slot: usize,
}

/// The resolution of one range id: both markers found, only a start (ECMA-376 Part 1 §17.13.6.2:
/// "the document \[is\] non-conformant" — real files have this anyway, so it is reported, not
/// rejected), or — the symmetric case the same prose does not name but a malformed file can still
/// produce — only an end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeResolution {
    /// Both the start and the end marker for this id were found.
    Resolved {
        /// The start marker's own location.
        start: MarkerLocation,
        /// The end marker's own location.
        end: MarkerLocation,
    },
    /// A start marker exists with no matching end.
    UnmatchedStart(MarkerLocation),
    /// An end marker exists with no matching start.
    UnmatchedEnd(MarkerLocation),
}

/// Every paragraph reachable from `content`, in document order, recursing into every nested table's
/// cells — a table does not break the surrounding paragraph sequence: a bookmark that starts inside a
/// cell and ends in the paragraph after the table is exactly this ticket's own fixture, and the two
/// locations it produces must still compare correctly against each other.
pub(crate) fn flatten_paragraphs(content: &[BlockContent]) -> Vec<&Paragraph> {
    let mut out = Vec::new();
    collect_paragraphs(content, &mut out);
    out
}

fn collect_paragraphs<'a>(content: &'a [BlockContent], out: &mut Vec<&'a Paragraph>) {
    for item in content {
        match item {
            BlockContent::Paragraph(paragraph) => out.push(paragraph),
            BlockContent::Table(table) => {
                for row in table.rows() {
                    for cell in row.cells() {
                        collect_paragraphs(cell.content(), out);
                    }
                }
            }
            _ => {}
        }
    }
}

/// A full index of one marker kind's start/end pairs across a whole container, built with **one**
/// linear scan over `flatten_paragraphs`'s own output. Resolving ranges one id at a time by
/// rescanning the whole tree per id would be `O(n·m)` for `m` ranges in an `n`-item document; building
/// this index once and looking a resolved id up in the resulting map is `O(n)` to build and `O(1)` per
/// lookup after that.
#[derive(Debug, Clone)]
pub struct RangeIndex {
    resolutions: HashMap<i64, RangeResolution>,
}

impl RangeIndex {
    /// Builds the index: every marker in `content` (document order, recursing into table cells) that
    /// `classify` recognizes is keyed by its own id, and paired **by that id alone** — see this
    /// module's own doc comment for why that, and not a stack, is the correct pairing rule. A marker
    /// whose id attribute is present but malformed (`classify` returning `None` for it because reading
    /// it failed) simply does not participate in resolution — it still round-trips untouched, exactly
    /// as every other malformed-but-present attribute in this crate does; only resolution, not
    /// fidelity, is affected. A duplicate start or end sharing an id that already has one keeps the
    /// first occurrence (preserve-what-is-there, matching `fields.rs`'s own precedent for the
    /// analogous over-long-value case) — this project reports it does not correct it.
    pub(crate) fn build(
        content: &[BlockContent],
        interner: &Interner,
        classify: impl Fn(&ParagraphContent, &Interner) -> Option<(MarkerRole, i64)>,
    ) -> Self {
        let paragraphs = flatten_paragraphs(content);
        let mut starts: HashMap<i64, MarkerLocation> = HashMap::new();
        let mut ends: HashMap<i64, MarkerLocation> = HashMap::new();
        for (paragraph_index, paragraph) in paragraphs.iter().enumerate() {
            for (slot, item) in paragraph.content().iter().enumerate() {
                let Some((role, id)) = classify(item, interner) else {
                    continue;
                };
                let location = MarkerLocation {
                    paragraph: paragraph_index,
                    slot,
                };
                match role {
                    MarkerRole::Start => {
                        starts.entry(id).or_insert(location);
                    }
                    MarkerRole::End => {
                        ends.entry(id).or_insert(location);
                    }
                }
            }
        }
        let mut resolutions = HashMap::with_capacity(starts.len().max(ends.len()));
        for (&id, &start) in &starts {
            let resolution = match ends.get(&id) {
                Some(&end) => RangeResolution::Resolved { start, end },
                None => RangeResolution::UnmatchedStart(start),
            };
            resolutions.insert(id, resolution);
        }
        for (&id, &end) in &ends {
            resolutions
                .entry(id)
                .or_insert(RangeResolution::UnmatchedEnd(end));
        }
        Self { resolutions }
    }

    /// This id's own resolution, or `None` if no marker of the classified kind named this id at all.
    #[must_use]
    pub fn get(&self, id: i64) -> Option<RangeResolution> {
        self.resolutions.get(&id).copied()
    }

    /// Every id with a start marker but no matching end — ECMA-376 Part 1 §17.13.6.2's own
    /// non-conformance case, occurring in real files anyway.
    pub fn unmatched_starts(&self) -> impl Iterator<Item = i64> + '_ {
        self.resolutions.iter().filter_map(|(&id, resolution)| {
            matches!(resolution, RangeResolution::UnmatchedStart(_)).then_some(id)
        })
    }

    /// Every id with an end marker but no matching start — the symmetric, unnamed-by-the-prose case.
    pub fn unmatched_ends(&self) -> impl Iterator<Item = i64> + '_ {
        self.resolutions.iter().filter_map(|(&id, resolution)| {
            matches!(resolution, RangeResolution::UnmatchedEnd(_)).then_some(id)
        })
    }

    /// The highest id any marker of the classified kind carries, or `None` if there are none —
    /// [`crate::Document::add_bookmark`]'s own "one past the highest id already in use" rule reuses
    /// this rather than rescanning the tree with its own loop.
    #[must_use]
    pub fn max_id(&self) -> Option<i64> {
        self.resolutions.keys().copied().max()
    }
}

/// The text a resolved range covers: every run reachable from strictly between `start` and `end`
/// (the markers themselves carry no text) — the slots after `start` within its own paragraph, every
/// paragraph strictly between the two (joined by `\n`, matching [`super::tables::Cell::text`]'s own
/// paragraph-join convention), and the slots before `end` within its own paragraph, when `start` and
/// `end` land in different paragraphs; just the slots strictly between them, when they share one.
#[must_use]
pub fn covered_text(
    content: &[BlockContent],
    start: MarkerLocation,
    end: MarkerLocation,
) -> String {
    let paragraphs = flatten_paragraphs(content);
    let mut text = String::new();
    let Some(start_paragraph) = paragraphs.get(start.paragraph) else {
        return text;
    };
    if start.paragraph == end.paragraph {
        let slots = start_paragraph.content();
        let lo = (start.slot + 1).min(slots.len());
        let hi = end.slot.min(slots.len());
        if lo < hi {
            paragraph_content_text(&slots[lo..hi], &mut text);
        }
        return text;
    }
    let start_slots = start_paragraph.content();
    let lo = (start.slot + 1).min(start_slots.len());
    if lo < start_slots.len() {
        paragraph_content_text(&start_slots[lo..], &mut text);
    }
    if start.paragraph + 1 < end.paragraph {
        for paragraph in &paragraphs[start.paragraph + 1..end.paragraph] {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&paragraph.text());
        }
    }
    if let Some(end_paragraph) = paragraphs.get(end.paragraph) {
        if !text.is_empty() {
            text.push('\n');
        }
        let end_slots = end_paragraph.content();
        let hi = end.slot.min(end_slots.len());
        paragraph_content_text(&end_slots[..hi], &mut text);
    }
    text
}

/// How many paragraphs `start`..=`end` spans, inclusive (`1` when both land in the same paragraph).
#[must_use]
pub fn paragraphs_spanned(start: MarkerLocation, end: MarkerLocation) -> usize {
    end.paragraph.saturating_sub(start.paragraph) + 1
}

/// Removes, in place, every `ParagraphContent` item `matches_paragraph_item` accepts and every
/// `RunInnerContent` item `matches_run_item` accepts, from every paragraph reachable from `content`
/// (recursing into every table cell, mirroring `flatten_paragraphs`'s own reach — **not** into a
/// `w:hyperlink`'s own nested content, the same documented scope [`RangeIndex`] leaves open for a
/// later child; see this module's own doc comment). Returns `(paragraph-level items removed,
/// run-level items removed)` — [`crate::Document::remove_comment`]/`remove_bookmark`/`remove_footnote`
/// all call this once each, with a predicate matched to their own marker/reference shape.
pub(crate) fn remove_matching(
    content: &mut [BlockContent],
    matches_paragraph_item: &impl Fn(&ParagraphContent) -> bool,
    matches_run_item: &impl Fn(&RunInnerContent) -> bool,
) -> (usize, usize) {
    let mut paragraph_removed = 0;
    let mut run_removed = 0;
    for item in content.iter_mut() {
        match item {
            BlockContent::Paragraph(paragraph) => {
                let items = paragraph.content_mut();
                let before = items.len();
                items.retain(|item| !matches_paragraph_item(item));
                paragraph_removed += before - items.len();
                for item in items.iter_mut() {
                    if let ParagraphContent::Run(run) = item {
                        let run_items = run.content_mut();
                        let before = run_items.len();
                        run_items.retain(|item| !matches_run_item(item));
                        run_removed += before - run_items.len();
                    }
                }
            }
            BlockContent::Table(table) => {
                for row in table.rows_mut() {
                    for cell in row.cells_mut() {
                        let (p, r) = remove_matching(
                            cell.content_mut(),
                            matches_paragraph_item,
                            matches_run_item,
                        );
                        paragraph_removed += p;
                        run_removed += r;
                    }
                }
            }
            _ => {}
        }
    }
    (paragraph_removed, run_removed)
}

/// The resolution of a bookmark by name — [`crate::Document::resolve_bookmark`]'s own return type,
/// and the seam MJXOFF-121's `Hyperlink::anchor` resolves through (see `hyperlinks.rs`'s own doc
/// comment for the whole story).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BookmarkResolution {
    /// Both `w:bookmarkStart`/`w:bookmarkEnd` were found: the bookmark's own id and the text between
    /// them.
    Resolved {
        /// The bookmark's own id (`w:bookmarkStart/@id`).
        id: i64,
        /// The text between the start and end markers.
        text: String,
    },
    /// A `w:bookmarkStart` named this exists, but no `w:bookmarkEnd` shares its id — non-conformant
    /// per ECMA-376 Part 1 §17.13.6.2, but real files have this; reported, not panicked on.
    UnmatchedStart {
        /// The bookmark's own id.
        id: i64,
    },
}

// =================================================================================================
// The two classifiers this child needs — see this module's own doc comment for MJXOFF-126's own.
// =================================================================================================

/// Classifies `item` as a `w:bookmarkStart`/`w:bookmarkEnd` marker, or `None` if it is neither (or its
/// `id` attribute failed to read).
pub(crate) fn classify_bookmark(
    item: &ParagraphContent,
    interner: &Interner,
) -> Option<(MarkerRole, i64)> {
    match item {
        ParagraphContent::BookmarkStart(bookmark) => {
            bookmark.id(interner).ok().map(|id| (MarkerRole::Start, id))
        }
        ParagraphContent::BookmarkEnd(end) => end.id(interner).ok().map(|id| (MarkerRole::End, id)),
        _ => None,
    }
}

/// Classifies `item` as a `w:commentRangeStart`/`w:commentRangeEnd` marker, or `None` if it is neither
/// (or its `id` attribute failed to read).
pub(crate) fn classify_comment_range(
    item: &ParagraphContent,
    interner: &Interner,
) -> Option<(MarkerRole, i64)> {
    match item {
        ParagraphContent::CommentRangeStart(start) => {
            start.id(interner).ok().map(|id| (MarkerRole::Start, id))
        }
        ParagraphContent::CommentRangeEnd(end) => {
            end.id(interner).ok().map(|id| (MarkerRole::End, id))
        }
        _ => None,
    }
}
