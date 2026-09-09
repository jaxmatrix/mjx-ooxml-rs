//! **The second box model.** Plain text, reflowed into a fixed-width column, with no OOXML anywhere
//! in it.
//!
//! # Why this exists
//!
//! An abstraction with one implementation is a guess. A contract validated only by the
//! implementation that shaped it proves nothing, because every corner it got wrong is a corner that
//! implementation happens not to use — and the obvious gate, *"the trait compiles and the tree
//! round-trips"*, is satisfied by an interface that is secretly OOXML-shaped.
//!
//! So this is a box model with no shared ancestry with the three real ones. Its content is a `Vec` of
//! `String`s. It has never heard of a part name, a relationship, a paragraph property, a theme or a
//! `.docx`. If writing it had required bending anything in `mjx-layout`, the contract would have been
//! OOXML-shaped and the child would not have been done.
//!
//! # What it does
//!
//! Each paragraph is reflowed into the content area's first column; each line is shaped through
//! `mjx-text` and emitted as a [`LineFragment`] with one [`GlyphRunFragment`] under it. A paragraph's
//! first line is indented and its continuation lines are not, which is what makes the *resumption*
//! observable: a page that begins in the middle of a paragraph must not indent its first line.
//!
//! # What its checkpoint carries
//!
//! Eight bytes: the paragraph index and the byte offset inside it, little-endian. A real box model
//! carries more — Word's open list counters and its running line number for `w:lnNumType`, Excel's
//! frozen-pane origin, the height of a table row that spilled — and this one is deliberately
//! minimal, because what is being tested is the *contract*, not the model.
//!
//! [`Checkpoint::position`] carries the same address in the shared, inspectable form. The
//! duplication is on purpose and is the shape a real box model has too: the position is what a
//! caller reads to answer "which page is this paragraph on" without the box model, and the bytes are
//! what the box model reads to resume. Only the model may interpret the bytes, which is why they are
//! bytes.

#![allow(dead_code)]

use std::sync::Arc;

use mjx_layout::{
    BoxFragment, BoxModel, ChangeSet, Checkpoint, ComposedLine, Constraints, DirtyPages, Extent,
    ExtentPrecision, Fragment, FragmentId, FragmentTree, FragmentTreeBuilder, GlyphRunFragment,
    LayoutError, LayoutPoint, LayoutRect, LineComposer, LineFragment, ModelSignature,
    PageFragments, PageIndex, PartId, SourcePath, SourceRef, TextRun,
};
use mjx_ooxml_core::measure::Emu;
use mjx_text::{
    BidiAnalysis, FaceId, FeatureSet, FontError, FontFace, FontSize, GlyphRasteriser,
    LineBreakOptions, ParagraphDirection, Shaper, TextScript,
};

/// A document that is a list of paragraphs and nothing else.
#[derive(Clone, PartialEq, Eq, Default, Debug)]
pub(crate) struct PlainTextDocument {
    pub(crate) paragraphs: Vec<String>,
}

impl PlainTextDocument {
    pub(crate) fn from_paragraphs<I, S>(paragraphs: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            paragraphs: paragraphs.into_iter().map(Into::into).collect(),
        }
    }

    fn paragraph(&self, index: usize) -> &str {
        self.paragraphs.get(index).map_or("", String::as_str)
    }

    fn characters(&self) -> usize {
        self.paragraphs.iter().map(String::len).sum()
    }
}

