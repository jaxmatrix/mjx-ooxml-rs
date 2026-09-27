//! Tables: the grid, the two layout algorithms, the cells, and the **slices** a page boundary cuts
//! between.
//!
//! # A table is a block with units, exactly as a paragraph is
//!
//! MJXOFF-174's paginator fills a column with *blocks*, each of which has some number of indivisible
//! units it may be cut between — for a paragraph, its lines. A table's units are its **slices**: the
//! horizontal bands a page break may fall between, which are the row boundaries *and*, inside a row
//! that may split, the line boundaries every one of its cells agrees on. That is the whole reason
//! this crate did not need a second paginator: `w:cantSplit` is `w:keepLines` at row granularity,
//! `w:tblHeader` is a repeating prefix, and everything else the paginator already knew how to do.
//!
//! **A row's split offsets are the union of its cells' line bottoms.** Cutting there always leaves
//! whole lines in every cell, because a cut at the bottom of *some* cell's line is at or below the
//! bottom of every line that started above it in every other cell. The alternative — cutting at an
//! arbitrary height and clipping — is what a renderer does when it treats a table as an image.
//!
//! # Fixed and auto-fit are genuinely different algorithms, and that is asserted
//!
//! * **Fixed** (`w:tblLayout@type="fixed"`) reads `w:tblGrid` and stops. Content never moves a
//!   column edge, which is the entire point of the setting: an author who has dragged a column
//!   boundary expects it to stay dragged.
//! * **Auto-fit** (the default) is a **constraint solve over content widths**, not a heuristic. Each
//!   cell is measured twice — once at an unbounded measure, which gives the width at which its
//!   content needs no line break at all, and once at a measure of one EMU, which gives the width of
//!   its widest unbreakable piece — and the column widths are then the unique solution that
//!   distributes the slack between those two bounds in proportion to each column's own range. It is
//!   the algorithm HTML's automatic table layout describes, for the same reason: it is what "as wide
//!   as it needs to be, and no wider" means when the total is constrained.
//!
//! The two produce **different numbers for the same content**, which is what
//! `tests/a_table_splits_across_a_page.rs` asserts rather than asserting that either is right.
//!
//! # ⚠ Provenance
//!
//! §17.4.52 defines `w:tblLayout`'s two values in one sentence each and defines neither algorithm.
//! §17.4.80's `w:trHeight` rules and §17.4.19's `w:tblHeader` are `SpecCode`; everything about *how*
//! a width is solved, where a row splits, and what a vertically merged cell does to a row's height is
//! `EngineDerived` and marked `GUESS:` at the site.

use std::ops::Range;

use mjx_docx::{
    BlockFormatting, CellFormatting, DocumentLayoutSettings, ParagraphFormatting, RowFormatting,
    TableFormatting,
};
use mjx_ooxml_core::measure::Emu;
use mjx_ooxml_types::wordprocessingml::{HeightRule, VerticalJustification};

use crate::block::{BlockConstraints, BlockLayout};
use crate::float::{self, Anchorage};
use crate::flow::ParagraphLayout;
use crate::paginate::LayoutRequest;
use crate::wrap::Exclusion;

/// The measure a cell is given when the question is *how wide would this be with no line breaks*.
///
/// Ten metres. Not near [`Emu`]'s own ceiling, because a measure is added to indents and compared
/// against accumulated widths, and a value that large would overflow those sums rather than answer
/// the question.
pub const UNBOUNDED_MEASURE: Emu = Emu::from_emu(360_000_000);

/// One band of a table that a page break may fall between.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Slice {
    /// Which row it belongs to.
    pub row: usize,
    /// Where it starts, from the top of the table.
    pub top: Emu,
    /// How tall it is.
    pub height: Emu,
    /// Whether it is the first slice of its row — where the horizontal rule above it is drawn, and
    /// what a page-assignment assertion reads.
    pub starts_row: bool,
}

