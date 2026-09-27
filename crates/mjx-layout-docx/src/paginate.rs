//! Where the page ends: the four constraints that move content between pages, the columns the
//! content flows through, the floats the content flows around, and the termination argument for
//! every one of them.
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
//! * **`w:pageBreakBefore`** — the block starts a page. It cannot loop: the break is taken only
//!   when something is already on the page, so taking it always leaves a non-empty page behind.
//! * **`w:keepLines`** — the paragraph is not split. A paragraph taller than a whole column could
//!   otherwise be pushed forward for ever, so it is placed **anyway** when the column it is pushed
//!   on to is already empty. That is the same page Word puts it on, overflowing.
//! * **`w:widowControl`** — no single line of a paragraph is left alone at the foot of a column or
//!   carried alone to the top of the next. It can only ever move lines *forward*, and it is switched
//!   off entirely for a paragraph that has nowhere forward to go, so the count of lines placed is
//!   monotone.
//! * **`w:keepNext`** — the block shares a column with the one after it. This is the classic
//!   infinite loop: a chain longer than a column has no satisfying assignment, so the chain is
//!   broken at the point where breaking it would leave the column **empty**, and the first block
//!   of an unsatisfiable chain is placed where it does not fit. `tests/termination.rs` runs a
//!   thousand-paragraph chain and asserts it produces pages.
//!
//! Above all of them sits the one guarantee the whole engine rests on: **every page places at least
//! one unit.** A page that placed nothing would produce a next position equal to its own start, and
//! the page after it would be identical, for ever. Every constraint above is allowed to refuse
//! content only while something else is already on the page.
//!
//! # A block is a paragraph or a table, and the loop does not care which
//!
//! MJXOFF-176 widened the content this fills a column with. A [`crate::block::BlockLayout`] has
//! units — a paragraph's lines, a table's [`crate::table::Slice`]s — and everything above is stated
//! in units. Two things a table brings are named rather than special-cased: `w:cantSplit` makes a
//! row **one** unit, so it moves whole through the same arithmetic `w:keepLines` already used, and
//! `w:tblHeader` makes a continuation taller than its units through
//! [`crate::block::BlockLayout::repeated_height`]. That is why `w:keepNext` still works across a
//! table: there is one ordered list of content on a page, not two.
//!
//! # Floats, and why they are resolved here rather than before
//!
//! A `wp:anchor` positioned `relativeFrom="paragraph"` cannot be placed until its paragraph's top is
//! known, and that is known only when the column has been filled down to it. So exclusions
//! accumulate **as the column fills**, and a block is laid out against the ones that exist by the
//! time it is reached. The block that anchors a float is laid out twice — once to settle its own top,
//! once against the float that top produced — and **never three times**, because the floats a block
//! anchors are a function of the block and its top, not of its lines.
//!
//! # Columns, and why balancing is a search rather than a division
//!
//! Filling *n* columns is filling one column *n* times and threading the position through, which is
//! the whole of [`assemble`] once [`fill_column`] exists. **Balancing is not.** A section that ends
//! at a `continuous` break has its columns levelled — the obvious implementation, dividing the total
//! height by the column count, is wrong, because content is placed in whole units and a paragraph
//! may not be splittable at all. What is actually wanted is *the shortest column height at which the
//! remaining content still fits in n columns*, and that is a **monotone predicate**: content placed
//! never decreases as the height grows. So [`assemble`] **bisects** it, which terminates in
//! `log2` of the page height in EMU — around thirty-one fills of already-laid-out blocks — and
//! is exact rather than approximate.

use std::collections::BTreeMap;

use mjx_docx::{BlockFormatting, DrawingFormatting, DrawingPlacement, ParagraphFormatting};
use mjx_ooxml_core::measure::Emu;
use mjx_ooxml_types::wordprocessingml::BreakType;

use crate::block::BlockLayout;
use crate::float::{self, Anchorage, PlacedFloat};
use crate::wrap::Exclusion;

/// Where in the document a page starts.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Debug)]
pub struct FlowPosition {
    /// Which block, counted from zero across the flow.
    pub block: u32,
    /// Which of its units, counted from zero — a paragraph's line, or a table's slice. Zero is the
    /// start of the block.
    pub unit: u32,
}

