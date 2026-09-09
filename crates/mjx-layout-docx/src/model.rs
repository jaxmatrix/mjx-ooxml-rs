//! [`DocumentBoxModel`] — Word's implementation of [`BoxModel`], and [`DocumentFlow`], the document
//! it lays out.
//!
//! # The one that reflows
//!
//! PowerPoint places absolutely: a slide is a page and page *N* is reachable without page *N−1*.
//! Excel's addresses a grid: a band's first row comes from the page number and the row geometry.
//! **Word's pagination is emergent** — where page 200 begins depends on everything on the 199 pages
//! before it — and it is the reason `mjx-layout`'s [`Checkpoint`] exists.
//!
//! So this box model is the first that can be asked a question the other two cannot: *lay out page
//! 200*. It answers it in two very different ways depending on whether it is given the checkpoint
//! that ended page 199:
//!
//! * **with one** — it starts at the paragraph the checkpoint names and looks at the paragraphs on
//!   page 200. That is a handful, whatever 200 is.
//! * **without one** — it has no choice but to assemble every page from the first, because there is
//!   no arithmetic that answers where page 200 starts. It does exactly that, and
//!   [`PageFragments`]'s content is identical either way, which is the equivalence
//!   [`BoxModel::layout_page`] promises.
//!
//! **Those two produce the same page and do wildly different amounts of work**, and that is why the
//! work is counted: [`DocumentBoxModel::paragraphs_visited`] reports what the last call looked at.
//! A gate on the fragments alone is green for an implementation with no checkpoints in it at all.
//!
//! # What one page is, once sections exist
//!
//! MJXOFF-174's page was a rectangle and a column. MJXOFF-175's is five things stacked, in the order
//! they must be resolved because each one's size depends on the ones before it:
//!
//! 1. the **section**, which decides the sheet, the margins and the columns
//!    ([`crate::section`]) — and which is a function of where the page *starts*, so it must be
//!    settled first;
//! 2. the **header and footer**, whose heights push the body's top down and its bottom up
//!    ([`crate::stream`]);
//! 3. the **note area**, whose height is decided together with the body's by a fixed point
//!    ([`crate::notes`], where the whole argument is written out);
//! 4. the **body**, one or more column groups ([`crate::paginate`]);
//! 5. the **generated marks** — line numbers in the margin, which nothing in the document's run
//!    stream contains and nothing else will ever draw.
//!
//! The one that reads backwards is the third, and it is the one this child is really about.

use std::collections::BTreeMap;

use mjx_docx::{
    Document, DocumentFormatting, NoteFormatting, ParagraphFormatting, SectionFormatting,
};
use mjx_layout::{
    BoxFragment, BoxModel, ChangeSet, Checkpoint, Constraints, DirtyPages, Extent, ExtentPrecision,
    Fragment, FragmentTree, FragmentTreeBuilder, GlyphRunFragment, LayoutPoint, LayoutRect,
    LineFragment, ModelSignature, PageFragments, PageIndex, PartId,
};
use mjx_ooxml_core::measure::Emu;
use mjx_ooxml_types::wordprocessingml::{
    EndnotePosition, FootnoteEndnoteType, FootnotePosition, LineNumberRestart, NumberFormat,
    NumberingRestartLocation,
};
use mjx_text::{
    FeatureSet, FontResolver, FontSize, GlyphRasteriser, Hyphenator, ShapingRequest, TextScript,
};

use crate::address;
use crate::checkpoint::Continuation;
use crate::decoration::{DecorationCatalogue, ParagraphDecoration};
use crate::error::DocumentLayoutError;
use crate::flow::{lay_out, FlowContext, ParagraphLayout};
use crate::justify::points;
use crate::notes::{self, DemandedNote, NoteArea, NoteCarry, NoteContent};
use crate::numbering::{format_number, is_numbered};
use crate::paginate::{assemble, FlowPosition, LayoutCache, PageAssembly, PageShape};
use crate::section::{required_parity, starts_a_page, SectionGeometry};
use crate::stream::{lay_out_stream, StreamLayout};
use crate::tabs::leader_character;
use crate::text::TextEngine;

/// Where one paragraph of the flow came from.
///
/// The body is most of it; the rest is endnote content, which is *flowed as ordinary body content*
/// at the end of its scope rather than laid out in an area of its own. That is what an endnote is —
/// §17.11.3's `sectEnd`/`docEnd` are positions in the **flow**, not a second area beside it — and it
/// is the difference between an endnote and a footnote in one sentence.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FlowOrigin {
    /// The body paragraph at this index of `word/document.xml`.
    Body(usize),
    /// A paragraph of the endnote at `note` in `word/endnotes.xml`.
    Endnote {
        /// Which entry.
        note: usize,
        /// Which of its paragraphs.
        paragraph: usize,
    },
}

/// One section, in the flow's own paragraph numbering.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct FlowSection {
    /// Which entry of [`DocumentFormatting::sections`] it is.
    section: usize,
    /// Its first paragraph, in flow indices.
    first: usize,
    /// Its last, inclusive, or `None` when it governs nothing.
    last: Option<usize>,
}

/// A document, read once, ready to lay out.
///
/// Owns a [`DocumentFormatting`] — every part parsed once, every rung of the effective-property
/// ladder already resolved — and nothing borrowed from the [`Document`] it came from. That is what
/// makes it a *snapshot*: an edit to the document does not reach it, and the caller re-reads.
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentFlow {
    formatting: DocumentFormatting,
    program: Vec<ParagraphFormatting>,
    origins: Vec<FlowOrigin>,
    sections: Vec<FlowSection>,
    footnote_prefix: Vec<u32>,
}

impl DocumentFlow {
    /// Reads `document` once.
    ///
    /// # Errors
    /// [`DocumentLayoutError::Document`] when a part cannot be read or a style chain does not
    /// terminate.
    pub fn read(document: &mut Document) -> Result<Self, DocumentLayoutError> {
        Ok(Self::from_formatting(document.formatting()?))
    }

    /// The same from a [`DocumentFormatting`] a caller already holds.
    #[must_use]
    pub fn from_formatting(formatting: DocumentFormatting) -> Self {
        let (program, origins, sections) = build_program(&formatting);
        let footnote_prefix = prefix_of_references(&program, false);
        Self {
            formatting,
            program,
            origins,
            sections,
            footnote_prefix,
        }
    }

    /// What was read.
    #[must_use]
    pub fn formatting(&self) -> &DocumentFormatting {
        &self.formatting
    }

    /// The body's paragraphs, in document order.
    #[must_use]
    pub fn paragraphs(&self) -> &[ParagraphFormatting] {
        self.formatting.paragraphs()
    }

    /// Every paragraph the flow actually lays out: the body's, with each endnote's spliced in at the
    /// end of the scope it belongs to.
    ///
    /// Identical to [`DocumentFlow::paragraphs`] for a document with no endnotes, which is most of
    /// them.
    #[must_use]
    pub fn flowed_paragraphs(&self) -> &[ParagraphFormatting] {
        &self.program
    }

    /// Where the paragraph at flow index `index` came from.
    #[must_use]
    pub fn origin(&self, index: usize) -> Option<FlowOrigin> {
        self.origins.get(index).copied()
    }

    /// How many paragraphs the flow holds — the number the continuation state guards against.
    #[must_use]
    pub fn paragraph_count(&self) -> usize {
        self.program.len()
    }

    /// Which section governs flow index `index`.
    #[must_use]
    fn section_at(&self, index: usize) -> Option<usize> {
        let found = self
            .sections
            .iter()
            .position(|span| index >= span.first && span.last.is_some_and(|last| index <= last));
        found.or_else(|| self.sections.len().checked_sub(1))
    }

    /// The section entry at `index`, if the document has one.
    fn section(&self, index: usize) -> Option<&SectionFormatting> {
        self.sections
            .get(index)
            .and_then(|span| self.formatting.sections().get(span.section))
    }

    /// How many footnote references precede flow index `index`.
    fn footnotes_before(&self, index: usize) -> u32 {
        self.footnote_prefix
            .get(index.min(self.footnote_prefix.len().saturating_sub(1)))
            .copied()
            .unwrap_or(0)
    }
}

