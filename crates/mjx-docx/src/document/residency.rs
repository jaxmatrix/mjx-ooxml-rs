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
//! | `w:delText` | its own text (MJXOFF-177) |
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
//!
//! # MJXOFF-177: the four tracked-change containers are descended into, and that fixed a bug
//!
//! `w:ins`, `w:del`, `w:moveFrom` and `w:moveTo` used to fall to this file's own wildcard, so
//! **content inside one contributed nothing at all** — and a document with tracked insertions was
//! read with the inserted text *missing*, as though every change had been rejected. It was
//! consistent with `revisions.rs`'s crate-wide rule that a run inside one of the four consumes no
//! run-index *slot*, and that rule is about addressing rather than about text.
//!
//! They are now walked, with `addressable = false` for the same reason a `w:fldSimple`'s content
//! is: the runs keep their text and lose their address, and no existing `RunPath` moves.
//! [`ParagraphFormatting::text`] is therefore the **all-markup** view and
//! [`ParagraphFormatting::revisions`] says which bytes are which; see [`RevisionSpan`] for the three
//! subsets each of Word's display modes is.
//!
//! The same walk records [`ParagraphFormatting::fields`] (a field's instruction and the bytes of its
//! cached result) and [`ParagraphFormatting::equations`] (an `m:oMath` resolved to plain values by
//! this crate’s own `document/equations.rs`), and `Document::formatting` resolves every `w:num` a paragraph reaches into
//! a [`NumberingDefinition`]. All four exist for one reason: a box model has to **measure** what a
//! reader sees, and a marker, a field's value, a deletion and an equation are all things a reader
//! sees that a paragraph's own `w:t` runs do not contain.

use std::collections::HashMap;
use std::ops::Range;

use mjx_ooxml_core::{FromXml, Interner};
use mjx_ooxml_types::shared::{RelativeHorizontalAlignment, RelativeVerticalAlignment};
use mjx_ooxml_types::wordprocessingdrawing::{
    HorizontalAlignment, HorizontalRelativeFrom, VerticalAlignment, VerticalRelativeFrom, WrapText,
};
use mjx_ooxml_types::wordprocessingml::{
    BreakType, EndnotePosition, FieldCharacterType, FootnoteEndnoteType, FootnotePosition,
    HeaderFooterType, HeightRule, HorizontalAnchor, Justification, LineNumberRestart,
    MergedCellType, NumberFormat, NumberingLevelSuffix, NumberingRestartLocation, SectionBreakType,
    TableJustification, TableLayoutType, TableWidthUnit, TextFlowDirection, VerticalAnchor,
    VerticalJustification,
};

use super::annotations::{Endnotes, Footnotes};
use super::body::{BlockContent, Paragraph, ParagraphContent, Run, RunInnerContent};
use super::effective::{
    attr, combine_paragraph_tiers, combine_run_tiers, extract_numbering_reference,
    extract_paragraph_properties, extract_run_properties, extract_style_paragraph_properties,
    merge_character_chain, merge_paragraph_chain, numbering_reference_from_chain, ChainCache,
    EffectiveCharacterProperties, EffectiveNumberingReference, EffectiveParagraphProperties,
    ThemeContext,
};
use super::equations::{resolve as resolve_equation, EquationFormatting};
use super::fields::FieldForm;
use super::headers::HdrFtr;
use super::numbering::{LevelTextSegment, NumberingLevel, NumberingLookup};
use super::paragraph_properties::ParagraphProperties;
use super::revisions::{RevisionKind, RunTrackChange};
use super::run_properties::RunProperties;
use super::sections::{SectionProperties, SectionSpan};
use super::styles::{
    DefaultParagraphProperties, DefaultRunProperties, DocumentDefaults, StyleSheet,
};
use super::table_properties::{CellMargins, TableCellMargins, TableProperties, TableWidth};
use super::tables::CellProperties;
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

/// One field in a paragraph's run stream, located in [`ParagraphFormatting::text`].
///
/// # The cached result is text and the instruction is not, which is the whole shape of this type
///
/// Both wire forms carry two things: what the field *says to compute*, and what it computed **last
/// time Word saved the file**. The second is ordinary `w:t` content, so it is already in
/// [`ParagraphFormatting::text`] — [`FieldSpan::result`] says which bytes it is. The first is
/// `w:instrText` (or the `instr` attribute), which no renderer displays, so it is carried here as a
/// string and contributes no character.
///
/// **A renderer that displays [`FieldSpan::result`] and computes nothing looks perfect on any file
/// Word last saved.** That is the trap this type exists to make avoidable rather than to spring: a
/// layout engine reads [`FieldSpan::instruction`], computes, and *replaces* those bytes — and when
/// it cannot compute (a `MERGEFIELD` with no data source), it renders the bytes that are there and
/// says so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldSpan {
    /// The byte of [`ParagraphFormatting::text`] the field begins at.
    pub at: usize,
    /// The bytes its cached result occupies — empty for a complex field with no `w:separate`, which
    /// is legal markup and means the field has never been computed.
    pub result: Range<usize>,
    /// The instruction, verbatim and unparsed, concatenated from **this** field's own `w:instrText`
    /// runs only — a nested field's instruction belongs to the nested field.
    pub instruction: String,
    /// Which wire form it was read from.
    pub form: FieldForm,
    /// `w:dirty` — the author asked for it to be recomputed on open.
    pub dirty: bool,
    /// `w:fldLock` — the result is locked and must **not** be recomputed.
    pub locked: bool,
    /// The index in [`ParagraphFormatting::fields`] of the field this one is nested inside, if any.
    ///
    /// A `TOC` field's own `PAGEREF`s are its children; each is a field in its own right and each
    /// names the `TOC` here. Nesting is what stops a renderer computing a `TOC` by concatenating its
    /// children's instructions.
    pub parent: Option<usize>,
}

/// A span of [`ParagraphFormatting::text`] that one of the four tracked-change containers holds.
///
/// # Deleted text is in the text, and that is a decision
///
/// `w:ins` wraps ordinary `w:t`; `w:del` wraps `w:delText`. Both contribute characters here, so
/// [`ParagraphFormatting::text`] is the **all-markup** view — everything the file holds — and each
/// of Word's four display modes is a *subset* of it, selected by dropping spans:
///
/// | view | drops |
/// |---|---|
/// | all markup / simple markup | nothing |
/// | no markup (final) | [`RevisionKind::Deleted`], [`RevisionKind::MovedFromContent`] |
/// | original | [`RevisionKind::Inserted`], [`RevisionKind::MovedToContent`] |
///
/// One string and three subsets, rather than three strings: a layout engine that had to ask the
/// residency for a different `text()` per view could not keep one set of byte offsets, and every
/// address it produced would mean something different depending on a setting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionSpan {
    /// The bytes it covers.
    pub range: Range<usize>,
    /// Which container it is. Only the four *content* kinds appear here; the property-change kinds
    /// [`Document::revisions`] also reports change no character and therefore no span.
    pub kind: RevisionKind,
    /// `w:author`.
    pub author: Option<String>,
    /// `w:date`, as the wire string — never parsed, exactly as [`RevisionInfo`](crate::RevisionInfo)
    /// does not parse it.
    pub date: Option<String>,
}

/// One level of one `w:num`, resolved: everything needed to *compose the marker* a paragraph at that
/// level shows.
///
/// # Why the residency resolves this and not just the reference
///
/// [`EffectiveParagraphProperties::numbering`](crate::EffectiveParagraphProperties) has always
/// carried the `(w:numId, w:ilvl)` pair, and the level's own `w:pPr`/`w:rPr` have always been folded
/// into the ladder — so a list's *indents* were already right and its *number* was not drawn at all.
///
/// The number needs the level's `w:lvlText`, `w:numFmt`, `w:start`, `w:lvlRestart`, `w:suff` and
/// `w:isLgl`, and reaching them means [`Document::resolve_numbering`], a `&mut self` call that
/// re-parses `word/numbering.xml` and follows `w:numStyleLink` through `word/styles.xml`. A box
/// model calling that per paragraph is the quadratic cost this whole module exists to remove, and
/// `crates/mjx-layout-docx/tests/the_ladder_is_consumed.rs` refuses the identifier outright. So it is
/// resolved here, once per `w:numId` the document actually reaches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumberingLevelFormatting {
    /// `w:ilvl` — which of the nine this is.
    pub level: i64,
    /// `w:start`, defaulting to 1 — §17.9.25's own default.
    pub start: i64,
    /// The `w:startOverride` this instance states for this level, which outranks `w:start`.
    pub start_override: Option<i64>,
    /// `w:numFmt`. `None` when the level states none.
    pub format: Option<NumberFormat>,
    /// `w:lvlText`, parsed into its literal and `%1`…`%9` parts.
    ///
    /// Empty when the level states none, which is a level that draws nothing.
    pub template: Vec<LevelTextSegment>,
    /// `w:suff` — what separates the marker from the paragraph's own text.
    pub suffix: Option<NumberingLevelSuffix>,
    /// `w:lvlRestart` — the **one-based** level whose advance resets this one's count. Zero means
    /// *never restart*, which is the one value a reader must not treat as "level zero".
    pub restart_after: Option<i64>,
    /// `w:isLgl` — every placeholder in the template is written in Arabic numerals whatever each
    /// level's own `w:numFmt` says. A III.B.2 outline becomes 3.2.2.
    pub legal: bool,
    /// `w:lvlJc` — how the marker is aligned in the space before the text.
    pub alignment: Option<Justification>,
    /// `w:lvlPicBulletId` — which `w:numPicBullet` a picture bullet draws.
    pub picture_bullet: Option<i64>,
}