impl FlowPosition {
    /// The start of the document.
    pub const START: Self = Self { block: 0, unit: 0 };

    /// The start of the block at `index`.
    #[must_use]
    pub fn at(index: usize) -> Self {
        Self {
            block: u32::try_from(index).unwrap_or(u32::MAX),
            unit: 0,
        }
    }
}

/// One block, or the part of one, placed in a column.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PlacedBlock {
    /// Which block of the flow.
    pub block: usize,
    /// Which of its units are here.
    pub units: std::ops::Range<usize>,
    /// Where the first of them sits, from the top of the column — the repeated header, when there is
    /// one, occupies `top..top + repeated_header` and the units start below it.
    pub top: Emu,
    /// How much empty space was left above it.
    pub space_before: Emu,
    /// How tall a repeated table heading is drawn above `units`, or zero.
    pub repeated_header: Emu,
    /// The third field of the [`LayoutCache`] key this block's layout is under.
    ///
    /// Zero unless a float was in the column, which is what makes the lookup exact rather than a
    /// range query: a paragraph beside a float has one layout per position, and emitting fragments
    /// from the wrong one would draw the right text at the wrong widths.
    pub key_top: i64,
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
    /// The section's last block was placed.
    SectionEnded,
}

/// One column, filled.
#[derive(Clone, PartialEq, Debug)]
pub struct ColumnFill {
    /// What is in it, in document order.
    pub blocks: Vec<PlacedBlock>,
    /// The floating objects anchored in it, in the order their anchors were reached.
    pub floats: Vec<PlacedFloat>,
    /// Where the content after it starts, or `None` when the document ended here.
    pub next: Option<FlowPosition>,
    /// How tall the content is, from the column's top.
    pub used: Emu,
    /// Why it stopped.
    pub ended: ColumnEnd,
}

/// One page's body, assembled.
#[derive(Clone, PartialEq, Debug)]
pub struct PageAssembly {
    /// Its columns, left to right. Never empty.
    pub columns: Vec<ColumnFill>,
    /// Where the next page starts, or `None` when the document ended here.
    pub next: Option<FlowPosition>,
    /// Whether the page ended because its section did.
    pub ended_section: bool,
    /// How many blocks this page's assembly had to lay out.
    ///
    /// **The instrument the checkpoint gate rests on**, and it counts *work* rather than output: a
    /// page assembled from a checkpoint looks at the blocks on it, and a page assembled by
    /// walking from the beginning looks at every block before it too. A gate on the *fragments*
    /// cannot tell the two apart, because they produce the same page — which is exactly why a
    /// checkpoint that is never used still passes every output assertion.
    pub paragraphs_visited: u32,
}

impl PageAssembly {
    /// Every block on the page, column by column, with the column index each came from.
    pub fn blocks(&self) -> impl Iterator<Item = (usize, &PlacedBlock)> {
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

/// The content one column is filled from: the block tree, and the paragraphs it indexes.
///
/// Two slices rather than one owned tree, because both already exist — the flow's own program and
/// `mjx-docx`'s one flat paragraph list — and copying either to pass it here would undo the residency
/// this crate is built on.
#[derive(Clone, Copy, Debug)]
pub struct FlowProgram<'a> {
    /// The blocks, in document order.
    pub blocks: &'a [BlockFormatting],
    /// The paragraphs a [`BlockFormatting::Paragraph`] indexes.
    pub paragraphs: &'a [ParagraphFormatting],
}

impl FlowProgram<'_> {
    /// The paragraph at block `index`, when that block is one.
    #[must_use]
    pub fn paragraph(&self, index: usize) -> Option<&ParagraphFormatting> {
        match self.blocks.get(index)? {
            BlockFormatting::Paragraph(at) => self.paragraphs.get(*at),
            BlockFormatting::Table(_) => None,
        }
    }