/// The flow's paragraphs, where each came from, and the sections in flow indices.
fn build_program(
    formatting: &DocumentFormatting,
) -> (Vec<ParagraphFormatting>, Vec<FlowOrigin>, Vec<FlowSection>) {
    let body = formatting.paragraphs();
    let mut program: Vec<ParagraphFormatting> = Vec::with_capacity(body.len());
    let mut origins: Vec<FlowOrigin> = Vec::with_capacity(body.len());
    let mut sections: Vec<FlowSection> = Vec::new();
    let mut deferred: Vec<usize> = Vec::new();

    let mut placed_notes: Vec<bool> = vec![false; formatting.endnotes().len()];
    for (index, section) in formatting.sections().iter().enumerate() {
        let first = program.len();
        let Some(last_body) = section.last_paragraph else {
            sections.push(FlowSection {
                section: index,
                first,
                last: None,
            });
            continue;
        };
        for body_index in section.first_paragraph..=last_body.min(body.len().saturating_sub(1)) {
            let Some(paragraph) = body.get(body_index) else {
                break;
            };
            program.push(paragraph.clone());
            origins.push(FlowOrigin::Body(body_index));
        }
        // §17.11.3: `sectEnd` puts this section's endnotes here; anything else (including nothing,
        // which is the schema's silence) puts them at the end of the document.
        let at_section_end = section.notes.endnote_position == Some(EndnotePosition::SectionEnd)
            && !section.suppress_endnotes;
        let referenced = endnotes_referenced(formatting, section);
        for note in referenced {
            if placed_notes.get(note).copied().unwrap_or(true) {
                continue;
            }
            if at_section_end {
                splice_note(formatting.endnotes(), note, &mut program, &mut origins);
                if let Some(slot) = placed_notes.get_mut(note) {
                    *slot = true;
                }
            } else {
                deferred.push(note);
            }
        }
        sections.push(FlowSection {
            section: index,
            first,
            last: program.len().checked_sub(1).filter(|last| *last >= first),
        });
    }

    if !deferred.is_empty() {
        let first = program.len();
        for note in deferred {
            if placed_notes.get(note).copied().unwrap_or(true) {
                continue;
            }
            splice_note(formatting.endnotes(), note, &mut program, &mut origins);
            if let Some(slot) = placed_notes.get_mut(note) {
                *slot = true;
            }
        }
        // The document-end endnotes belong to the last section's pages: they are laid out on the
        // sheet the section that precedes them uses, which is what Word does and what a reader
        // expects — the endnote page of a landscape document is landscape.
        if let Some(last) = sections.last_mut() {
            if program.len() > first {
                last.last = Some(program.len() - 1);
            }
        }
    }

    if sections.is_empty() {
        // A body with no `w:sectPr` anywhere. One section governing everything, so the caller's own
        // constraints are what every page is laid out under.
        for (index, paragraph) in body.iter().enumerate() {
            program.push(paragraph.clone());
            origins.push(FlowOrigin::Body(index));
        }
        sections.push(FlowSection {
            section: usize::MAX,
            first: 0,
            last: program.len().checked_sub(1),
        });
    }
    (program, origins, sections)
}

/// Every endnote the paragraphs of `section` refer to, in reference order.
fn endnotes_referenced(formatting: &DocumentFormatting, section: &SectionFormatting) -> Vec<usize> {
    let body = formatting.paragraphs();
    let Some(last) = section.last_paragraph else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for paragraph in body
        .get(section.first_paragraph..=last.min(body.len().saturating_sub(1)))
        .unwrap_or_default()
    {
        for reference in paragraph.note_references() {
            if !reference.endnote {
                continue;
            }
            if let Some(at) = formatting
                .endnotes()
                .iter()
                .position(|note| note.id() == reference.id)
            {
                found.push(at);
            }
        }
    }
    found
}

/// Appends one endnote's paragraphs to the flow.
fn splice_note(
    notes: &[NoteFormatting],
    note: usize,
    program: &mut Vec<ParagraphFormatting>,
    origins: &mut Vec<FlowOrigin>,
) {
    let Some(entry) = notes.get(note) else {
        return;
    };
    for (index, paragraph) in entry.paragraphs().iter().enumerate() {
        program.push(paragraph.clone());
        origins.push(FlowOrigin::Endnote {
            note,
            paragraph: index,
        });
    }
}

/// How many footnote (or endnote) references precede each paragraph, as a prefix sum.
///
/// One pass of the document, and it is what makes a note's *number* free: the *n*th reference in
/// document order is note *n*, so no page needs to know how the pages before it were broken.
fn prefix_of_references(program: &[ParagraphFormatting], endnotes: bool) -> Vec<u32> {
    let mut prefix = Vec::with_capacity(program.len() + 1);
    let mut running = 0_u32;
    prefix.push(0);
    for paragraph in program {
        running = running.saturating_add(
            u32::try_from(
                paragraph
                    .note_references()
                    .iter()
                    .filter(|reference| reference.endnote == endnotes)
                    .count(),
            )
            .unwrap_or(0),
        );
        prefix.push(running);
    }
    prefix
}

/// What a page carries in from the page before it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct PageState {
    position: FlowPosition,
    page_number: i64,
    line_number: i64,
    carry: Option<NoteCarry>,
}

/// What the last laid-out page was, for a caller that needs the numbers a fragment tree cannot
/// carry.
///
/// # Why this exists rather than a richer [`PageFragments`]
///
/// A page number is not a fragment: nothing draws it until a `PAGE` field does, which is MJXOFF-177
/// (R22). Neither is *which* header a page showed, or how many body assemblies the footnote fixed
/// point needed. All three are facts a caller — and a gate — needs, and none of them belongs in a
/// tree of positioned boxes. This is the same instrument [`DocumentBoxModel::paragraphs_visited`]
/// already is, widened.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct PageReport {
    /// The number the page displays, after this section's own restarts.
    pub page_number: i64,
    /// Which entry of [`DocumentFormatting::sections`] governs it.
    pub section: Option<usize>,
    /// Whether the page is a blank one an `evenPage`/`oddPage` break demanded.
    pub blank: bool,
    /// How tall each column group's columns came out, in order — what a balancing assertion reads.
    pub column_heights: Vec<Emu>,
    /// How many sections' content shares this page, which is more than one only after a
    /// `continuous` break.
    pub column_groups: usize,
    /// `w:cols@sep` — whether a vertical rule belongs between this page's columns.
    ///
    /// **Reported and not drawn**, and that is the same decision `w:pBdr/w:between` and a `bar` tab
    /// stop already carry in this crate: a rule is a *paint*, and a box model that produced one
    /// would have merged two stages the architecture separates on purpose. `mjx-scene-docx`
    /// (MJXOFF-255) is what draws it, and until it exists nothing draws anything a Word document
    /// says at all — see the crate documentation.
    pub column_separator: bool,
    /// The header stream this page showed.
    pub header: Option<usize>,
    /// The footer stream.
    pub footer: Option<usize>,
    /// The footnotes on it, as `(entry, number)`.
    pub notes: Vec<(usize, i64)>,
    /// How tall the note area is.
    pub note_area_height: Emu,
    /// The note whose tail is carried to the next page.
    pub carried_note: Option<usize>,
    /// The line numbers printed in the margin, in order.
    pub line_numbers: Vec<i64>,
    /// The body area the page's content flowed into, after the header and footer took their share.
    pub body: LayoutRect,
    /// How many times the body had to be assembled before the note area stopped changing.
    ///
    /// **One or two, always** — see [`crate::notes`] for why it cannot be three.
    pub assemblies: u32,
    /// Whether every number on this page could be written in the numeral system the document asked
    /// for; see [`crate::numbering::format_number`].
    pub numerals_exact: bool,
}

/// Word's box model.
///
/// # It shares a rasteriser with the painter, and that is not an optimisation
///
/// A [`FaceId`](mjx_text::FaceId) is minted by a [`GlyphRasteriser`] and looked up by whatever
/// draws the glyphs. A box model that measured against a rasteriser of its own would issue
/// identifiers a painter's atlas has never heard of, and every glyph would draw from an empty page
/// **with no error anywhere** — a blank document that reports success.
/// [`DocumentBoxModel::rasteriser_mut`] is how a caller hands the same one to both.
#[derive(Debug)]
pub struct DocumentBoxModel {
    fonts: FontResolver,
    rasteriser: GlyphRasteriser,
    shaper: mjx_text::Shaper,
    features: FeatureSet,
    hyphenator: Option<Box<dyn Hyphenator>>,
    last_visited: u32,
    total_visited: u64,
    dirty_from: Option<u32>,
    report: PageReport,
}

impl DocumentBoxModel {
    /// The signature this box model's checkpoints carry.
    ///
    /// A constant, and one that changes when the continuation state changes shape — see
    /// [`crate::checkpoint::VERSION`], which is the finer-grained half of the same guard.
    pub const SIGNATURE: ModelSignature = ModelSignature::new(0x6D6A_785F_646F_6378);

    /// A box model that resolves faces through `fonts`.
    #[must_use]
    pub fn new(fonts: FontResolver) -> Self {
        Self {
            fonts,
            rasteriser: GlyphRasteriser::new(),
            shaper: mjx_text::Shaper::new(),
            features: FeatureSet::default(),
            hyphenator: None,
            last_visited: 0,
            total_visited: 0,
            dirty_from: None,
            report: PageReport::default(),
        }
    }

    /// The same, hyphenating words with `hyphenator` wherever `w:autoHyphenation` is on.
    ///
    /// Without one, a document with `w:autoHyphenation` on hyphenates **only at the soft hyphens its
    /// author wrote**, because the Liang patterns a pattern hyphenator needs are language data and
    /// none is committed to this repository. That is a real limitation and it is reported rather
    /// than hidden: see [`crate::flow`].
    #[must_use]
    pub fn with_hyphenator(mut self, hyphenator: Box<dyn Hyphenator>) -> Self {
        self.hyphenator = Some(hyphenator);
        self
    }

    /// The features every run is shaped with.
    #[must_use]
    pub fn with_features(mut self, features: FeatureSet) -> Self {
        self.features = features;
        self
    }

    /// The rasteriser this box model mints [`FaceId`](mjx_text::FaceId)s from.
    ///
    /// **Hand this to the painter.** See the type's own documentation for what happens otherwise.
    pub fn rasteriser_mut(&mut self) -> &mut GlyphRasteriser {
        &mut self.rasteriser
    }

