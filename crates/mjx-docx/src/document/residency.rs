//! [`DocumentFormatting`] — the whole document read **once**, so that laying it out is not
//! quadratic.
//!
//! # The measurement that made this exist
//!
//! [`Document::effective_paragraph_properties`] and [`Document::effective_run_properties`] are
//! correct and are shaped for a caller asking about *one* paragraph. Each call re-parses
//! `word/document.xml`, re-parses `word/styles.xml`, rebuilds the whole [`StyleIndex`], rebuilds a
//! chain cache it then throws away, and re-parses `word/theme/theme1.xml`. That is stated in the
//! guide's own "Cost and caching" section, and it is the right trade for a caller that asks once.
//!
//! A **flow layout engine** asks once per paragraph, for every paragraph, and a two-hundred-page
//! document holds thousands of them. The cost of laying one out would then be the number of
//! paragraphs times the size of the document — and the crate above it (`mjx-layout-docx`,
//! MJXOFF-174) would have had no way out except to re-derive the ladder itself, which is the one
//! thing it must not do.
//!
//! So this is the same answer `mjx-xlsx` already gives for a worksheet (`SheetFormatting`: the
//! styles part and one sheet, parsed once, resolved many times): **read once, resolve many**. Every
//! part is parsed once, the style index and the chain cache are built once, each distinct
//! `(w:numId, w:ilvl)` pair is resolved once however many paragraphs share it, and the theme is
//! loaded once.
//!
//! # This adds no resolution of its own
//!
//! The ladder's *order* is stated in exactly one place — `combine_paragraph_tiers` and
//! `combine_run_tiers` in `effective.rs` — and both orchestrations call it. The extraction of a
//! single rung is `extract_paragraph_properties` / `extract_run_properties`, which every rung
//! already shared before this file existed. `crates/mjx-docx/tests/residency.rs` asserts the two
//! orchestrations agree paragraph by paragraph and run by run over the committed corpus, which is
//! what makes "one implementation" a checked claim rather than a comment.
//!
//! # What the text is, and why it is not [`Paragraph::text`]
//!
//! [`Paragraph::text`] concatenates `w:t` and nothing else, which is right for a caller reading what
//! a paragraph *says*. A renderer needs what a paragraph *is*: a tab occupies horizontal space, a
//! `w:br` ends a line, a soft hyphen is an authored hyphenation point and a non-breaking hyphen is a
//! hyphen that may not be broken at. So [`ParagraphFormatting::text`] carries
//!
//! | markup | character |
//! |---|---|
//! | `w:t` | its own text |
//! | `w:tab` | `U+0009 TAB` |
//! | `w:br` (any `@type`) | `U+000A LINE FEED` |
//! | `w:cr` | `U+000A LINE FEED` |
//! | `w:softHyphen` | `U+00AD SOFT HYPHEN` |
//! | `w:noBreakHyphen` | `U+2011 NON-BREAKING HYPHEN` |
//!
//! and nothing else. The two hyphens are not an interpretation: UAX #14 gives `U+00AD` class `BA`
//! (break after) and `U+2011` class `GL` (non-breaking), which is exactly what the two elements
//! mean, so the line breaker gets them right without anything here teaching it.
//!
//! A `w:br@type="page"` or `="column"` is *also* recorded as a [`HardBreak`], because ending a line
//! and ending a page are different facts and the line feed carries only the first.
//!
//! **`w:sym` contributes nothing**, and that is reported rather than guessed at. A symbol is a
//! character code in a *named font* — the pair is the content — so a renderer that dropped the font
//! would draw a different glyph while looking entirely plausible.

use std::collections::HashMap;
use std::ops::Range;

use mjx_ooxml_core::{FromXml, Interner};
use mjx_ooxml_types::wordprocessingml::{
    BreakType, EndnotePosition, FootnoteEndnoteType, FootnotePosition, HeaderFooterType,
    LineNumberRestart, NumberFormat, NumberingRestartLocation, SectionBreakType,
    VerticalJustification,
};

use super::annotations::{Endnotes, Footnotes};
use super::body::{Paragraph, ParagraphContent, Run, RunInnerContent};
use super::effective::{
    attr, combine_paragraph_tiers, combine_run_tiers, extract_numbering_reference,
    extract_paragraph_properties, extract_run_properties, extract_style_paragraph_properties,
    merge_character_chain, merge_paragraph_chain, numbering_reference_from_chain, ChainCache,
    EffectiveCharacterProperties, EffectiveNumberingReference, EffectiveParagraphProperties,
    ThemeContext,
};
use super::headers::HdrFtr;
use super::numbering::NumberingLookup;
use super::paragraph_properties::ParagraphProperties;
use super::run_properties::RunProperties;
use super::sections::{SectionProperties, SectionSpan};
use super::styles::{
    DefaultParagraphProperties, DefaultRunProperties, DocumentDefaults, StyleSheet,
};
use super::{Document, MainDocument, StyleIndex};
use crate::address::RunPath;
use crate::error::DocxError;
use crate::page::{PageMargins, PageSize};

/// `U+00AD SOFT HYPHEN` — what `w:softHyphen` is, in one character.
pub const SOFT_HYPHEN: char = '\u{00AD}';

/// `U+2011 NON-BREAKING HYPHEN` — what `w:noBreakHyphen` is, in one character.
pub const NON_BREAKING_HYPHEN: char = '\u{2011}';

/// One run of a paragraph, with its formatting already resolved.
#[derive(Debug, Clone, PartialEq)]
pub struct RunFormatting {
    /// The bytes of [`ParagraphFormatting::text`] it covers.
    pub range: Range<usize>,
    /// Where it is in the paragraph, when [`Paragraph::run`] can address it.
    ///
    /// `None` for a run inside a `w:fldSimple`: [`Paragraph::text`] descends into one and
    /// [`Paragraph::run_count`]'s own slot walk does not, so such a run has text but no address.
    /// Reported rather than papered over — a path invented here would resolve to a different run.
    pub path: Option<RunPath>,
    /// Every `EG_RPrBase` member, resolved across the whole ladder.
    pub properties: EffectiveCharacterProperties,
}

/// A `w:br` that ends more than a line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HardBreak {
    /// The byte of [`ParagraphFormatting::text`] the break's own line feed sits at.
    pub at: usize,
    /// `w:br@type` — [`BreakType::Page`] or [`BreakType::Column`]. A `textWrapping` break is only a
    /// line feed and is not recorded here.
    pub kind: BreakType,
}

/// A `w:footnoteReference` or `w:endnoteReference` in a paragraph's run stream.
///
/// # The mark itself contributes no character, and that is reported rather than guessed at
///
/// A footnote's *reference mark* is a generated number — Word draws it from the note's own
/// numbering scheme, not from anything the run stream holds — so there is no character in
/// [`ParagraphFormatting::text`] for it and its own advance is therefore not measured when a line is
/// broken. That is the same decision `w:sym` gets in this module (a symbol is a code point in a
/// *named font*, and dropping the font would draw a different glyph while looking plausible) and the
/// same one MJXOFF-174 made for a list's number: **a generated mark is drawn by whatever renders
/// fields, not by the residency**. What travels here is *where* the mark is, so a layout engine can
/// say which line — and therefore which page — a note is referenced from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoteReference {
    /// The byte of [`ParagraphFormatting::text`] the mark sits at.
    pub at: usize,
    /// `w:id` — which [`NoteFormatting`] in the matching part it names.
    pub id: i64,
    /// Whether it is an endnote reference rather than a footnote one.
    pub endnote: bool,
}