/// Every level of one `w:num`, resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumberingDefinition {
    /// `w:numId`.
    pub numbering_id: i64,
    /// The levels this definition states, in `w:ilvl` order. A level the definition omits is absent
    /// rather than defaulted: a paragraph at an undefined level draws no marker, which is what Word
    /// does and is safer than inventing a template.
    pub levels: Vec<NumberingLevelFormatting>,
}

impl NumberingDefinition {
    /// The level at `index`, or `None` when the definition does not state it.
    #[must_use]
    pub fn level(&self, index: i64) -> Option<&NumberingLevelFormatting> {
        self.levels.iter().find(|level| level.level == index)
    }
}

/// One paragraph, with everything a box model needs and nothing it would have to re-derive.
#[derive(Debug, Clone, PartialEq)]
pub struct ParagraphFormatting {
    text: String,
    runs: Vec<RunFormatting>,
    hard_breaks: Vec<HardBreak>,
    note_references: Vec<NoteReference>,
    drawings: Vec<DrawingFormatting>,
    fields: Vec<FieldSpan>,
    revisions: Vec<RevisionSpan>,
    equations: Vec<EquationFormatting>,
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

    /// Every `w:drawing` anchored in it, in run order — inline and floating alike.
    ///
    /// **Resolved to plain numbers**, never to a `mjx-dml` type: a box model above this crate reads
    /// extents, distances, anchors and wrap polygons and has no business parsing DrawingML. That is
    /// the same line [`ParagraphFormatting::properties`] already draws for `w:pPr`.
    #[must_use]
    pub fn drawings(&self) -> &[DrawingFormatting] {
        &self.drawings
    }

    /// Every field in it, in document order, outermost before innermost.
    ///
    /// **A field's *value* is not resolved here and never will be**, and that is the same line
    /// [`ParagraphFormatting::note_references`] draws: a `PAGE` field's value is a fact about
    /// pagination, which is a fact about a box model, which is a crate that does not exist at this
    /// rank. What travels is the instruction, the cached result's bytes, and the nesting.
    #[must_use]
    pub fn fields(&self) -> &[FieldSpan] {
        &self.fields
    }

    /// Every tracked insertion, deletion and move in it, in document order.
    ///
    /// See [`RevisionSpan`] for why [`ParagraphFormatting::text`] is the all-markup view and each
    /// display mode is a subset of it.
    #[must_use]
    pub fn revisions(&self) -> &[RevisionSpan] {
        &self.revisions
    }

    /// Every `m:oMath` and `m:oMathPara` in it, in document order, resolved to plain values.
    #[must_use]
    pub fn equations(&self) -> &[EquationFormatting] {
        &self.equations
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
    blocks: Vec<BlockFormatting>,
}

impl HeaderFooterFormatting {
    /// Which part it was read from.
    #[must_use]
    pub fn part(&self) -> &mjx_opc::PartName {
        &self.part
    }

    /// Its paragraphs, in document order, resolved exactly as a body paragraph is — its top-level
    /// ones first, then any inside its own tables' cells.
    #[must_use]
    pub fn paragraphs(&self) -> &[ParagraphFormatting] {
        &self.paragraphs
    }

    /// Its content in document order, indexing [`HeaderFooterFormatting::paragraphs`].
    #[must_use]
    pub fn blocks(&self) -> &[BlockFormatting] {
        &self.blocks
    }
}

/// One footnote or endnote, read and resolved.
#[derive(Debug, Clone, PartialEq)]
pub struct NoteFormatting {
    id: i64,
    kind: FootnoteEndnoteType,
    paragraphs: Vec<ParagraphFormatting>,
    blocks: Vec<BlockFormatting>,
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

    /// Its content in document order, indexing [`NoteFormatting::paragraphs`].
    #[must_use]
    pub fn blocks(&self) -> &[BlockFormatting] {
        &self.blocks
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
    top_level_paragraphs: usize,
    paragraphs: Vec<ParagraphFormatting>,
    blocks: Vec<BlockFormatting>,
    sections: Vec<SectionFormatting>,
    settings: DocumentLayoutSettings,
    header_footer_streams: Vec<HeaderFooterFormatting>,
    footnotes: Vec<NoteFormatting>,
    endnotes: Vec<NoteFormatting>,
    numbering: Vec<NumberingDefinition>,
}

impl DocumentFormatting {
    /// Every paragraph of the body: its **top-level** ones first, in document order, and then the
    /// paragraphs inside its tables' cells.
    ///
    /// # The order is a guarantee, not an accident
    ///
    /// The first [`DocumentFormatting::top_level_paragraph_count`] entries are exactly what
    /// [`super::body::Body::paragraphs`] yields, in exactly that order, because a `w:sectPr`'s span
    /// is numbered against *that* walk — a cell's paragraph interleaved into it would move every
    /// section boundary in the document. Cell paragraphs follow, in the order
    /// [`DocumentFormatting::blocks`] reaches them, and [`SectionFormatting`] says nothing about
    /// them.
    ///
    /// This is one list because the ladder resolves once, for every paragraph in the document,
    /// whichever stream and whatever depth it came from; see this module's own documentation.
    #[must_use]
    pub fn paragraphs(&self) -> &[ParagraphFormatting] {
        &self.paragraphs
    }

    /// How many of [`DocumentFormatting::paragraphs`] are at the body's top level.
    #[must_use]
    pub fn top_level_paragraph_count(&self) -> usize {
        self.top_level_paragraphs
    }

    /// The body's content in document order: paragraphs and tables interleaved as the file has them.
    ///
    /// **This, and not [`DocumentFormatting::paragraphs`], is what a box model lays out.** A
    /// paragraph list cannot say that a table sits between two paragraphs, and a document whose
    /// tables were dropped paginates differently from the one the author wrote.
    #[must_use]
    pub fn blocks(&self) -> &[BlockFormatting] {
        &self.blocks
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

    /// Every `w:num` the document's paragraphs actually reach, resolved into what its markers are
    /// composed from, in `w:numId` order.
    ///
    /// **Only the ones reached.** `word/numbering.xml` in a document made from a template routinely
    /// defines dozens of lists nothing uses, and resolving them all would pay a `w:numStyleLink`
    /// walk each for a marker nobody will ever draw.
    #[must_use]
    pub fn numbering_definitions(&self) -> &[NumberingDefinition] {
        &self.numbering
    }

    /// The definition for `numbering_id`, or `None` when nothing reaches it.
    #[must_use]
    pub fn numbering_definition(&self, numbering_id: i64) -> Option<&NumberingDefinition> {
        self.numbering
            .iter()
            .find(|definition| definition.numbering_id == numbering_id)
    }
}

/// What `word/document.xml` alone states about one paragraph, before any style is consulted.
struct DirectParagraph {
    text: String,
    runs: Vec<DirectRun>,
    hard_breaks: Vec<HardBreak>,
    note_references: Vec<NoteReference>,
    drawings: Vec<DrawingFormatting>,
    fields: Vec<FieldSpan>,
    revisions: Vec<RevisionSpan>,
    equations: Vec<EquationFormatting>,
    direct: EffectiveParagraphProperties,
    style_id: Option<String>,
    own_numbering: Option<EffectiveNumberingReference>,
    /// Which table style and conditional regions govern it, when it is inside a cell.
    table_style: Option<TableStyleContext>,
}

/// Where one stream's paragraphs sit in the one flat list `Document::formatting` resolves.
#[derive(Clone)]
struct StreamSpan {
    start: usize,
    length: usize,
    /// The stream's own block tree, whose paragraph indices are **stream-local** — that is, already
    /// rebased by `start`, so a caller of [`HeaderFooterFormatting::blocks`] indexes
    /// [`HeaderFooterFormatting::paragraphs`] and never the flat list this span was cut from.
    blocks: Vec<BlockFormatting>,
}

impl StreamSpan {
    /// This stream's paragraphs, cloned out of the resolved list.
    fn slice(&self, all: &[ParagraphFormatting]) -> Vec<ParagraphFormatting> {
        all.get(self.start..self.start + self.length)
            .unwrap_or_default()
            .to_vec()
    }
}

/// `blocks`, with every paragraph index moved down by `start`.
fn rebased(blocks: Vec<BlockFormatting>, start: usize) -> Vec<BlockFormatting> {
    blocks
        .into_iter()
        .map(|block| match block {
            BlockFormatting::Paragraph(index) => {
                BlockFormatting::Paragraph(index.saturating_sub(start))
            }
            BlockFormatting::Table(mut table) => {
                for row in &mut table.as_mut().rows {
                    for cell in &mut row.cells {
                        cell.content = rebased(std::mem::take(&mut cell.content), start);
                    }
                }
                BlockFormatting::Table(table)
            }
        })
        .collect()
}

/// What one parse of `word/document.xml` yields.
struct DirectRead {
    paragraphs: Vec<DirectParagraph>,
    /// The body's block tree, indexing `paragraphs`.
    blocks: Vec<BlockFormatting>,
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
                blocks: rebased(note.span.blocks.clone(), note.span.start),
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
            blocks,
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
        // Everything up to here is the body: its top-level paragraphs first (whose indices the
        // section spans are stated in) and then the paragraphs inside its tables' cells, which
        // `read_direct`'s second pass appended.
        let body_count = direct_paragraphs.len();
        let top_level_paragraphs = blocks
            .iter()
            .filter(|block| matches!(block, BlockFormatting::Paragraph(_)))
            .count();
        let mut stream_spans: Vec<StreamSpan> = Vec::with_capacity(header_footer_parts.len());
        for part in &header_footer_parts {
            let (stream, stream_blocks) = self.read_header_footer_direct(part, &theme)?;
            stream_spans.push(StreamSpan {
                start: direct_paragraphs.len(),
                length: stream.len(),
                blocks: stream_blocks,
            });
            direct_paragraphs.extend(stream);
        }
        let footnotes = self.read_notes_direct(&theme, true, &mut direct_paragraphs)?;
        let endnotes = self.read_notes_direct(&theme, false, &mut direct_paragraphs)?;