/// What laying plain text out can go wrong with.
#[derive(Debug, thiserror::Error)]
pub(crate) enum PlainTextError {
    /// The shared machinery refused something — a foreign checkpoint, a misplaced one, an empty
    /// content area.
    #[error(transparent)]
    Layout(#[from] LayoutError),
    /// A face would not shape.
    #[error(transparent)]
    Text(#[from] FontError),
    /// The checkpoint's own bytes were not eight bytes of paragraph-and-offset.
    ///
    /// Only reachable by handing this model a checkpoint whose signature matches and whose state
    /// does not, which is what the perturbation test does deliberately.
    #[error("a continuation carried {0} bytes; this box model writes exactly eight")]
    MalformedContinuation(usize),
}

/// The plain-text box model.
#[derive(Debug)]
pub(crate) struct PlainTextColumn {
    face: Arc<FontFace>,
    face_id: FaceId,
    size: FontSize,
    features: FeatureSet,
    shaper: Shaper,
    line_gap: Emu,
    first_line_indent: Emu,
    /// Where each page laid out so far began, so that [`BoxModel::invalidate`] can say which page a
    /// change lands on. Grown as pages are laid out, which is exactly how a real box model learns
    /// the same thing.
    page_starts: Vec<SourceRef>,
}

impl PlainTextColumn {
    /// This box model's signature. Any constant will do, as long as it is this model's alone.
    pub(crate) const SIGNATURE: ModelSignature = ModelSignature::new(0x504C_4149_4E54_5854);

    /// A column setting `face` at `size`.
    pub(crate) fn new(
        rasteriser: &mut GlyphRasteriser,
        face: Arc<FontFace>,
        size: FontSize,
    ) -> Self {
        let face_id = rasteriser
            .register(&face)
            .expect("the bundled faces register");
        Self {
            face,
            face_id,
            size,
            features: FeatureSet::default(),
            shaper: Shaper::new(),
            line_gap: Emu::ZERO,
            first_line_indent: Emu::from_points(18.0),
            page_starts: Vec::new(),
        }
    }

    /// The same column with no first-line indent, for a test that wants every line to start at the
    /// same place.
    pub(crate) fn without_indent(mut self) -> Self {
        self.first_line_indent = Emu::ZERO;
        self
    }

    /// Where each page laid out so far began.
    pub(crate) fn page_starts(&self) -> &[SourceRef] {
        &self.page_starts
    }

    fn address(&self, paragraph: usize, characters: std::ops::Range<usize>) -> SourceRef {
        SourceRef::new(
            PartId::PRIMARY,
            SourcePath::new(&[u32::try_from(paragraph).unwrap_or(u32::MAX)]),
            u32::try_from(characters.start).unwrap_or(u32::MAX)
                ..u32::try_from(characters.end).unwrap_or(u32::MAX),
        )
    }

    /// Read a continuation, or the beginning of the document when there is none.
    fn resume_from(
        &self,
        page: PageIndex,
        resume: Option<&Checkpoint>,
    ) -> Result<(usize, usize), PlainTextError> {
        let Some(checkpoint) = resume else {
            return Ok((0, 0));
        };
        let state = checkpoint.state_for(Self::SIGNATURE, page)?;
        let bytes: [u8; 8] = state
            .try_into()
            .map_err(|_| PlainTextError::MalformedContinuation(state.len()))?;
        let paragraph = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        let offset = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
        Ok((paragraph, offset))
    }

    /// How tall one line of this face at this size is.
    fn line_height(&self, line: &ComposedLine) -> Emu {
        Emu::from_points(line.height_in_points()) + self.line_gap
    }
}

/// One line, laid out but not yet a fragment.
struct PlacedLine {
    paragraph: usize,
    line: ComposedLine,
    /// The line's box, in page space.
    rect: LayoutRect,
    /// Where the pen starts, on the baseline.
    origin: LayoutPoint,
    ascent: Emu,
    descent: Emu,
}

impl BoxModel for PlainTextColumn {
    type Content = PlainTextDocument;
    type Error = PlainTextError;

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
        let column = constraints.column(0).ok_or(LayoutError::EmptyContentArea {
            width: constraints.content.width().emu(),
            height: constraints.content.height().emu(),
        })?;
        if column.is_empty() {
            return Err(LayoutError::EmptyContentArea {
                width: column.width().emu(),
                height: column.height().emu(),
            }
            .into());
        }

        let (mut paragraph, mut offset) = self.resume_from(page, resume)?;
        let start = self.address(paragraph, offset..offset);
        let page_number = page.number() as usize;
        if self.page_starts.len() <= page_number {
            self.page_starts.resize(page_number + 1, start.clone());
        }
        if let Some(slot) = self.page_starts.get_mut(page_number) {
            *slot = start;
        }

        let mut placed: Vec<PlacedLine> = Vec::new();
        let mut pen_y = column.top;

        'pages: while paragraph < content.paragraphs.len() {
            let text = content.paragraph(paragraph);
            let bidi = BidiAnalysis::resolve(text, ParagraphDirection::LeftToRight);
            let runs = [TextRun {
                range: 0..text.len(),
                face: &self.face,
                face_id: self.face_id,
                script: TextScript::of_character('a'),
                direction: constraints.base_direction,
                level: mjx_text::BidiLevel::LEFT_TO_RIGHT,
                size: self.size,
                features: &self.features,
                language: None,
                // Plain text and nothing else: this harness has no inline objects, so it declares
                // no fixed advance. See `mjx_layout::TextRun::advance`.
                advance: None,
            }];
            let composer = LineComposer::new(text, &runs, &bidi, LineBreakOptions::default());

            loop {
                let indent = if offset == 0 {
                    self.first_line_indent
                } else {
                    Emu::ZERO
                };
                let measure = (column.width() - indent).points();
                let line = composer.next_line(&mut self.shaper, offset, measure)?;
                if line.range.start >= line.range.end && text.is_empty() {
                    // An empty paragraph still occupies a line, so that a reader can put a caret in
                    // it. It is composed with no segments and the face's own height.
                    let height = Emu::from_points(
                        f64::from(self.face.metrics().line_height()) * self.size.in_points()
                            / f64::from(self.face.metrics().units_per_em.max(1)),
                    );
                    if pen_y + height > column.bottom && !placed.is_empty() {
                        break 'pages;
                    }
                    placed.push(PlacedLine {
                        paragraph,
                        line,
                        rect: LayoutRect::from_edges(
                            column.left,
                            pen_y,
                            column.left,
                            pen_y + height,
                        ),
                        origin: LayoutPoint::new(column.left, pen_y + height),
                        ascent: height,
                        descent: Emu::ZERO,
                    });
                    pen_y += height;
                    break;
                }

                let ascent = Emu::from_points(line.ascent_in_points);
                let descent = Emu::from_points(line.descent_in_points);
                let height = self.line_height(&line);
                if pen_y + height > column.bottom && !placed.is_empty() {
                    break 'pages;
                }
                let left = column.left + indent;
                let width = Emu::from_points(line.width_in_points);
                let baseline = pen_y + ascent;
                let end = line.range.end;
                placed.push(PlacedLine {
                    paragraph,
                    rect: LayoutRect::from_edges(left, pen_y, left + width, pen_y + height),
                    origin: LayoutPoint::new(left, baseline),
                    ascent,
                    descent,
                    line,
                });
                pen_y += height;
                offset = end;
                if offset >= text.len() {
                    break;
                }
            }

            if offset >= content.paragraph(paragraph).len() {
                paragraph += 1;
                offset = 0;
            }
        }

        let finished = paragraph >= content.paragraphs.len();
        let continuation = if finished {
            None
        } else {
            let mut state = [0_u8; 8];
            state[..4].copy_from_slice(&(paragraph as u32).to_le_bytes());
            state[4..].copy_from_slice(&(offset as u32).to_le_bytes());
            Some(Checkpoint::new(
                Self::SIGNATURE,
                page,
                self.address(paragraph, offset..offset),
                state.to_vec(),
            )?)
        };

        Ok(PageFragments::new(
            page,
            self.build_tree(column, &placed),
            continuation,
        ))
    }