/// One cell, laid out.
#[derive(Clone, PartialEq, Debug)]
pub struct CellLayout {
    /// The first grid column it covers.
    pub column: usize,
    /// How many (`w:gridSpan`).
    pub span: usize,
    /// Its left edge, from the table's left.
    pub left: Emu,
    /// Its full width, margins included.
    pub width: Emu,
    /// Where its content starts, from the table's left.
    pub content_left: Emu,
    /// How wide its content may be.
    pub content_width: Emu,
    /// How far below the cell's top its content starts.
    pub content_top: Emu,
    /// How tall its content came out.
    pub content_height: Emu,
    /// `w:vMerge`: `Some(true)` for the anchor, `Some(false)` for a covered continuation.
    pub vertical_merge: Option<bool>,
    /// `w:vAlign`.
    pub vertical_alignment: VerticalJustification,
    /// What is in it, each positioned from the cell's content top.
    pub content: Vec<PlacedCellBlock>,
}

/// One block inside a cell, placed.
#[derive(Clone, PartialEq, Debug)]
pub struct PlacedCellBlock {
    /// Where it starts, from the cell's content top.
    pub top: Emu,
    /// Which paragraph of the stream it is, when it is one.
    pub paragraph: Option<usize>,
    /// Its layout — a nested table is a [`BlockLayout::Table`] here, which is the whole of
    /// "nested tables to arbitrary depth".
    pub layout: Box<BlockLayout>,
}

/// One row, laid out.
#[derive(Clone, PartialEq, Debug)]
pub struct RowLayout {
    /// Its final height.
    pub height: Emu,
    /// Where it starts, from the table's top.
    pub top: Emu,
    /// Its cells, in order.
    pub cells: Vec<CellLayout>,
    /// `w:tblHeader`.
    pub repeat_as_header: bool,
    /// `w:cantSplit`.
    pub cannot_split: bool,
}

/// A whole table, laid out.
#[derive(Clone, PartialEq, Debug)]
pub struct TableLayout {
    /// The resolved grid column widths, in order.
    pub columns: Vec<Emu>,
    /// Where each grid column starts, from the table's left.
    pub offsets: Vec<Emu>,
    /// Where the table itself starts, from the column's left — `w:tblInd` and `w:jc` together.
    pub left: Emu,
    /// Its rows.
    pub rows: Vec<RowLayout>,
    /// The flat unit list the paginator cuts between.
    pub slices: Vec<Slice>,
    /// How many leading slices belong to rows that repeat as a heading.
    pub repeated_slices: usize,
    /// Whether the file asked for [`mjx_ooxml_types::wordprocessingml::TableLayoutType::Fixed`].
    pub fixed: bool,
    /// Its pagination constraints.
    pub constraints: BlockConstraints,
    /// `w:tblpPr`, when it floats.
    pub floating: Option<mjx_docx::FloatingTableAnchoring>,
}

impl TableLayout {
    /// How tall slices `units` are.
    #[must_use]
    pub fn height_of(&self, units: Range<usize>) -> Emu {
        self.slices
            .get(units)
            .map(|slice| {
                slice
                    .iter()
                    .fold(Emu::ZERO, |total, unit| total + unit.height)
            })
            .unwrap_or(Emu::ZERO)
    }

    /// How tall the whole table is.
    #[must_use]
    pub fn height(&self) -> Emu {
        self.height_of(0..self.slices.len())
    }

    /// How tall the repeating header is.
    #[must_use]
    pub fn repeated_height(&self) -> Emu {
        self.height_of(0..self.repeated_slices)
    }

    /// Which row the slice at `unit` belongs to.
    #[must_use]
    pub fn row_of(&self, unit: usize) -> Option<usize> {
        self.slices.get(unit).map(|slice| slice.row)
    }

    /// The rows any part of which lies in `units`, first and last inclusive.
    #[must_use]
    pub fn rows_in(&self, units: Range<usize>) -> Option<(usize, usize)> {
        let first = self.slices.get(units.start)?.row;
        let last = self.slices.get(units.end.checked_sub(1)?)?.row;
        Some((first, last))
    }

    /// How wide the table is.
    #[must_use]
    pub fn width(&self) -> Emu {
        self.columns
            .iter()
            .fold(Emu::ZERO, |total, width| total + *width)
    }
}