    /// The font resolver, whose substitution manifest records every face this document did not have.
    pub fn fonts_mut(&mut self) -> &mut FontResolver {
        &mut self.fonts
    }

    /// How many paragraphs the **last** [`BoxModel::layout_page`] call had to look at.
    ///
    /// The instrument the checkpoint gate rests on. Laying out page *N* from *N−1*'s checkpoint
    /// visits the paragraphs on page *N*; laying it out without one visits every paragraph before it
    /// as well. Both produce the same page.
    #[must_use]
    pub fn paragraphs_visited(&self) -> u32 {
        self.last_visited
    }

    /// How many paragraph layouts this box model has performed in its whole life.
    #[must_use]
    pub fn paragraphs_visited_in_total(&self) -> u64 {
        self.total_visited
    }

    /// What the last laid-out page was.
    #[must_use]
    pub fn last_page(&self) -> &PageReport {
        &self.report
    }

    /// The **paragraph** the last [`BoxModel::invalidate`] found the earliest change in, or `None`
    /// when nothing has been invalidated.
    ///
    /// [`BoxModel::invalidate`] answers [`DirtyPages::From`]`(`[`PageIndex::FIRST`]`)` for any edit
    /// at all, and that is not laziness: a flowing document's page boundaries are held by the
    /// **caller's checkpoints** and not by the box model, so the *page* an edit dirties is a
    /// question this type genuinely cannot answer. This is the half it can — and a caller that kept
    /// its checkpoints turns one into the other with a binary search over their positions, which is
    /// exactly why a [`Checkpoint`]'s position is a `SourceRef` in the same space as a fragment's.
    #[must_use]
    pub fn dirty_from_paragraph(&self) -> Option<u32> {
        self.dirty_from
    }

    /// Lays a paragraph out. Every call is one unit of the work the counter reports.
    fn lay_out_paragraph(
        &mut self,
        content: &DocumentFlow,
        index: usize,
        column: LayoutRect,
    ) -> Result<ParagraphLayout, DocumentLayoutError> {
        let Some(paragraph) = content.program.get(index) else {
            return Err(DocumentLayoutError::EmptyContentArea {
                width: column.width().emu(),
                height: column.height().emu(),
            });
        };
        let Self {
            fonts,
            rasteriser,
            shaper,
            features,
            hyphenator,
            ..
        } = self;
        let mut engine = TextEngine {
            fonts,
            rasteriser,
            shaper,
            features,
        };
        Ok(lay_out(
            &mut engine,
            paragraph,
            FlowContext {
                column,
                settings: content.formatting().settings(),
                hyphenator: hyphenator.as_deref(),
            },
        )?)
    }

    /// Lays a secondary stream out — a header, a footer or a note.
    fn lay_out_stream_of(
        &mut self,
        content: &DocumentFlow,
        paragraphs: &[ParagraphFormatting],
        width: Emu,
    ) -> Result<StreamLayout, DocumentLayoutError> {
        let Self {
            fonts,
            rasteriser,
            shaper,
            features,
            hyphenator,
            ..
        } = self;
        let mut engine = TextEngine {
            fonts,
            rasteriser,
            shaper,
            features,
        };
        Ok(lay_out_stream(
            &mut engine,
            paragraphs,
            width,
            content.formatting().settings(),
            hyphenator.as_deref(),
        )?)
    }
}

/// One section's content on one page: its geometry, and the columns it filled.
///
/// A page holds more than one only after a `continuous` section break, which is what "continuous"
/// means — the next section carries on **below** this one on the same sheet, with its own column
/// count. That is why a page is a stack of groups rather than a list of columns.
#[derive(Clone, PartialEq, Eq, Debug)]
struct ColumnGroup {
    section: usize,
    geometry: SectionGeometry,
    assembly: PageAssembly,
    top: Emu,
}

/// What one call to `assemble_body` is asked for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct BodyPlan {
    /// The section the page opens in.
    section: usize,
    /// Where in the flow it starts.
    position: FlowPosition,
    /// The number the page displays, which `w:mirrorMargins` reads.
    page_number: i64,
    /// The text area, after the header and footer took their share.
    body: LayoutRect,
    /// How much of that area the body may use, after the note area's reservation.
    height: Emu,
}

/// What filling one page's body produced.
#[derive(Clone, PartialEq, Eq, Debug)]
struct BodyFill {
    groups: Vec<ColumnGroup>,
    next: Option<FlowPosition>,
    used: Emu,
    last_section: usize,
}

/// One page, laid out but not yet turned into fragments.
struct LaidPage {
    section: usize,
    geometry: SectionGeometry,
    blank: bool,
    header: Option<(usize, StreamLayout)>,
    footer: Option<(usize, StreamLayout)>,
    body: LayoutRect,
    fill: BodyFill,
    layouts: LayoutCache,
    notes: NoteArea,
    note_layouts: BTreeMap<usize, StreamLayout>,
    note_separator: Option<StreamLayout>,
    note_notice: Option<StreamLayout>,
    line_marks: Vec<LineMark>,
    next: Option<PageState>,
    page_number: i64,
    assemblies: u32,
    numerals_exact: bool,
}

/// One printed line number, and the line it belongs beside.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct LineMark {
    paragraph: usize,
    line: usize,
    number: i64,
    distance: Emu,
}

/// Which content stream a fragment belongs to, for addressing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Stream {
    /// `word/document.xml`, addressed `[paragraph, line, segment]`.
    Body,
    /// A header, footer or note, addressed `[container, paragraph, line, segment]`.
    Secondary {
        part: PartId,
        container: usize,
        paragraph: usize,
    },
}

impl DocumentBoxModel {
    /// Lays out the page that starts at `state`.
    #[allow(clippy::too_many_lines)]
    fn lay_out_page(
        &mut self,
        content: &DocumentFlow,
        constraints: &Constraints,
        state: PageState,
    ) -> Result<LaidPage, DocumentLayoutError> {
        let settings = *content.formatting().settings();
        let section = content
            .section_at(state.position.paragraph as usize)
            .unwrap_or(0);
        let first_of_section = content
            .sections
            .get(section)
            .is_some_and(|span| span.first == state.position.paragraph as usize)
            && state.position.line == 0;
        let geometry = SectionGeometry::of(
            content.section(section),
            constraints,
            &settings,
            state.page_number,
        );

        // A blank page an `evenPage` or `oddPage` break demanded. It is a real page — Word prints
        // it, its header and footer are on it — and it is completely invisible to any assertion
        // about which paragraph is on which page, which is why `tests/a_section_changes_the_page.rs`
        // counts pages as well.
        let blank = first_of_section
            && section > 0
            && content.section(section).is_some_and(|entry| {
                required_parity(entry.break_kind)
                    .is_some_and(|parity| !parity.holds_for(state.page_number))
            });

        // The header and the footer, and the body area that is left once they have grown into it.
        //
        // **GUESS:** a header taller than the top margin pushes the body down rather than being
        // clipped or overlapping it. Word does this — the margin grows to fit the header — and the
        // alternative is text printed on top of text. `w:pgMar@header` is measured from the sheet's
        // own top edge (§17.6.11), not from the text margin, which is what makes the comparison
        // below a comparison rather than an addition.
        let width = geometry.body.width();
        let header = self.furniture(content, section, first_of_section, state, true, width)?;
        let footer = self.furniture(content, section, first_of_section, state, false, width)?;
        // A page with **no** header does not have its margin grown: `w:pgMar@header` is where a
        // header would start and not a second top margin, so a section that references none must
        // leave the body exactly where `w:pgMar@top` put it. Reading it the other way puts half an
        // inch of white space at the top of every page of every document that has no header, which
        // is most of them.
        let header_height = header.as_ref().map_or(Emu::ZERO, |(_, laid)| laid.height());
        let footer_height = footer.as_ref().map_or(Emu::ZERO, |(_, laid)| laid.height());
        let body_top = if header_height > Emu::ZERO {
            geometry
                .body
                .top
                .maximum(geometry.header_distance + header_height)
        } else {
            geometry.body.top
        };
        let body_bottom = if footer_height > Emu::ZERO {
            geometry
                .body
                .bottom
                .minimum(geometry.page.height - geometry.footer_distance - footer_height)
        } else {
            geometry.body.bottom
        };
        let body = LayoutRect::from_edges(
            geometry.body.left,
            body_top,
            geometry.body.right,
            body_bottom.maximum(body_top),
        );

        let mut layouts = LayoutCache::new();
        let mut note_layouts: BTreeMap<usize, StreamLayout> = BTreeMap::new();
        if blank {
            return Ok(LaidPage {
                section,
                geometry,
                blank: true,
                header,
                footer,
                body,
                fill: BodyFill {
                    groups: Vec::new(),
                    next: Some(state.position),
                    used: Emu::ZERO,
                    last_section: section,
                },
                layouts,
                notes: NoteArea::empty(),
                note_layouts,
                note_separator: None,
                note_notice: None,
                line_marks: Vec::new(),
                next: Some(PageState {
                    position: state.position,
                    page_number: state.page_number + 1,
                    line_number: state.line_number,
                    carry: state.carry,
                }),
                page_number: state.page_number,
                assemblies: 0,
                numerals_exact: true,
            });
        }

        // --- the note/body fixed point; `crate::notes` carries the whole argument -------------
        let full = body.height();
        let separators = self.note_separators(content, width)?;
        let cap = self.note_cap(content, state.position, body, full)?;
        if let Some(carry) = state.carry {
            self.note_stream(content, carry.note, width, &mut note_layouts)?;
        }
        // *R*₀ is the height already committed to a note carried in from the previous page — see
        // `crate::notes`. Starting from zero would be correct and would cost every such page a
        // second assembly for nothing, because the carry cannot be refused.
        let mut reserve = {
            let view = note_content(&note_layouts, &separators, content, section);
            notes::demanded_height(&view, state.carry, &[]).minimum(cap)
        };
        let mut fill;
        let mut demand;
        let mut assemblies = 0_u32;
        loop {
            assemblies += 1;
            fill = self.assemble_body(
                content,
                constraints,
                BodyPlan {
                    section,
                    position: state.position,
                    page_number: state.page_number,
                    body,
                    height: (full - reserve).maximum(Emu::ZERO),
                },
                &mut layouts,
            )?;
            demand = demanded_notes(content, &fill, section, &layouts);
            for note in &demand {
                self.note_stream(content, note.note, width, &mut note_layouts)?;
            }
            let needed = {
                let view = note_content(&note_layouts, &separators, content, section);
                notes::demanded_height(&view, state.carry, &demand).minimum(cap)
            };
            if needed <= reserve {
                break;
            }
            reserve = needed;
            // **Two assemblies, never three.** `needed` is non-increasing in `reserve`, so the
            // second pass is laid out under a reservation that already covers whatever it can
            // demand. The bound is asserted rather than trusted: `PageReport::assemblies`.
            if assemblies >= 2 {
                break;
            }
        }
        let area = {
            let view = note_content(&note_layouts, &separators, content, section);
            notes::build(&view, state.carry, &demand, reserve)
        };
        let note_separator = area.separator.and_then(|kind| match kind {
            FootnoteEndnoteType::ContinuationSeparator => separators
                .continuation
                .clone()
                .or_else(|| separators.separator.clone()),
            _ => separators.separator.clone(),
        });
        let note_notice = area.continuation_notice.and(separators.notice.clone());

        let (line_marks, line_number_after) =
            line_numbers(content, &fill, &layouts, section, first_of_section, state);
        let numerals_exact = content
            .section(section)
            .is_none_or(|entry| crate::numbering::is_written_exactly(entry.page_numbering.format));
        let next = next_state(content, &fill, &area, state);

        Ok(LaidPage {
            section,
            geometry,
            blank: false,
            header,
            footer,
            body,
            fill,
            layouts,
            notes: area,
            note_layouts,
            note_separator,
            note_notice,
            line_marks,
            next: next.map(|mut carried| {
                carried.line_number = line_number_after;
                carried
            }),
            page_number: state.page_number,
            assemblies,
            numerals_exact,
        })
    }