    fn estimate_extent(&self, content: &Self::Content, constraints: &Constraints) -> Extent {
        // A character count divided by a characters-per-page figure, which is what an estimate is:
        // cheap enough to run over a whole document on every edit, and wrong by a few percent.
        let metrics = self.face.metrics();
        let em = f64::from(metrics.units_per_em.max(1));
        let line_height = f64::from(metrics.line_height()) / em * self.size.in_points();
        let average_character = self.size.in_points() * 0.5;
        let column = constraints.column(0).unwrap_or(constraints.content);
        let per_line = (column.width().points() / average_character).max(1.0);
        let lines_per_page = (column.height().points() / line_height.max(1.0)).max(1.0);
        let characters = content.characters() as f64 + content.paragraphs.len() as f64 * per_line;
        let pages = (characters / (per_line * lines_per_page)).ceil().max(1.0);
        Extent {
            pages: pages as u32,
            page_size: constraints.page,
            precision: ExtentPrecision::Estimated,
        }
    }

    fn invalidate(&mut self, change: &ChangeSet) -> DirtyPages {
        let Some(earliest) = change.earliest() else {
            return DirtyPages::None;
        };
        // Flow layout: everything from the first page that could contain the change onward. The page
        // starts are in document order, so this is the last page whose start is at or before it.
        let first = self
            .page_starts
            .iter()
            .rposition(|start| start <= earliest)
            .unwrap_or(0);
        DirtyPages::From(PageIndex::new(u32::try_from(first).unwrap_or(u32::MAX)))
    }
}

