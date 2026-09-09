//! Where the page ends: the four constraints that move content between pages, the columns the
//! content flows through, and the termination argument for every one of them.
//!
//! # The dangerous gate
//!
//! *"The document paginates"* is green for an implementation that breaks every page at a fixed line
//! count and honours nothing. Every constraint below is therefore stated with **the page assignment
//! it changes**, and `tests/each_constraint_moves_a_paragraph.rs` asserts, for each of the four,
//! that a named paragraph lands on a different page when the constraint is switched off.
//!
//! # The four, and why each terminates
//!
//! * **`w:pageBreakBefore`** — the paragraph starts a page. It cannot loop: the break is taken only
//!   when something is already on the page, so taking it always leaves a non-empty page behind.
//! * **`w:keepLines`** — the paragraph is not split. A paragraph taller than a whole column could
//!   otherwise be pushed forward for ever, so it is placed **anyway** when the column it is pushed
//!   on to is already empty. That is the same page Word puts it on, overflowing.
//! * **`w:widowControl`** — no single line of a paragraph is left alone at the foot of a column or
//!   carried alone to the top of the next. It can only ever move lines *forward*, and it is switched
//!   off entirely for a paragraph that has nowhere forward to go, so the count of lines placed is
//!   monotone.
//! * **`w:keepNext`** — the paragraph shares a column with the one after it. This is the classic
//!   infinite loop: a chain longer than a column has no satisfying assignment, so the chain is
//!   broken at the point where breaking it would leave the column **empty**, and the first paragraph
//!   of an unsatisfiable chain is placed where it does not fit. `tests/termination.rs` runs a
//!   thousand-paragraph chain and asserts it produces pages.
//!
//! Above all of them sits the one guarantee the whole engine rests on: **every page places at least
//! one line.** A page that placed nothing would produce a next position equal to its own start, and
//! the page after it would be identical, for ever. Every constraint above is allowed to refuse
//! content only while something else is already on the page.
//!
//! # Columns, and why balancing is a search rather than a division
//!
//! Filling *n* columns is filling one column *n* times and threading the position through, which is
//! the whole of [`assemble`] once [`fill_column`] exists. **Balancing is not.** A section that ends
//! at a `continuous` break has its columns levelled — the obvious implementation, dividing the total
//! height by the column count, is wrong, because content is placed in whole lines and a paragraph
//! may not be splittable at all. What is actually wanted is *the shortest column height at which the
//! remaining content still fits in n columns*, and that is a **monotone predicate**: content placed
//! never decreases as the height grows. So [`assemble`] **bisects** it, which terminates in
//! `log2` of the page height in EMU — around thirty-one fills of already-laid-out paragraphs — and
//! is exact rather than approximate.

use std::collections::BTreeMap;

use mjx_docx::ParagraphFormatting;
use mjx_ooxml_core::measure::Emu;
use mjx_ooxml_types::wordprocessingml::BreakType;

use crate::flow::ParagraphLayout;

/// Where in the document a page starts.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Debug)]
pub struct FlowPosition {
    /// Which paragraph, counted from zero across the body.
    pub paragraph: u32,
    /// Which of its lines, counted from zero. Zero is the start of the paragraph.
    pub line: u32,
}

impl FlowPosition {
    /// The start of the document.
    pub const START: Self = Self {
        paragraph: 0,
        line: 0,
    };

    /// The start of the paragraph at `index`.
    #[must_use]
    pub fn at(index: usize) -> Self {
        Self {
            paragraph: u32::try_from(index).unwrap_or(u32::MAX),
            line: 0,
        }
    }
}

/// One paragraph, or the part of one, placed in a column.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PlacedParagraph {
    /// Which paragraph.
    pub paragraph: usize,
    /// Which of its lines are here.
    pub lines: std::ops::Range<usize>,
    /// Where the first of them sits, from the top of the column.
    pub top: Emu,
    /// How much empty space was left above it.
    pub space_before: Emu,
    /// Whether it began in an earlier column or page.
    pub continued: bool,
    /// Whether it carries on into a later one.
    pub continues: bool,
}

/// Why a column stopped taking content.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ColumnEnd {
    /// The content ran out — there is nothing after it in this document.
    ContentEnded,
    /// The column filled up, or a constraint refused what came next.
    Filled,
    /// A `w:br@type="column"` ended it.
    ColumnBreak,
    /// A `w:br@type="page"` or a `w:pageBreakBefore` ended it, which ends the whole page.
    PageBreak,
    /// The section's last paragraph was placed.
    SectionEnded,
}

