//! A **secondary content stream**: a header, a footer, a footnote, an endnote or one of Word's own
//! separator rules — anything that is paragraphs and is not the body.
//!
//! # One flow engine, four kinds of content
//!
//! A header's paragraphs are `w:p`s of exactly the shape the body's are: the same `w:pPr`, the same
//! runs, the same tab stops, resolved through the same ladder by the same residency. So a header is
//! laid out by [`crate::flow::lay_out`] and by nothing else — MJXOFF-175's ticket says it in as many
//! words (*"a footnote area is a constraint on the body flow, not a second flow engine"*), and this
//! module is the whole of the sharing: a list of paragraph layouts, and a flat index of their lines
//! so that a stream can be **split across pages** the way the body is.
//!
//! # Why the lines are flattened
//!
//! A footnote is the one secondary stream that splits. Word carries the tail of a long note on to
//! the next page under a continuation separator, and it splits at a *line*, not at a paragraph — so
//! the unit a note area places is "lines *k* to *m* of this note", counted across the note's
//! paragraphs. A header never splits, and flattening costs it one small vector; the alternative is
//! two representations of the same thing, which is how a header and a footnote start disagreeing
//! about what a line is.

use mjx_docx::{DocumentLayoutSettings, ParagraphFormatting};
use mjx_layout::LayoutRect;
use mjx_ooxml_core::measure::Emu;
use mjx_text::{FontError, Hyphenator};

use crate::flow::{lay_out, FlowContext, ParagraphLayout};
use crate::text::TextEngine;

/// The height a secondary stream is laid out against.
///
/// A thousand inches, and deliberately a number rather than [`Emu::MAXIMUM`]: nothing in
/// [`crate::flow`] reads a column's height — a paragraph is laid out into lines and the *page*
/// decides which of them are where — but a rectangle whose bottom edge is `i64::MAX` overflows the
/// moment anything subtracts from it, and a bound that cannot be reached by a real header is worth
/// more than one that cannot be exceeded by arithmetic.
pub const UNBOUNDED_HEIGHT: Emu = Emu::from_emu(1000 * mjx_ooxml_core::measure::EMU_PER_INCH);

/// One line of a stream, addressed across the stream's paragraphs.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct StreamLine {
    /// Which paragraph of the stream it belongs to.
    pub paragraph: usize,
    /// Which line of that paragraph.
    pub line: usize,
    /// How tall it is.
    pub height: Emu,
    /// The space above its paragraph, applied only to that paragraph's first line.
    pub space_before: Emu,
}

/// A secondary stream, laid out.
#[derive(Clone, PartialEq, Debug)]
pub struct StreamLayout {
    /// Its paragraphs, in order.
    pub paragraphs: Vec<ParagraphLayout>,
    /// Every line of every paragraph, in order.
    pub lines: Vec<StreamLine>,
}

impl StreamLayout {
    /// A stream with nothing in it.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            paragraphs: Vec::new(),
            lines: Vec::new(),
        }
    }

    /// How tall lines `from..to` are, with the space above each paragraph they open.
    ///
    /// The space above the stream's **first placed line** is suppressed, exactly as the body
    /// suppresses the space above the first paragraph on a page: a band of white space above a
    /// header's first line is immediately visible, and it is the same GUESS `crate::paginate` makes
    /// for the same reason.
    #[must_use]
    pub fn height_of(&self, lines: std::ops::Range<usize>) -> Emu {
        let start = lines.start;
        self.lines
            .get(lines)
            .unwrap_or_default()
            .iter()
            .enumerate()
            .fold(Emu::ZERO, |total, (offset, line)| {
                let space = if start + offset == start {
                    Emu::ZERO
                } else {
                    line.space_before
                };
                total + space + line.height
            })
    }

    /// How tall the whole stream is.
    #[must_use]
    pub fn height(&self) -> Emu {
        self.height_of(0..self.lines.len())
    }

    /// How many lines it has.
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Whether it holds nothing at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// How many of its lines from `from` fit in `available`, and how tall those are.
    ///
    /// Never zero while `from` names a line: a note area that placed no lines of the note it is
    /// there for would carry the same note to the next page and to the page after it, for ever. That
    /// is the same "every page places at least one line" rule the body flow rests on, applied to the
    /// second flow — see [`crate::notes`], where the whole termination argument is written out.
    #[must_use]
    pub fn lines_that_fit(&self, from: usize, available: Emu) -> (usize, Emu) {
        let mut used = Emu::ZERO;
        let mut fitted = 0_usize;
        for (offset, line) in self.lines.iter().enumerate().skip(from) {
            let space = if offset == from {
                Emu::ZERO
            } else {
                line.space_before
            };
            let next = used + space + line.height;
            if next > available && fitted > 0 {
                break;
            }
            used = next;
            fitted += 1;
            if next > available {
                break;
            }
        }
        (fitted, used)
    }
}

/// Lays a stream's paragraphs out into a column `width` wide.
///
/// # Errors
/// [`FontError`] when a face will not shape.
pub fn lay_out_stream(
    engine: &mut TextEngine<'_>,
    paragraphs: &[ParagraphFormatting],
    width: Emu,
    settings: &DocumentLayoutSettings,
    hyphenator: Option<&dyn Hyphenator>,
) -> Result<StreamLayout, FontError> {
    // The column a secondary stream flows through is as wide as the body's text area and as tall as
    // it needs to be: nothing here is clipped by a height, because *where* the stream ends up is the
    // page's decision and not the stream's.
    let column = LayoutRect::from_edges(Emu::ZERO, Emu::ZERO, width, UNBOUNDED_HEIGHT);
    let mut layouts = Vec::with_capacity(paragraphs.len());
    let mut lines = Vec::new();
    for (index, paragraph) in paragraphs.iter().enumerate() {
        let layout = lay_out(
            engine,
            paragraph,
            FlowContext {
                top: Emu::ZERO,
                exclusions: &[],
                column,
                settings,
                hyphenator,
            },
        )?;
        for (number, line) in layout.lines.iter().enumerate() {
            lines.push(StreamLine {
                paragraph: index,
                line: number,
                height: line.height,
                space_before: if number == 0 {
                    layout.space_before
                } else {
                    Emu::ZERO
                },
            });
        }
        layouts.push(layout);
    }
    Ok(StreamLayout {
        paragraphs: layouts,
        lines,
    })
}