        let resolved = self.style_sheet(|sheet, interner| -> Result<_, DocxError> {
            let index = StyleIndex::build(sheet, interner)?;
            let cache = ChainCache::new(&index, interner);
            let defaults = LadderDefaults::read(sheet, &theme, interner)?;
            let mut tables: HashMap<TableStyleContext, TableStyleTier> = HashMap::new();
            let mut tiers = Vec::with_capacity(direct_paragraphs.len());
            for paragraph in &direct_paragraphs {
                tiers.push(ParagraphTiers::read(
                    paragraph,
                    &cache,
                    &index,
                    &mut tables,
                    &theme,
                    interner,
                )?);
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
        // And every distinct `w:numId`, resolved once into what its markers are *composed* from —
        // which is per definition rather than per level, because `w:lvlText` reaches every level.
        let mut numbering_definitions: Vec<NumberingDefinition> = Vec::new();
        for (paragraph, tier) in direct_paragraphs.iter().zip(&tiers) {
            let Some(reference) = paragraph.own_numbering.or(tier.style_numbering) else {
                continue;
            };
            // `w:numId="0"` is the explicit *removal* of an inherited numbering reference rather
            // than a list, so there is nothing to resolve and an entry for it would make a caller
            // asking "which lists does this document use" answer with one that does not exist.
            //
            // A definition that resolves to **no levels** is not stored either: a `w:numPr` naming a
            // list `word/numbering.xml` does not define is a defect in the document, and reporting
            // an empty definition rather than none would make `numbering_definition` answer `Some`
            // for a list nobody can draw.
            if reference.numbering_id != 0
                && !numbering_definitions
                    .iter()
                    .any(|definition| definition.numbering_id == reference.numbering_id)
            {
                let definition = self.numbering_definition(reference.numbering_id)?;
                if !definition.levels.is_empty() {
                    numbering_definitions.push(definition);
                }
            }
            let key = (reference.numbering_id, reference.level);
            if numbering_effects.contains_key(&key) {
                continue;
            }
            let resolved = self.numbering_tier(&theme, reference)?;
            numbering_effects.insert(key, resolved);
        }
        numbering_definitions.sort_by_key(|definition| definition.numbering_id);

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
                &tier.table_paragraph,
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
                        &tier.table_character,
                    ),
                })
                .collect();
            paragraphs.push(ParagraphFormatting {
                text: paragraph.text,
                runs,
                hard_breaks: paragraph.hard_breaks,
                note_references: paragraph.note_references,
                drawings: paragraph.drawings,
                fields: paragraph.fields,
                revisions: paragraph.revisions,
                equations: paragraph.equations,
                properties,
                style_id: paragraph.style_id,
            });
        }

        // The flat list, cut back into the streams it came from. Cutting from the end forward would
        // need every length again; cutting by recorded span needs none of them.
        let header_footer_streams = header_footer_parts
            .into_iter()
            .zip(stream_spans)
            .map(|(part, span)| HeaderFooterFormatting {
                part,
                paragraphs: span.slice(&paragraphs),
                blocks: rebased(span.blocks.clone(), span.start),
            })
            .collect();
        let footnotes = footnotes.resolved(&paragraphs);
        let endnotes = endnotes.resolved(&paragraphs);
        paragraphs.truncate(body_count);

