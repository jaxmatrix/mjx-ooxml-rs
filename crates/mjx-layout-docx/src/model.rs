//! [`DocumentBoxModel`] — Word's implementation of [`BoxModel`], and [`DocumentFlow`], the document
//! it lays out.
//!
//! # The one that reflows
//!
//! PowerPoint's box model places absolutely: a slide is a page and page *N* is reachable without
//! page *N−1*. Excel's addresses a grid: a band's first row comes from the page number and the row
//! geometry. **Word's pagination is emergent** — where page 200 begins depends on everything on the
//! 199 pages before it — and it is the reason `mjx-layout`'s [`Checkpoint`] exists.
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

use std::collections::BTreeMap;

use mjx_docx::{Document, DocumentFormatting, ParagraphFormatting};
use mjx_layout::{
    BoxFragment, BoxModel, ChangeSet, Checkpoint, Constraints, DirtyPages, Extent, ExtentPrecision,
    Fragment, FragmentTree, FragmentTreeBuilder, GlyphRunFragment, LayoutPoint, LayoutRect,
    LineFragment, ModelSignature, PageFragments, PageIndex,
};
use mjx_ooxml_core::measure::Emu;
use mjx_text::{
    FeatureSet, FontResolver, FontSize, GlyphRasteriser, Hyphenator, ShapingRequest, TextScript,
};

use crate::address;
use crate::checkpoint::Continuation;
use crate::decoration::{DecorationCatalogue, ParagraphDecoration};
use crate::error::DocumentLayoutError;
use crate::flow::{lay_out, FlowContext, ParagraphLayout};
use crate::justify::points;
use crate::paginate::{assemble, FlowPosition, PageAssembly, PageShape};
use crate::tabs::leader_character;
use crate::text::TextEngine;

/// A document, read once, ready to lay out.
///
/// Owns a [`DocumentFormatting`] — every part parsed once, every rung of the effective-property
/// ladder already resolved — and nothing borrowed from the [`Document`] it came from. That is what
/// makes it a *snapshot*: an edit to the document does not reach it, and the caller re-reads.
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentFlow {
    formatting: DocumentFormatting,
}

impl DocumentFlow {
    /// Reads `document` once.
    ///
    /// # Errors
    /// [`DocumentLayoutError::Document`] when a part cannot be read or a style chain does not
    /// terminate.
    pub fn read(document: &mut Document) -> Result<Self, DocumentLayoutError> {
        Ok(Self {
            formatting: document.formatting()?,
        })
    }

    /// The same from a [`DocumentFormatting`] a caller already holds.
    #[must_use]
    pub fn from_formatting(formatting: DocumentFormatting) -> Self {
        Self { formatting }
    }

    /// What was read.
    #[must_use]
    pub fn formatting(&self) -> &DocumentFormatting {
        &self.formatting
    }

    /// Its paragraphs, in document order.
    #[must_use]
    pub fn paragraphs(&self) -> &[ParagraphFormatting] {
        self.formatting.paragraphs()
    }

    /// How many there are.
    #[must_use]
    pub fn paragraph_count(&self) -> usize {
        self.formatting.paragraphs().len()
    }
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

    /// The column a page's content flows into.
    ///
    /// One column: multi-column flow is MJXOFF-175 (R20), and `Constraints::columns` is honoured
    /// only in that it is *read* — a caller that asks for three columns gets the first one and a
    /// document that fills it, which is a wrong-looking page rather than a wrong answer about what
    /// fits. Saying so is better than silently ignoring the field.
    fn column(constraints: &Constraints) -> LayoutRect {
        constraints.column(0).unwrap_or(constraints.content)
    }