/// One paragraph, with everything a box model needs and nothing it would have to re-derive.
#[derive(Debug, Clone, PartialEq)]
pub struct ParagraphFormatting {
    text: String,
    runs: Vec<RunFormatting>,
    hard_breaks: Vec<HardBreak>,
    note_references: Vec<NoteReference>,
    properties: EffectiveParagraphProperties,
    style_id: Option<String>,
}

impl ParagraphFormatting {
    /// What the paragraph reads as. See this module's own documentation for the six markup elements
    /// that contribute a character.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Its runs, in document order, covering [`ParagraphFormatting::text`] without gaps or overlaps.
    #[must_use]
    pub fn runs(&self) -> &[RunFormatting] {
        &self.runs
    }

    /// Every page or column break inside it, in order.
    #[must_use]
    pub fn hard_breaks(&self) -> &[HardBreak] {
        &self.hard_breaks
    }

    /// Every footnote and endnote reference inside it, in order.
    #[must_use]
    pub fn note_references(&self) -> &[NoteReference] {
        &self.note_references
    }

    /// Every `CT_PPrBase` member, resolved across the whole ladder.
    #[must_use]
    pub fn properties(&self) -> &EffectiveParagraphProperties {
        &self.properties
    }

    /// The `w:pStyle` it names, if any — the **id**, not the resolved values.
    ///
    /// The ladder resolves a style into values and then has no further use for its name, so this is
    /// the one thing about a style that survives resolution. `w:contextualSpacing` is why it has
    /// to: *"don't add space between paragraphs of the same style"* is a question about two
    /// paragraphs' **identity**, which no amount of resolved values can answer — two paragraphs of
    /// different styles that happen to resolve alike are not the same style.
    #[must_use]
    pub fn style_id(&self) -> Option<&str> {
        self.style_id.as_deref()
    }
}

/// One column of a section, in twips.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColumnFormatting {
    /// `w:col@w` — how wide it is.
    pub width_twips: i64,
    /// `w:col@space` — the gap to the **next** column. Meaningless on the last one.
    pub space_after_twips: i64,
}

/// `w:cols`, resolved to a list of columns.
///
/// # `w:equalWidth` wins, and the resolution is done here rather than left to a caller
///
/// `sections.rs`'s own doc comment quotes ECMA-376 Part 1 §17.6.4: when `w:equalWidth` is true the
/// columns come from `w:num`/`w:space` and an explicit `w:col` list is *ignored*, even when the file
/// states both. That crate-level type deliberately exposes the two independently, because it has no
/// page-margin knowledge to compute a width from `w:num` and hiding the file's own contradiction
/// from a caller who might want to see it would be wrong. **A layout engine is not that caller** —
/// it needs one answer — so the precedence is applied once, here, and the widths that come out are
/// the widths content flows through.
///
/// An equal-width section reports [`SectionColumns::count`] and [`SectionColumns::space_twips`] with
/// an **empty** [`SectionColumns::columns`], because the width of an equal column is the text area
/// less the gaps divided by the count, and the text area is not known until the page size and
/// margins are chosen. An explicit list reports its own widths and needs no such arithmetic.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SectionColumns {
    /// How many columns there are, from `w:num` or from the explicit list's length. Never zero.
    pub count: usize,
    /// `w:cols@space` — the gap between two equal columns, in twips.
    pub space_twips: i64,
    /// `w:cols@sep` — whether a vertical rule is drawn between them.
    pub separator: bool,
    /// The explicit `w:col` list, empty when the columns are equal-width.
    pub columns: Vec<ColumnFormatting>,
}

impl SectionColumns {
    /// `w:cols@space`'s schema default, in twips — half an inch.
    pub const DEFAULT_SPACE_TWIPS: i64 = 720;

    /// One column, no gap: what a section that states no `w:cols` at all has.
    #[must_use]
    pub fn single() -> Self {
        Self {
            count: 1,
            space_twips: Self::DEFAULT_SPACE_TWIPS,
            separator: false,
            columns: Vec::new(),
        }
    }
}

/// `w:pgNumType`, resolved: what this section's page numbers look like and where they restart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SectionPageNumbering {
    /// `w:fmt` — the numeral system. `Decimal` is the schema default.
    pub format: NumberFormat,
    /// `w:start` — the number this section's **first** page carries. `None` continues the previous
    /// section's count, which is what an absent attribute means.
    pub start: Option<i64>,
}

impl Default for SectionPageNumbering {
    fn default() -> Self {
        Self {
            format: NumberFormat::Decimal,
            start: None,
        }
    }
}

/// `w:lnNumType`, resolved: this section's line numbering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SectionLineNumbering {
    /// `w:countBy` — number every *n*th line. `1` when absent, which numbers every line.
    pub count_by: i64,
    /// `w:start` — the first number. The schema default is `1`.
    pub start: i64,
    /// `w:distance` — how far from the text the numbers sit, in twips. `None` when unstated.
    pub distance_twips: Option<i64>,
    /// `w:restart` — where the count starts over. The schema default is `newPage`.
    pub restart: LineNumberRestart,
}

/// `w:footnotePr`/`w:endnotePr`, resolved: where a section's notes go and how they are numbered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SectionNoteRules {
    /// `w:footnotePr/w:numFmt` — the numeral system a footnote mark is drawn in.
    pub footnote_format: NumberFormat,
    /// `w:footnotePr/w:numStart` — the first number, `1` when unstated.
    pub footnote_start: i64,
    /// `w:footnotePr/w:numRestart` — where footnote numbering starts over. `Continuous` when
    /// unstated.
    pub footnote_restart: NumberingRestartLocation,
    /// `w:footnotePr/w:pos` — the bottom of the page, or directly beneath the text. `None` when
    /// unstated.
    pub footnote_position: Option<FootnotePosition>,
    /// `w:endnotePr/w:numFmt`.
    pub endnote_format: NumberFormat,
    /// `w:endnotePr/w:numStart`.
    pub endnote_start: i64,
    /// `w:endnotePr/w:numRestart`.
    pub endnote_restart: NumberingRestartLocation,
    /// `w:endnotePr/w:pos` — the end of the section, or the end of the document. `None` when
    /// unstated.
    pub endnote_position: Option<EndnotePosition>,
}

impl Default for SectionNoteRules {
    fn default() -> Self {
        Self {
            footnote_format: NumberFormat::Decimal,
            footnote_start: 1,
            footnote_restart: NumberingRestartLocation::Continuous,
            footnote_position: None,
            endnote_format: NumberFormat::LowercaseRomanNumerals,
            endnote_start: 1,
            endnote_restart: NumberingRestartLocation::Continuous,
            endnote_position: None,
        }
    }
}

/// Which header or footer part a section shows for each of the three page kinds, **already
/// resolved**.
///
/// # This is `resolve_reference`'s answer and not a copy of the `w:sectPr`'s own list
///
/// A section's `w:headerReference` list is not what a page shows. `w:titlePg` and
/// `w:evenAndOddHeaders` can each *downgrade* a query to the default variant, and a variant a
/// section does not state is **inherited from the nearest preceding section that does** — all three
/// rules quoted from ECMA-376 Part 1 §17.10.1/.2/.5/.6 in `crate::document::headers`'s own doc
/// comment, and implemented there once. These slots are that implementation's output, so a consumer
/// of a [`DocumentFormatting`] resolves nothing: `first` on a section with `w:titlePg` off is
/// literally the same index as `default`.
///
/// Each is an index into [`DocumentFormatting::header_footer_streams`], or `None` where no section
/// back to the document's first states one — which is where Word would create a blank header and
/// this crate does not fabricate one on a read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HeaderFooterSlots {
    /// What the first page of the section shows.
    pub first: Option<usize>,
    /// What an even page shows.
    pub even: Option<usize>,
    /// What every other page shows.
    pub default: Option<usize>,
}