    /// Fills one page's body: one column group per section that shares the sheet.
    fn assemble_body(
        &mut self,
        content: &DocumentFlow,
        constraints: &Constraints,
        plan: BodyPlan,
        layouts: &mut LayoutCache,
    ) -> Result<BodyFill, DocumentLayoutError> {
        let BodyPlan {
            section,
            position: from,
            page_number,
            body,
            height,
        } = plan;
        let settings = *content.formatting().settings();
        let mut groups: Vec<ColumnGroup> = Vec::new();
        let mut y = Emu::ZERO;
        let mut current = section;
        let mut position = Some(from);

        while let Some(start) = position {
            // A section that governs no paragraph — two `w:sectPr`s with nothing between them — has
            // no content to lay out and must not be allowed to swallow the next section's. It is
            // skipped rather than filled, and skipping advances `current`, so the loop is bounded by
            // the section count however many empty ones a document strings together.
            if content
                .sections
                .get(current)
                .is_some_and(|span| span.last.is_none())
                && content.sections.len() > current + 1
            {
                current += 1;
                continue;
            }
            let geometry = SectionGeometry::of(
                content.section(current),
                constraints,
                &settings,
                page_number,
            );
            let available = height - y;
            if available <= Emu::ZERO && !groups.is_empty() {
                break;
            }
            let widths: Vec<Emu> = geometry.columns.iter().map(|band| band.width()).collect();
            let section_last = content.sections.get(current).and_then(|span| span.last);
            // **Balancing happens at a `continuous` break and nowhere else**, which is the rule the
            // ticket names: a multi-column section that simply runs out of document keeps its short
            // last column, and adding a trailing continuous break is the thing that levels them.
            let balance = following_is_continuous(content, current);
            let shape = PageShape {
                height: available,
                columns: geometry.column_count(),
                widths: &widths,
                width: widths.first().copied().unwrap_or_else(|| body.width()),
                section_last,
                balance,
            };
            let assembly = {
                let mut layout_of = |index: usize, measure: Emu| {
                    self.lay_out_paragraph(
                        content,
                        index,
                        LayoutRect::from_edges(
                            Emu::ZERO,
                            Emu::ZERO,
                            measure,
                            crate::stream::UNBOUNDED_HEIGHT,
                        ),
                    )
                };
                assemble(&content.program, start, shape, layouts, &mut layout_of)?
            };
            let used = assembly.used();
            let next = assembly.next;
            let ended_section = assembly.ended_section;
            let page_geometry = geometry.clone();
            groups.push(ColumnGroup {
                section: current,
                geometry,
                assembly,
                top: y,
            });
            y += used;
            position = next;

            if !ended_section {
                break;
            }
            let Some(following) = content.sections.get(current + 1) else {
                break;
            };
            let entry = content.formatting().sections().get(following.section);
            if entry.is_none_or(|entry| starts_a_page(entry.break_kind)) {
                break;
            }
            // **GUESS:** a `continuous` break whose section changes the sheet or the margins starts a
            // page anyway. Word does this — two page sizes cannot share one sheet — and the
            // alternative is a second section drawn at the first one's geometry, which is a document
            // that looks nothing like the file.
            let next_geometry = SectionGeometry::of(entry, constraints, &settings, page_number);
            if next_geometry.page != page_geometry.page || next_geometry.body != page_geometry.body
            {
                break;
            }
            if y >= height {
                break;
            }
            current += 1;
        }

        Ok(BodyFill {
            groups,
            next: position,
            used: y,
            last_section: current,
        })
    }

    /// The header (`is_header`) or footer this page shows, laid out.
    fn furniture(
        &mut self,
        content: &DocumentFlow,
        section: usize,
        first_of_section: bool,
        state: PageState,
        is_header: bool,
        width: Emu,
    ) -> Result<Option<(usize, StreamLayout)>, DocumentLayoutError> {
        let Some(entry) = content.section(section) else {
            return Ok(None);
        };
        let slots = if is_header {
            entry.headers
        } else {
            entry.footers
        };
        // **GUESS:** "an even page" is a page whose *displayed number* is even. §17.10.1 says only
        // that `w:evenAndOddHeaders` makes even pages take the even header and does not say which
        // number decides; a section that restarts its numbering at 1 half way through a document
        // therefore has its even and odd headers swapped under the other reading, which is a
        // visible difference on every page of that section.
        let even = state.page_number.rem_euclid(2) == 0;
        let Some(stream) = slots.for_page(first_of_section, even) else {
            return Ok(None);
        };
        let Some(paragraphs) = content
            .formatting()
            .header_footer_stream(stream)
            .map(|held| held.paragraphs().to_vec())
        else {
            return Ok(None);
        };
        let laid = self.lay_out_stream_of(content, &paragraphs, width)?;
        Ok(Some((stream, laid)))
    }

    /// Word's three pieces of note furniture, laid out: `w:separator`, `w:continuationSeparator`
    /// and `w:continuationNotice`.
    fn note_separators(
        &mut self,
        content: &DocumentFlow,
        width: Emu,
    ) -> Result<NoteFurniture, DocumentLayoutError> {
        let separator = content
            .formatting()
            .footnote_of_kind(FootnoteEndnoteType::Separator)
            .map(|note| note.paragraphs().to_vec());
        let continuation = content
            .formatting()
            .footnote_of_kind(FootnoteEndnoteType::ContinuationSeparator)
            .map(|note| note.paragraphs().to_vec());
        let notice = content
            .formatting()
            .footnote_of_kind(FootnoteEndnoteType::ContinuationNotice)
            .map(|note| note.paragraphs().to_vec());
        let separator = match separator {
            Some(paragraphs) => Some(self.lay_out_stream_of(content, &paragraphs, width)?),
            None => None,
        };
        let continuation = match continuation {
            Some(paragraphs) => Some(self.lay_out_stream_of(content, &paragraphs, width)?),
            None => None,
        };
        let notice = match notice {
            Some(paragraphs) => Some(self.lay_out_stream_of(content, &paragraphs, width)?),
            None => None,
        };
        Ok(NoteFurniture {
            separator,
            continuation,
            notice,
        })
    }