    /// Lays a paragraph out. Every call is one unit of the work the counter reports.
    fn lay_out_paragraph(
        &mut self,
        content: &DocumentFlow,
        index: usize,
        column: LayoutRect,
    ) -> Result<ParagraphLayout, DocumentLayoutError> {
        let Some(paragraph) = content.paragraphs().get(index) else {
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

    /// Walks pages from `from` until `page`, returning where that page starts.
    ///
    /// The expensive road, taken only when the caller has no checkpoint. It is not a fallback and it
    /// is not a defect: page 200's start is not computable and somebody has to compute it.
    fn walk_to(
        &mut self,
        content: &DocumentFlow,
        page: PageIndex,
        column: LayoutRect,
        visited: &mut u32,
    ) -> Result<FlowPosition, DocumentLayoutError> {
        let mut position = FlowPosition::START;
        let shape = PageShape {
            height: column.height(),
        };
        for _ in 0..page.number() {
            let (assembly, _) = self.assemble_at(content, position, shape, column)?;
            *visited = visited.saturating_add(assembly.paragraphs_visited);
            match assembly.next {
                Some(next) => position = next,
                None => {
                    return Err(mjx_layout::LayoutError::PageBeyondContent {
                        requested: page,
                        last: PageIndex::new(page.number().saturating_sub(1)),
                    }
                    .into())
                }
            }
        }
        Ok(position)
    }

    fn assemble_at(
        &mut self,
        content: &DocumentFlow,
        from: FlowPosition,
        shape: PageShape,
        column: LayoutRect,
    ) -> Result<(PageAssembly, BTreeMap<usize, ParagraphLayout>), DocumentLayoutError> {
        // `assemble` needs `&mut self` to lay a paragraph out and holds no borrow of `self` itself,
        // which is what lets the closure below be `&mut`.
        let mut error: Option<DocumentLayoutError> = None;
        let mut layout_of = |index: usize| -> Result<ParagraphLayout, DocumentLayoutError> {
            self.lay_out_paragraph(content, index, column)
        };
        let result = assemble(content.paragraphs(), from, shape, &mut layout_of);
        if let Some(error) = error.take() {
            return Err(error);
        }
        result
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
        let column = Self::column(constraints);
        if column.width() <= Emu::ZERO || column.height() <= Emu::ZERO {
            return Err(DocumentLayoutError::EmptyContentArea {
                width: column.width().emu(),
                height: column.height().emu(),
            });
        }
        let paragraphs = u32::try_from(content.paragraph_count()).unwrap_or(u32::MAX);
        let mut visited = 0_u32;

        let start = match resume {
            Some(checkpoint) => {
                let state = checkpoint.state_for(Self::SIGNATURE, page)?;
                Continuation::decode(state, paragraphs)?.position
            }
            None if page == PageIndex::FIRST => FlowPosition::START,
            // No checkpoint and not the first page: there is no arithmetic that answers where this
            // page starts, so the pages before it are assembled and thrown away. The counter says
            // so.
            None => self.walk_to(content, page, column, &mut visited)?,
        };

        let shape = PageShape {
            height: column.height(),
        };
        let (assembly, layouts) = self.assemble_at(content, start, shape, column)?;
        visited = visited.saturating_add(assembly.paragraphs_visited);
        self.last_visited = visited;
        self.total_visited = self.total_visited.saturating_add(u64::from(visited));

        let tree = self.build(content, &assembly, &layouts, column)?;
        let continuation = match assembly.next {
            Some(position) => Some(
                Continuation {
                    position,
                    paragraphs,
                }
                .into_checkpoint(Self::SIGNATURE, page)?,
            ),
            None => None,
        };
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
        let column = Self::column(constraints);
        let characters: usize = content
            .paragraphs()
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
    /// Turns an assembled page into fragments.
    fn build(
        &mut self,
        content: &DocumentFlow,
        assembly: &PageAssembly,
        layouts: &BTreeMap<usize, ParagraphLayout>,
        column: LayoutRect,
    ) -> Result<FragmentTree, DocumentLayoutError> {
        let mut builder = FragmentTreeBuilder::new();
        let mut catalogue = DecorationCatalogue::new();
        let page = builder.push_simple(
            None,
            address::root(),
            column,
            Fragment::Box(BoxFragment {
                decoration: None,
                cell: None,
            }),
        );

        for block in &assembly.blocks {
            let Some(layout) = layouts.get(&block.paragraph) else {
                continue;
            };
            let Some(paragraph) = content.paragraphs().get(block.paragraph) else {
                continue;
            };
            let top = column.top + block.top;
            let height = layout.height_of(block.lines.clone());
            let rect = LayoutRect::from_edges(column.left, top, column.right, top + height);
            let decoration = catalogue.intern(ParagraphDecoration::of(paragraph.properties()));
            let body = builder.push_simple(
                page,
                address::paragraph(block.paragraph),
                rect,
                Fragment::Box(BoxFragment {
                    decoration,
                    cell: None,
                }),
            );

            let mut y = top;
            for line_index in block.lines.clone() {
                let Some(line) = layout.lines.get(line_index) else {
                    continue;
                };
                let line_rect =
                    LayoutRect::from_edges(column.left, y, column.right, y + line.height);
                let baseline_y = y + line.baseline;
                let node = builder.push_simple(
                    body,
                    address::line(block.paragraph, line_index, line.range.clone()),
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
                        // A tab is placed and draws nothing. Its leader, if it has one, is drawn
                        // below from the placement's own record.
                        continue;
                    }
                    let origin = LayoutPoint::new(column.left + placed.x, baseline_y);
                    let rect = LayoutRect::from_edges(
                        origin.x,
                        y,
                        origin.x + placed.width,
                        y + line.height,
                    );
                    builder.push_simple(
                        node,
                        address::segment(
                            block.paragraph,
                            line_index,
                            placed.segment,
                            segment.range.clone(),
                        ),
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
                    paragraph: block.paragraph,
                    line: line_index,
                    top: y,
                    baseline: baseline_y,
                };
                self.draw_leaders(&mut builder, node, line, column, at, layout)?;
                self.draw_hyphen(&mut builder, node, line, column, at, layout);
                y += line.height;
            }
        }
        Ok(builder.finish())
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
        column: LayoutRect,
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
            let origin = LayoutPoint::new(column.left + leader.from, at.baseline);
            builder.push_simple(
                parent,
                address::line(at.paragraph, at.line, line.range.clone()),
                LayoutRect::from_edges(
                    origin.x,
                    at.top,
                    column.left + leader.to,
                    at.top + line.height,
                ),
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

    /// The face a leader and a hyphen are drawn in: the paragraph's own, through the hyphen it
    /// already resolved.
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
        column: LayoutRect,
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
        let origin = LayoutPoint::new(column.left + end, at.baseline);
        builder.push_simple(
            parent,
            address::line(at.paragraph, at.line, line.range.end..line.range.end),
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
}

/// Where a line sits, for the two fragments that are drawn beside its text rather than out of it.
///
/// A hyphen and a tab leader are glyphs the **document does not contain**, and they still need an
/// address: a hit test that could not name them would report a caret position inside a paragraph
/// that has no such character, and a selection would jump. They take their own line's address, so
/// they sort into reading order with everything else on it.
#[derive(Clone, Copy, Debug)]
struct LineAddress {
    paragraph: usize,
    line: usize,
    top: Emu,
    baseline: Emu,
}

/// The most glyphs one tab leader may draw.
///
/// A tab stop may legally sit metres from the margin, and a dotted leader filling that gap is
/// furniture nobody reads. The bound is what stops a document from turning one tab into a page of
/// glyphs; it is generous enough that no ordinary table of contents reaches it.
pub const MAXIMUM_LEADER_GLYPHS: usize = 4096;