    /// Whether the blocks at `one` and `other` are paragraphs naming the same `w:pStyle`.
    ///
    /// Two paragraphs that name **no** style are the same style: that is the document's default
    /// paragraph style, which is what an unstyled body is made of, and reading `None` as *different*
    /// would turn `w:contextualSpacing` off for exactly the documents that use it most. A table is
    /// never the same style as anything — it has none — so a paragraph next to a table keeps its
    /// space.
    #[must_use]
    pub fn same_style(&self, one: usize, other: usize) -> bool {
        match (self.paragraph(one), self.paragraph(other)) {
            (Some(left), Some(right)) => left.style_id() == right.style_id(),
            _ => false,
        }
    }

    /// The drawings block `index` anchors, or an empty slice.
    #[must_use]
    pub fn floats_of(&self, index: usize) -> &[DrawingFormatting] {
        self.paragraph(index)
            .map_or(&[][..], ParagraphFormatting::drawings)
    }

    /// Every float block `index` anchors, placed against `frame`.
    #[must_use]
    pub fn place_floats(&self, index: usize, frame: Anchorage) -> Vec<PlacedFloat> {
        self.floats_of(index)
            .iter()
            .enumerate()
            .filter(|(_, drawing)| matches!(drawing.placement, DrawingPlacement::Anchored(_)))
            .filter_map(|(at, drawing)| float::place(drawing, index, at, frame))
            .collect()
    }

    /// The unit a hard break inside block `index` ends the column at, and which kind it was.
    ///
    /// A table has no `w:br` of its own; one inside a cell ends a line in that cell and not the page,
    /// which is why this asks only paragraphs.
    #[must_use]
    pub fn hard_break_within(
        &self,
        index: usize,
        layout: &BlockLayout,
        from: usize,
        count: usize,
    ) -> Option<(usize, BreakType)> {
        let paragraph = self.paragraph(index)?;
        let lines = &layout.as_paragraph()?.lines;
        for break_at in paragraph.hard_breaks() {
            if !matches!(break_at.kind, BreakType::Page | BreakType::Column) {
                continue;
            }
            for offset in 0..count {
                let at = from + offset;
                let Some(line) = lines.get(at) else {
                    break;
                };
                if break_at.at >= line.range.start && break_at.at < line.range.end {
                    return Some((at, break_at.kind));
                }
            }
        }
        None
    }
}

/// What one block's layout is asked for.
#[derive(Clone, Copy, Debug)]
pub struct LayoutRequest<'a> {
    /// The measure it is fitted against.
    pub width: Emu,
    /// Where in the column it starts, which is what decides which exclusions its lines meet.
    pub top: Emu,
    /// The floats already placed in this column, in the column's own coordinates.
    pub exclusions: &'a [Exclusion],
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
    /// The last block of this page's section, inclusive — past it, the page ends.
    pub section_last: Option<usize>,
    /// Whether to level the columns when the section's content ends on this page.
    pub balance: bool,
    /// The frames a float anchored on this page is positioned against, before the column's own left
    /// edge is applied.
    pub frame: Anchorage,
}

/// The layouts one page's assembly reused, so a fixed point over the note area and a balancing
/// search cost no further block layouts.
///
/// # Keyed by block, column **width** and the height it starts at
///
/// A paragraph's lines depend on the measure they were fitted against, so a paragraph that appears
/// in two columns of different widths has two layouts and a cache keyed by paragraph alone would
/// hand the second column the first one's lines — a visible overrun that no assertion on *which*
/// paragraph is where can see. Keying by the width rather than by the column index is what keeps
/// the common case free: `w:equalWidth` columns are all the same measure, so they share one layout,
/// and a single-column document behaves exactly as it did before columns existed.
///
/// **The third field is zero unless the column actually carries a float.** A paragraph beside a
/// wrapped object has different lines depending on where down the column it sits, so it genuinely
/// has one layout per position; a paragraph in a column with nothing floating in it does not, and
/// pays nothing for the possibility. That is what keeps
/// [`PageAssembly::paragraphs_visited`] comparable with MJXOFF-174's own numbers on every document
/// without a drawing in it — which is every fixture the checkpoint gate uses.
pub type LayoutCache = BTreeMap<(usize, i64, i64), BlockLayout>;