impl HeaderFooterSlots {
    /// The stream this page kind shows, given whether the page is the section's first and whether
    /// it is an even one.
    ///
    /// The two flags have **already been applied** to which slot holds what — a section with
    /// `w:titlePg` off has the same index in `first` as in `default` — so this is a selection and
    /// not a resolution, which is the whole point of resolving the variants once in `mjx-docx`.
    #[must_use]
    pub fn for_page(self, is_first_of_section: bool, is_even: bool) -> Option<usize> {
        if is_first_of_section {
            self.first
        } else if is_even {
            self.even
        } else {
            self.default
        }
    }

    /// The same slots, with each relationship index replaced by the stream index `table` gives it.
    fn remapped(self, table: &[Option<usize>]) -> Self {
        let map = |slot: Option<usize>| slot.and_then(|at| table.get(at).copied().flatten());
        Self {
            first: map(self.first),
            even: map(self.even),
            default: map(self.default),
        }
    }
}

/// One section's page geometry, in plain numbers.
///
/// The `w:sectPr` itself is [`SectionProperties`] and needs an [`Interner`] to read; this is what it
/// resolves to, so a caller holding a [`DocumentFormatting`] does not have to hold an interner
/// beside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionFormatting {
    /// The first paragraph index this section governs.
    pub first_paragraph: usize,
    /// The last, inclusive, or `None` when it governs no paragraph.
    pub last_paragraph: Option<usize>,
    /// `w:pgSz`, or `None` when the section states none.
    pub page_size: Option<PageSize>,
    /// `w:pgMar`, or `None` when the section states none.
    pub page_margins: Option<PageMargins>,
    /// `w:type` — which kind of break **starts** this section.
    ///
    /// `None` when the section states none. ECMA-376 gives `w:type/@val` no schema default and
    /// `sections.rs` therefore refuses to assert one; Word's own behaviour for an absent `w:type` is
    /// `nextPage`, and reading that convention is a layout decision rather than a parsing one, so it
    /// is made above this crate and not here.
    pub break_kind: Option<SectionBreakType>,
    /// `w:cols`, with §17.6.4's precedence already applied.
    pub columns: SectionColumns,
    /// `w:titlePg` — whether the first page of this section has its own header and footer.
    pub title_page: bool,
    /// `w:pgNumType`.
    pub page_numbering: SectionPageNumbering,
    /// `w:lnNumType`, or `None` when this section numbers no lines.
    pub line_numbering: Option<SectionLineNumbering>,
    /// `w:vAlign` — how the section's text sits vertically on a page it does not fill.
    pub vertical_alignment: Option<VerticalJustification>,
    /// `w:footnotePr` and `w:endnotePr`, merged into one set of rules.
    pub notes: SectionNoteRules,
    /// `w:noEndnote` — whether this section suppresses endnotes at its own end.
    pub suppress_endnotes: bool,
    /// Which header part each page kind shows, resolved.
    pub headers: HeaderFooterSlots,
    /// The same for footers.
    pub footers: HeaderFooterSlots,
}

/// The document settings a layout engine reads: hyphenation, and the default tab interval.
///
/// Every field is the value **in force**, with the schema default already applied, so a consumer
/// never has to know which of them default to on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentLayoutSettings {
    /// `w:autoHyphenation` — off unless the document says otherwise.
    pub auto_hyphenation: bool,
    /// `w:doNotHyphenateCaps`.
    pub do_not_hyphenate_capitals: bool,
    /// `w:consecutiveHyphenLimit` — how many consecutive lines may end in a hyphen. `None` is "no
    /// limit", which is what an absent element means.
    pub consecutive_hyphen_limit: Option<i64>,
    /// `w:hyphenationZone` in twips — how close to the right margin a line must come before
    /// hyphenation is attempted at all. `None` when the document states none.
    pub hyphenation_zone_twips: Option<i64>,
    /// `w:defaultTabStop` in twips — the interval of the implicit tab grid past the last stated
    /// stop.
    pub default_tab_stop_twips: i64,
    /// `w:evenAndOddHeaders` (§17.10.1) — whether an even page shows its own header and footer.
    ///
    /// Document-wide rather than per section, which is why it lives here and not on
    /// [`SectionFormatting`]. It has **already been applied** to every section's
    /// [`HeaderFooterSlots`]; it travels for a consumer that wants to say *why* an even page shows
    /// the odd header.
    pub even_and_odd_headers: bool,
    /// `w:mirrorMargins` (§17.15.1.71) — whether the left and right margins swap on an even page,
    /// so that the wider one is always on the binding side.
    pub mirror_margins: bool,
}

impl DocumentLayoutSettings {
    /// `w:defaultTabStop`'s value when the document states none.
    ///
    /// Half an inch. ECMA-376 gives `w:defaultTabStop` **no** schema default, and this is the value
    /// Word's own `Normal.dotm` writes into every document it creates — a fact about Word rather
    /// than about the specification, which is why it is named here rather than buried in an
    /// `unwrap_or`.
    pub const DEFAULT_TAB_STOP_TWIPS: i64 = 720;
}

impl Default for DocumentLayoutSettings {
    fn default() -> Self {
        Self {
            auto_hyphenation: false,
            do_not_hyphenate_capitals: false,
            consecutive_hyphen_limit: None,
            hyphenation_zone_twips: None,
            default_tab_stop_twips: Self::DEFAULT_TAB_STOP_TWIPS,
            even_and_odd_headers: false,
            mirror_margins: false,
        }
    }
}

/// One header or footer part, read and resolved.
///
/// A header is a **content stream of its own**: block-level content that lays out in its own band at
/// the top of a page, in the same paragraphs and runs the body is made of, resolved against the same
/// style ladder. It is not a property of a section — several sections share one — which is why these
/// are held once on the [`DocumentFormatting`] and referenced by index from
/// [`SectionFormatting::headers`]/`footers`.
#[derive(Debug, Clone, PartialEq)]
pub struct HeaderFooterFormatting {
    part: mjx_opc::PartName,
    paragraphs: Vec<ParagraphFormatting>,
}

impl HeaderFooterFormatting {
    /// Which part it was read from.
    #[must_use]
    pub fn part(&self) -> &mjx_opc::PartName {
        &self.part
    }

    /// Its paragraphs, in document order, resolved exactly as a body paragraph is.
    #[must_use]
    pub fn paragraphs(&self) -> &[ParagraphFormatting] {
        &self.paragraphs
    }
}

/// One footnote or endnote, read and resolved.
#[derive(Debug, Clone, PartialEq)]
pub struct NoteFormatting {
    id: i64,
    kind: FootnoteEndnoteType,
    paragraphs: Vec<ParagraphFormatting>,
}

impl NoteFormatting {
    /// `w:id` — what a [`NoteReference`] names.
    #[must_use]
    pub fn id(&self) -> i64 {
        self.id
    }

    /// `w:type`, with the schema's own default applied: an entry that states none is
    /// [`FootnoteEndnoteType::Normal`], which is a user's own note.
    ///
    /// The other three are Word's furniture: the rule drawn above a page's notes
    /// ([`FootnoteEndnoteType::Separator`]), the one drawn above a note carried over from the page
    /// before ([`FootnoteEndnoteType::ContinuationSeparator`]) and the notice that says a note
    /// continues ([`FootnoteEndnoteType::ContinuationNotice`]).
    #[must_use]
    pub fn kind(&self) -> FootnoteEndnoteType {
        self.kind
    }