/// One column, filled.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ColumnFill {
    /// What is in it, in document order.
    pub blocks: Vec<PlacedParagraph>,
    /// Where the content after it starts, or `None` when the document ended here.
    pub next: Option<FlowPosition>,
    /// How tall the content is, from the column's top.
    pub used: Emu,
    /// Why it stopped.
    pub ended: ColumnEnd,
}

/// One page's body, assembled.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PageAssembly {
    /// Its columns, left to right. Never empty.
    pub columns: Vec<ColumnFill>,
    /// Where the next page starts, or `None` when the document ended here.
    pub next: Option<FlowPosition>,
    /// Whether the page ended because its section did.
    pub ended_section: bool,
    /// How many paragraphs this page's assembly had to lay out.
    ///
    /// **The instrument the checkpoint gate rests on**, and it counts *work* rather than output: a
    /// page assembled from a checkpoint looks at the paragraphs on it, and a page assembled by
    /// walking from the beginning looks at every paragraph before it too. A gate on the *fragments*
    /// cannot tell the two apart, because they produce the same page — which is exactly why a
    /// checkpoint that is never used still passes every output assertion.
    pub paragraphs_visited: u32,
}

impl PageAssembly {
    /// Every block on the page, column by column, with the column index each came from.
    pub fn blocks(&self) -> impl Iterator<Item = (usize, &PlacedParagraph)> {
        self.columns
            .iter()
            .enumerate()
            .flat_map(|(column, fill)| fill.blocks.iter().map(move |block| (column, block)))
    }

    /// How tall the tallest column is.
    #[must_use]
    pub fn used(&self) -> Emu {
        self.columns
            .iter()
            .fold(Emu::ZERO, |tallest, fill| tallest.maximum(fill.used))
    }

    /// Whether anything at all was placed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.columns.iter().all(|fill| fill.blocks.is_empty())
    }
}

/// What the paginator needs to know about the page it is filling.
#[derive(Clone, Copy, Debug)]
pub struct PageShape<'a> {
    /// How tall each column is.
    pub height: Emu,
    /// How many columns there are. Zero is read as one.
    pub columns: usize,
    /// How wide each column is, in order. A column past the end of this list is `width` wide.
    pub widths: &'a [Emu],
    /// The width a column takes when `widths` does not name it.
    pub width: Emu,
    /// The last paragraph of this page's section, inclusive — past it, the page ends.
    pub section_last: Option<usize>,
    /// Whether to level the columns when the section's content ends on this page.
    pub balance: bool,
}

/// The layouts one page's assembly reused, so a fixed point over the note area and a balancing
/// search cost no further paragraph layouts.
///
/// # Keyed by paragraph **and column width**, which is not the same as by column
///
/// A paragraph's lines depend on the measure they were fitted against, so a paragraph that appears
/// in two columns of different widths has two layouts and a cache keyed by paragraph alone would
/// hand the second column the first one's lines — a visible overrun that no assertion on *which*
/// paragraph is where can see. Keying by the width rather than by the column index is what keeps
/// the common case free: `w:equalWidth` columns are all the same measure, so they share one layout,
/// and a single-column document behaves exactly as it did before columns existed — which is what
/// keeps [`PageAssembly::paragraphs_visited`] comparable with MJXOFF-174's own numbers.
pub type LayoutCache = BTreeMap<(usize, i64), ParagraphLayout>;