/// Everything laying a table out needs that is not the table.
#[derive(Clone, Copy, Debug)]
pub struct TableContext<'a> {
    /// The width available to the table.
    pub available: Emu,
    /// The stream the block tree's paragraph indices address.
    pub paragraphs: &'a [ParagraphFormatting],
    /// The document's settings, for the paragraphs inside the cells.
    pub settings: &'a DocumentLayoutSettings,
}

/// Lays `table` out inside `context`.
///
/// `lay_out_paragraph` is asked for one paragraph's lines at a stated measure and must answer the
/// same lines for the same pair every time — the same contract [`crate::paginate::assemble`] states,
/// for the same reason.
///
/// # Errors
/// Whatever `lay_out_paragraph` fails with.
pub fn lay_out<E>(
    table: &TableFormatting,
    context: TableContext<'_>,
    lay_out_paragraph: &mut dyn FnMut(usize, LayoutRequest<'_>) -> Result<ParagraphLayout, E>,
) -> Result<TableLayout, E> {
    let columns = solve_columns(table, context, lay_out_paragraph)?;
    let mut offsets = Vec::with_capacity(columns.len());
    let mut running = Emu::ZERO;
    for width in &columns {
        offsets.push(running);
        running += *width;
    }

    let mut rows: Vec<RowLayout> = Vec::with_capacity(table.rows.len());
    for row in &table.rows {
        rows.push(lay_out_row(
            table,
            row,
            &columns,
            &offsets,
            context,
            lay_out_paragraph,
        )?);
    }
    settle_heights(table, &mut rows);

    let mut top = Emu::ZERO;
    for row in &mut rows {
        row.top = top;
        top += row.height;
    }
    let slices = slice(&rows);
    let repeated_slices = slices
        .iter()
        .take_while(|unit| {
            rows.get(unit.row).is_some_and(|row| row.repeat_as_header)
                && rows
                    .iter()
                    .take(unit.row + 1)
                    .all(|row| row.repeat_as_header)
        })
        .count();

    Ok(TableLayout {
        left: table_left(table, &columns, context.available),
        columns,
        offsets,
        rows,
        slices,
        repeated_slices,
        fixed: matches!(
            table.layout,
            mjx_ooxml_types::wordprocessingml::TableLayoutType::Fixed
        ),
        constraints: BlockConstraints {
            // A table never carries `w:pageBreakBefore` of its own; its first paragraph might, and
            // that is that paragraph's business inside its cell.
            page_break_before: false,
            keep_units_together: false,
            keep_with_next: false,
            // Widow control is a rule about *lines of a paragraph*. A table's units are rows, and
            // refusing to leave one row at the foot of a page is `w:cantSplit`'s job, stated per row
            // by the author rather than assumed here.
            widow_control: false,
            contextual_spacing: false,
        },
        floating: table.floating,
    })
}

/// Where the table's own left edge sits inside its column.
///
/// **`GUESS:`** `w:tblInd` is measured from the column's left edge and `w:jc` centres or right-aligns
/// what is left over. §17.4.64 calls `w:tblInd` "table indent from leading margin" and does not say
/// whether it composes with `w:jc`; adding them would move a centred, indented table twice.
fn table_left(table: &TableFormatting, columns: &[Emu], available: Emu) -> Emu {
    use mjx_ooxml_types::wordprocessingml::TableJustification;
    let width = columns
        .iter()
        .fold(Emu::ZERO, |total, column| total + *column);
    match table.alignment {
        Some(TableJustification::Center) => (available - width).divided_by(2).maximum(Emu::ZERO),
        Some(TableJustification::Right | TableJustification::End) => {
            (available - width).maximum(Emu::ZERO)
        }
        _ => Emu::from_twips(table.indent_twips),
    }
}

/// The grid, resolved — the fixed reading, or the auto-fit solve.
fn solve_columns<E>(
    table: &TableFormatting,
    context: TableContext<'_>,
    lay_out_paragraph: &mut dyn FnMut(usize, LayoutRequest<'_>) -> Result<ParagraphLayout, E>,
) -> Result<Vec<Emu>, E> {
    let count = table
        .column_count()
        .max(table.rows.iter().map(spanned_columns).max().unwrap_or(0))
        .max(1);
    let declared: Vec<Emu> = (0..count)
        .map(|index| Emu::from_twips(table.grid_twips.get(index).copied().unwrap_or(0)))
        .collect();
    let target = preferred_width(table, context.available);

    if matches!(
        table.layout,
        mjx_ooxml_types::wordprocessingml::TableLayoutType::Fixed
    ) {
        return Ok(fixed_columns(&declared, target, context.available));
    }

    // Auto-fit: measure every cell twice, aggregate to columns, then distribute.
    let mut minimum = vec![Emu::ZERO; count];
    let mut maximum = vec![Emu::ZERO; count];
    for row in &table.rows {
        let mut column = row.grid_before;
        for cell in &row.cells {
            let span = cell.grid_span.max(1);
            let (inner_min, inner_max) = measure_cell(cell, lay_out_paragraph)?;
            let padding = cell_padding(table, cell);
            let share = span.max(1);
            for offset in 0..span {
                let Some(index) = column.checked_add(offset) else {
                    break;
                };
                if index >= count {
                    break;
                }
                // A spanned cell's demand is shared equally across the columns it covers.
                // **`GUESS:`** ECMA-376 says nothing; sharing equally is the only distribution that
                // does not depend on an order the file does not state.
                let each_min = (inner_min + padding).divided_by(i64::try_from(share).unwrap_or(1));
                let each_max = (inner_max + padding).divided_by(i64::try_from(share).unwrap_or(1));
                minimum[index] = minimum[index].maximum(each_min);
                maximum[index] = maximum[index].maximum(each_max);
            }
            column += span;
        }
    }
    // A column no cell reached keeps whatever the grid declared for it, so an empty trailing column
    // does not collapse to nothing.
    for index in 0..count {
        if maximum[index] == Emu::ZERO {
            minimum[index] = declared[index];
            maximum[index] = declared[index];
        }
    }
    Ok(distribute(&minimum, &maximum, target))
}

/// The width the table asks for, or the space it has.
fn preferred_width(table: &TableFormatting, available: Emu) -> Emu {
    match table.width {
        Some(width) => match width.twips() {
            Some(twips) if twips > 0 => Emu::from_twips(twips),
            _ => match width.fraction() {
                #[allow(clippy::cast_possible_truncation)]
                Some(fraction) if fraction > 0.0 => {
                    Emu::from_emu((available.emu() as f64 * fraction) as i64)
                }
                _ => available,
            },
        },
        None => available,
    }
}

/// The fixed algorithm: the grid as declared, scaled to `target` when it states one and the grid
/// does not already agree.
fn fixed_columns(declared: &[Emu], target: Emu, available: Emu) -> Vec<Emu> {
    let total = declared.iter().fold(Emu::ZERO, |sum, width| sum + *width);
    if total <= Emu::ZERO {
        // No grid at all: equal columns across the space. **`GUESS:`** §17.4.49 requires
        // `w:tblGrid`, so this is a malformed file; equal columns is what Word shows for one.
        let each = available.divided_by(i64::try_from(declared.len().max(1)).unwrap_or(1));
        return vec![each; declared.len()];
    }
    if target <= Emu::ZERO || target == total {
        return declared.to_vec();
    }
    declared
        .iter()
        .map(|width| Emu::from_emu(width.emu().saturating_mul(target.emu()) / total.emu()))
        .collect()
}

/// The auto-fit distribution.
///
/// Three cases, and the middle one is the solve:
///
/// * everything fits at its **maximum** — the table is content-width and no wider;
/// * nothing fits even at its **minimum** — every column takes its minimum and the table overflows,
///   which is what Word shows and is better than clipping;
/// * otherwise each column takes its minimum plus its own share of the slack, in proportion to how
///   much room it could use. That is the unique assignment for which every column is the same
///   fraction of the way from its minimum to its maximum, which is what makes it a solve rather than
///   a rule of thumb.
fn distribute(minimum: &[Emu], maximum: &[Emu], target: Emu) -> Vec<Emu> {
    let sum_min = minimum.iter().fold(Emu::ZERO, |sum, w| sum + *w);
    let sum_max = maximum.iter().fold(Emu::ZERO, |sum, w| sum + *w);
    if sum_max <= target {
        return maximum.to_vec();
    }
    if sum_min >= target {
        return minimum.to_vec();
    }
    let slack = (target - sum_min).emu();
    let range = (sum_max - sum_min).emu().max(1);
    let mut widths: Vec<Emu> = minimum
        .iter()
        .zip(maximum)
        .map(|(low, high)| {
            let own = (*high - *low).emu();
            *low + Emu::from_emu(own.saturating_mul(slack) / range)
        })
        .collect();
    // Integer division loses a few EMU; the last column absorbs them so the table is exactly as wide
    // as it asked to be. Anything else leaves a hairline gap at the right rule of every table.
    let placed = widths.iter().fold(Emu::ZERO, |sum, w| sum + *w);
    if let Some(last) = widths.last_mut() {
        *last += target - placed;
    }
    widths
}

/// The horizontal space a cell's margins take.
fn cell_padding(table: &TableFormatting, cell: &CellFormatting) -> Emu {
    let (_, _, start, end) = cell.margins.or(table.cell_margins).settled();
    Emu::from_twips(start) + Emu::from_twips(end)
}

/// How many grid columns a row's cells cover.
fn spanned_columns(row: &RowFormatting) -> usize {
    row.grid_before
        + row
            .cells
            .iter()
            .map(|cell| cell.grid_span.max(1))
            .sum::<usize>()
        + row.grid_after
}

/// A cell's content width bounds: the widest it would ever need, and the narrowest it can survive.
fn measure_cell<E>(
    cell: &CellFormatting,
    lay_out_paragraph: &mut dyn FnMut(usize, LayoutRequest<'_>) -> Result<ParagraphLayout, E>,
) -> Result<(Emu, Emu), E> {
    let mut minimum = Emu::ZERO;
    let mut maximum = Emu::ZERO;
    for block in &cell.content {
        match block {
            BlockFormatting::Paragraph(index) => {
                let wide = lay_out_paragraph(
                    *index,
                    LayoutRequest {
                        width: UNBOUNDED_MEASURE,
                        top: Emu::ZERO,
                        exclusions: &[],
                    },
                )?;
                maximum = maximum.maximum(widest_line(&wide));
                let narrow = lay_out_paragraph(
                    *index,
                    LayoutRequest {
                        width: Emu::from_emu(1),
                        top: Emu::ZERO,
                        exclusions: &[],
                    },
                )?;
                minimum = minimum.maximum(widest_line(&narrow));
            }
            BlockFormatting::Table(nested) => {
                // A nested table's own grid is its demand: it cannot be narrower than the sum of its
                // columns' minima, and it does not want to be wider than their maxima.
                let inner = nested
                    .grid_twips
                    .iter()
                    .fold(0_i64, |sum, width| sum.saturating_add(*width));
                minimum = minimum.maximum(Emu::from_twips(inner));
                maximum = maximum.maximum(Emu::from_twips(inner));
            }
        }
    }
    Ok((minimum, maximum))
}

/// The widest line of a laid-out paragraph, its indents included.
fn widest_line(layout: &ParagraphLayout) -> Emu {
    layout
        .lines
        .iter()
        .enumerate()
        .fold(Emu::ZERO, |widest, (index, line)| {
            widest.maximum(layout.style.leading_indent_of(index) + line.placement.natural_width)
        })
}

/// One row, laid out at the settled column widths.
fn lay_out_row<E>(
    table: &TableFormatting,
    row: &RowFormatting,
    columns: &[Emu],
    offsets: &[Emu],
    context: TableContext<'_>,
    lay_out_paragraph: &mut dyn FnMut(usize, LayoutRequest<'_>) -> Result<ParagraphLayout, E>,
) -> Result<RowLayout, E> {
    let spacing = Emu::from_twips(table.cell_spacing_twips);
    let mut cells: Vec<CellLayout> = Vec::with_capacity(row.cells.len());
    let mut column = row.grid_before;
    for cell in &row.cells {
        let span = cell.grid_span.max(1);
        let left = offsets.get(column).copied().unwrap_or(Emu::ZERO);
        let width = (0..span)
            .filter_map(|offset| columns.get(column + offset))
            .fold(Emu::ZERO, |total, each| total + *each);
        let margins = cell.margins.or(table.cell_margins).settled();
        let (top_margin, bottom_margin, start_margin, end_margin) = margins;
        let content_left = left + Emu::from_twips(start_margin) + spacing;
        let content_width = (width
            - Emu::from_twips(start_margin)
            - Emu::from_twips(end_margin)
            - spacing
            - spacing)
            .maximum(Emu::from_emu(1));

        // A `w:vMerge` continuation contributes no content: its text belongs to the anchor above it,
        // and laying it out again would draw the same paragraphs twice.
        let mut content: Vec<PlacedCellBlock> = Vec::new();
        let mut used = Emu::ZERO;
        // **A float anchored inside a cell wraps against the cell**, not against the page: its
        // `Anchorage` is the cell's own content box, so `relativeFrom="column"` means *this cell*.
        // That is what `wp:anchor@layoutInCell` asks for and it is the whole of the interaction the
        // ticket names between the two halves of this child.
        let mut exclusions: Vec<Exclusion> = Vec::new();
        if cell.vertical_merge_anchor != Some(false) {
            for block in &cell.content {
                match block {
                    BlockFormatting::Paragraph(index) => {
                        let mut layout = lay_out_paragraph(
                            *index,
                            LayoutRequest {
                                width: content_width,
                                top: used,
                                exclusions: &exclusions,
                            },
                        )?;
                        let space_before = if content.is_empty() {
                            Emu::ZERO
                        } else {
                            layout.space_before
                        };
                        let anchored = context
                            .paragraphs
                            .get(*index)
                            .map_or(&[][..], mjx_docx::ParagraphFormatting::drawings);
                        if !anchored.is_empty() {
                            let frame = Anchorage::contained(
                                content_width,
                                UNBOUNDED_MEASURE,
                                used + space_before,
                            );
                            let mut added = false;
                            for (at, drawing) in anchored.iter().enumerate() {
                                let Some(placed) = float::place(drawing, *index, at, frame) else {
                                    continue;
                                };
                                if let Some(exclusion) = placed.exclusion {
                                    exclusions.push(exclusion);
                                    added = true;
                                }
                            }
                            if added {
                                layout = lay_out_paragraph(
                                    *index,
                                    LayoutRequest {
                                        width: content_width,
                                        top: used + space_before,
                                        exclusions: &exclusions,
                                    },
                                )?;
                            }
                        }
                        let height = layout.height_of(0..layout.lines.len());
                        let after = layout.space_after;
                        content.push(PlacedCellBlock {
                            top: used + space_before,
                            paragraph: Some(*index),
                            layout: Box::new(BlockLayout::Paragraph(Box::new(layout))),
                        });
                        used = used + space_before + height + after;
                    }
                    BlockFormatting::Table(nested) => {
                        let inner = lay_out(
                            nested,
                            TableContext {
                                available: content_width,
                                ..context
                            },
                            lay_out_paragraph,
                        )?;
                        let height = inner.height();
                        content.push(PlacedCellBlock {
                            top: used,
                            paragraph: None,
                            layout: Box::new(BlockLayout::Table(Box::new(inner))),
                        });
                        used += height;
                    }
                }
            }
        }

        cells.push(CellLayout {
            column,
            span,
            left,
            width,
            content_left,
            content_width,
            content_top: Emu::from_twips(top_margin),
            content_height: used + Emu::from_twips(top_margin) + Emu::from_twips(bottom_margin),
            vertical_merge: cell.vertical_merge_anchor,
            vertical_alignment: cell
                .vertical_alignment
                .unwrap_or(VerticalJustification::Top),
            content,
        });
        column += span;
    }

    Ok(RowLayout {
        height: Emu::ZERO,
        top: Emu::ZERO,
        cells,
        repeat_as_header: row.repeat_as_header,
        cannot_split: row.cannot_split,
    })
}

/// Turns each row's cell heights and its `w:trHeight` into a settled row height.
///
/// # The vertical merge, and where its deficit goes
///
/// A `w:vMerge="restart"` cell's content belongs to the whole run of rows it anchors, so it is not
/// allowed to make its **own** row tall enough on its own — that would produce one deep row followed
/// by several thin ones, which is not what a merged cell looks like. Its own row is therefore sized
/// from the cells that are not merged, and any deficit is added to the **last** row of the merge
/// group. **`GUESS:`** ECMA-376 says nothing at all about this; adding the deficit to the last row is
/// what keeps the merged cell's text inside its own borders, and distributing it evenly would move
/// every rule in the group.
fn settle_heights(table: &TableFormatting, rows: &mut [RowLayout]) {
    for (index, row) in rows.iter_mut().enumerate() {
        let natural = row
            .cells
            .iter()
            .filter(|cell| cell.vertical_merge.is_none())
            .fold(Emu::ZERO, |tallest, cell| {
                tallest.maximum(cell.content_height)
            });
        let requested = table.rows.get(index).and_then(|source| source.height);
        row.height = match requested {
            Some(height) => {
                let stated = Emu::from_twips(height.twips);
                match height.rule {
                    HeightRule::Exact => stated,
                    HeightRule::AtLeast => natural.maximum(stated),
                    HeightRule::Auto => natural,
                }
            }
            None => natural,
        };
        // A row with nothing in it is still a row a caret can enter.
        row.height = row.height.maximum(Emu::from_emu(1));
    }

    // The merge deficits, in a second pass because a group's span is not known during the first.
    let count = rows.len();
    for start in 0..count {
        let anchors: Vec<(usize, Emu)> = rows[start]
            .cells
            .iter()
            .filter(|cell| cell.vertical_merge == Some(true))
            .map(|cell| (cell.column, cell.content_height))
            .collect();
        for (column, demand) in anchors {
            let mut end = start + 1;
            while end < count
                && rows[end]
                    .cells
                    .iter()
                    .any(|cell| cell.column == column && cell.vertical_merge == Some(false))
            {
                end += 1;
            }
            let available = rows[start..end]
                .iter()
                .fold(Emu::ZERO, |total, row| total + row.height);
            if demand > available {
                if let Some(last) = rows.get_mut(end - 1) {
                    last.height += demand - available;
                }
            }
        }
    }
}

/// The flat unit list: where a page break may fall.
///
/// A `w:cantSplit` row contributes **exactly one** slice, of its whole height, and that is the entire
/// implementation of the attribute: a row that is one indivisible unit either fits where it is or
/// moves to the next column whole, through the same arithmetic that already moved a `w:keepLines`
/// paragraph. Nothing downstream has to know the attribute exists.
///
/// Every other row contributes one slice per height at which **all** of its cells have a whole line
/// above the cut — the union of its cells' line bottoms — so a cut never leaves half a line of text
/// on a page.
fn slice(rows: &[RowLayout]) -> Vec<Slice> {
    let mut slices: Vec<Slice> = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        if row.cannot_split {
            slices.push(Slice {
                row: index,
                top: row.top,
                height: row.height,
                starts_row: true,
            });
            continue;
        }
        let mut offsets: Vec<Emu> = Vec::new();
        for cell in &row.cells {
            for block in &cell.content {
                match block.layout.as_ref() {
                    BlockLayout::Paragraph(layout) => {
                        let mut line_bottom = cell.content_top + block.top;
                        for line in &layout.lines {
                            line_bottom += line.height;
                            offsets.push(line_bottom);
                        }
                    }
                    // A nested table is atomic inside its cell: cutting a row of the outer table
                    // through a row of an inner one would need the inner table's own header repeated
                    // inside a cell, which Word does not do either.
                    BlockLayout::Table(inner) => {
                        offsets.push(cell.content_top + block.top + inner.height());
                    }
                }
            }
        }
        offsets.retain(|offset| *offset > Emu::ZERO && *offset < row.height);
        offsets.sort_unstable();
        offsets.dedup();
        offsets.push(row.height);

        let mut previous = Emu::ZERO;
        for (position, offset) in offsets.iter().enumerate() {
            slices.push(Slice {
                row: index,
                top: row.top + previous,
                height: *offset - previous,
                starts_row: position == 0,
            });
            previous = *offset;
        }
    }
    slices
}