    /// Whether it is one of the document's own notes rather than one of Word's separators.
    #[must_use]
    pub fn is_user_visible(&self) -> bool {
        self.kind == FootnoteEndnoteType::Normal
    }

    /// Its paragraphs, in document order, resolved exactly as a body paragraph is.
    #[must_use]
    pub fn paragraphs(&self) -> &[ParagraphFormatting] {
        &self.paragraphs
    }
}

/// A whole document, parsed once and resolved once: what a box model lays out.
///
/// Owns nothing borrowed from the [`Document`] it came from, so a caller may keep one while the
/// document is edited elsewhere — and it can never write, so keeping one cannot make the package
/// stale in the other direction. It is a **snapshot**: an edit to the document does not reach it,
/// and the caller re-reads.
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentFormatting {
    paragraphs: Vec<ParagraphFormatting>,
    sections: Vec<SectionFormatting>,
    settings: DocumentLayoutSettings,
    header_footer_streams: Vec<HeaderFooterFormatting>,
    footnotes: Vec<NoteFormatting>,
    endnotes: Vec<NoteFormatting>,
}

impl DocumentFormatting {
    /// Every paragraph of the body, in document order.
    ///
    /// Paragraphs inside a table are **not** here: a table is a different layout discipline and its
    /// own child (MJXOFF-176, R21). [`super::body::Body::paragraphs`] is what this walks, and that
    /// walks the body's top level.
    #[must_use]
    pub fn paragraphs(&self) -> &[ParagraphFormatting] {
        &self.paragraphs
    }

    /// Every section, in document order.
    #[must_use]
    pub fn sections(&self) -> &[SectionFormatting] {
        &self.sections
    }

    /// The settings in force.
    #[must_use]
    pub fn settings(&self) -> &DocumentLayoutSettings {
        &self.settings
    }

    /// Which section governs the paragraph at `paragraph`, or `None` when none does.
    #[must_use]
    pub fn section_of(&self, paragraph: usize) -> Option<&SectionFormatting> {
        self.sections.iter().find(|section| {
            section
                .last_paragraph
                .is_some_and(|last| paragraph >= section.first_paragraph && paragraph <= last)
        })
    }

    /// Every header and footer part this document's sections reach, read once each.
    ///
    /// One list for both, because a slot on [`SectionFormatting::headers`] and one on
    /// [`SectionFormatting::footers`] are indices into the same table — and because two sections
    /// that reference the same part share one entry, which is what makes an inherited header cost
    /// nothing.
    #[must_use]
    pub fn header_footer_streams(&self) -> &[HeaderFooterFormatting] {
        &self.header_footer_streams
    }

    /// The stream at `index`, for a [`HeaderFooterSlots`] member.
    #[must_use]
    pub fn header_footer_stream(&self, index: usize) -> Option<&HeaderFooterFormatting> {
        self.header_footer_streams.get(index)
    }

    /// `word/footnotes.xml`'s entries, in document order — the reserved separators included, because
    /// a renderer draws them.
    #[must_use]
    pub fn footnotes(&self) -> &[NoteFormatting] {
        &self.footnotes
    }

    /// `word/endnotes.xml`'s entries, likewise.
    #[must_use]
    pub fn endnotes(&self) -> &[NoteFormatting] {
        &self.endnotes
    }

    /// The footnote `id` names, or `None` when the part does not define one.
    #[must_use]
    pub fn footnote(&self, id: i64) -> Option<&NoteFormatting> {
        self.footnotes.iter().find(|note| note.id == id)
    }

    /// The endnote `id` names.
    #[must_use]
    pub fn endnote(&self, id: i64) -> Option<&NoteFormatting> {
        self.endnotes.iter().find(|note| note.id == id)
    }

    /// The first entry of `kind` in `word/footnotes.xml` — how a renderer finds the separator to
    /// draw above a page's notes.
    #[must_use]
    pub fn footnote_of_kind(&self, kind: FootnoteEndnoteType) -> Option<&NoteFormatting> {
        self.footnotes.iter().find(|note| note.kind == kind)
    }

    /// The same in `word/endnotes.xml`.
    #[must_use]
    pub fn endnote_of_kind(&self, kind: FootnoteEndnoteType) -> Option<&NoteFormatting> {
        self.endnotes.iter().find(|note| note.kind == kind)
    }
}

/// What `word/document.xml` alone states about one paragraph, before any style is consulted.
struct DirectParagraph {
    text: String,
    runs: Vec<DirectRun>,
    hard_breaks: Vec<HardBreak>,
    note_references: Vec<NoteReference>,
    direct: EffectiveParagraphProperties,
    style_id: Option<String>,
    own_numbering: Option<EffectiveNumberingReference>,
}

/// Where one stream's paragraphs sit in the one flat list `Document::formatting` resolves.
#[derive(Clone, Copy)]
struct StreamSpan {
    start: usize,
    length: usize,
}

impl StreamSpan {
    /// This stream's paragraphs, cloned out of the resolved list.
    fn slice(self, all: &[ParagraphFormatting]) -> Vec<ParagraphFormatting> {
        all.get(self.start..self.start + self.length)
            .unwrap_or_default()
            .to_vec()
    }
}

/// What one parse of `word/document.xml` yields.
struct DirectRead {
    paragraphs: Vec<DirectParagraph>,
    sections: Vec<SectionFormatting>,
    /// Every `r:id` a section's resolved header/footer slots name, in first-seen order; the slots
    /// hold indices into this until `Document::formatting` turns each into a stream index.
    header_footer_relationships: Vec<String>,
}

/// One note's identity and where its paragraphs sit in the flat list.
struct NoteStream {
    id: i64,
    kind: FootnoteEndnoteType,
    span: StreamSpan,
}

/// Every note of one part.
#[derive(Default)]
struct NoteStreams {
    entries: Vec<NoteStream>,
}

impl NoteStreams {
    fn resolved(self, all: &[ParagraphFormatting]) -> Vec<NoteFormatting> {
        self.entries
            .into_iter()
            .map(|note| NoteFormatting {
                id: note.id,
                kind: note.kind,
                paragraphs: note.span.slice(all),
            })
            .collect()
    }
}

/// The same for one run.
struct DirectRun {
    range: Range<usize>,
    path: Option<RunPath>,
    direct: EffectiveCharacterProperties,
    character_style_id: Option<String>,
}