impl PlainTextColumn {
    fn build_tree(&self, column: LayoutRect, placed: &[PlacedLine]) -> FragmentTree {
        let mut builder = FragmentTreeBuilder::with_capacity(placed.len() * 3 + 2);
        let body = builder.push_simple(
            None,
            SourceRef::node(PartId::PRIMARY, SourcePath::root()),
            column,
            Fragment::Box(BoxFragment {
                decoration: None,
                cell: None,
            }),
        );

        // A node's rectangle is fixed when it is pushed, so a paragraph's box has to be known before
        // its lines are added. One pass over the placed lines gives every paragraph piece its
        // extent; a builder that let a rect be edited afterwards would let a caller move a fragment
        // out from under a hit-test result, which is why it does not.
        let mut extents: Vec<(usize, LayoutRect)> = Vec::new();
        for line in placed {
            match extents.last_mut() {
                Some((index, bounds)) if *index == line.paragraph => {
                    *bounds = bounds.union(line.rect);
                }
                _ => extents.push((line.paragraph, line.rect)),
            }
        }

        let mut extents = extents.into_iter();
        let mut current: Option<(usize, FragmentId)> = None;
        for line in placed {
            let paragraph_box = match current {
                Some((index, id)) if index == line.paragraph => id,
                _ => {
                    let bounds = extents.next().map_or(line.rect, |(_, bounds)| bounds);
                    let Some(id) = builder.push_simple(
                        body,
                        SourceRef::node(
                            PartId::PRIMARY,
                            SourcePath::new(&[u32::try_from(line.paragraph).unwrap_or(u32::MAX)]),
                        ),
                        bounds,
                        Fragment::Box(BoxFragment {
                            decoration: None,
                            cell: None,
                        }),
                    ) else {
                        continue;
                    };
                    current = Some((line.paragraph, id));
                    id
                }
            };

            let Some(line_id) = builder.push_simple(
                Some(paragraph_box),
                self.address(line.paragraph, line.line.range.clone()),
                line.rect,
                Fragment::Line(LineFragment {
                    baseline: line.origin.y - line.rect.top,
                    ascent: line.ascent,
                    descent: line.descent,
                    base_direction: mjx_text::TextDirection::LeftToRight,
                    hanging_width: Emu::from_points(line.line.hanging_width_in_points),
                }),
            ) else {
                continue;
            };

            let mut pen = line.origin;
            for segment in &line.line.segments {
                let width = Emu::from_points(segment.width_in_points);
                let rect = LayoutRect::from_edges(
                    pen.x,
                    line.rect.top,
                    pen.x + width,
                    line.rect.top + line.rect.height(),
                );
                builder.push_simple(
                    Some(line_id),
                    self.address(line.paragraph, segment.range.clone()),
                    rect,
                    Fragment::GlyphRun(GlyphRunFragment {
                        face: segment.face,
                        run: segment.run.clone(),
                        origin: pen,
                        direction: segment.direction,
                        level: segment.level,
                    }),
                );
                pen = pen.translated(width, Emu::ZERO);
            }
        }

        builder.finish()
    }
}