        Ok(DocumentFormatting {
            top_level_paragraphs,
            paragraphs,
            blocks,
            sections,
            settings,
            header_footer_streams,
            footnotes,
            endnotes,
            numbering: numbering_definitions,
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

        // **Two passes, and the order is load-bearing.** The first reads the body's *top-level*
        // paragraphs in exactly the order `Body::paragraphs` yields them, because that order is what
        // `sections_in` numbers a `w:sectPr` against — a cell's paragraph interleaved here would
        // move every section boundary in the document. The second walks the same content as a block
        // tree, hands each top-level paragraph the index it already has, and appends only what the
        // first pass could not see: the paragraphs inside cells.
        let mut paragraphs = Vec::new();
        for paragraph in body.paragraphs() {
            paragraphs.push(read_direct_paragraph(paragraph, theme, interner)?);
        }
        let mut assigned = 0..paragraphs.len();
        let blocks = walk_blocks(
            body.content(),
            theme,
            interner,
            None,
            &mut assigned,
            &mut paragraphs,
        )?;

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
            blocks,
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
    ) -> Result<(Vec<DirectParagraph>, Vec<BlockFormatting>), DocxError> {
        let Ok(doc) = self.package.part_tree(part) else {
            return Ok((Vec::new(), Vec::new()));
        };
        let Ok(content) = HdrFtr::from_xml(&doc.root, &doc.interner) else {
            return Ok((Vec::new(), Vec::new()));
        };
        let mut paragraphs = Vec::new();
        for paragraph in content.paragraphs() {
            paragraphs.push(read_direct_paragraph(paragraph, theme, &doc.interner)?);
        }
        let mut assigned = 0..paragraphs.len();
        let blocks = walk_blocks(
            content.content(),
            theme,
            &doc.interner,
            None,
            &mut assigned,
            &mut paragraphs,
        )?;
        Ok((paragraphs, blocks))
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
        let entries: Vec<(
            i64,
            FootnoteEndnoteType,
            Vec<DirectParagraph>,
            Vec<BlockFormatting>,
        )> = if footnotes {
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
        for (id, kind, paragraphs, blocks) in entries {
            streams.entries.push(NoteStream {
                id,
                kind,
                span: StreamSpan {
                    start: into.len(),
                    length: paragraphs.len(),
                    blocks,
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

    /// Every level of one `w:num`, resolved into what a marker is composed from.
    ///
    /// **All nine, not the one the paragraph names**, and that is the whole reason this is a
    /// definition rather than a level: `w:lvlText` may hold `%1` through `%9`, so composing a
    /// third-level marker needs the *first* and *second* levels' own formats. A resolver that
    /// fetched one level would render `1.1.a` as `a`.
    fn numbering_definition(
        &mut self,
        numbering_id: i64,
    ) -> Result<NumberingDefinition, DocxError> {
        let mut levels = Vec::new();
        for index in 0..NUMBERING_LEVELS {
            let resolved = self.resolve_numbering(
                numbering_id,
                index,
                |lookup, interner| -> Result<Option<NumberingLevelFormatting>, DocxError> {
                    let NumberingLookup::Resolved(resolution) = lookup else {
                        return Ok(None);
                    };
                    let start_override = resolution
                        .instance()
                        .level_override(index, interner)
                        .ok()
                        .flatten()
                        .and_then(|entry| entry.start_override(interner).ok().flatten());
                    let Some(level) = resolution.level() else {
                        return Ok(None);
                    };
                    Ok(Some(read_numbering_level(
                        level,
                        index,
                        start_override,
                        interner,
                    )?))
                },
            );
            match resolved {
                Ok(Ok(Some(level))) => levels.push(level),
                Ok(Ok(None)) => {}
                Ok(Err(error)) => return Err(error),
                // Same posture as `numbering_tier`: a list the document does not define costs the
                // paragraph its marker and nothing else.
                Err(DocxError::UnknownNumberingId(_)) => break,
                Err(other) => return Err(other),
            }
        }
        Ok(NumberingDefinition {
            numbering_id,
            levels,
        })
    }
}

/// How many levels a `w:num` has. Nine, and stated by the schema rather than chosen: `w:ilvl` is
/// `ST_DecimalNumber` restricted to 0–8 by §17.9.4's own prose, and `w:lvlText`'s placeholders run
/// `%1` to `%9`.
const NUMBERING_LEVELS: i64 = 9;

/// One `w:lvl`, resolved.
fn read_numbering_level(
    level: &NumberingLevel,
    index: i64,
    start_override: Option<i64>,
    interner: &Interner,
) -> Result<NumberingLevelFormatting, DocxError> {
    Ok(NumberingLevelFormatting {
        level: index,
        // §17.9.25: `w:start`'s own default is 1. A level that states nothing counts from one, which
        // is what every list in every document does.
        start: attr(level.start(interner))?.unwrap_or(1),
        start_override,
        format: level
            .format()
            .map(|format| attr(format.format(interner)))
            .transpose()?,
        template: level
            .text_template()
            .map(|template| attr(template.segments(interner)))
            .transpose()?
            .flatten()
            .unwrap_or_default(),
        suffix: level
            .suffix()
            .map(|suffix| attr(suffix.suffix(interner)))
            .transpose()?,
        restart_after: attr(level.restart_after_level(interner))?,
        legal: attr(level.is_legal_numbering(interner))?.unwrap_or(false),
        alignment: level
            .alignment()
            .map(|alignment| attr(alignment.value(interner)))
            .transpose()?,
        picture_bullet: attr(level.picture_bullet_id(interner))?,
    })
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
    /// The table style's own contribution, already folded across every conditional region that
    /// covers this paragraph's cell. All-`None` outside a table, which is the identity.
    table_paragraph: EffectiveParagraphProperties,
    /// The same for a run.
    table_character: EffectiveCharacterProperties,
}

impl ParagraphTiers {
    /// What a document with no `word/styles.xml` contributes: nothing, once per run.
    fn empty(runs: usize) -> Self {
        Self {
            paragraph: EffectiveParagraphProperties::default(),
            character_from_paragraph_style: EffectiveCharacterProperties::default(),
            characters: vec![EffectiveCharacterProperties::default(); runs],
            style_numbering: None,
            table_paragraph: EffectiveParagraphProperties::default(),
            table_character: EffectiveCharacterProperties::default(),
        }
    }

    fn read(
        paragraph: &DirectParagraph,
        cache: &ChainCache<'_>,
        index: &StyleIndex<'_>,
        tables: &mut HashMap<TableStyleContext, TableStyleTier>,
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
        let table = match &paragraph.table_style {
            Some(context) => {
                if !tables.contains_key(context) {
                    let resolved = TableStyleTier::resolve(context, index, theme, interner)?;
                    tables.insert(context.clone(), resolved);
                }
                tables.get(context).cloned().unwrap_or_default()
            }
            None => TableStyleTier::default(),
        };
        Ok(Self {
            paragraph: merge_paragraph_chain(&chain, theme, interner)?,
            character_from_paragraph_style: merge_character_chain(&chain, theme, interner)?,
            characters,
            style_numbering: numbering_reference_from_chain(&chain, interner)?,
            table_paragraph: table.paragraph,
            table_character: table.character,
        })
    }
}

/// One table style's contribution to the ladder, folded over the regions that cover a cell.
#[derive(Clone, Default)]
struct TableStyleTier {
    paragraph: EffectiveParagraphProperties,
    character: EffectiveCharacterProperties,
}

impl TableStyleTier {
    /// Resolves `context` against the style sheet, once per distinct `(style, regions)` pair.
    ///
    /// A `w:tblStyle` naming a style the document does not define contributes **nothing** rather
    /// than refusing the document — the same leniency `numbering_tier` states for a `w:numPr` naming
    /// a list that is not defined.
    fn resolve(
        context: &TableStyleContext,
        index: &StyleIndex<'_>,
        theme: &ThemeContext,
        interner: &Interner,
    ) -> Result<Self, DocxError> {
        let chain = match index.based_on_chain(&context.style_id, interner) {
            Ok(chain) => chain,
            Err(DocxError::UnknownStyleId(_)) => return Ok(Self::default()),
            Err(other) => return Err(other),
        };
        Ok(Self {
            paragraph: super::table_regions::paragraph_properties_tier(
                &chain,
                &context.regions,
                theme,
                interner,
            )?,
            character: super::table_regions::run_properties_tier(
                &chain,
                &context.regions,
                theme,
                interner,
            )?,
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
        drawings: collected.drawings,
        fields: collected.fields,
        revisions: collected.revisions,
        equations: collected.equations,
        direct,
        style_id,
        own_numbering,
        table_style: None,
    })
}

/// A complex field whose `w:fldChar begin` has been seen and whose `end` has not.
///
/// The stack this lives on is what makes nesting work: a `TOC`'s `PAGEREF` opens while the `TOC` is
/// still open, its `w:instrText` belongs to it and not to the `TOC`, and when it closes the `TOC` is
/// on top again.
struct OpenField {
    /// Which entry of `Collected::fields` this is — reserved at `begin` so that a nested field can
    /// name it as its parent before it closes.
    index: usize,
    /// Where the result begins, once `w:separate` has been seen.
    result_start: Option<usize>,
}

/// What one walk of a paragraph's content accumulates.
#[derive(Default)]
struct Collected {
    text: String,
    runs: Vec<DirectRun>,
    hard_breaks: Vec<HardBreak>,
    note_references: Vec<NoteReference>,
    drawings: Vec<DrawingFormatting>,
    fields: Vec<FieldSpan>,
    revisions: Vec<RevisionSpan>,
    equations: Vec<EquationFormatting>,
    /// The complex fields currently open, innermost last.
    open_fields: Vec<OpenField>,
}

impl Collected {
    /// Opens a complex field at the current position, reserving its entry.
    fn begin_field(&mut self, dirty: bool, locked: bool) {
        let index = self.fields.len();
        let parent = self.open_fields.last().map(|open| open.index);
        self.fields.push(FieldSpan {
            at: self.text.len(),
            result: self.text.len()..self.text.len(),
            instruction: String::new(),
            form: FieldForm::Complex,
            dirty,
            locked,
            parent,
        });
        self.open_fields.push(OpenField {
            index,
            result_start: None,
        });
    }

    /// Appends `text` to the innermost open field's instruction.
    ///
    /// **Only before its own `w:separate`.** A `w:instrText` after the separator is not part of the
    /// instruction — Word writes none there, and a reader that appended it anyway would turn a
    /// malformed file into a field whose instruction changes what it computes.
    fn field_instruction(&mut self, text: &str) {
        let Some(open) = self.open_fields.last() else {
            return;
        };
        if open.result_start.is_some() {
            return;
        }
        let index = open.index;
        if let Some(field) = self.fields.get_mut(index) {
            field.instruction.push_str(text);
        }
    }

    /// Marks the innermost open field's result as starting here.
    fn separate_field(&mut self) {
        let at = self.text.len();
        if let Some(open) = self.open_fields.last_mut() {
            open.result_start = Some(at);
        }
    }

    /// Closes the innermost open field.
    ///
    /// An `end` with nothing open is a malformed file and is **ignored** rather than refused: a
    /// stray `w:fldChar` must not stop a document opening, and this crate's whole posture towards
    /// untrusted input is to render what can be rendered.
    fn end_field(&mut self) {
        let at = self.text.len();
        let Some(open) = self.open_fields.pop() else {
            return;
        };
        if let Some(field) = self.fields.get_mut(open.index) {
            let start = open.result_start.unwrap_or(at);
            field.result = start.min(at)..at;
        }
    }
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
                let index = collected.fields.len();
                let at = collected.text.len();
                let parent = collected.open_fields.last().map(|open| open.index);
                collected.fields.push(FieldSpan {
                    at,
                    result: at..at,
                    instruction: field.instruction(interner),
                    form: FieldForm::Simple,
                    dirty: attr(field.dirty(interner))?,
                    locked: attr(field.locked(interner))?,
                    parent,
                });
                walk_paragraph_content(
                    field.content(),
                    &mut Vec::new(),
                    false,
                    theme,
                    interner,
                    collected,
                )?;
                let end = collected.text.len();
                if let Some(span) = collected.fields.get_mut(index) {
                    span.result = at..end;
                }
            }
            // **The four tracked-change containers, descended into rather than skipped.** Before
            // MJXOFF-177 they fell to the wildcard below, which meant a document with tracked
            // insertions laid out with the inserted text *missing* — every `w:ins` is ordinary
            // `w:t`, and dropping it silently renders the file as though every change had been
            // rejected. See `RevisionSpan` for why the text is now the all-markup view.
            //
            // `addressable` is `false` for the same reason a `w:fldSimple`'s is: `revisions.rs`'s
            // own crate-wide invariant is that a run inside one of these four consumes no run-index
            // slot, so producing a `RunPath` here would invent an address that does not resolve.
            ParagraphContent::Ins(change) => {
                read_revision(change, RevisionKind::Inserted, theme, interner, collected)?;
            }
            ParagraphContent::Del(change) => {
                read_revision(change, RevisionKind::Deleted, theme, interner, collected)?;
            }
            ParagraphContent::MoveFrom(change) => {
                read_revision(
                    change,
                    RevisionKind::MovedFromContent,
                    theme,
                    interner,
                    collected,
                )?;
            }
            ParagraphContent::MoveTo(change) => {
                read_revision(
                    change,
                    RevisionKind::MovedToContent,
                    theme,
                    interner,
                    collected,
                )?;
            }
            // An equation is a sibling of `w:r`, not a child of one — `EG_MathContent` sits at run
            // level — so it is met here rather than in `read_run`. Like a note reference it
            // contributes no character: what is drawn is generated by a typesetter, and the run
            // stream holds no glyph for it.
            ParagraphContent::Math(math) => {
                collected.equations.push(EquationFormatting {
                    at: collected.text.len(),
                    display: false,
                    justification: None,
                    nodes: resolve_equation(math, interner),
                });
            }
            ParagraphContent::MathParagraph(paragraph) => {
                let justification = paragraph
                    .properties(interner)
                    .and_then(|properties| properties.justification(interner));
                for equation in paragraph.equations(interner) {
                    collected.equations.push(EquationFormatting {
                        at: collected.text.len(),
                        display: true,
                        justification,
                        nodes: resolve_equation(&equation, interner),
                    });
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// One `w:ins`/`w:del`/`w:moveFrom`/`w:moveTo`: the characters its content contributes, and the span
/// that says which they are.
fn read_revision(
    change: &RunTrackChange,
    kind: RevisionKind,
    theme: &ThemeContext,
    interner: &Interner,
    collected: &mut Collected,
) -> Result<(), DocxError> {
    let start = collected.text.len();
    walk_paragraph_content(
        change.content(),
        &mut Vec::new(),
        false,
        theme,
        interner,
        collected,
    )?;
    collected.revisions.push(RevisionSpan {
        range: start..collected.text.len(),
        kind,
        author: change.author(interner),
        date: change.date(interner),
    });
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
            // `w:delText` is `w:t` for content that has been deleted, and it contributes its
            // characters for the same reason `w:t` does: in an all-markup view a deletion is drawn,
            // struck through, and **occupies space** — which is what makes the same document
            // paginate differently in two views. Which bytes they are is `Collected::revisions`.
            RunInnerContent::DeletedText(value) => collected.text.push_str(value.text()),
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
            RunInnerContent::Drawing(drawing) => {
                if let Some(read) = read_drawing(
                    drawing,
                    collected.text.len(),
                    collected.runs.len(),
                    interner,
                ) {
                    collected.drawings.push(read);
                }
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
            // A field's three markers. Each contributes no character — a `w:fldChar` is a bracket,
            // not content — and together they delimit the instruction and the cached result. See
            // `FieldSpan`.
            RunInnerContent::ComplexFieldCharacter(marker) => match attr(marker.kind(interner))? {
                FieldCharacterType::Begin => collected.begin_field(
                    attr(marker.dirty(interner))?,
                    attr(marker.locked(interner))?,
                ),
                FieldCharacterType::Separate => collected.separate_field(),
                FieldCharacterType::End => collected.end_field(),
            },
            // The instruction itself: read, and deliberately **not** pushed to the text. It is what
            // the field says to compute, and no renderer displays it.
            RunInnerContent::FieldCode(value) | RunInnerContent::DeletedFieldCode(value) => {
                collected.field_instruction(value.text());
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
) -> Result<
    (
        i64,
        FootnoteEndnoteType,
        Vec<DirectParagraph>,
        Vec<BlockFormatting>,
    ),
    DocxError,
> {
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
    let mut assigned = 0..paragraphs.len();
    let blocks = walk_blocks(
        note.content(),
        theme,
        interner,
        None,
        &mut assigned,
        &mut paragraphs,
    )?;
    Ok((id, kind, paragraphs, blocks))
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

// =================================================================================================
// MJXOFF-176 (R21) — drawings and tables, resolved to plain numbers.
//
// Everything below answers one question: *what does a box model above this crate need to know about
// a `w:drawing` and a `w:tbl` that it must not re-derive?* The answer has the same shape the rest of
// this module already has — every wire string parsed once, every inheritance resolved once, and no
// `mjx-dml` type in the surface, because `mjx-layout-docx` deliberately does not depend on
// DrawingML (see its own `Cargo.toml`, which states that as a decision rather than an omission).
// =================================================================================================

/// Four distances in EMU, in the order `w:drawing`'s own attributes name them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DrawingDistances {
    /// `distT`.
    pub top: i64,
    /// `distB`.
    pub bottom: i64,
    /// `distL`.
    pub left: i64,
    /// `distR`.
    pub right: i64,
}

/// One axis of a floating drawing's position: a named alignment or an explicit offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisPlacement<A> {
    /// `wp:align` — a keyword resolved against whatever `relativeFrom` names.
    Aligned(A),
    /// `wp:posOffset` — a signed offset in EMU from that same origin.
    Offset(i64),
}

/// `wp:positionH`, resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HorizontalAnchoring {
    /// `@relativeFrom` — the frame the placement is measured in.
    pub relative_to: HorizontalRelativeFrom,
    /// Where in that frame.
    pub placement: AxisPlacement<HorizontalAlignment>,
}

/// `wp:positionV`, resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerticalAnchoring {
    /// `@relativeFrom`.
    pub relative_to: VerticalRelativeFrom,
    /// Where in that frame.
    pub placement: AxisPlacement<VerticalAlignment>,
}

/// How text behaves around a floating drawing.
///
/// The five `wp:wrap*` elements, with the two that carry a real polygon keeping it. **The polygon's
/// coordinates travel exactly as the file wrote them** — this module does not decide what unit they
/// are in, because that decision is a layout reading rather than a fact and belongs beside the rest
/// of them (`mjx_layout_docx::wrap`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WrapFormatting {
    /// `wp:wrapNone` — the text is not displaced at all; the drawing sits behind it or in front of
    /// it, which [`AnchoredDrawing::behind_text`] decides.
    None,
    /// `wp:wrapSquare` — the drawing's bounding box, plus its distances, displaces text.
    Square {
        /// `@wrapText` — which sides text may flow down.
        side: WrapText,
        /// `wp:wrapSquare`'s own four distances, which override the anchor's.
        distance: DrawingDistances,
    },
    /// `wp:wrapTight` — text follows the polygon, and does not enter it.
    Tight {
        /// `@wrapText`.
        side: WrapText,
        /// `wp:wrapPolygon`'s points, in the file's own coordinates.
        polygon: Vec<(i64, i64)>,
        /// `@distL`.
        distance_left: i64,
        /// `@distR`.
        distance_right: i64,
    },
    /// `wp:wrapThrough` — the same, except that text may also enter a concavity that opens to the
    /// side, which is the whole difference between the two.
    Through {
        /// `@wrapText`.
        side: WrapText,
        /// `wp:wrapPolygon`'s points.
        polygon: Vec<(i64, i64)>,
        /// `@distL`.
        distance_left: i64,
        /// `@distR`.
        distance_right: i64,
    },
    /// `wp:wrapTopAndBottom` — no text beside the drawing at all; the band it occupies is cleared
    /// across the whole measure.
    TopAndBottom {
        /// `@distT`.
        distance_top: i64,
        /// `@distB`.
        distance_bottom: i64,
    },
}

/// A `wp:anchor`, resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnchoredDrawing {
    /// The anchor's own four distances.
    pub distance: DrawingDistances,
    /// `wp:effectExtent` — how far the drawing's effects reach past its extent.
    pub effect_extent: DrawingDistances,
    /// `@behindDoc`.
    pub behind_text: bool,
    /// `@allowOverlap`.
    pub allow_overlap: bool,
    /// `@layoutInCell` — whether a drawing anchored inside a table cell is positioned against the
    /// cell or against the page.
    pub layout_in_cell: bool,
    /// `@relativeHeight` — the z order.
    pub relative_height: u32,
    /// `@hidden`.
    pub hidden: bool,
    /// `wp:positionH`.
    pub horizontal: HorizontalAnchoring,
    /// `wp:positionV`.
    pub vertical: VerticalAnchoring,
    /// The wrap mode.
    pub wrap: WrapFormatting,
}

/// Whether a drawing sits in the line or floats beside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DrawingPlacement {
    /// `wp:inline` — the drawing is a character of the line, with its own four distances.
    Inline(DrawingDistances),
    /// `wp:anchor` — the drawing floats.
    Anchored(Box<AnchoredDrawing>),
}

/// One `w:drawing` in a run stream, resolved to plain numbers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawingFormatting {
    /// The byte of [`ParagraphFormatting::text`] the drawing sits at.
    ///
    /// **It contributes no character**, for the same reason a `w:footnoteReference` mark does not:
    /// what the residency knows is *where* the object is anchored, and what it looks like is a
    /// question for whatever draws it. See [`NoteReference`]'s own doc comment, which states the
    /// rule this follows.
    pub at: usize,
    /// Which entry of [`ParagraphFormatting::runs`] holds it.
    pub run: usize,
    /// `wp:extent/@cx`, in EMU.
    pub width: i64,
    /// `wp:extent/@cy`, in EMU.
    pub height: i64,
    /// Inline or floating.
    pub placement: DrawingPlacement,
    /// The `wp:docPr@id` the drawing states, or `None` for one that states none.
    ///
    /// **The address `Document::chart_part_bytes` takes** (MJXOFF-178). A box model that has laid a
    /// drawing out and wants to know what is *inside* it has to name it back to the format crate,
    /// and this is the only identifier a `w:drawing` carries.
    pub id: Option<u32>,
    /// Whether the drawing frames a chart (`a:graphicData > c:chart`).
    ///
    /// Read here rather than left to the box model because the residency already has the
    /// `a:graphic` open: asking again would mean re-parsing `word/document.xml`, which is the
    /// quadratic read this whole module exists to end.
    pub frames_a_chart: bool,
}

/// One block of body content: a paragraph, or a table.
///
/// # Why a paragraph is an index and a table is a value
///
/// Every paragraph in the document — the body's own, a table cell's, a header's, a note's — lives in
/// **one** flat list, resolved through the ladder in one pass, which is the whole reason this module
/// exists. A block tree that owned its paragraphs would be a second copy of that list; a block tree
/// that indexes into it is free. A table has no such list to live in, so it is owned here, and its
/// cells hold [`BlockFormatting`]s of their own — which is exactly what makes a nested table a
/// nested table with nothing further written for it.
#[derive(Debug, Clone, PartialEq)]
pub enum BlockFormatting {
    /// The paragraph at this index of whichever list this block tree belongs to —
    /// [`DocumentFormatting::paragraphs`] for the body, [`HeaderFooterFormatting::paragraphs`] for a
    /// header or footer, [`NoteFormatting::paragraphs`] for a note.
    Paragraph(usize),
    /// A table.
    ///
    /// Boxed because a `TableFormatting` is two orders of magnitude larger than a paragraph index,
    /// and a body of ten thousand paragraphs with three tables in it would otherwise pay the table's
    /// size ten thousand times over.
    Table(Box<TableFormatting>),
}

/// `w:tblW` and its five siblings, resolved to a unit and a number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WidthSpecification {
    /// Which unit `value` is in.
    pub unit: TableWidthUnit,
    /// The number itself: twips for [`TableWidthUnit::Twips`], fiftieths of a percent for
    /// [`TableWidthUnit::Percent`], and meaningless for the other two.
    pub value: i64,
}

impl WidthSpecification {
    /// The width in twips, or `None` when this specification does not state one.
    #[must_use]
    pub fn twips(self) -> Option<i64> {
        match self.unit {
            TableWidthUnit::Twips => Some(self.value),
            _ => None,
        }
    }

    /// The width as a fraction of its container, or `None` when it is not a percentage.
    ///
    /// `w:tblW@type="pct"` is in **fiftieths of a percent** (`5000` is 100 %), which is the one
    /// place in WordprocessingML where a percentage is not `ST_Percentage`'s thousandths.
    #[must_use]
    pub fn fraction(self) -> Option<f64> {
        match self.unit {
            #[allow(clippy::cast_precision_loss)]
            TableWidthUnit::Percent => Some(self.value as f64 / 5000.0),
            _ => None,
        }
    }
}

/// One row's height request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowHeightSpecification {
    /// `w:trHeight/@val`, in twips.
    pub twips: i64,
    /// `w:trHeight/@hRule`.
    pub rule: HeightRule,
}

/// A cell's four margins, each stated or inherited.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CellMarginsSpecification {
    /// `w:top`, in twips.
    pub top: Option<i64>,
    /// `w:bottom`.
    pub bottom: Option<i64>,
    /// `w:start`/`w:left`.
    pub start: Option<i64>,
    /// `w:end`/`w:right`.
    pub end: Option<i64>,
}

impl CellMarginsSpecification {
    /// Word's own default cell margins: nothing above or below, `0.08"` (115 twips) each side.
    ///
    /// **`GUESS:`** ECMA-376 states no default for `w:tblCellMar`. 115 twips is what every table
    /// Word creates writes explicitly, and a table with no side margins at all sets its text hard
    /// against its own rules, which is immediately visible.
    pub const WORD_DEFAULT: Self = Self {
        top: Some(0),
        bottom: Some(0),
        start: Some(115),
        end: Some(115),
    };

    /// This specification with every side `other` states and this one does not.
    #[must_use]
    pub fn or(self, other: Self) -> Self {
        Self {
            top: self.top.or(other.top),
            bottom: self.bottom.or(other.bottom),
            start: self.start.or(other.start),
            end: self.end.or(other.end),
        }
    }

    /// Every side settled as `(top, bottom, start, end)`, falling back to
    /// [`CellMarginsSpecification::WORD_DEFAULT`].
    #[must_use]
    pub fn settled(self) -> (i64, i64, i64, i64) {
        let filled = self.or(Self::WORD_DEFAULT);
        (
            filled.top.unwrap_or(0),
            filled.bottom.unwrap_or(0),
            filled.start.unwrap_or(0),
            filled.end.unwrap_or(0),
        )
    }
}

/// `w:tblpPr`, resolved — a floating table's own anchoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloatingTableAnchoring {
    /// `@horzAnchor`.
    pub horizontal_anchor: Option<HorizontalAnchor>,
    /// `@vertAnchor`.
    pub vertical_anchor: Option<VerticalAnchor>,
    /// `@tblpXSpec`, when the position is a keyword.
    pub x_alignment: Option<RelativeHorizontalAlignment>,
    /// `@tblpX`, in twips, when it is a number.
    pub x_twips: Option<i64>,
    /// `@tblpYSpec`.
    pub y_alignment: Option<RelativeVerticalAlignment>,
    /// `@tblpY`, in twips.
    pub y_twips: Option<i64>,
    /// `@leftFromText`, in twips.
    pub left_from_text: i64,
    /// `@rightFromText`.
    pub right_from_text: i64,
    /// `@topFromText`.
    pub top_from_text: i64,
    /// `@bottomFromText`.
    pub bottom_from_text: i64,
}

/// One `w:tc`, resolved.
#[derive(Debug, Clone, PartialEq)]
pub struct CellFormatting {
    /// `w:gridSpan` — how many grid columns it covers. Never below one.
    pub grid_span: usize,
    /// `w:vMerge` — `Some(true)` for the anchor (`restart`), `Some(false)` for a covered
    /// continuation, `None` for a cell in no vertical merge.
    pub vertical_merge_anchor: Option<bool>,
    /// `w:tcW`.
    pub width: Option<WidthSpecification>,
    /// `w:tcMar`, before the table's own `w:tblCellMar` fills the gaps.
    pub margins: CellMarginsSpecification,
    /// `w:vAlign`.
    pub vertical_alignment: Option<VerticalJustification>,
    /// `w:textDirection`.
    pub text_direction: Option<TextFlowDirection>,
    /// `w:noWrap`.
    pub no_wrap: bool,
    /// What is in it: paragraphs and, recursively, tables.
    pub content: Vec<BlockFormatting>,
}

/// One `w:tr`, resolved.
#[derive(Debug, Clone, PartialEq)]
pub struct RowFormatting {
    /// Its cells, in order.
    pub cells: Vec<CellFormatting>,
    /// `w:cantSplit` — the row moves whole to the next page rather than breaking across one.
    pub cannot_split: bool,
    /// `w:tblHeader` — the row repeats at the top of every page the table spans.
    pub repeat_as_header: bool,
    /// `w:trHeight`.
    pub height: Option<RowHeightSpecification>,
    /// `w:gridBefore` — grid columns left empty before the first cell.
    pub grid_before: usize,
    /// `w:gridAfter`.
    pub grid_after: usize,
    /// `w:hidden`.
    pub hidden: bool,
}

/// One `w:tbl`, resolved.
#[derive(Debug, Clone, PartialEq)]
pub struct TableFormatting {
    /// `w:tblGrid/w:gridCol/@w`, in twips, in order. The authority on how many columns there are.
    pub grid_twips: Vec<i64>,
    /// `w:tblLayout/@type` — absent reads as [`TableLayoutType::Autofit`], which is §17.4.52's own
    /// default.
    pub layout: TableLayoutType,
    /// `w:tblW`.
    pub width: Option<WidthSpecification>,
    /// `w:tblInd`, in twips.
    pub indent_twips: i64,
    /// `w:tblCellSpacing`, in twips.
    pub cell_spacing_twips: i64,
    /// `w:tblCellMar` — the table-wide cell margins a cell's own `w:tcMar` overrides.
    pub cell_margins: CellMarginsSpecification,
    /// `w:jc` — how the table sits in its column.
    pub alignment: Option<TableJustification>,
    /// `w:tblpPr`, when the table floats.
    pub floating: Option<FloatingTableAnchoring>,
    /// `w:tblStyle/@val` — the id, for whatever resolves conditional formatting.
    pub style_id: Option<String>,
    /// Its rows, in order.
    pub rows: Vec<RowFormatting>,
}

impl TableFormatting {
    /// How many grid columns it has.
    #[must_use]
    pub fn column_count(&self) -> usize {
        self.grid_twips.len()
    }
}

/// Reads one `w:drawing` into plain numbers, or `None` when it states neither placement.
fn read_drawing(
    drawing: &super::drawing::Drawing,
    at: usize,
    run: usize,
    interner: &Interner,
) -> Option<DrawingFormatting> {
    if let Some(inline) = drawing.inline() {
        let extent = inline.extent(interner);
        return Some(DrawingFormatting {
            at,
            run,
            width: extent.map_or(0, |size| size.width.emu()),
            height: extent.map_or(0, |size| size.height.emu()),
            id: inline
                .doc_properties(interner)
                .and_then(|properties| properties.id(interner).ok()),
            frames_a_chart: inline
                .graphic(interner)
                .and_then(|graphic| graphic.data().chart_relationship_id(interner))
                .is_some(),
            placement: DrawingPlacement::Inline(DrawingDistances {
                top: emu_or_zero(inline.distance_top(interner)),
                bottom: emu_or_zero(inline.distance_bottom(interner)),
                left: emu_or_zero(inline.distance_left(interner)),
                right: emu_or_zero(inline.distance_right(interner)),
            }),
        });
    }
    let anchor = drawing.anchor()?;
    let extent = anchor.extent(interner);
    let effect = anchor.effect_extent(interner);
    let horizontal = anchor
        .position_horizontal(interner)
        .and_then(|position| {
            Some(HorizontalAnchoring {
                relative_to: position.relative_from()?,
                placement: match position.value()? {
                    mjx_dml::wordprocessing_drawing::PositionValue::Align(alignment) => {
                        AxisPlacement::Aligned(alignment)
                    }
                    mjx_dml::wordprocessing_drawing::PositionValue::Offset(offset) => {
                        AxisPlacement::Offset(offset.emu())
                    }
                },
            })
        })
        .unwrap_or(HorizontalAnchoring {
            // A malformed `wp:positionH` is read as *no displacement from the column* rather than
            // refused: an anchor whose position will not parse still has an extent, and dropping the
            // whole drawing would lose an object a reader can see.
            relative_to: HorizontalRelativeFrom::Column,
            placement: AxisPlacement::Offset(0),
        });
    let vertical = anchor
        .position_vertical(interner)
        .and_then(|position| {
            Some(VerticalAnchoring {
                relative_to: position.relative_from()?,
                placement: match position.value()? {
                    mjx_dml::wordprocessing_drawing::PositionValue::Align(alignment) => {
                        AxisPlacement::Aligned(alignment)
                    }
                    mjx_dml::wordprocessing_drawing::PositionValue::Offset(offset) => {
                        AxisPlacement::Offset(offset.emu())
                    }
                },
            })
        })
        .unwrap_or(VerticalAnchoring {
            relative_to: VerticalRelativeFrom::Paragraph,
            placement: AxisPlacement::Offset(0),
        });
    let distance = DrawingDistances {
        top: emu_or_zero(anchor.distance_top(interner)),
        bottom: emu_or_zero(anchor.distance_bottom(interner)),
        left: emu_or_zero(anchor.distance_left(interner)),
        right: emu_or_zero(anchor.distance_right(interner)),
    };
    Some(DrawingFormatting {
        at,
        run,
        width: extent.map_or(0, |size| size.width.emu()),
        height: extent.map_or(0, |size| size.height.emu()),
        id: anchor
            .doc_properties(interner)
            .and_then(|properties| properties.id(interner).ok()),
        frames_a_chart: anchor
            .graphic(interner)
            .and_then(|graphic| graphic.data().chart_relationship_id(interner))
            .is_some(),
        placement: DrawingPlacement::Anchored(Box::new(AnchoredDrawing {
            distance,
            effect_extent: DrawingDistances {
                top: effect.as_ref().map_or(0, |extent| {
                    extent
                        .top(interner)
                        .ok()
                        .map_or(0, mjx_ooxml_core::measure::Emu::emu)
                }),
                bottom: effect.as_ref().map_or(0, |extent| {
                    extent
                        .bottom(interner)
                        .ok()
                        .map_or(0, mjx_ooxml_core::measure::Emu::emu)
                }),
                left: effect.as_ref().map_or(0, |extent| {
                    extent
                        .left(interner)
                        .ok()
                        .map_or(0, mjx_ooxml_core::measure::Emu::emu)
                }),
                right: effect.as_ref().map_or(0, |extent| {
                    extent
                        .right(interner)
                        .ok()
                        .map_or(0, mjx_ooxml_core::measure::Emu::emu)
                }),
            },
            behind_text: anchor.behind_doc(interner).ok().unwrap_or(false),
            allow_overlap: anchor.allow_overlap(interner).ok().unwrap_or(true),
            layout_in_cell: anchor.layout_in_cell(interner).ok().unwrap_or(true),
            relative_height: anchor.relative_height(interner).ok().unwrap_or(0),
            hidden: anchor.hidden(interner).ok().flatten().unwrap_or(false),
            horizontal,
            vertical,
            wrap: read_wrap(anchor, distance, interner),
        })),
    })
}

/// Which of the five `wp:wrap*` elements the anchor carries, resolved.
///
/// An anchor with no wrap element at all is read as `wp:wrapNone`: the schema requires one, a file
/// that states none has told us nothing about displacement, and *not displacing text* is the reading
/// that cannot move a line that Word would have left alone.
fn read_wrap(
    anchor: &mjx_dml::wordprocessing_drawing::Anchor,
    inherited: DrawingDistances,
    interner: &Interner,
) -> WrapFormatting {
    use mjx_dml::wordprocessing_drawing::Wrap;
    let Some(wrap) = anchor.wrap(interner) else {
        return WrapFormatting::None;
    };
    match wrap {
        Wrap::None(_) => WrapFormatting::None,
        Wrap::Square(square) => WrapFormatting::Square {
            side: square
                .wrap_text(interner)
                .ok()
                .unwrap_or(WrapText::BothSides),
            distance: DrawingDistances {
                top: square
                    .distance_top(interner)
                    .ok()
                    .flatten()
                    .map_or(inherited.top, mjx_ooxml_core::measure::Emu::emu),
                bottom: square
                    .distance_bottom(interner)
                    .ok()
                    .flatten()
                    .map_or(inherited.bottom, mjx_ooxml_core::measure::Emu::emu),
                left: square
                    .distance_left(interner)
                    .ok()
                    .flatten()
                    .map_or(inherited.left, mjx_ooxml_core::measure::Emu::emu),
                right: square
                    .distance_right(interner)
                    .ok()
                    .flatten()
                    .map_or(inherited.right, mjx_ooxml_core::measure::Emu::emu),
            },
        },
        Wrap::Tight(outline) => {
            let (side, polygon, left, right) = read_outline(&outline, inherited, interner);
            WrapFormatting::Tight {
                side,
                polygon,
                distance_left: left,
                distance_right: right,
            }
        }
        Wrap::Through(outline) => {
            let (side, polygon, left, right) = read_outline(&outline, inherited, interner);
            WrapFormatting::Through {
                side,
                polygon,
                distance_left: left,
                distance_right: right,
            }
        }
        Wrap::TopAndBottom(band) => WrapFormatting::TopAndBottom {
            distance_top: band
                .distance_top(interner)
                .ok()
                .flatten()
                .map_or(inherited.top, mjx_ooxml_core::measure::Emu::emu),
            distance_bottom: band
                .distance_bottom(interner)
                .ok()
                .flatten()
                .map_or(inherited.bottom, mjx_ooxml_core::measure::Emu::emu),
        },
    }
}

/// The half of `wp:wrapTight`/`wp:wrapThrough` the two share.
fn read_outline(
    outline: &mjx_dml::wordprocessing_drawing::WrapOutline,
    inherited: DrawingDistances,
    interner: &Interner,
) -> (WrapText, Vec<(i64, i64)>, i64, i64) {
    let mut polygon: Vec<(i64, i64)> = Vec::new();
    if let Some(path) = outline.polygon(interner) {
        if let Some(start) = path.start(interner) {
            polygon.push((start.x.emu(), start.y.emu()));
        }
        for point in path.line_to(interner) {
            polygon.push((point.x.emu(), point.y.emu()));
        }
    }
    (
        outline
            .wrap_text(interner)
            .ok()
            .unwrap_or(WrapText::BothSides),
        polygon,
        outline
            .distance_left(interner)
            .ok()
            .flatten()
            .map_or(inherited.left, mjx_ooxml_core::measure::Emu::emu),
        outline
            .distance_right(interner)
            .ok()
            .flatten()
            .map_or(inherited.right, mjx_ooxml_core::measure::Emu::emu),
    )
}

/// A `ST_TwipsMeasure` wire value in twips, or `None` when it will not parse.
///
/// The union's second arm is a `ST_UniversalMeasure` string — `w:tblGrid/w:gridCol@w="0.5in"` is
/// schema-legal — which is exactly why this goes through `mjx_ooxml_types::support` rather than
/// through `parse::<i64>`.
fn twips_of(measure: &mjx_ooxml_types::shared::TwipsMeasure) -> Option<i64> {
    mjx_ooxml_types::support::universal_measure::twips_from_wire(measure.to_wire())
}

/// The same for `ST_SignedTwipsMeasure`, whose bare arm may be negative.
fn signed_twips_of(measure: &mjx_ooxml_types::wordprocessingml::SignedTwipsMeasure) -> Option<i64> {
    mjx_ooxml_types::support::universal_measure::twips_from_wire(measure.to_wire())
}

/// An optional EMU attribute, in EMU, treating both absence and a value that will not parse as zero.
fn emu_or_zero(
    read: Result<Option<mjx_ooxml_core::measure::Emu>, mjx_ooxml_core::AttributeError>,
) -> i64 {
    read.ok()
        .flatten()
        .map_or(0, mjx_ooxml_core::measure::Emu::emu)
}

/// One `CT_TblWidth`-shaped element, resolved.
///
/// A `w` without a `type` is not readable — [`TableWidth::measure`] is what enforces that — so a
/// malformed one contributes nothing rather than a number in a unit nobody stated.
fn read_width(width: Option<&TableWidth>, interner: &Interner) -> Option<WidthSpecification> {
    let measure = width?.measure(interner).ok()?;
    let raw = measure.value.0.trim().to_owned();
    let value = match measure.unit {
        TableWidthUnit::Twips => {
            mjx_ooxml_types::support::universal_measure::twips_from_wire(&raw)?
        }
        TableWidthUnit::Percent => {
            // `pct` is fiftieths of a percent as a bare number, and Word also writes `"50%"` here;
            // both are legal `ST_MeasurementOrPercent` and both mean the same thing.
            match raw.strip_suffix('%') {
                Some(number) => number.trim().parse::<f64>().ok().map(|percent| {
                    #[allow(clippy::cast_possible_truncation)]
                    {
                        (percent * 50.0).round() as i64
                    }
                })?,
                None => raw.parse::<i64>().ok()?,
            }
        }
        TableWidthUnit::Auto | TableWidthUnit::Nil => 0,
    };
    Some(WidthSpecification {
        unit: measure.unit,
        value,
    })
}

/// `w:tblCellMar` or `w:tcMar`, resolved — a table's four sides.
fn read_table_cell_margins(
    margins: Option<&TableCellMargins>,
    interner: &Interner,
) -> CellMarginsSpecification {
    let Some(margins) = margins else {
        return CellMarginsSpecification::default();
    };
    CellMarginsSpecification {
        top: read_width(margins.top(), interner).and_then(WidthSpecification::twips),
        bottom: read_width(margins.bottom(), interner).and_then(WidthSpecification::twips),
        start: read_width(margins.start().or_else(|| margins.left()), interner)
            .and_then(WidthSpecification::twips),
        end: read_width(margins.end().or_else(|| margins.right()), interner)
            .and_then(WidthSpecification::twips),
    }
}

/// The same for a single cell's `w:tcMar`.
fn read_cell_margins(
    margins: Option<&CellMargins>,
    interner: &Interner,
) -> CellMarginsSpecification {
    let Some(margins) = margins else {
        return CellMarginsSpecification::default();
    };
    CellMarginsSpecification {
        top: read_width(margins.top(), interner).and_then(WidthSpecification::twips),
        bottom: read_width(margins.bottom(), interner).and_then(WidthSpecification::twips),
        start: read_width(margins.start().or_else(|| margins.left()), interner)
            .and_then(WidthSpecification::twips),
        end: read_width(margins.end().or_else(|| margins.right()), interner)
            .and_then(WidthSpecification::twips),
    }
}

/// Which table style, and which of its conditional regions, a cell's paragraph is under.
///
/// Recorded during the direct read and resolved once per distinct `(style, regions)` pair in the one
/// pass that has the [`StyleIndex`] — the same "read once, resolve many" shape every other tier in
/// this module already has. A paragraph outside a table carries `None`, which is the identity.
#[derive(Clone, PartialEq, Eq, Hash)]
struct TableStyleContext {
    style_id: String,
    regions: Vec<super::table_regions::ConditionalFormatRegion>,
}

/// Walks a block-level content list, appending every paragraph it finds to `paragraphs` and
/// returning the block tree that indexes them.
///
/// `assigned` is the index the *next* top-level paragraph already has: a stream's own paragraphs were
/// read into `paragraphs` before this walk (they are what [`Body::paragraphs`] and therefore
/// `sections_in` number), so this walk must hand them their existing indices rather than reading
/// them a second time. A paragraph inside a cell has no such index, so it is read here and appended.
fn walk_blocks(
    content: &[BlockContent],
    theme: &ThemeContext,
    interner: &Interner,
    style: Option<&TableStyleContext>,
    assigned: &mut std::ops::Range<usize>,
    paragraphs: &mut Vec<DirectParagraph>,
) -> Result<Vec<BlockFormatting>, DocxError> {
    let mut blocks = Vec::new();
    for item in content {
        match item {
            BlockContent::Paragraph(paragraph) => {
                let index = match assigned.next() {
                    Some(index) => index,
                    None => {
                        let mut read = read_direct_paragraph(paragraph, theme, interner)?;
                        read.table_style = style.cloned();
                        paragraphs.push(read);
                        paragraphs.len() - 1
                    }
                };
                blocks.push(BlockFormatting::Paragraph(index));
            }
            BlockContent::Table(table) => {
                blocks.push(BlockFormatting::Table(Box::new(read_table(
                    table, theme, interner, paragraphs,
                )?)));
            }
            _ => {}
        }
    }
    Ok(blocks)
}

/// One `w:tbl`, resolved — its grid, its properties, and every row and cell beneath it.
fn read_table(
    table: &super::tables::Table,
    theme: &ThemeContext,
    interner: &Interner,
    paragraphs: &mut Vec<DirectParagraph>,
) -> Result<TableFormatting, DocxError> {
    let properties = table.properties();
    let grid_twips: Vec<i64> = table.grid().map_or_else(Vec::new, |grid| {
        grid.columns()
            .map(|column| {
                column
                    .width(interner)
                    .ok()
                    .flatten()
                    .and_then(|measure| twips_of(&measure))
                    .unwrap_or(0)
            })
            .collect()
    });
    let style_id = properties
        .map(|value| attr(value.style_id(interner)))
        .transpose()?
        .flatten();
    let look = super::table_regions::TableLookFlags::from_look(
        properties.and_then(TableProperties::look),
        interner,
    )
    .map_err(|error| DocxError::from(mjx_ooxml_core::FromXmlError::from(error)))?;
    let row_band_size = properties
        .map(|value| attr(value.effective_row_band_size(interner)))
        .transpose()?
        .unwrap_or(1);
    let column_band_size = properties
        .map(|value| attr(value.effective_column_band_size(interner)))
        .transpose()?
        .unwrap_or(1);

    let row_count = table.row_count();
    let column_count = grid_twips.len().max(table.column_count());
    let mut rows = Vec::with_capacity(row_count);
    for (row_index, row) in table.rows().enumerate() {
        let row_properties = row.properties();
        let mut cells = Vec::new();
        let mut column_index = row_properties
            .and_then(|value| value.grid_before(interner).ok().flatten())
            .map_or(0_usize, |value| usize::try_from(value).unwrap_or(0));
        for cell in row.cells() {
            let cell_properties = cell.properties();
            let span = cell.column_span(interner);
            let regions = style_id.as_ref().map(|id| TableStyleContext {
                style_id: id.clone(),
                regions: super::table_regions::applicable_regions(
                    row_index,
                    column_index,
                    row_count,
                    column_count,
                    look,
                    row_band_size,
                    column_band_size,
                ),
            });
            let mut nothing = 0..0;
            let content = walk_blocks(
                cell.content(),
                theme,
                interner,
                regions.as_ref(),
                &mut nothing,
                paragraphs,
            )?;
            cells.push(CellFormatting {
                grid_span: span,
                vertical_merge_anchor: cell
                    .vertical_merge_kind(interner)
                    .map(|kind| matches!(kind, MergedCellType::Restart)),
                width: read_width(cell_properties.and_then(CellProperties::width), interner),
                margins: read_cell_margins(
                    cell_properties.and_then(CellProperties::margins),
                    interner,
                ),
                vertical_alignment: cell_properties
                    .and_then(CellProperties::vertical_alignment)
                    .map(|value| attr(value.value(interner)))
                    .transpose()?,
                text_direction: cell_properties
                    .and_then(CellProperties::text_direction)
                    .map(|value| attr(value.value(interner)))
                    .transpose()?
                    .flatten(),
                no_wrap: cell_properties
                    .map(|value| attr(value.no_wrap(interner)))
                    .transpose()?
                    .flatten()
                    .unwrap_or(false),
                content,
            });
            column_index += span;
        }
        rows.push(RowFormatting {
            cells,
            cannot_split: row_properties
                .map(|value| attr(value.cant_split(interner)))
                .transpose()?
                .flatten()
                .unwrap_or(false),
            repeat_as_header: row_properties
                .map(|value| attr(value.table_header(interner)))
                .transpose()?
                .flatten()
                .unwrap_or(false),
            height: row_properties
                .and_then(super::table_properties::RowProperties::height)
                .and_then(|height| {
                    let twips = twips_of(&height.height(interner).ok().flatten()?)?;
                    Some(RowHeightSpecification {
                        twips,
                        rule: height
                            .rule(interner)
                            .ok()
                            .flatten()
                            .unwrap_or(HeightRule::AtLeast),
                    })
                }),
            grid_before: row_properties
                .and_then(|value| value.grid_before(interner).ok().flatten())
                .map_or(0, |value| usize::try_from(value).unwrap_or(0)),
            grid_after: row_properties
                .and_then(|value| value.grid_after(interner).ok().flatten())
                .map_or(0, |value| usize::try_from(value).unwrap_or(0)),
            hidden: row_properties
                .map(|value| attr(value.hidden(interner)))
                .transpose()?
                .flatten()
                .unwrap_or(false),
        });
    }

    Ok(TableFormatting {
        grid_twips,
        layout: properties
            .and_then(TableProperties::layout)
            .and_then(|value| value.layout(interner).ok().flatten())
            .unwrap_or(TableLayoutType::Autofit),
        width: read_width(properties.and_then(TableProperties::width), interner),
        indent_twips: read_width(properties.and_then(TableProperties::indent), interner)
            .and_then(WidthSpecification::twips)
            .unwrap_or(0),
        cell_spacing_twips: read_width(
            properties.and_then(TableProperties::cell_spacing),
            interner,
        )
        .and_then(WidthSpecification::twips)
        .unwrap_or(0),
        cell_margins: read_table_cell_margins(
            properties.and_then(TableProperties::cell_margins),
            interner,
        ),
        alignment: properties
            .and_then(TableProperties::justification)
            .map(|value| attr(value.value(interner)))
            .transpose()?,
        floating: properties
            .and_then(TableProperties::floating_position)
            .map(|position| -> Result<FloatingTableAnchoring, DocxError> {
                Ok(FloatingTableAnchoring {
                    horizontal_anchor: attr(position.horizontal_anchor(interner))?,
                    vertical_anchor: attr(position.vertical_anchor(interner))?,
                    x_alignment: attr(position.x_alignment(interner))?,
                    x_twips: attr(position.x(interner))?
                        .as_ref()
                        .and_then(signed_twips_of),
                    y_alignment: attr(position.y_alignment(interner))?,
                    y_twips: attr(position.y(interner))?
                        .as_ref()
                        .and_then(signed_twips_of),
                    left_from_text: attr(position.left_from_text(interner))?
                        .as_ref()
                        .and_then(twips_of)
                        .unwrap_or(0),
                    right_from_text: attr(position.right_from_text(interner))?
                        .as_ref()
                        .and_then(twips_of)
                        .unwrap_or(0),
                    top_from_text: attr(position.top_from_text(interner))?
                        .as_ref()
                        .and_then(twips_of)
                        .unwrap_or(0),
                    bottom_from_text: attr(position.bottom_from_text(interner))?
                        .as_ref()
                        .and_then(twips_of)
                        .unwrap_or(0),
                })
            })
            .transpose()?,
        style_id,
        rows,
    })
}