impl Document {
    /// Reads the whole document once and resolves every paragraph's and run's effective formatting.
    ///
    /// This is the reader a **layout engine** uses; [`Document::effective_paragraph_properties`] is
    /// the reader a caller asking one question uses. They compute the same thing (see this module's
    /// own documentation, and `tests/residency.rs`, which asserts it); they differ only in what
    /// they re-parse.
    ///
    /// Reading does not dirty any part.
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body,
    /// [`DocxError::BasedOnChainTooDeep`] if a style chain does not terminate, or another
    /// [`DocxError`] if a related part cannot be read.
    pub fn formatting(&mut self) -> Result<DocumentFormatting, DocxError> {
        let theme = self.load_theme_context()?;
        let settings = self.read_layout_settings()?;
        let DirectRead {
            paragraphs: body,
            mut sections,
            header_footer_relationships,
        } = self.read_direct(&theme, settings.even_and_odd_headers)?;

        // Every header and footer part the sections reach, once each: two sections that reference
        // the same part share one stream, which is what makes an *inherited* header cost nothing. A
        // reference whose relationship does not resolve loses its slot rather than refusing to lay
        // the document out — the same leniency a `w:numPr` naming an undefined list already gets.
        let mut header_footer_parts: Vec<mjx_opc::PartName> = Vec::new();
        let mut slot_of_relationship: Vec<Option<usize>> =
            Vec::with_capacity(header_footer_relationships.len());
        for relationship in &header_footer_relationships {
            match self.part_for_document_rel(relationship) {
                Ok(part) => {
                    let at = match header_footer_parts.iter().position(|held| held == &part) {
                        Some(at) => at,
                        None => {
                            header_footer_parts.push(part);
                            header_footer_parts.len() - 1
                        }
                    };
                    slot_of_relationship.push(Some(at));
                }
                Err(_) => slot_of_relationship.push(None),
            }
        }
        for section in &mut sections {
            section.headers = section.headers.remapped(&slot_of_relationship);
            section.footers = section.footers.remapped(&slot_of_relationship);
        }

        // **One flat list of every paragraph in the document, whichever stream it came from.** The
        // style index, the chain cache, the `w:docDefaults` tier and every numbering resolution
        // below are therefore built once for all of them. Resolving a header through a second pass
        // would be a second orchestration of the ladder, which is the one thing this module exists
        // to prevent — and a header's paragraphs are `w:p`s of exactly the same shape as the body's.
        let mut direct_paragraphs = body;
        let body_count = direct_paragraphs.len();
        let mut stream_spans: Vec<StreamSpan> = Vec::with_capacity(header_footer_parts.len());
        for part in &header_footer_parts {
            let stream = self.read_header_footer_direct(part, &theme)?;
            stream_spans.push(StreamSpan {
                start: direct_paragraphs.len(),
                length: stream.len(),
            });
            direct_paragraphs.extend(stream);
        }
        let footnotes = self.read_notes_direct(&theme, true, &mut direct_paragraphs)?;
        let endnotes = self.read_notes_direct(&theme, false, &mut direct_paragraphs)?;

        let resolved = self.style_sheet(|sheet, interner| -> Result<_, DocxError> {
            let index = StyleIndex::build(sheet, interner)?;
            let cache = ChainCache::new(&index, interner);
            let defaults = LadderDefaults::read(sheet, &theme, interner)?;
            let mut tiers = Vec::with_capacity(direct_paragraphs.len());
            for paragraph in &direct_paragraphs {
                tiers.push(ParagraphTiers::read(paragraph, &cache, &theme, interner)?);
            }
            Ok((defaults, tiers))
        })?;
        let (defaults, tiers) = match resolved {
            Some(result) => result?,
            None => (
                LadderDefaults::default(),
                direct_paragraphs
                    .iter()
                    .map(|paragraph| ParagraphTiers::empty(paragraph.runs.len()))
                    .collect(),
            ),
        };

        // Every distinct list the document reaches, resolved once each rather than once per
        // paragraph: a hundred bullets in one list is one resolution, not a hundred.
        let mut numbering_effects: HashMap<(i64, i64), NumberingTier> = HashMap::new();
        for (paragraph, tier) in direct_paragraphs.iter().zip(&tiers) {
            let Some(reference) = paragraph.own_numbering.or(tier.style_numbering) else {
                continue;
            };
            let key = (reference.numbering_id, reference.level);
            if numbering_effects.contains_key(&key) {
                continue;
            }
            let resolved = self.numbering_tier(&theme, reference)?;
            numbering_effects.insert(key, resolved);
        }

        let mut paragraphs = Vec::with_capacity(direct_paragraphs.len());
        for (paragraph, tier) in direct_paragraphs.into_iter().zip(tiers) {
            let numbering = paragraph
                .own_numbering
                .or(tier.style_numbering)
                .and_then(|reference| {
                    numbering_effects.get(&(reference.numbering_id, reference.level))
                })
                .cloned()
                .unwrap_or_default();
            let properties = combine_paragraph_tiers(
                &paragraph.direct,
                &tier.paragraph,
                &numbering.paragraph,
                &defaults.paragraph,
            );
            let runs = paragraph
                .runs
                .into_iter()
                .zip(tier.characters)
                .map(|(run, character_tier)| RunFormatting {
                    range: run.range,
                    path: run.path,
                    properties: combine_run_tiers(
                        &run.direct,
                        &character_tier,
                        &tier.character_from_paragraph_style,
                        &numbering.character,
                        &defaults.character,
                        // No table style applies outside a table cell; a table's own paragraphs are
                        // R21's, and this reader does not descend into one.
                        &EffectiveCharacterProperties::default(),
                    ),
                })
                .collect();
            paragraphs.push(ParagraphFormatting {
                text: paragraph.text,
                runs,
                hard_breaks: paragraph.hard_breaks,
                note_references: paragraph.note_references,
                properties,
                style_id: paragraph.style_id,
            });
        }

        // The flat list, cut back into the streams it came from. Cutting from the end forward would
        // need every length again; cutting by recorded span needs none of them.
        let header_footer_streams = header_footer_parts
            .into_iter()
            .zip(&stream_spans)
            .map(|(part, span)| HeaderFooterFormatting {
                part,
                paragraphs: span.slice(&paragraphs),
            })
            .collect();
        let footnotes = footnotes.resolved(&paragraphs);
        let endnotes = endnotes.resolved(&paragraphs);
        paragraphs.truncate(body_count);

        Ok(DocumentFormatting {
            paragraphs,
            sections,
            settings,
            header_footer_streams,
            footnotes,
            endnotes,
        })
    }

    /// One parse of `word/document.xml`: every paragraph's text, runs, direct formatting and style
    /// references, and every section resolved to plain numbers — **including which header and footer
    /// each of its three page kinds actually shows**, which is `headers::resolve_reference`'s answer
    /// and not a copy of the `w:sectPr`'s own reference list.
    fn read_direct(
        &mut self,
        theme: &ThemeContext,
        even_and_odd_headers: bool,
    ) -> Result<DirectRead, DocxError> {
        let doc = self.package.part_tree(&self.document_part)?;
        let main = MainDocument::from_xml(&doc.root, &doc.interner)?;
        let body = main.body().ok_or(DocxError::NoBody)?;
        let interner = &doc.interner;

        let mut paragraphs = Vec::new();
        for paragraph in body.paragraphs() {
            paragraphs.push(read_direct_paragraph(paragraph, theme, interner)?);
        }

        let spans = super::sections::sections_in(body);
        let mut relationships: Vec<String> = Vec::new();
        let mut sections = Vec::with_capacity(spans.len());
        for (index, span) in spans.iter().enumerate() {
            let mut section = read_section(span, interner)?;
            section.headers = resolve_slots(
                &spans,
                index,
                even_and_odd_headers,
                interner,
                true,
                &mut relationships,
            )?;
            section.footers = resolve_slots(
                &spans,
                index,
                even_and_odd_headers,
                interner,
                false,
                &mut relationships,
            )?;
            sections.push(section);
        }
        Ok(DirectRead {
            paragraphs,
            sections,
            header_footer_relationships: relationships,
        })
    }