/// Assembles the page that starts at `from`.
///
/// `layout_of` is asked for a paragraph's lines and must answer the same lines for the same
/// paragraph however many times it is asked — that equivalence is what makes page *N* alone and
/// pages 1..=*N* in order agree, and it is why paragraph layout takes no argument that depends on
/// which page the paragraph is on. `layouts` is the memo that makes asking cheap; it is the caller's
/// so that laying the same page out twice — which is what the footnote fixed point does — costs one
/// set of paragraph layouts and not two.
///
/// # Errors
/// Whatever `layout_of` fails with.
pub fn assemble<E>(
    paragraphs: &[ParagraphFormatting],
    from: FlowPosition,
    shape: PageShape<'_>,
    layouts: &mut LayoutCache,
    layout_of: &mut dyn FnMut(usize, Emu) -> Result<ParagraphLayout, E>,
) -> Result<PageAssembly, E> {
    let before = layouts.len();
    let mut columns = fill_all(paragraphs, from, shape, layouts, layout_of)?;

    // Balancing, and the one condition it applies under: the section's content ran out on this page,
    // so there is a fixed amount of it and levelling the columns is a question with an answer. A
    // page whose columns are full has nothing to balance — the content does not fit either way.
    if shape.balance && shape.columns > 1 && ends_here(&columns, shape.section_last) {
        if let Some(levelled) = balance(paragraphs, from, shape, layouts, layout_of)? {
            columns = levelled;
        }
    }

    let ended_section = matches!(
        columns.last().map(|fill| fill.ended),
        Some(ColumnEnd::SectionEnded)
    );
    let next = columns.last().and_then(|fill| fill.next);
    #[allow(clippy::cast_possible_truncation)]
    let visited = layouts.len().saturating_sub(before) as u32;
    Ok(PageAssembly {
        columns,
        next,
        ended_section,
        paragraphs_visited: visited,
    })
}

/// Every column of one page, filled left to right at `shape.height`.
fn fill_all<E>(
    paragraphs: &[ParagraphFormatting],
    from: FlowPosition,
    shape: PageShape<'_>,
    layouts: &mut LayoutCache,
    layout_of: &mut dyn FnMut(usize, Emu) -> Result<ParagraphLayout, E>,
) -> Result<Vec<ColumnFill>, E> {
    fill_at(paragraphs, from, shape, shape.height, layouts, layout_of)
}

/// The same at a stated column height, which is what balancing varies.
///
/// A page whose first column was ended by a page break, by the end of the section or by the end of
/// the document still **has** its other columns — a two-column page cut short is a two-column page —
/// so they are pushed empty rather than omitted, carrying the reason the page stopped.
fn fill_at<E>(
    paragraphs: &[ParagraphFormatting],
    from: FlowPosition,
    shape: PageShape<'_>,
    height: Emu,
    layouts: &mut LayoutCache,
    layout_of: &mut dyn FnMut(usize, Emu) -> Result<ParagraphLayout, E>,
) -> Result<Vec<ColumnFill>, E> {
    let count = shape.columns.max(1);
    let mut filled: Vec<ColumnFill> = Vec::with_capacity(count);
    let mut position: Option<FlowPosition> = Some(from);
    let mut stopped: Option<ColumnEnd> = None;
    for _ in 0..count {
        let carried = match (stopped, position) {
            (Some(end), _) => Some(end),
            (None, None) => Some(ColumnEnd::ContentEnded),
            (None, Some(_)) => None,
        };
        if let Some(ended) = carried {
            filled.push(ColumnFill {
                blocks: Vec::new(),
                next: position,
                used: Emu::ZERO,
                ended,
            });
            continue;
        }
        let start = position.unwrap_or(from);
        let page_empty = filled.iter().all(|fill| fill.blocks.is_empty());
        let fill = fill_column(
            paragraphs,
            start,
            ColumnShape {
                height,
                width: shape
                    .widths
                    .get(filled.len())
                    .copied()
                    .unwrap_or(shape.width),
                section_last: shape.section_last,
                page_empty,
            },
            layouts,
            layout_of,
        )?;
        position = fill.next;
        if matches!(
            fill.ended,
            ColumnEnd::PageBreak | ColumnEnd::SectionEnded | ColumnEnd::ContentEnded
        ) {
            stopped = Some(fill.ended);
        }
        filled.push(fill);
    }
    Ok(filled)
}

/// Whether this page's columns took everything up to the end of the section (or of the document).
fn ends_here(columns: &[ColumnFill], section_last: Option<usize>) -> bool {
    let Some(fill) = columns.last() else {
        return false;
    };
    if matches!(
        fill.ended,
        ColumnEnd::ContentEnded | ColumnEnd::SectionEnded
    ) {
        return true;
    }
    match (fill.next, section_last) {
        (None, _) => true,
        (Some(next), Some(last)) => next.paragraph as usize > last,
        (Some(_), None) => false,
    }
}