    /// The most a page's note area may take.
    ///
    /// The body's first line, and never more: a note taller than the page would otherwise reserve
    /// the whole sheet, the body would place nothing, and the page after this one would start where
    /// this one did — for ever. See [`crate::notes`].
    fn note_cap(
        &mut self,
        content: &DocumentFlow,
        position: FlowPosition,
        body: LayoutRect,
        full: Emu,
    ) -> Result<Emu, DocumentLayoutError> {
        let index = position.paragraph as usize;
        if index >= content.program.len() {
            // No body content is left at all, so there is no first line to protect and the notes may
            // have the sheet. This is the case that lets a document whose last footnote is taller
            // than a page finish at all.
            return Ok(full);
        }
        let layout = self.lay_out_paragraph(content, index, body)?;
        let first = layout
            .lines
            .get(position.line as usize)
            .or_else(|| layout.lines.first())
            .map_or(Emu::ZERO, |line| line.height);
        Ok((full - first).maximum(Emu::ZERO))
    }

    /// One note's own layout, memoised for this page.
    fn note_stream(
        &mut self,
        content: &DocumentFlow,
        note: usize,
        width: Emu,
        into: &mut BTreeMap<usize, StreamLayout>,
    ) -> Result<(), DocumentLayoutError> {
        if into.contains_key(&note) {
            return Ok(());
        }
        let paragraphs = content
            .formatting()
            .footnotes()
            .get(note)
            .map(|entry| entry.paragraphs().to_vec())
            .unwrap_or_default();
        let laid = self.lay_out_stream_of(content, &paragraphs, width)?;
        into.insert(note, laid);
        Ok(())
    }
}

/// Whether the section after `section` continues on the same page.
fn following_is_continuous(content: &DocumentFlow, section: usize) -> bool {
    content
        .sections
        .get(section + 1)
        .and_then(|span| content.formatting().sections().get(span.section))
        .is_some_and(|entry| !starts_a_page(entry.break_kind))
}

/// Word's own note furniture, laid out once per page.
#[derive(Debug, Default)]
struct NoteFurniture {
    separator: Option<StreamLayout>,
    continuation: Option<StreamLayout>,
    notice: Option<StreamLayout>,
}

/// The note layouts and furniture, in the shape [`crate::notes`] reads them.
fn note_content<'a>(
    layouts: &'a BTreeMap<usize, StreamLayout>,
    furniture: &'a NoteFurniture,
    content: &DocumentFlow,
    section: usize,
) -> NoteContent<'a> {
    NoteContent {
        layouts,
        separator: furniture.separator.as_ref(),
        continuation_separator: furniture.continuation.as_ref(),
        continuation_notice: furniture.notice.as_ref(),
        position: content
            .section(section)
            .and_then(|entry| entry.notes.footnote_position)
            .unwrap_or(FootnotePosition::PageBottom),
    }
}

/// Which footnotes the content on this page refers to, and what each is numbered.
fn demanded_notes(
    content: &DocumentFlow,
    fill: &BodyFill,
    section: usize,
    layouts: &LayoutCache,
) -> Vec<DemandedNote> {
    let Some(entry) = content.section(section) else {
        return Vec::new();
    };
    let rules = entry.notes;
    let section_first = content.sections.get(section).map_or(0, |span| span.first);
    let mut found: Vec<DemandedNote> = Vec::new();
    let mut on_page = 0_i64;
    for group in &fill.groups {
        for (column, block) in group.assembly.blocks() {
            let Some(paragraph) = content.program.get(block.paragraph) else {
                continue;
            };
            let width = group.geometry.column(column).width();
            let Some(layout) = layouts.get(&(block.paragraph, width.emu())) else {
                continue;
            };
            let Some(span) = byte_span(layout, block.lines.clone()) else {
                continue;
            };
            // A reference at the very end of a paragraph sits at `text.len()`, which is one past the
            // last line's own range — so the last line of a paragraph owns its closing edge and every
            // other line does not. Reading the range as half-open everywhere loses the reference of
            // every paragraph that ends with one, which is where an author actually puts them.
            let closes_paragraph = block.lines.end == layout.lines.len();
            let mut within = 0_i64;
            for reference in paragraph.note_references() {
                if reference.endnote {
                    continue;
                }
                within += 1;
                let inside = reference.at >= span.start
                    && (reference.at < span.end || (closes_paragraph && reference.at == span.end));
                if !inside {
                    continue;
                }
                let Some(at) = content
                    .formatting()
                    .footnotes()
                    .iter()
                    .position(|note| note.id() == reference.id)
                else {
                    continue;
                };
                if content
                    .formatting()
                    .footnotes()
                    .get(at)
                    .is_none_or(|note| !note.is_user_visible())
                {
                    continue;
                }
                on_page += 1;
                let before = i64::from(content.footnotes_before(block.paragraph));
                let ordinal = match rules.footnote_restart {
                    NumberingRestartLocation::EachPage => on_page,
                    NumberingRestartLocation::EachSection => {
                        before - i64::from(content.footnotes_before(section_first)) + within
                    }
                    NumberingRestartLocation::Continuous => before + within,
                };
                found.push(DemandedNote {
                    note: at,
                    number: rules.footnote_start + ordinal - 1,
                });
            }
        }
    }
    found
}

/// The bytes of a paragraph's text that lines `lines` cover.
fn byte_span(
    layout: &ParagraphLayout,
    lines: std::ops::Range<usize>,
) -> Option<std::ops::Range<usize>> {
    let first = layout.lines.get(lines.start)?;
    let last = layout.lines.get(lines.end.checked_sub(1)?)?;
    Some(first.range.start..last.range.end)
}

/// The line numbers printed beside this page's body, and the count the next page opens with.
///
/// A paragraph carrying `w:suppressLineNumbers` is **skipped and does not advance the count** — the
/// numbered line after it carries the number it would have carried had the suppressed one not
/// existed, which is what "suppress" means and is the difference between skipping a line and hiding
/// its number.
fn line_numbers(
    content: &DocumentFlow,
    fill: &BodyFill,
    layouts: &LayoutCache,
    section: usize,
    first_of_section: bool,
    state: PageState,
) -> (Vec<LineMark>, i64) {
    let Some(entry) = content.section(section) else {
        return (Vec::new(), state.line_number);
    };
    let Some(rules) = entry.line_numbering else {
        return (Vec::new(), state.line_number);
    };
    let mut counter = match rules.restart {
        LineNumberRestart::NewPage => rules.start,
        LineNumberRestart::NewSection => {
            if first_of_section {
                rules.start
            } else {
                state.line_number
            }
        }
        LineNumberRestart::Continuous => state.line_number,
    };
    // **GUESS:** a section with no `w:distance` puts its numbers a quarter of an inch from the text.
    // §17.6.10 calls the attribute the "Distance Between Text and Line Numbering" and says an absent
    // value means the numbers are placed automatically, without saying where automatic is.
    let distance = rules
        .distance_twips
        .map_or(DEFAULT_LINE_NUMBER_DISTANCE, Emu::from_twips);
    let mut marks = Vec::new();
    for group in &fill.groups {
        for (column, block) in group.assembly.blocks() {
            let width = group.geometry.column(column).width();
            let Some(layout) = layouts.get(&(block.paragraph, width.emu())) else {
                continue;
            };
            if layout.style.suppress_line_numbers {
                continue;
            }
            for line in block.lines.clone() {
                if is_numbered(counter, rules.count_by, rules.start) {
                    marks.push(LineMark {
                        paragraph: block.paragraph,
                        line,
                        number: counter,
                        distance,
                    });
                }
                counter += 1;
            }
        }
    }
    (marks, counter)
}

/// Where the page after this one starts, or `None` when the document ended here.
fn next_state(
    content: &DocumentFlow,
    fill: &BodyFill,
    area: &NoteArea,
    state: PageState,
) -> Option<PageState> {
    let position = match fill.next {
        Some(position) => position,
        None => {
            area.carry?;
            // The body is finished and a note is not. The page after this one is note-only, which is
            // a page Word prints too: a footnote longer than the space its own page had left.
            FlowPosition::at(content.program.len())
        }
    };
    let next_section = content.section_at(position.paragraph as usize);
    let restart = next_section
        .filter(|index| *index != fill.last_section)
        .and_then(|index| content.section(index))
        .and_then(|entry| entry.page_numbering.start);
    Some(PageState {
        position,
        page_number: restart.unwrap_or(state.page_number + 1),
        line_number: state.line_number,
        carry: area.carry,
    })
}

/// How far a line number sits from the text when the section states no `w:distance`.
pub const DEFAULT_LINE_NUMBER_DISTANCE: Emu =
    Emu::from_emu(360 * mjx_ooxml_core::measure::EMU_PER_TWIP);

impl PageState {
    /// The state page one starts in.
    fn start(content: &DocumentFlow) -> Self {
        let first = content.section(0);
        Self {
            position: FlowPosition::START,
            page_number: first
                .and_then(|entry| entry.page_numbering.start)
                .unwrap_or(1),
            line_number: first
                .and_then(|entry| entry.line_numbering)
                .map_or(1, |rules| rules.start),
            carry: None,
        }
    }
}

impl BoxModel for DocumentBoxModel {
    type Content = DocumentFlow;
    type Error = DocumentLayoutError;

    fn signature(&self) -> ModelSignature {
        Self::SIGNATURE
    }