    /// One header or footer part's paragraphs, read but not yet resolved.
    ///
    /// A part that will not parse contributes **no** paragraphs rather than refusing the whole
    /// document: a header is furniture, and a document whose header part is damaged is still a
    /// document a reader should be able to open. The same reasoning `numbering_tier` states for a
    /// `w:numPr` naming a list that is not defined.
    fn read_header_footer_direct(
        &mut self,
        part: &mjx_opc::PartName,
        theme: &ThemeContext,
    ) -> Result<Vec<DirectParagraph>, DocxError> {
        let Ok(doc) = self.package.part_tree(part) else {
            return Ok(Vec::new());
        };
        let Ok(content) = HdrFtr::from_xml(&doc.root, &doc.interner) else {
            return Ok(Vec::new());
        };
        let mut paragraphs = Vec::new();
        for paragraph in content.paragraphs() {
            paragraphs.push(read_direct_paragraph(paragraph, theme, &doc.interner)?);
        }
        Ok(paragraphs)
    }

    /// `word/footnotes.xml` (or `word/endnotes.xml`), read: one entry per note, its paragraphs
    /// appended to `into` and their positions recorded.
    fn read_notes_direct(
        &mut self,
        theme: &ThemeContext,
        footnotes: bool,
        into: &mut Vec<DirectParagraph>,
    ) -> Result<NoteStreams, DocxError> {
        let part = if footnotes {
            self.parts.footnotes.clone()
        } else {
            self.parts.endnotes.clone()
        };
        let Some(part) = part else {
            return Ok(NoteStreams::default());
        };
        let doc = self.package.part_tree(&part)?;
        let interner = &doc.interner;
        // Both parts have the identical `CT_Footnotes`/`CT_Endnotes` shape, and the discriminant is
        // the root's own name rather than anything inside it — which is exactly why `mjx-docx` keeps
        // the two Rust types apart. Reading each through its own type keeps that distinction here.
        let entries: Vec<(i64, FootnoteEndnoteType, Vec<DirectParagraph>)> = if footnotes {
            let read = Footnotes::from_xml(&doc.root, interner)?;
            let mut collected = Vec::new();
            for note in read.footnotes() {
                collected.push(read_note(note, theme, interner)?);
            }
            collected
        } else {
            let read = Endnotes::from_xml(&doc.root, interner)?;
            let mut collected = Vec::new();
            for note in read.endnotes() {
                collected.push(read_note(note, theme, interner)?);
            }
            collected
        };

        let mut streams = NoteStreams::default();
        for (id, kind, paragraphs) in entries {
            streams.entries.push(NoteStream {
                id,
                kind,
                span: StreamSpan {
                    start: into.len(),
                    length: paragraphs.len(),
                },
            });
            into.extend(paragraphs);
        }
        Ok(streams)
    }

    /// `word/settings.xml`'s layout-relevant half, with every schema default applied.
    fn read_layout_settings(&mut self) -> Result<DocumentLayoutSettings, DocxError> {
        let read = self.document_settings(|settings, interner| -> Result<_, DocxError> {
            Ok(DocumentLayoutSettings {
                auto_hyphenation: attr(settings.auto_hyphenation(interner))?.unwrap_or(false),
                do_not_hyphenate_capitals: attr(settings.do_not_hyphenate_caps(interner))?
                    .unwrap_or(false),
                consecutive_hyphen_limit: settings
                    .consecutive_hyphen_limit()
                    .map(|value| attr(value.value(interner)))
                    .transpose()?,
                hyphenation_zone_twips: settings
                    .hyphenation_zone()
                    .map(|value| attr(value.twips(interner)))
                    .transpose()?
                    .and_then(|measure| {
                        mjx_ooxml_types::support::universal_measure::twips_from_wire(
                            measure.to_wire(),
                        )
                    }),
                even_and_odd_headers: attr(settings.even_and_odd_headers(interner))?
                    .unwrap_or(false),
                mirror_margins: attr(settings.mirror_margins(interner))?.unwrap_or(false),
                default_tab_stop_twips: attr(settings.default_tab_stop_twips(interner))?
                    .and_then(|measure| {
                        mjx_ooxml_types::support::universal_measure::twips_from_wire(
                            measure.to_wire(),
                        )
                    })
                    .unwrap_or(DocumentLayoutSettings::DEFAULT_TAB_STOP_TWIPS),
            })
        })?;
        match read {
            Some(result) => result,
            None => Ok(DocumentLayoutSettings::default()),
        }
    }

    /// The numbering level `reference` names, as the two tiers it contributes to the ladder.
    fn numbering_tier(
        &mut self,
        theme: &ThemeContext,
        reference: EffectiveNumberingReference,
    ) -> Result<NumberingTier, DocxError> {
        let resolved = self.resolve_numbering(
            reference.numbering_id,
            reference.level,
            |lookup, interner| -> Result<NumberingTier, DocxError> {
                let NumberingLookup::Resolved(resolution) = lookup else {
                    return Ok(NumberingTier::default());
                };
                let Some(level) = resolution.level() else {
                    return Ok(NumberingTier::default());
                };
                Ok(NumberingTier {
                    paragraph: level
                        .paragraph_properties()
                        .map(|ppr| extract_style_paragraph_properties(ppr, theme, interner))
                        .transpose()?
                        .unwrap_or_default(),
                    character: level
                        .run_properties()
                        .map(|rpr| extract_run_properties(rpr, theme, interner))
                        .transpose()?
                        .unwrap_or_default(),
                })
            },
        );
        match resolved {
            Ok(inner) => inner,
            // A `w:numPr` naming a list the document does not define is a defect in the document,
            // not a reason to refuse to lay the document out: the paragraph loses its list
            // formatting and keeps its text. `effective_paragraph_properties` propagates the error
            // because its caller asked about that one paragraph; a whole-document read cannot.
            Err(DocxError::UnknownNumberingId(_)) => Ok(NumberingTier::default()),
            Err(other) => Err(other),
        }
    }
}

/// `w:docDefaults`, both halves, read once.
#[derive(Default)]
struct LadderDefaults {
    paragraph: EffectiveParagraphProperties,
    character: EffectiveCharacterProperties,
}

impl LadderDefaults {
    fn read(
        sheet: &StyleSheet,
        theme: &ThemeContext,
        interner: &Interner,
    ) -> Result<Self, DocxError> {
        Ok(Self {
            paragraph: sheet
                .document_defaults()
                .and_then(DocumentDefaults::paragraph_properties_default)
                .and_then(DefaultParagraphProperties::paragraph_properties)
                .map(|ppr| extract_style_paragraph_properties(ppr, theme, interner))
                .transpose()?
                .unwrap_or_default(),
            character: sheet
                .document_defaults()
                .and_then(DocumentDefaults::run_properties_default)
                .and_then(DefaultRunProperties::run_properties)
                .map(|rpr| extract_run_properties(rpr, theme, interner))
                .transpose()?
                .unwrap_or_default(),
        })
    }
}

/// What a numbering level contributes to the two ladders.
#[derive(Default, Clone)]
struct NumberingTier {
    paragraph: EffectiveParagraphProperties,
    character: EffectiveCharacterProperties,
}

/// One paragraph's style tiers, resolved against the one style index this read builds.
struct ParagraphTiers {
    paragraph: EffectiveParagraphProperties,
    character_from_paragraph_style: EffectiveCharacterProperties,
    characters: Vec<EffectiveCharacterProperties>,
    style_numbering: Option<EffectiveNumberingReference>,
}

impl ParagraphTiers {
    /// What a document with no `word/styles.xml` contributes: nothing, once per run.
    fn empty(runs: usize) -> Self {
        Self {
            paragraph: EffectiveParagraphProperties::default(),
            character_from_paragraph_style: EffectiveCharacterProperties::default(),
            characters: vec![EffectiveCharacterProperties::default(); runs],
            style_numbering: None,
        }
    }