/// Levels the columns of a section that ends on this page.
///
/// # The search, and why it is a search
///
/// The content on the page is fixed — it is everything from `from` to the end of the section — so
/// the question is *how short may a column be and still hold a `columns`-th of it*. Dividing the
/// total height by the count answers a different question, because lines are indivisible and a
/// `w:keepLines` paragraph may not split at all: the division's answer is routinely a hair too
/// short, and a column that is a hair too short spills a whole line into the next one, which
/// **unbalances** the very thing being balanced.
///
/// So the predicate is *does everything still fit in `columns` columns of height h*, which is
/// monotone in `h` — a taller column never holds less — and the answer is its smallest true `h`,
/// found by bisection over `[0, shape.height]`. Termination is the bisection's own: the interval
/// halves every step and the values are integers, so at most `log2(shape.height)` steps, and
/// `shape.height` itself is known to satisfy the predicate before the search begins.
///
/// Returns `None` when the levelled fill would not actually be an improvement, in which case the
/// unbalanced one stands.
fn balance<E>(
    paragraphs: &[ParagraphFormatting],
    from: FlowPosition,
    shape: PageShape<'_>,
    layouts: &mut LayoutCache,
    layout_of: &mut dyn FnMut(usize, Emu) -> Result<ParagraphLayout, E>,
) -> Result<Option<Vec<ColumnFill>>, E> {
    let mut low = Emu::ZERO;
    let mut high = shape.height;
    while low < high {
        let middle = low + (high - low).divided_by(2);
        if middle == low {
            break;
        }
        let trial = fill_at(paragraphs, from, shape, middle, layouts, layout_of)?;
        if ends_here(&trial, shape.section_last) {
            high = middle;
        } else {
            low = middle;
        }
    }
    let levelled = fill_at(paragraphs, from, shape, high, layouts, layout_of)?;
    if ends_here(&levelled, shape.section_last) {
        Ok(Some(levelled))
    } else {
        Ok(None)
    }
}

/// What one column is: how big it is, where its section ends, and whether the page it is on has
/// anything on it yet.
///
/// The last of those is what `w:pageBreakBefore` reads — a break at the very top of a page would
/// open a blank one — and it is a fact about the **page**, not the column, which is why it travels
/// beside the measurements rather than being inferred from them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ColumnShape {
    /// How tall it is.
    pub height: Emu,
    /// How wide, which is the measure its paragraphs were fitted against.
    pub width: Emu,
    /// The last paragraph of the section it belongs to, inclusive.
    pub section_last: Option<usize>,
    /// Whether the page holds nothing at all yet.
    pub page_empty: bool,
}