    fn layout_page(
        &mut self,
        content: &Self::Content,
        page: PageIndex,
        constraints: &Constraints,
        resume: Option<&Checkpoint>,
    ) -> Result<PageFragments, Self::Error> {
        if constraints.content.width() <= Emu::ZERO || constraints.content.height() <= Emu::ZERO {
            return Err(DocumentLayoutError::EmptyContentArea {
                width: constraints.content.width().emu(),
                height: constraints.content.height().emu(),
            });
        }
        let paragraphs = u32::try_from(content.paragraph_count()).unwrap_or(u32::MAX);
        let mut visited = 0_u32;

        let start = match resume {
            Some(checkpoint) => {
                let state = checkpoint.state_for(Self::SIGNATURE, page)?;
                let carried = Continuation::decode(state, paragraphs)?;
                PageState {
                    position: carried.position,
                    page_number: carried.page_number,
                    line_number: carried.line_number,
                    carry: carried.carry,
                }
            }
            None if page == PageIndex::FIRST => PageState::start(content),
            // No checkpoint and not the first page: there is no arithmetic that answers where this
            // page starts, so the pages before it are assembled and thrown away. The counter says
            // so.
            None => self.walk_to(content, page, constraints, &mut visited)?,
        };

        let laid = self.lay_out_page(content, constraints, start)?;
        visited = visited.saturating_add(page_work(&laid));
        self.last_visited = visited;
        self.total_visited = self.total_visited.saturating_add(u64::from(visited));

        let continuation = match laid.next {
            Some(next) => Some(
                Continuation {
                    position: next.position,
                    paragraphs,
                    page_number: next.page_number,
                    line_number: next.line_number,
                    carry: next.carry,
                }
                .into_checkpoint(Self::SIGNATURE, page)?,
            ),
            None => None,
        };
        self.report = report_of(content, &laid);
        let tree = self.build(content, &laid)?;
        Ok(PageFragments::new(page, tree, continuation))
    }

    fn estimate_extent(&self, content: &Self::Content, constraints: &Constraints) -> Extent {
        // **An estimate, and the first one in this workspace that genuinely is one.** PowerPoint
        // counts slides and Excel divides a measured height; a flowing document's page count is not
        // knowable without laying it out, and laying it out is exactly what a scrollbar cannot
        // afford to wait for.
        //
        // So: characters per page, from the page's area and an assumed average glyph. It is wrong by
        // a few per cent on ordinary prose and by a great deal on a document of headings — which is
        // what `ExtentPrecision::Estimated` is for, and why a caller must let the scrollbar move
        // under the reader as real pages arrive.
        let column = constraints.column(0).unwrap_or(constraints.content);
        let characters: usize = content
            .flowed_paragraphs()
            .iter()
            .map(|paragraph| paragraph.text().chars().count().max(1))
            .sum();
        let per_page = characters_per_page(column);
        #[allow(clippy::cast_possible_truncation)]
        let pages = characters.div_ceil(per_page.max(1)) as u32;
        Extent {
            pages: pages.max(1),
            page_size: constraints.page,
            precision: ExtentPrecision::Estimated,
        }
    }

    fn invalidate(&mut self, change: &ChangeSet) -> DirtyPages {
        if change.is_empty() {
            return DirtyPages::None;
        }
        // **This is what flow means.** A reformatted run cannot move anything before it, and neither
        // can an insertion — but *everything after it* can move, because the text after the change
        // reflows through it. So the answer is always a suffix, and naming individual pages here
        // would leave a reader looking at content laid out at the old offsets.
        let Some(earliest) = change.earliest() else {
            return DirtyPages::None;
        };
        let Some(&paragraph) = earliest.path().segments().first() else {
            // The document itself changed — a section's page size, a margin, a font substitution.
            // Every page moves.
            self.dirty_from = Some(0);
            return DirtyPages::All;
        };
        // A header, a footer or a note is not addressed in the body's own numbering, and an edit to
        // one moves **every** page rather than a suffix: a header is on all of them.
        if earliest.part() != address::BODY {
            self.dirty_from = Some(0);
            return DirtyPages::All;
        }
        self.dirty_from = Some(paragraph);
        // Which *page* that paragraph is on is a question about **checkpoints**, which the caller
        // holds and this box model does not. `From(FIRST)` is the honest answer from here — it says
        // "a suffix" and refuses to guess where it starts — and
        // [`DocumentBoxModel::dirty_from_paragraph`] is how a caller narrows it: a binary search
        // over the checkpoint positions it kept, which is the whole reason a checkpoint's position
        // is a `SourceRef` in the same space as a fragment's.
        DirtyPages::From(PageIndex::FIRST)
    }
}

/// How much work one laid-out page's body cost.
fn page_work(laid: &LaidPage) -> u32 {
    laid.fill.groups.iter().fold(0_u32, |total, group| {
        total.saturating_add(group.assembly.paragraphs_visited)
    })
}

/// What the last page was, for [`DocumentBoxModel::last_page`].
fn report_of(content: &DocumentFlow, laid: &LaidPage) -> PageReport {
    let mut heights = Vec::new();
    for group in &laid.fill.groups {
        for column in &group.assembly.columns {
            heights.push(column.used);
        }
    }
    PageReport {
        page_number: laid.page_number,
        section: content
            .sections
            .get(laid.section)
            .map(|span| span.section)
            .filter(|index| *index != usize::MAX),
        blank: laid.blank,
        column_heights: heights,
        column_groups: laid.fill.groups.len(),
        column_separator: laid
            .fill
            .groups
            .iter()
            .any(|group| group.geometry.column_separator),
        header: laid.header.as_ref().map(|(stream, _)| *stream),
        footer: laid.footer.as_ref().map(|(stream, _)| *stream),
        notes: laid
            .notes
            .notes
            .iter()
            .map(|note| (note.note, note.number))
            .collect(),
        note_area_height: laid.notes.height,
        carried_note: laid.notes.carry.map(|carry| carry.note),
        line_numbers: laid.line_marks.iter().map(|mark| mark.number).collect(),
        body: laid.body,
        assemblies: laid.assemblies,
        numerals_exact: laid.numerals_exact,
    }
}

impl DocumentBoxModel {
    /// Walks pages from the first until `page`, returning the state that page starts in.
    ///
    /// The expensive road, taken only when the caller has no checkpoint. It is not a fallback and it
    /// is not a defect: page 200's start is not computable and somebody has to compute it.
    fn walk_to(
        &mut self,
        content: &DocumentFlow,
        page: PageIndex,
        constraints: &Constraints,
        visited: &mut u32,
    ) -> Result<PageState, DocumentLayoutError> {
        let mut state = PageState::start(content);
        for _ in 0..page.number() {
            let laid = self.lay_out_page(content, constraints, state)?;
            *visited = visited.saturating_add(page_work(&laid));
            match laid.next {
                Some(next) => state = next,
                None => {
                    return Err(mjx_layout::LayoutError::PageBeyondContent {
                        requested: page,
                        last: PageIndex::new(page.number().saturating_sub(1)),
                    }
                    .into())
                }
            }
        }
        Ok(state)
    }
}

/// How many characters a column of this size holds.
///
/// **GUESS**, and openly one: an average glyph is half its point size wide and a line is 1.2 times
/// its point size tall, at an assumed 11-point body. Those are the ratios of ordinary Latin prose in
/// a serif face; a document of headings, of CJK (where a glyph is a full em) or of tables will be
/// estimated badly. Nothing depends on the number being right — [`ExtentPrecision::Estimated`] is
/// the contract — and it is stated here rather than buried so that a caller reading it knows what
/// kind of number it is.
fn characters_per_page(column: LayoutRect) -> usize {
    const ASSUMED_POINT_SIZE: f64 = 11.0;
    const AVERAGE_GLYPH_WIDTH_IN_EMS: f64 = 0.5;
    const LINE_HEIGHT_IN_EMS: f64 = 1.2;
    let glyph = Emu::from_points(ASSUMED_POINT_SIZE * AVERAGE_GLYPH_WIDTH_IN_EMS);
    let line = Emu::from_points(ASSUMED_POINT_SIZE * LINE_HEIGHT_IN_EMS);
    if glyph <= Emu::ZERO || line <= Emu::ZERO {
        return 1;
    }
    let per_line = (column.width().emu() / glyph.emu()).max(1);
    let lines = (column.height().emu() / line.emu()).max(1);
    usize::try_from(per_line.saturating_mul(lines)).unwrap_or(usize::MAX)
}

impl DocumentBoxModel {
    /// Turns a laid-out page into fragments, in reading order: header, body, notes, footer.
    fn build(
        &mut self,
        content: &DocumentFlow,
        laid: &LaidPage,
    ) -> Result<FragmentTree, DocumentLayoutError> {
        let mut builder = FragmentTreeBuilder::new();
        let mut catalogue = DecorationCatalogue::new();
        let page = builder.push_simple(
            None,
            address::root(),
            laid.body,
            Fragment::Box(BoxFragment {
                decoration: None,
                cell: None,
            }),
        );

        if let Some((stream, layout)) = &laid.header {
            let top = laid.geometry.header_distance;
            self.emit_stream(
                &mut builder,
                page,
                address::HEADER,
                *stream,
                layout,
                0..layout.line_count(),
                laid.geometry.body.left,
                top,
                laid.geometry.body.width(),
            )?;
        }

        self.emit_body(&mut builder, &mut catalogue, page, content, laid)?;
        self.emit_notes(&mut builder, page, laid)?;

        if let Some((stream, layout)) = &laid.footer {
            let bottom = laid.geometry.page.height - laid.geometry.footer_distance;
            self.emit_stream(
                &mut builder,
                page,
                address::FOOTER,
                *stream,
                layout,
                0..layout.line_count(),
                laid.geometry.body.left,
                bottom - layout.height(),
                laid.geometry.body.width(),
            )?;
        }
        Ok(builder.finish())
    }