    fn read(
        paragraph: &DirectParagraph,
        cache: &ChainCache<'_>,
        theme: &ThemeContext,
        interner: &Interner,
    ) -> Result<Self, DocxError> {
        let chain = match &paragraph.style_id {
            Some(id) => cache.chain(id)?,
            None => Vec::new(),
        };
        let mut characters = Vec::with_capacity(paragraph.runs.len());
        for run in &paragraph.runs {
            let character_chain = match &run.character_style_id {
                Some(id) => cache.chain(id)?,
                None => Vec::new(),
            };
            characters.push(merge_character_chain(&character_chain, theme, interner)?);
        }
        Ok(Self {
            paragraph: merge_paragraph_chain(&chain, theme, interner)?,
            character_from_paragraph_style: merge_character_chain(&chain, theme, interner)?,
            characters,
            style_numbering: numbering_reference_from_chain(&chain, interner)?,
        })
    }
}

/// One paragraph of `word/document.xml`, read.
fn read_direct_paragraph(
    paragraph: &Paragraph,
    theme: &ThemeContext,
    interner: &Interner,
) -> Result<DirectParagraph, DocxError> {
    let mut collected = Collected::default();
    walk_paragraph_content(
        paragraph.content(),
        &mut Vec::new(),
        true,
        theme,
        interner,
        &mut collected,
    )?;

    let direct = match paragraph.properties() {
        Some(ppr) => extract_paragraph_properties(ppr, theme, interner)?,
        None => EffectiveParagraphProperties::default(),
    };
    let style_id = paragraph
        .properties()
        .and_then(ParagraphProperties::style)
        .map(|reference| attr(reference.style_id(interner)))
        .transpose()?
        .map(std::borrow::Cow::into_owned);
    let own_numbering = paragraph
        .properties()
        .and_then(ParagraphProperties::numbering)
        .map(|value| extract_numbering_reference(value, interner))
        .transpose()?
        .flatten();

    Ok(DirectParagraph {
        text: collected.text,
        runs: collected.runs,
        hard_breaks: collected.hard_breaks,
        note_references: collected.note_references,
        direct,
        style_id,
        own_numbering,
    })
}

/// What one walk of a paragraph's content accumulates.
#[derive(Default)]
struct Collected {
    text: String,
    runs: Vec<DirectRun>,
    hard_breaks: Vec<HardBreak>,
    note_references: Vec<NoteReference>,
}

/// Walks a paragraph's content the way `paragraph_content_text` does — descending into every
/// wrapper, so no text is lost — while numbering the slots the way [`Paragraph::run_count`] does, so
/// that the [`RunPath`]s produced actually resolve.
///
/// The two walks differ in exactly one place, and it is reported rather than reconciled: a
/// `w:fldSimple` contributes text and is **not** a run slot, so a run inside one gets `None` for its
/// path (`addressable` is how that travels down). Adding it to the slot walk would renumber every
/// existing address in the crate.
fn walk_paragraph_content(
    content: &[ParagraphContent],
    prefix: &mut Vec<usize>,
    addressable: bool,
    theme: &ThemeContext,
    interner: &Interner,
    collected: &mut Collected,
) -> Result<(), DocxError> {
    let mut slot = 0_usize;
    let descend = |inner: &[ParagraphContent],
                   prefix: &mut Vec<usize>,
                   slot: &mut usize,
                   collected: &mut Collected|
     -> Result<(), DocxError> {
        prefix.push(*slot);
        *slot += 1;
        let result = walk_paragraph_content(inner, prefix, addressable, theme, interner, collected);
        prefix.pop();
        result
    };
    for item in content {
        match item {
            ParagraphContent::Run(run) => {
                let path = if addressable {
                    prefix.push(slot);
                    let path = RunPath::from(prefix.clone());
                    prefix.pop();
                    Some(path)
                } else {
                    None
                };
                slot += 1;
                read_run(run, path, theme, interner, collected)?;
            }
            ParagraphContent::Hyperlink(hyperlink) => {
                descend(hyperlink.content(), prefix, &mut slot, collected)?;
            }
            ParagraphContent::CustomXml(wrapper) => {
                descend(wrapper.content(), prefix, &mut slot, collected)?;
            }
            ParagraphContent::SmartTag(wrapper) => {
                descend(wrapper.content(), prefix, &mut slot, collected)?;
            }
            ParagraphContent::StructuredDocumentTag(control) => {
                let inner = control.content_run().map_or(&[][..], |run| run.content());
                descend(inner, prefix, &mut slot, collected)?;
            }
            ParagraphContent::BidirectionalEmbedding(wrapper) => {
                descend(wrapper.content(), prefix, &mut slot, collected)?;
            }
            ParagraphContent::BidirectionalOverride(wrapper) => {
                descend(wrapper.content(), prefix, &mut slot, collected)?;
            }
            ParagraphContent::SimpleField(field) => {
                // Not a run slot; see this function's own doc comment. Its runs keep their text and
                // lose their address, and `slot` deliberately does not advance — a `w:fldSimple` is
                // invisible to `Paragraph::run_count`, so counting it here would shift every
                // address after it.
                walk_paragraph_content(
                    field.content(),
                    &mut Vec::new(),
                    false,
                    theme,
                    interner,
                    collected,
                )?;
            }
            _ => {}
        }
    }
    Ok(())
}

/// One `w:r`: the characters it contributes and the formatting it states.
fn read_run(
    run: &Run,
    path: Option<RunPath>,
    theme: &ThemeContext,
    interner: &Interner,
    collected: &mut Collected,
) -> Result<(), DocxError> {
    let start = collected.text.len();
    for item in run.content() {
        match item {
            RunInnerContent::Text(value) => collected.text.push_str(value.text()),
            RunInnerContent::TabCharacter(_) => collected.text.push('\t'),
            RunInnerContent::CarriageReturn(_) => collected.text.push('\n'),
            RunInnerContent::OptionalHyphen(_) => collected.text.push(SOFT_HYPHEN),
            RunInnerContent::NonBreakingHyphen(_) => collected.text.push(NON_BREAKING_HYPHEN),
            RunInnerContent::FootnoteReference(reference) => {
                collected.note_references.push(NoteReference {
                    at: collected.text.len(),
                    id: attr(reference.id(interner))?,
                    endnote: false,
                });
            }
            RunInnerContent::EndnoteReference(reference) => {
                collected.note_references.push(NoteReference {
                    at: collected.text.len(),
                    id: attr(reference.id(interner))?,
                    endnote: true,
                });
            }
            RunInnerContent::Break(value) => {
                if let Some(kind @ (BreakType::Page | BreakType::Column)) =
                    attr(value.kind(interner))?
                {
                    collected.hard_breaks.push(HardBreak {
                        at: collected.text.len(),
                        kind,
                    });
                }
                collected.text.push('\n');
            }
            _ => {}
        }
    }
    let direct = match run.run_properties() {
        Some(rpr) => extract_run_properties(rpr, theme, interner)?,
        None => EffectiveCharacterProperties::default(),
    };
    let character_style_id = run
        .run_properties()
        .and_then(RunProperties::character_style)
        .map(|reference| attr(reference.style_id(interner)))
        .transpose()?
        .map(std::borrow::Cow::into_owned);
    collected.runs.push(DirectRun {
        range: start..collected.text.len(),
        path,
        direct,
        character_style_id,
    });
    Ok(())
}