/// Fills one column from `from`, stopping at `shape.height`.
///
/// # Errors
/// Whatever `layout_of` fails with.
#[allow(clippy::too_many_lines)]
pub fn fill_column<E>(
    paragraphs: &[ParagraphFormatting],
    from: FlowPosition,
    shape: ColumnShape,
    layouts: &mut LayoutCache,
    layout_of: &mut dyn FnMut(usize, Emu) -> Result<ParagraphLayout, E>,
) -> Result<ColumnFill, E> {
    let ColumnShape {
        height,
        width,
        section_last,
        page_empty,
    } = shape;
    let mut blocks: Vec<PlacedParagraph> = Vec::new();
    let mut y = Emu::ZERO;
    let mut index = from.paragraph as usize;
    let mut line = from.line as usize;
    let mut next: Option<FlowPosition> = None;
    let mut ended = ColumnEnd::ContentEnded;

    while index < paragraphs.len() {
        if section_last.is_some_and(|last| index > last) {
            next = Some(FlowPosition::at(index));
            ended = ColumnEnd::SectionEnded;
            break;
        }
        let key = (index, width.emu());
        if let std::collections::btree_map::Entry::Vacant(slot) = layouts.entry(key) {
            slot.insert(layout_of(index, width)?);
        }
        // Inserted immediately above when absent.
        let Some(layout) = layouts.get(&key) else {
            break;
        };
        let total = layout.lines.len();
        if line >= total {
            index += 1;
            line = 0;
            continue;
        }

        // `w:pageBreakBefore` — but never at the very start of the document, where it would open the
        // file with a blank page. **GUESS:** Word ignores it on the first paragraph; ECMA-376 does
        // not say, and honouring it would be a visible extra page in every document whose first
        // heading style carries the flag.
        if layout.style.page_break_before
            && line == 0
            && index > 0
            && !(page_empty && blocks.is_empty())
        {
            next = Some(FlowPosition::at(index));
            ended = ColumnEnd::PageBreak;
            break;
        }

        // GUESS: the space above a paragraph is suppressed at the top of a column. Word does this,
        // and the alternative — a band of white space above the first line of every page — is
        // immediately visible.
        //
        // `w:contextualSpacing` is the document's own suppression: *don't add space between
        // paragraphs of the same style*, which is what every bulleted list in every document relies
        // on. It is a question about two paragraphs' **identity** and not about their resolved
        // values, which is why `ParagraphFormatting::style_id` exists at all.
        let suppressed = line > 0
            || blocks.is_empty()
            || (layout.style.contextual_spacing
                && same_style(paragraphs, index, index.wrapping_sub(1)));
        let space_before = if suppressed {
            Emu::ZERO
        } else {
            layout.space_before
        };

        let available = height - y - space_before;
        let mut fitted = lines_that_fit(layout, line, available);
        let remaining = total - line;

        if layout.style.keep_lines_together && fitted < remaining {
            // Not splittable. Push it whole — unless the column it would be pushed on to is this
            // one, which is the case that does not terminate.
            fitted = if blocks.is_empty() { remaining } else { 0 };
        }

        if layout.style.widow_control {
            fitted = widow_control(fitted, remaining);
        }

        if fitted == 0 {
            if blocks.is_empty() {
                // Nothing else is in the column, so refusing again would produce a column with
                // nothing in it and a next position identical to this one. One line is placed,
                // overflowing.
                fitted = 1;
            } else {
                next = Some(FlowPosition {
                    paragraph: index as u32,
                    line: line as u32,
                });
                ended = ColumnEnd::Filled;
                break;
            }
        }

        // A `w:br@type="page"` or `="column"` inside the paragraph ends the column at the line it
        // sits on, whatever else would have fitted. The two differ in what they end: a column break
        // moves to the next column and a page break ends the page.
        let hard = hard_break_within(&paragraphs[index], layout, line, fitted);
        let (fitted, hard_ended) = match hard {
            Some((at_line, kind)) => (
                at_line - line + 1,
                Some(match kind {
                    BreakType::Page => ColumnEnd::PageBreak,
                    _ => ColumnEnd::ColumnBreak,
                }),
            ),
            None => (fitted, None),
        };

        blocks.push(PlacedParagraph {
            paragraph: index,
            lines: line..line + fitted,
            top: y + space_before,
            space_before,
            continued: line > 0,
            continues: line + fitted < total,
        });
        y = y + space_before + layout.height_of(line..line + fitted);

        if hard_ended.is_some() || line + fitted < total {
            let resume_line = line + fitted;
            next = Some(if resume_line >= total {
                FlowPosition::at(index + 1)
            } else {
                FlowPosition {
                    paragraph: index as u32,
                    line: resume_line as u32,
                }
            });
            ended = hard_ended.unwrap_or(ColumnEnd::Filled);
            break;
        }

        // The section's last paragraph was placed in full. That ends the page — but **only when
        // there is something after it**: the last section of a document ends with the document, and
        // reporting a next position one past the end there would produce an endless run of empty
        // pages that every "walk until the content stops" caller would take as content.
        if section_last.is_some_and(|last| index >= last) {
            if index + 1 >= paragraphs.len() {
                next = None;
                ended = ColumnEnd::ContentEnded;
            } else {
                next = Some(FlowPosition::at(index + 1));
                ended = ColumnEnd::SectionEnded;
            }
            break;
        }

        if !(layout.style.contextual_spacing && same_style(paragraphs, index, index + 1)) {
            y += layout.space_after;
        }
        index += 1;
        line = 0;
    }

    // `w:keepNext`, applied after the column is full because it is a statement about the *boundary*
    // and the boundary is not known until then. A section's own end is not a boundary `w:keepNext`
    // may move content across — the next paragraph is in a different section and would be on a
    // different page shape — so the chain is left alone there.
    if next.is_some() && !matches!(ended, ColumnEnd::SectionEnded) {
        apply_keep_with_next(&mut blocks, layouts, width, &mut next);
    }

    let used = blocks.last().map_or(Emu::ZERO, |last| {
        let layout = layouts.get(&(last.paragraph, width.emu()));
        let height = layout.map_or(Emu::ZERO, |layout| layout.height_of(last.lines.clone()));
        last.top + height
    });
    Ok(ColumnFill {
        blocks,
        next,
        used,
        ended,
    })
}