    /// The body's column groups, column by column.
    fn emit_body(
        &mut self,
        builder: &mut FragmentTreeBuilder,
        catalogue: &mut DecorationCatalogue,
        page: Option<mjx_layout::FragmentId>,
        content: &DocumentFlow,
        laid: &LaidPage,
    ) -> Result<(), DocumentLayoutError> {
        let available = laid.body.height() - laid.notes.height;
        for group in &laid.fill.groups {
            let slack = (available - laid.fill.used).maximum(Emu::ZERO);
            let offset = vertical_offset(content, group.section, slack);
            for (index, column) in group.assembly.columns.iter().enumerate() {
                let band = group.geometry.column(index);
                let width = band.width();
                let column_top = laid.body.top + group.top + offset;
                for block in &column.blocks {
                    let Some(layout) = laid.layouts.get(&(block.paragraph, width.emu())) else {
                        continue;
                    };
                    let Some(paragraph) = content.program.get(block.paragraph) else {
                        continue;
                    };
                    let top = column_top + block.top;
                    let height = layout.height_of(block.lines.clone());
                    let rect = LayoutRect::from_edges(band.left, top, band.right, top + height);
                    let decoration =
                        catalogue.intern(ParagraphDecoration::of(paragraph.properties()));
                    let stream = match content.origin(block.paragraph) {
                        Some(FlowOrigin::Endnote { note, paragraph }) => Stream::Secondary {
                            part: address::ENDNOTES,
                            container: note,
                            paragraph,
                        },
                        _ => Stream::Body,
                    };
                    let body = builder.push_simple(
                        page,
                        stream.paragraph(block.paragraph),
                        rect,
                        Fragment::Box(BoxFragment {
                            decoration,
                            cell: None,
                        }),
                    );
                    self.emit_lines(
                        builder,
                        body,
                        stream,
                        block.paragraph,
                        layout,
                        block.lines.clone(),
                        band.left,
                        top,
                        width,
                        Some((band.left, &laid.line_marks)),
                    )?;
                }
            }
        }
        Ok(())
    }

    /// The page's footnote area, with Word's own rule above it.
    fn emit_notes(
        &mut self,
        builder: &mut FragmentTreeBuilder,
        page: Option<mjx_layout::FragmentId>,
        laid: &LaidPage,
    ) -> Result<(), DocumentLayoutError> {
        if laid.notes.is_empty() {
            return Ok(());
        }
        // **GUESS:** `w:pos="pageBottom"` puts the area flush with the foot of the text area and
        // `"beneathText"` puts it directly under the last body line, with no gap in either case.
        // §17.11.16 names the four values and says nothing about the space either leaves, which is
        // the number a reader would notice; `sectEnd` and `docEnd` are `ST_FtnPos` members Word does
        // not offer for footnotes and are read here as `pageBottom`.
        let width = laid.body.width();
        let position = laid.notes.position;
        let top = if position == FootnotePosition::BeneathText {
            (laid.body.top + laid.fill.used).minimum(laid.body.bottom - laid.notes.height)
        } else {
            laid.body.bottom - laid.notes.height
        };
        let area = builder.push_simple(
            page,
            address::notes_root(address::FOOTNOTES),
            LayoutRect::from_edges(laid.body.left, top, laid.body.right, laid.body.bottom),
            Fragment::Box(BoxFragment {
                decoration: None,
                cell: None,
            }),
        );
        if let Some(separator) = &laid.note_separator {
            self.emit_stream(
                builder,
                area,
                address::FOOTNOTES,
                SEPARATOR_CONTAINER,
                separator,
                0..separator.line_count(),
                laid.body.left,
                top,
                width,
            )?;
        }
        for note in &laid.notes.notes {
            let Some(layout) = laid.note_layouts.get(&note.note) else {
                continue;
            };
            self.emit_stream(
                builder,
                area,
                address::FOOTNOTES,
                note.note,
                layout,
                note.lines.clone(),
                laid.body.left,
                top + note.top,
                width,
            )?;
        }
        if let (Some(at), Some(notice)) = (laid.notes.continuation_notice, &laid.note_notice) {
            self.emit_stream(
                builder,
                area,
                address::FOOTNOTES,
                NOTICE_CONTAINER,
                notice,
                0..notice.line_count(),
                laid.body.left,
                top + at,
                width,
            )?;
        }
        Ok(())
    }

    /// One secondary stream's lines, from `at`.
    #[allow(clippy::too_many_arguments)]
    fn emit_stream(
        &mut self,
        builder: &mut FragmentTreeBuilder,
        parent: Option<mjx_layout::FragmentId>,
        part: PartId,
        container: usize,
        layout: &StreamLayout,
        lines: std::ops::Range<usize>,
        left: Emu,
        top: Emu,
        width: Emu,
    ) -> Result<(), DocumentLayoutError> {
        let mut y = top;
        let mut index = lines.start;
        while index < lines.end {
            let Some(line) = layout.lines.get(index) else {
                break;
            };
            // Every line of one paragraph, together, so a paragraph is one box even when the stream
            // is split across pages.
            let paragraph = line.paragraph;
            let mut end = index;
            while end < lines.end
                && layout
                    .lines
                    .get(end)
                    .is_some_and(|line| line.paragraph == paragraph)
            {
                end += 1;
            }
            let Some(laid) = layout.paragraphs.get(paragraph) else {
                break;
            };
            let space = if index == lines.start {
                Emu::ZERO
            } else {
                line.space_before
            };
            y += space;
            let first = layout.lines.get(index).map_or(0, |line| line.line);
            let last = layout.lines.get(end - 1).map_or(0, |line| line.line) + 1;
            let height = laid.height_of(first..last);
            let stream = Stream::Secondary {
                part,
                container,
                paragraph,
            };
            let node = builder.push_simple(
                parent,
                stream.paragraph(paragraph),
                LayoutRect::from_edges(left, y, left + width, y + height),
                Fragment::Box(BoxFragment {
                    decoration: None,
                    cell: None,
                }),
            );
            self.emit_lines(
                builder,
                node,
                stream,
                paragraph,
                laid,
                first..last,
                left,
                y,
                width,
                None,
            )?;
            y += height;
            index = end;
        }
        Ok(())
    }

    /// The lines of one laid-out paragraph, wherever it is.
    #[allow(clippy::too_many_arguments)]
    fn emit_lines(
        &mut self,
        builder: &mut FragmentTreeBuilder,
        parent: Option<mjx_layout::FragmentId>,
        stream: Stream,
        paragraph: usize,
        layout: &ParagraphLayout,
        lines: std::ops::Range<usize>,
        left: Emu,
        top: Emu,
        width: Emu,
        marks: Option<(Emu, &[LineMark])>,
    ) -> Result<(), DocumentLayoutError> {
        let mut y = top;
        for line_index in lines {
            let Some(line) = layout.lines.get(line_index) else {
                continue;
            };
            let line_rect = LayoutRect::from_edges(left, y, left + width, y + line.height);
            let baseline_y = y + line.baseline;
            let node = builder.push_simple(
                parent,
                stream.line(paragraph, line_index, line.range.clone()),
                line_rect,
                Fragment::Line(LineFragment {
                    baseline: line.baseline,
                    ascent: line.ascent,
                    descent: line.descent,
                    base_direction: layout.style.direction,
                    hanging_width: points(line.composed.hanging_width_in_points),
                }),
            );

            for placed in &line.placement.segments {
                let Some(segment) = line.composed.segments.get(placed.segment) else {
                    continue;
                };
                if line.is_tab.get(placed.segment).copied().unwrap_or(false) {
                    // A tab is placed and draws nothing. Its leader, if it has one, is drawn below
                    // from the placement's own record.
                    continue;
                }
                let origin = LayoutPoint::new(left + placed.x, baseline_y);
                let rect =
                    LayoutRect::from_edges(origin.x, y, origin.x + placed.width, y + line.height);
                builder.push_simple(
                    node,
                    stream.segment(paragraph, line_index, placed.segment, segment.range.clone()),
                    rect,
                    Fragment::GlyphRun(GlyphRunFragment {
                        face: segment.face,
                        run: segment.run.clone(),
                        origin,
                        direction: segment.direction,
                        level: segment.level,
                    }),
                );
            }

            let at = LineAddress {
                stream,
                paragraph,
                line: line_index,
                top: y,
                baseline: baseline_y,
            };
            self.draw_leaders(builder, node, line, left, at, layout)?;
            self.draw_hyphen(builder, node, line, left, at, layout);
            if let Some((column_left, marks)) = marks {
                if let Some(mark) = marks
                    .iter()
                    .find(|mark| mark.paragraph == paragraph && mark.line == line_index)
                {
                    self.draw_line_number(builder, node, line, column_left, at, *mark)?;
                }
            }
            y += line.height;
        }
        Ok(())
    }