/// Assembles the page that starts at `from`.
///
/// `layout_of` is asked for a block's units and must answer the same units for the same request
/// however many times it is asked — that equivalence is what makes page *N* alone and
/// pages 1..=*N* in order agree.  `layouts` is the memo that makes asking cheap; it is the caller's
/// so that laying the same page out twice — which is what the footnote fixed point does — costs one
/// set of block layouts and not two.
///
/// # Errors
/// Whatever `layout_of` fails with.
pub fn assemble<E>(
    program: FlowProgram<'_>,
    from: FlowPosition,
    shape: PageShape<'_>,
    layouts: &mut LayoutCache,
    layout_of: &mut dyn FnMut(usize, LayoutRequest<'_>) -> Result<BlockLayout, E>,
) -> Result<PageAssembly, E> {
    let before = layouts.len();
    let mut columns = fill_all(program, from, shape, layouts, layout_of)?;

    // Balancing, and the one condition it applies under: the section's content ran out on this page,
    // so there is a fixed amount of it and levelling the columns is a question with an answer. A
    // page whose columns are full has nothing to balance — the content does not fit either way.
    if shape.balance && shape.columns > 1 && ends_here(&columns, shape.section_last) {
        if let Some(levelled) = balance(program, from, shape, layouts, layout_of)? {
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
    program: FlowProgram<'_>,
    from: FlowPosition,
    shape: PageShape<'_>,
    layouts: &mut LayoutCache,
    layout_of: &mut dyn FnMut(usize, LayoutRequest<'_>) -> Result<BlockLayout, E>,
) -> Result<Vec<ColumnFill>, E> {
    fill_at(program, from, shape, shape.height, layouts, layout_of)
}

/// The same at a stated column height, which is what balancing varies.
///
/// A page whose first column was ended by a page break, by the end of the section or by the end of
/// the document still **has** its other columns — a two-column page cut short is a two-column page —
/// so they are pushed empty rather than omitted, carrying the reason the page stopped.
fn fill_at<E>(
    program: FlowProgram<'_>,
    from: FlowPosition,
    shape: PageShape<'_>,
    height: Emu,
    layouts: &mut LayoutCache,
    layout_of: &mut dyn FnMut(usize, LayoutRequest<'_>) -> Result<BlockLayout, E>,
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
                floats: Vec::new(),
                next: position,
                used: Emu::ZERO,
                ended,
            });
            continue;
        }
        let start = position.unwrap_or(from);
        let page_empty = filled.iter().all(|fill| fill.blocks.is_empty());
        let width = shape
            .widths
            .get(filled.len())
            .copied()
            .unwrap_or(shape.width);
        // **The column's *width* varies per column and its *height* deliberately does not.** A
        // float's rectangle must not depend on the trial height this fill is running at, for two
        // reasons that are both correctness rather than efficiency:
        //
        // * the footnote fixed point assembles the body at a **reduced** height and then again at
        //   another, and its two-assembly proof rests on *the body content placed is non-increasing
        //   in the reservation* — which would be false if a float anchored to the column's bottom
        //   moved between the two, because the second assembly could then place text the first did
        //   not and the iteration would have no bound at all;
        // * the balancing search bisects thirty-one heights, and a float that moved with each of
        //   them would make the predicate it is searching non-monotone.
        //
        // So `frame.column_height` is the page's own body height, set once by the caller, and this
        // loop never touches it. See `crate::notes` for the proof this preserves.
        let mut frame = shape.frame;
        frame.column_width = width;
        let fill = fill_column(
            program,
            start,
            ColumnShape {
                height,
                width,
                section_last: shape.section_last,
                page_empty,
                frame,
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
        (Some(next), Some(last)) => next.block as usize > last,
        (Some(_), None) => false,
    }
}

/// Levels the columns of a section that ends on this page.
///
/// # The search, and why it is a search
///
/// The content on the page is fixed — it is everything from `from` to the end of the section — so
/// the question is *how short may a column be and still hold a `columns`-th of it*. Dividing the
/// total height by the count answers a different question, because units are indivisible and a
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
    program: FlowProgram<'_>,
    from: FlowPosition,
    shape: PageShape<'_>,
    layouts: &mut LayoutCache,
    layout_of: &mut dyn FnMut(usize, LayoutRequest<'_>) -> Result<BlockLayout, E>,
) -> Result<Option<Vec<ColumnFill>>, E> {
    let mut low = Emu::ZERO;
    let mut high = shape.height;
    while low < high {
        let middle = low + (high - low).divided_by(2);
        if middle == low {
            break;
        }
        let trial = fill_at(program, from, shape, middle, layouts, layout_of)?;
        if ends_here(&trial, shape.section_last) {
            high = middle;
        } else {
            low = middle;
        }
    }
    let levelled = fill_at(program, from, shape, high, layouts, layout_of)?;
    if ends_here(&levelled, shape.section_last) {
        Ok(Some(levelled))
    } else {
        Ok(None)
    }
}

/// What one column is: how big it is, where its section ends, whether the page it is on has
/// anything on it yet, and the frame a float anchored in it is positioned against.
///
/// The `page_empty` flag is what `w:pageBreakBefore` reads — a break at the very top of a page would
/// open a blank one — and it is a fact about the **page**, not the column, which is why it travels
/// beside the measurements rather than being inferred from them.
#[derive(Clone, Copy, Debug)]
pub struct ColumnShape {
    /// How tall it is.
    pub height: Emu,
    /// How wide, which is the measure its paragraphs were fitted against.
    pub width: Emu,
    /// The last paragraph of the section it belongs to, inclusive.
    pub section_last: Option<usize>,
    /// Whether the page holds nothing at all yet.
    pub page_empty: bool,
    /// The frames a floating object anchored in this column is positioned against.
    pub frame: Anchorage,
}

impl ColumnShape {
    /// A column with no page or margin outside it — what a header, a note or a table cell gives.
    #[must_use]
    pub fn contained(height: Emu, width: Emu) -> Self {
        Self {
            height,
            width,
            section_last: None,
            page_empty: true,
            frame: Anchorage::contained(width, height, Emu::ZERO),
        }
    }
}

/// Fills one column from `from`, stopping at `shape.height`.
///
/// # Errors
/// Whatever `layout_of` fails with.
#[allow(clippy::too_many_lines)]
pub fn fill_column<E>(
    program: FlowProgram<'_>,
    from: FlowPosition,
    shape: ColumnShape,
    layouts: &mut LayoutCache,
    layout_of: &mut dyn FnMut(usize, LayoutRequest<'_>) -> Result<BlockLayout, E>,
) -> Result<ColumnFill, E> {
    let ColumnShape {
        height,
        width,
        section_last,
        page_empty,
        frame,
    } = shape;
    let mut blocks: Vec<PlacedBlock> = Vec::new();
    let mut floats: Vec<PlacedFloat> = Vec::new();
    let mut exclusions: Vec<Exclusion> = Vec::new();
    let mut y = Emu::ZERO;
    let mut index = from.block as usize;
    let mut unit = from.unit as usize;
    let mut next: Option<FlowPosition> = None;
    let mut ended = ColumnEnd::ContentEnded;

    while index < program.blocks.len() {
        if section_last.is_some_and(|last| index > last) {
            next = Some(FlowPosition::at(index));
            ended = ColumnEnd::SectionEnded;
            break;
        }

        // A block is laid out at the height it starts at, because a float beside it displaces
        // different lines depending on where down the column it sits. **When the column carries no
        // floats at all the key's third field is zero**, so a document with no drawings in it caches
        // exactly as MJXOFF-174 did and `PageAssembly::paragraphs_visited` is comparable with that
        // child's own numbers.
        let anchored_here = program.floats_of(index);
        let mut layout = cached(layouts, index, width, y, &exclusions, layout_of)?;

        // A float anchored in this very block wraps this block's own text, and where it sits depends
        // on the block's top — which depends on its space-before, which is a property of the layout.
        // So the block is laid out once without its own floats, its top is settled, its floats are
        // placed, and it is laid out **once** more. Two passes, never three: the second pass cannot
        // add a float, because the floats a block anchors do not depend on its lines.
        let suppressed = unit > 0
            || blocks.is_empty()
            || (layout.constraints().contextual_spacing
                && program.same_style(index, index.wrapping_sub(1)));
        let space_before = if suppressed {
            Emu::ZERO
        } else {
            layout.space_before()
        };
        if !anchored_here.is_empty() {
            let top = y + space_before;
            let placed = program.place_floats(index, frame.at_paragraph(top));
            let added = placed.iter().any(|float| float.exclusion.is_some());
            for float in placed {
                if let Some(exclusion) = float.exclusion.clone() {
                    exclusions.push(exclusion);
                }
                floats.push(float);
            }
            if added {
                layout = cached(layouts, index, width, top, &exclusions, layout_of)?;
            }
        }

        let total = layout.unit_count();
        if unit >= total {
            index += 1;
            unit = 0;
            continue;
        }

        let constraints = layout.constraints();
        if constraints.page_break_before
            && unit == 0
            && index > 0
            && !(page_empty && blocks.is_empty())
        {
            next = Some(FlowPosition::at(index));
            ended = ColumnEnd::PageBreak;
            break;
        }

        // A continuation of a table redraws its heading rows, which cost height the units themselves
        // do not carry.
        let repeats_header = unit >= layout.repeated_units() && unit > 0;
        let header_height = if repeats_header {
            layout.repeated_height()
        } else {
            Emu::ZERO
        };

        let available = height - y - space_before - header_height;
        let mut fitted = units_that_fit(&layout, unit, available);
        let remaining = total - unit;

        if constraints.keep_units_together && fitted < remaining {
            fitted = if blocks.is_empty() { remaining } else { 0 };
        }

        if layout.widow_control() {
            fitted = widow_control(fitted, remaining);
        }

        if fitted == 0 {
            if blocks.is_empty() {
                fitted = 1;
            } else {
                next = Some(FlowPosition {
                    block: index as u32,
                    unit: unit as u32,
                });
                ended = ColumnEnd::Filled;
                break;
            }
        }

        let hard = program.hard_break_within(index, &layout, unit, fitted);
        let (fitted, hard_ended) = match hard {
            Some((at_unit, kind)) => (
                at_unit - unit + 1,
                Some(match kind {
                    BreakType::Page => ColumnEnd::PageBreak,
                    _ => ColumnEnd::ColumnBreak,
                }),
            ),
            None => (fitted, None),
        };

        blocks.push(PlacedBlock {
            block: index,
            units: unit..unit + fitted,
            top: y + space_before,
            space_before,
            repeated_header: header_height,
            key_top: if exclusions.is_empty() {
                NO_FLOATS
            } else {
                (y + space_before).emu()
            },
            continued: unit > 0,
            continues: unit + fitted < total,
        });
        y = y + space_before + header_height + layout.height_of(unit..unit + fitted);

        if hard_ended.is_some() || unit + fitted < total {
            let resume = unit + fitted;
            next = Some(if resume >= total {
                FlowPosition::at(index + 1)
            } else {
                FlowPosition {
                    block: index as u32,
                    unit: resume as u32,
                }
            });
            ended = hard_ended.unwrap_or(ColumnEnd::Filled);
            break;
        }

        if section_last.is_some_and(|last| index >= last) {
            if index + 1 >= program.blocks.len() {
                next = None;
                ended = ColumnEnd::ContentEnded;
            } else {
                next = Some(FlowPosition::at(index + 1));
                ended = ColumnEnd::SectionEnded;
            }
            break;
        }

        if !(constraints.contextual_spacing && program.same_style(index, index + 1)) {
            y += layout.space_after();
        }
        index += 1;
        unit = 0;
    }

    if next.is_some() && !matches!(ended, ColumnEnd::SectionEnded) {
        apply_keep_with_next(&mut blocks, layouts, width, &mut next);
    }

    let used = blocks.last().map_or(Emu::ZERO, |last| {
        let height = layouts
            .get(&(last.block, width.emu(), last.key_top))
            .map_or(Emu::ZERO, |layout| layout.height_of(last.units.clone()));
        last.top + last.repeated_header + height
    });
    Ok(ColumnFill {
        blocks,
        floats,
        next,
        used,
        ended,
    })
}

/// The key a block's layout is filed under: the block, the measure, and — only when the column
/// actually carries a float — the height it starts at.
#[must_use]
pub fn cache_key(index: usize, width: Emu, top: Emu, floating: bool) -> (usize, i64, i64) {
    (
        index,
        width.emu(),
        if floating { top.emu() } else { NO_FLOATS },
    )
}

/// The third key field of a block laid out with nothing floating beside it.
///
/// **A sentinel and not zero.** A block at the very top of a column has `top == 0`, so zero would
/// make *laid out with no floats* and *laid out at the top of a column that has one* the same key —
/// and the block that anchors the float is laid out at exactly that position, so the second layout
/// would hit the first one's entry and the float would change nothing at all. That was a live defect
/// for the length of one test run, and it produced a wrapping engine whose every geometric unit test
/// passed and whose documents were unchanged.
///
/// # ⚠ MJXOFF-177 did **not** widen the key, and the reason is the whole compatibility argument
///
/// A block's layout is now a function of its *composition* as well as its measure — the list marker,
/// the field values, the note marks and the inline objects [`crate::generated`] splices in. So the
/// obvious question is whether the key needs a fourth field, and the answer is no because every one
/// of those is **constant for a whole layout run**:
///
/// * a list marker is a function of the paragraph's position in the document, computed once by
///   [`crate::lists::ListNumbering::read`];
/// * a note mark is a function of the paragraph;
/// * an inline object's extent is a function of the drawing or the equation;
/// * a field's value comes from a [`FieldEnvironment`](crate::fields::FieldEnvironment), which
///   [`crate::model::DocumentFlow::with_fields`] fixes before the first page is laid out — see
///   [`crate::fields`] for why a body `PAGE` field reads *the page its block started on last pass*
///   rather than *the page being assembled*, which is exactly what would have made this key wrong;
/// * a [`RevisionView`](crate::revision::RevisionView) is fixed on the flow the same way.
///
/// **The one thing that is not constant is a stream's own page number**, and a header, a footer and
/// a note are laid out afresh per page and reach no cache at all. If a later child caches them, this
/// key is where the page number has to go — and it must be gated the way this constant now is, by a
/// test that reads a *document* and not only a geometry.
pub const NO_FLOATS: i64 = i64::MIN;

/// One block's layout, from the memo or from `layout_of`.
fn cached<E>(
    layouts: &mut LayoutCache,
    index: usize,
    width: Emu,
    top: Emu,
    exclusions: &[Exclusion],
    layout_of: &mut dyn FnMut(usize, LayoutRequest<'_>) -> Result<BlockLayout, E>,
) -> Result<BlockLayout, E> {
    let key = cache_key(index, width, top, !exclusions.is_empty());
    if let Some(found) = layouts.get(&key) {
        return Ok(found.clone());
    }
    let laid_out = layout_of(
        index,
        LayoutRequest {
            width,
            top,
            exclusions,
        },
    )?;
    layouts.insert(key, laid_out.clone());
    Ok(laid_out)
}

/// How many of `layout`'s units from `unit` onward fit in `available`.
fn units_that_fit(layout: &BlockLayout, unit: usize, available: Emu) -> usize {
    if available <= Emu::ZERO {
        return 0;
    }
    let mut used = Emu::ZERO;
    let mut fitted = 0_usize;
    for candidate in unit..layout.unit_count() {
        let next = used + layout.unit_height(candidate);
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
        return fitted;
    }
    let mut fitted = fitted;
    if remaining - fitted == 1 && fitted >= 2 {
        fitted -= 1;
    }
    if fitted == 1 {
        fitted = 0;
    }
    fitted
}

/// Moves a trailing `w:keepNext` chain on to the next column.
///
/// Only a block placed **in full** can be moved: one split across the boundary already has its last
/// unit in the same column as what follows it, which is what `w:keepNext` asks for.
fn apply_keep_with_next(
    blocks: &mut Vec<PlacedBlock>,
    layouts: &LayoutCache,
    width: Emu,
    next: &mut Option<FlowPosition>,
) {
    loop {
        if blocks.len() < 2 {
            return;
        }
        let Some(last) = blocks.last() else {
            return;
        };
        let Some(layout) = layouts.get(&(last.block, width.emu(), last.key_top)) else {
            return;
        };
        if !layout.constraints().keep_with_next || last.units.end != layout.unit_count() {
            return;
        }
        let moved = FlowPosition {
            block: last.block as u32,
            unit: last.units.start as u32,
        };
        if next.is_some_and(|position| position <= moved) {
            return;
        }
        *next = Some(moved);
        blocks.pop();
    }
}