/// Whether the paragraphs at `one` and `other` name the same `w:pStyle`.
///
/// Two paragraphs that name **no** style are the same style: that is the document's default
/// paragraph style, which is what an unstyled body is made of, and reading `None` as *different*
/// would turn `w:contextualSpacing` off for exactly the documents that use it most.
fn same_style(paragraphs: &[ParagraphFormatting], one: usize, other: usize) -> bool {
    match (paragraphs.get(one), paragraphs.get(other)) {
        (Some(left), Some(right)) => left.style_id() == right.style_id(),
        // A paragraph with no neighbour has nothing to share a style with, so the space stands.
        _ => false,
    }
}

/// How many of `layout`'s lines from `line` onward fit in `available`.
fn lines_that_fit(layout: &ParagraphLayout, line: usize, available: Emu) -> usize {
    if available <= Emu::ZERO {
        return 0;
    }
    let mut used = Emu::ZERO;
    let mut fitted = 0_usize;
    for candidate in layout.lines.iter().skip(line) {
        let next = used + candidate.height;
        if next > available {
            break;
        }
        used = next;
        fitted += 1;
    }
    fitted
}

/// `w:widowControl`, both halves.
///
/// An **orphan** is a single line left behind at the foot of a page; a **widow** is a single line
/// carried alone to the top of the next. Both are refused by moving lines *forward*, which is the
/// only direction that terminates: the number of lines placed never grows.
#[must_use]
pub fn widow_control(fitted: usize, remaining: usize) -> usize {
    if remaining < 2 || fitted == 0 || fitted >= remaining {
        // Nothing to protect: a one-line paragraph has no widow, and a paragraph that fits entirely
        // has no boundary inside it.
        return fitted;
    }
    let mut fitted = fitted;
    if remaining - fitted == 1 && fitted >= 2 {
        // A widow: one line would go over alone. Send a second with it.
        fitted -= 1;
    }
    if fitted == 1 {
        // An orphan: one line would stay behind alone. Send the paragraph whole.
        fitted = 0;
    }
    fitted
}

/// The line a hard break inside the paragraph ends the column at, and which kind it was.
fn hard_break_within(
    paragraph: &ParagraphFormatting,
    layout: &ParagraphLayout,
    from: usize,
    count: usize,
) -> Option<(usize, BreakType)> {
    for break_at in paragraph.hard_breaks() {
        if !matches!(break_at.kind, BreakType::Page | BreakType::Column) {
            continue;
        }
        for offset in 0..count {
            let index = from + offset;
            let Some(line) = layout.lines.get(index) else {
                break;
            };
            if break_at.at >= line.range.start && break_at.at < line.range.end {
                return Some((index, break_at.kind));
            }
        }
    }
    None
}

/// Moves a trailing `w:keepNext` chain on to the next column.
///
/// Only a paragraph placed **in full** can be moved: a paragraph split across the boundary already
/// has its last line in the same column as what follows it, which is what `w:keepNext` asks for.
fn apply_keep_with_next(
    blocks: &mut Vec<PlacedParagraph>,
    layouts: &LayoutCache,
    width: Emu,
    next: &mut Option<FlowPosition>,
) {
    loop {
        // Never empty the column. This is the termination argument for an unsatisfiable chain: a
        // chain of `w:keepNext` paragraphs longer than a column has no assignment that satisfies it,
        // so the chain is broken here and the reader sees it broken rather than seeing nothing.
        if blocks.len() < 2 {
            return;
        }
        let Some(last) = blocks.last() else {
            return;
        };
        let Some(layout) = layouts.get(&(last.paragraph, width.emu())) else {
            return;
        };
        if !layout.style.keep_with_next || last.lines.end != layout.lines.len() {
            return;
        }
        let moved = FlowPosition {
            paragraph: last.paragraph as u32,
            line: last.lines.start as u32,
        };
        // Only ever move backwards in the document; a `next` already earlier than this block would
        // mean the column was cut before it, which cannot happen.
        if next.is_some_and(|position| position <= moved) {
            return;
        }
        *next = Some(moved);
        blocks.pop();
    }
}