    /// Fills a tab's gap with its leader character.
    ///
    /// One glyph run rather than one per character: the leader is shaped as a *string* of repeats,
    /// so a dotted leader across three inches is one fragment and not sixty.
    fn draw_leaders(
        &mut self,
        builder: &mut FragmentTreeBuilder,
        parent: Option<mjx_layout::FragmentId>,
        line: &crate::flow::LaidOutLine,
        left: Emu,
        at: LineAddress,
        layout: &ParagraphLayout,
    ) -> Result<(), DocumentLayoutError> {
        for leader in &line.placement.leaders {
            let Some(character) = leader_character(leader.leader) else {
                continue;
            };
            let Some((face_id, size, face)) = self.leader_face(line) else {
                continue;
            };
            let one = {
                let text = character.to_string();
                let request = ShapingRequest::new(&text, TextScript::COMMON, size, &self.features);
                self.shaper.shape(&face, &request)?
            };
            let unit = points(one.advance_in_points());
            if unit <= Emu::ZERO {
                continue;
            }
            let span = leader.to - leader.from;
            let count = usize::try_from(span.emu() / unit.emu()).unwrap_or(0);
            if count == 0 {
                continue;
            }
            // A leader is furniture: bounded so that a tab stop at the far edge of a wide page
            // cannot turn one gap into ten thousand glyphs.
            let count = count.min(MAXIMUM_LEADER_GLYPHS);
            let text: String = std::iter::repeat_n(character, count).collect();
            let request = ShapingRequest::new(&text, TextScript::COMMON, size, &self.features);
            let run = self.shaper.shape(&face, &request)?;
            let origin = LayoutPoint::new(left + leader.from, at.baseline);
            builder.push_simple(
                parent,
                at.stream.line(at.paragraph, at.line, line.range.clone()),
                LayoutRect::from_edges(origin.x, at.top, left + leader.to, at.top + line.height),
                Fragment::GlyphRun(GlyphRunFragment {
                    face: face_id,
                    run,
                    origin,
                    direction: layout.style.direction,
                    level: mjx_text::BidiLevel::LEFT_TO_RIGHT,
                }),
            );
        }
        Ok(())
    }

    /// The face a leader, a hyphen and a line number are drawn in: the paragraph's own, through the
    /// first run it resolved.
    fn leader_face(
        &mut self,
        line: &crate::flow::LaidOutLine,
    ) -> Option<(
        mjx_text::FaceId,
        FontSize,
        std::sync::Arc<mjx_text::FontFace>,
    )> {
        let segment = line.composed.segments.first()?;
        let face = std::sync::Arc::clone(self.rasteriser.face(segment.face)?);
        Some((segment.face, segment.run.size(), face))
    }

    /// Draws the hyphen a hyphenated line ends with.
    fn draw_hyphen(
        &mut self,
        builder: &mut FragmentTreeBuilder,
        parent: Option<mjx_layout::FragmentId>,
        line: &crate::flow::LaidOutLine,
        left: Emu,
        at: LineAddress,
        layout: &ParagraphLayout,
    ) {
        if !line.hyphenated {
            return;
        }
        let Some((face_id, run, width)) = layout.hyphen.as_ref() else {
            return;
        };
        let end = line
            .placement
            .segments
            .last()
            .map_or(Emu::ZERO, |placed| placed.x + placed.width);
        let origin = LayoutPoint::new(left + end, at.baseline);
        builder.push_simple(
            parent,
            at.stream
                .line(at.paragraph, at.line, line.range.end..line.range.end),
            LayoutRect::from_edges(origin.x, at.top, origin.x + *width, at.top + line.height),
            Fragment::GlyphRun(GlyphRunFragment {
                face: *face_id,
                run: run.clone(),
                origin,
                direction: layout.style.direction,
                level: mjx_text::BidiLevel::LEFT_TO_RIGHT,
            }),
        );
    }

    /// Draws a line number in the margin.
    ///
    /// **A line number is the one generated mark this crate draws**, and the reason is that nothing
    /// else ever will: a page number is a `PAGE` field and a footnote's mark is a `w:footnoteRef`,
    /// both of which MJXOFF-177 (R22) renders from the run stream, and a line number is in no run
    /// stream at all — Word generates it from `w:lnNumType` and puts it outside the text. Computing
    /// it and leaving it undrawn would be a feature no renderer could ever complete.
    fn draw_line_number(
        &mut self,
        builder: &mut FragmentTreeBuilder,
        parent: Option<mjx_layout::FragmentId>,
        line: &crate::flow::LaidOutLine,
        column_left: Emu,
        at: LineAddress,
        mark: LineMark,
    ) -> Result<(), DocumentLayoutError> {
        let Some((face_id, size, face)) = self.leader_face(line) else {
            return Ok(());
        };
        let text = format_number(mark.number, NumberFormat::Decimal);
        if text.is_empty() {
            return Ok(());
        }
        let request = ShapingRequest::new(&text, TextScript::COMMON, size, &self.features);
        let run = self.shaper.shape(&face, &request)?;
        let width = points(run.advance_in_points());
        let origin = LayoutPoint::new(column_left - mark.distance - width, at.baseline);
        builder.push_simple(
            parent,
            at.stream
                .line(at.paragraph, at.line, line.range.start..line.range.start),
            LayoutRect::from_edges(origin.x, at.top, origin.x + width, at.top + line.height),
            Fragment::GlyphRun(GlyphRunFragment {
                face: face_id,
                run,
                origin,
                direction: mjx_text::TextDirection::LeftToRight,
                level: mjx_text::BidiLevel::LEFT_TO_RIGHT,
            }),
        );
        Ok(())
    }
}

impl Stream {
    /// The address of one of its paragraphs.
    fn paragraph(self, index: usize) -> mjx_layout::SourceRef {
        match self {
            Self::Body => address::paragraph(index),
            Self::Secondary {
                part,
                container,
                paragraph,
            } => {
                let _ = index;
                address::stream_paragraph(part, container, paragraph)
            }
        }
    }

    /// The address of one line of one of its paragraphs.
    fn line(
        self,
        index: usize,
        line: usize,
        characters: std::ops::Range<usize>,
    ) -> mjx_layout::SourceRef {
        match self {
            Self::Body => address::line(index, line, characters),
            Self::Secondary {
                part,
                container,
                paragraph,
            } => {
                let _ = index;
                address::stream_line(part, container, paragraph, line, characters)
            }
        }
    }

    /// The address of one shaped run on one of its lines.
    fn segment(
        self,
        index: usize,
        line: usize,
        segment: usize,
        characters: std::ops::Range<usize>,
    ) -> mjx_layout::SourceRef {
        match self {
            Self::Body => address::segment(index, line, segment, characters),
            Self::Secondary {
                part,
                container,
                paragraph,
            } => {
                let _ = index;
                address::stream_segment(part, container, paragraph, line, segment, characters)
            }
        }
    }
}

/// How far a section's own `w:vAlign` pushes its content down a page it does not fill.
///
/// **GUESS:** `both` — vertical justification — is treated as `top` here, because distributing the
/// slack between paragraphs changes every baseline on the page and §17.6.23 says only that the text
/// is "justified" vertically without saying between what. `centre` and `bottom` are unambiguous and
/// are honoured exactly.
fn vertical_offset(content: &DocumentFlow, section: usize, slack: Emu) -> Emu {
    use mjx_ooxml_types::wordprocessingml::VerticalJustification;
    match content
        .section(section)
        .and_then(|entry| entry.vertical_alignment)
    {
        Some(VerticalJustification::Center) => slack.divided_by(2),
        Some(VerticalJustification::Bottom) => slack,
        _ => Emu::ZERO,
    }
}

/// Where a line sits, for the fragments drawn beside its text rather than out of it.
///
/// A hyphen, a tab leader and a line number are glyphs the **document does not contain**, and they
/// still need an address: a hit test that could not name them would report a caret position inside a
/// paragraph that has no such character, and a selection would jump. They take their own line's
/// address, so they sort into reading order with everything else on it.
#[derive(Clone, Copy, Debug)]
struct LineAddress {
    stream: Stream,
    paragraph: usize,
    line: usize,
    top: Emu,
    baseline: Emu,
}

/// The container index the `continuationNotice`'s own fragments are addressed under, for the same
/// reason the separator has one: it is drawn from a `w:footnote` entry that is not a note.
const NOTICE_CONTAINER: usize = usize::MAX - 1;

/// The container index the footnote separator's own fragments are addressed under.
///
/// `usize::MAX`, and deliberately not an entry index: the separator is drawn from a `w:footnote`
/// entry that is **not** one of the document's notes, and giving it that entry's index would make a
/// hit test on the rule report a caret inside footnote three.
const SEPARATOR_CONTAINER: usize = usize::MAX;

/// The most glyphs one tab leader may draw.
///
/// A tab stop may legally sit metres from the margin, and a dotted leader filling that gap is
/// furniture nobody reads. The bound is what stops a document from turning one tab into a page of
/// glyphs; it is generous enough that no ordinary table of contents reaches it.
pub const MAXIMUM_LEADER_GLYPHS: usize = 4096;