/// One `w:sectPr`, resolved to plain numbers — everything except the header and footer slots, which
/// need the whole span list to walk the inheritance chain.
fn read_section(span: &SectionSpan, interner: &Interner) -> Result<SectionFormatting, DocxError> {
    let mut section = SectionFormatting {
        first_paragraph: span.first_paragraph,
        last_paragraph: span.last_paragraph,
        page_size: None,
        page_margins: None,
        break_kind: None,
        columns: SectionColumns::single(),
        title_page: false,
        page_numbering: SectionPageNumbering::default(),
        line_numbering: None,
        vertical_alignment: None,
        notes: SectionNoteRules::default(),
        suppress_endnotes: false,
        headers: HeaderFooterSlots::default(),
        footers: HeaderFooterSlots::default(),
    };
    let Some(properties) = span.properties.as_ref() else {
        return Ok(section);
    };

    section.page_size = read_page_size(properties, interner)?;
    section.page_margins = read_page_margins(properties, interner)?;
    section.break_kind = match properties.break_kind() {
        Some(kind) => attr(kind.kind(interner))?,
        None => None,
    };
    section.title_page = attr(properties.title_page(interner))?.unwrap_or(false);
    section.suppress_endnotes = attr(properties.no_endnote(interner))?.unwrap_or(false);
    section.vertical_alignment = match properties.vertical_alignment() {
        Some(alignment) => Some(attr(alignment.value(interner))?),
        None => None,
    };
    if let Some(columns) = properties.columns() {
        section.columns = read_columns(columns, interner)?;
    }
    if let Some(numbering) = properties.page_numbering() {
        section.page_numbering = SectionPageNumbering {
            format: attr(numbering.format(interner))?,
            start: attr(numbering.start(interner))?,
        };
    }
    if let Some(numbering) = properties.line_numbering() {
        section.line_numbering = Some(SectionLineNumbering {
            // `w:countBy` carries no schema default, and *every* line numbered is what Word does
            // with an absent one — a `w:lnNumType` that stated a distance and no interval would
            // otherwise number nothing at all, which is not what writing the element means.
            count_by: attr(numbering.count_by(interner))?.unwrap_or(1).max(1),
            start: attr(numbering.start(interner))?,
            distance_twips: attr(numbering.distance_twips(interner))?.map(i64::from),
            restart: attr(numbering.restart(interner))?,
        });
    }
    if let Some(footnotes) = properties.footnote_properties() {
        section.notes.footnote_position = attr_or_none(footnotes.position(interner));
        if let Some(format) = footnotes.number_format() {
            section.notes.footnote_format = attr(format.value(interner))?;
        }
        if let Some(start) = footnotes.number_start(interner) {
            section.notes.footnote_start = start;
        }
        if let Some(restart) = footnotes.number_restart(interner) {
            section.notes.footnote_restart = restart;
        }
    }
    if let Some(endnotes) = properties.endnote_properties() {
        section.notes.endnote_position = attr_or_none(endnotes.position(interner));
        if let Some(format) = endnotes.number_format() {
            section.notes.endnote_format = attr(format.value(interner))?;
        }
        if let Some(start) = endnotes.number_start(interner) {
            section.notes.endnote_start = start;
        }
        if let Some(restart) = endnotes.number_restart(interner) {
            section.notes.endnote_restart = restart;
        }
    }
    Ok(section)
}

/// `Option<T>` in, `Option<T>` out — for the annotation accessors that already swallow a malformed
/// attribute rather than returning a [`Result`].
fn attr_or_none<T>(value: Option<T>) -> Option<T> {
    value
}

/// `w:cols`, with §17.6.4's precedence applied: `w:equalWidth` wins over an explicit list.
fn read_columns(
    columns: &super::sections::Columns,
    interner: &Interner,
) -> Result<SectionColumns, DocxError> {
    let space = i64::from(attr(columns.space_between_twips(interner))?);
    let separator = attr(columns.separator_line(interner))?.unwrap_or(false);
    let equal = attr(columns.is_equal_width(interner))?;
    let stated: Vec<ColumnFormatting> = if equal {
        Vec::new()
    } else {
        let mut list = Vec::new();
        for column in columns.columns() {
            list.push(ColumnFormatting {
                // A `w:col` with no `w:w` states no width. Zero rather than a guess: the caller that
                // divides a text area between columns can see that the file said nothing, and a
                // fabricated width would be indistinguishable from a stated one.
                width_twips: i64::from(attr(column.width_twips(interner))?.unwrap_or(0)),
                space_after_twips: i64::from(attr(column.space_after_twips(interner))?),
            });
        }
        list
    };
    let count = if stated.is_empty() {
        usize::try_from(attr(columns.num(interner))?.max(1)).unwrap_or(1)
    } else {
        stated.len()
    };
    Ok(SectionColumns {
        count: count.max(1),
        space_twips: space,
        separator,
        columns: stated,
    })
}

/// Which header (or footer) stream each of a section's three page kinds shows.
///
/// Every answer comes from [`super::headers::resolve_reference`] — `w:titlePg`,
/// `w:evenAndOddHeaders` and the per-variant inheritance walk, quoted from ECMA-376 Part 1 in that
/// module's own doc comment and implemented there **once**. Nothing about the rules is restated
/// here; what this adds is the interning of the winning `r:id` into `relationships`, so that two
/// sections inheriting one header end up naming one stream.
fn resolve_slots(
    spans: &[SectionSpan],
    index: usize,
    even_and_odd_headers: bool,
    interner: &Interner,
    is_header: bool,
    relationships: &mut Vec<String>,
) -> Result<HeaderFooterSlots, DocxError> {
    let mut slot = |kind: HeaderFooterType| -> Result<Option<usize>, DocxError> {
        let resolved = super::headers::resolve_reference(
            spans,
            index,
            kind,
            even_and_odd_headers,
            interner,
            is_header,
        )?;
        let Some(id) = resolved else {
            return Ok(None);
        };
        Ok(Some(
            match relationships.iter().position(|held| held == &id) {
                Some(at) => at,
                None => {
                    relationships.push(id);
                    relationships.len() - 1
                }
            },
        ))
    };
    Ok(HeaderFooterSlots {
        first: slot(HeaderFooterType::First)?,
        even: slot(HeaderFooterType::Even)?,
        default: slot(HeaderFooterType::Default)?,
    })
}

/// One `w:footnote`/`w:endnote`: its id, its kind and its paragraphs.
fn read_note(
    note: &super::annotations::FootnoteEndnote,
    theme: &ThemeContext,
    interner: &Interner,
) -> Result<(i64, FootnoteEndnoteType, Vec<DirectParagraph>), DocxError> {
    let id = attr(note.id(interner))?;
    // A `w:type` that is present but not one of `ST_FtnEdn`'s four values is read as `normal`, which
    // is `FootnoteEndnote::is_user_visible`'s own leniency: an untrusted file's violation of its own
    // schema is never silently reclassified as "not a footnote".
    let kind = note
        .kind(interner)
        .ok()
        .flatten()
        .unwrap_or(FootnoteEndnoteType::Normal);
    let mut paragraphs = Vec::new();
    for paragraph in note.paragraphs() {
        paragraphs.push(read_direct_paragraph(paragraph, theme, interner)?);
    }
    Ok((id, kind, paragraphs))
}

fn read_page_size(
    properties: &SectionProperties,
    interner: &Interner,
) -> Result<Option<PageSize>, DocxError> {
    attr(properties.page_size(interner))
}

fn read_page_margins(
    properties: &SectionProperties,
    interner: &Interner,
) -> Result<Option<PageMargins>, DocxError> {
    attr(properties.page_margins(interner))
}
