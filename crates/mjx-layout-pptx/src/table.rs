//! Table layout: a grid of rectangles, and one text body in each cell that renders.
//!
//! # The grid is the authority, and a merge never removes a cell
//!
//! `docs/TABLES_HANDOFF.md`'s first decision is the one that shapes this module: **merging never
//! removes a cell.** A merged region is anchored at its top-left cell, which states `@gridSpan` /
//! `@rowSpan`; every position it covers stays in the table stating `@hMerge` / `@vMerge`. So the
//! grid is rectangular, `(row, column)` addressing has no holes, and the *only* thing that decides
//! whether a position draws is [`Cell::covered_by`](crate::Cell::covered_by).
//!
//! That is exactly the shape of the classic defect. A walk that visits every `a:tc` and draws it at
//! the rectangle of its own row and column draws **every** cell of a merged region, one on top of
//! the other, each at one cell's size — so a 1×3 merge renders as three unmerged cells and the
//! anchor's text is cut off at the first column's edge. It looks nearly right, which is why
//! `tests/merged_cells_are_not_a_naive_walk.rs` asserts the anchor's rectangle spans its columns
//! rather than asserting that something was drawn.
//!
//! # Row heights are a minimum, not a size
//!
//! `a:tr@h` is required by the schema and is nonetheless not the row's height: PowerPoint grows a
//! row whose text does not fit and never shrinks one whose text is short. That is why laying a
//! table out takes **two passes** over its text — one against an unbounded height to learn what
//! each cell needs, and one against the final rectangle so that a cell anchored to its bottom is
//! anchored to the right bottom. The alternative, laying out once and translating afterwards, gets
//! `@anchor` wrong for every cell in a row that grew.
//!
//! # What is a `GUESS:` here
//!
//! ECMA-376 says what the attributes are and nothing about how a renderer distributes slack, so
//! three readings are marked at their sites: which row of a vertical merge absorbs the growth, what
//! a column that states no width is worth, and which of two adjacent cells wins the edge they share.

use mjx_dml::{TextAnchoring, TextBodyPropertiesSpec, TextWrapping};
use mjx_layout::{LayoutRect, LayoutSize};
use mjx_ooxml_core::measure::Emu;

use crate::autofit::AutofitPolicy;
use crate::body::{self, PlacedBody};
use crate::deck::{Cell, CellInsets, TableContent, TextBody};
use crate::error::SlideLayoutError;
use crate::text::TextEngine;

/// A table, laid out.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct PlacedTable {
    /// The rectangle the grid occupies — the frame's origin, and the grid's own total size, which
    /// may be larger than the frame when the columns sum wider or a row grew.
    pub rect: LayoutRect,
    /// Each column's resolved width, left to right.
    pub columns: Vec<Emu>,
    /// Each row's resolved height, top to bottom, after growth.
    pub rows: Vec<Emu>,
    /// The cells that render, in row-major order. A position a merge covers is **not** here.
    pub cells: Vec<PlacedCell>,
}

impl PlacedTable {
    /// How many cells render — which is the grid's size less the positions merges cover.
    #[must_use]
    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }
}

/// One rendered cell of a table.
#[derive(Clone, PartialEq, Debug)]
pub struct PlacedCell {
    /// Its row in the whole grid, counted from zero.
    pub row: usize,
    /// Its column, counted from zero.
    pub column: usize,
    /// How many rows it spans; one when it is not merged down.
    pub row_span: usize,
    /// How many columns it spans; one when it is not merged across.
    pub column_span: usize,
    /// Its rectangle — the union of every grid position it covers.
    pub rect: LayoutRect,
    /// Its text, laid out inside it, or `None` when it has none.
    pub body: Option<PlacedBody>,
}

/// Lays `table` out inside `frame`.
///
/// # Errors
/// [`SlideLayoutError`] as [`body::lay_out`] reports it — a face that cannot be resolved, or a run
/// that cannot be shaped.
pub fn lay_out(
    engine: &mut TextEngine<'_>,
    frame: LayoutRect,
    table: &TableContent,
) -> Result<PlacedTable, SlideLayoutError> {
    let columns = column_widths(table, frame.width());
    let offsets = running_offsets(frame.left, &columns);

    // Pass one: what does each cell's text need, given the width its columns give it?
    let mut required = vec![Emu::ZERO; table.row_count()];
    for (row, column, cell) in rendered(table) {
        let Some(body) = &cell.body else {
            continue;
        };
        let width = span_extent(&columns, column, cell.column_span);
        let measured = measure(engine, width, cell, body)?;
        // GUESS: a cell merged down absorbs its growth into the **last** row of its span, so that
        // the rows above keep the heights they state. PowerPoint's own choice here has never been
        // written down; growing the first row instead moves every row below it, which is the more
        // visible of the two wrong answers.
        let last = row.saturating_add(cell.row_span.max(1)).saturating_sub(1);
        if let Some(slot) = required.get_mut(last.min(table.row_count().saturating_sub(1))) {
            *slot = (*slot).max(measured);
        }
    }

    let rows = row_heights(table, &required);
    let row_offsets = running_offsets(frame.top, &rows);

    // Pass two: place every cell that renders, and lay its text out in the rectangle it really has.
    let mut cells = Vec::with_capacity(table.cells.len());
    for (row, column, cell) in rendered(table) {
        let left = offsets.get(column).copied().unwrap_or(frame.left);
        let top = row_offsets.get(row).copied().unwrap_or(frame.top);
        let rect = LayoutRect::from_edges(
            left,
            top,
            left + span_extent(&columns, column, cell.column_span),
            top + span_extent(&rows, row, cell.row_span),
        );
        let body = match &cell.body {
            None => None,
            Some(body) => Some(body::lay_out(
                engine,
                rect,
                &TextBody {
                    geometry: cell_geometry(cell, cell.anchor.unwrap_or(TextAnchoring::Top)),
                    paragraphs: body.paragraphs.clone(),
                },
                // A cell has no `a:normAutofit`: `a:tcPr` carries no autofit at all, and the row
                // grows instead. Asking the autofit search to run here would shrink text that the
                // grid was about to make room for.
                AutofitPolicy::Disabled,
            )?),
        };
        cells.push(PlacedCell {
            row,
            column,
            row_span: cell.row_span.max(1),
            column_span: cell.column_span.max(1),
            rect,
            body,
        });
    }

    let total = LayoutSize::new(sum(&columns), sum(&rows));
    Ok(PlacedTable {
        rect: LayoutRect::from_origin_and_size(frame.origin(), total),
        columns,
        rows,
        cells,
    })
}

/// Every grid position that renders, with the row and column it sits at.
///
/// The `cells` vector is row-major over the whole grid, so a position's coordinates are its index
/// and nothing else — and a grid with no columns has no positions at all, which is a table stating
/// an empty `a:tblGrid`. That is legal, and it is why the column count is checked rather than
/// divided by.
fn rendered(table: &TableContent) -> impl Iterator<Item = (usize, usize, &Cell)> {
    let columns = table.column_count();
    table
        .cells
        .iter()
        .enumerate()
        .filter(move |_| columns > 0)
        .filter(|(_, cell)| cell.covered_by.is_none())
        .map(move |(index, cell)| (index / columns.max(1), index % columns.max(1), cell))
}

/// Each column's width.
///
/// A column that states one gets it. `docs/TABLES_HANDOFF.md`'s fourth decision makes `a:tblGrid`
/// the authority on how many columns there are, and its `@w` is required by the schema — so a
/// column with no width is a malformed file rather than a normal one.
///
/// GUESS: such a column is given an equal share of whatever the frame's width has left over, and
/// the frame's own width when nothing is left. PowerPoint's behaviour on a file it would not have
/// written is not specified anywhere, and an equal share is the only answer that keeps the grid
/// inside the frame the author drew.
fn column_widths(table: &TableContent, frame_width: Emu) -> Vec<Emu> {
    let stated: Emu = table
        .columns
        .iter()
        .filter_map(|width| *width)
        .fold(Emu::ZERO, |total, width| total + width);
    let unstated = table.columns.iter().filter(|width| width.is_none()).count();
    let share = if unstated == 0 {
        Emu::ZERO
    } else {
        let spare = frame_width - stated.min(frame_width);
        Emu::from_emu(spare.emu() / i64::try_from(unstated).unwrap_or(1).max(1))
    };
    table
        .columns
        .iter()
        .map(|width| width.unwrap_or(share).max(Emu::ZERO))
        .collect()
}

/// Each row's height: the larger of what it states and what its text needs.
///
/// A row never shrinks below its stated height and never leaves its text cut off, which is what
/// PowerPoint does and is the only reading under which a deck written by PowerPoint reproduces
/// itself: the heights in the file are already grown.
fn row_heights(table: &TableContent, required: &[Emu]) -> Vec<Emu> {
    table
        .rows
        .iter()
        .enumerate()
        .map(|(index, stated)| {
            let stated = stated.unwrap_or(Emu::ZERO).max(Emu::ZERO);
            stated.max(required.get(index).copied().unwrap_or(Emu::ZERO))
        })
        .collect()
}

/// The running offsets of `extents`, starting at `origin`.
fn running_offsets(origin: Emu, extents: &[Emu]) -> Vec<Emu> {
    let mut offsets = Vec::with_capacity(extents.len());
    let mut running = origin;
    for extent in extents {
        offsets.push(running);
        running += *extent;
    }
    offsets
}

/// How far `span` tracks reach from `start`.
fn span_extent(extents: &[Emu], start: usize, span: usize) -> Emu {
    extents
        .iter()
        .skip(start)
        .take(span.max(1))
        .fold(Emu::ZERO, |total, extent| total + *extent)
}

/// Everything in `extents`, added up.
fn sum(extents: &[Emu]) -> Emu {
    extents
        .iter()
        .fold(Emu::ZERO, |total, extent| total + *extent)
}

/// How tall `cell`'s text is, given a cell `width` and as much height as it likes.
///
/// The measuring pass. The rectangle is a column of `width` and a height nothing can overflow, so
/// the answer is the text's own height plus the cell's two vertical insets, and the anchor cannot
/// affect it — which is why the measure is taken at [`TextAnchoring::Top`] whatever the cell says.
fn measure(
    engine: &mut TextEngine<'_>,
    width: Emu,
    cell: &Cell,
    body: &TextBody,
) -> Result<Emu, SlideLayoutError> {
    // Tall enough that no realistic cell's text reaches the bottom, and small enough that adding it
    // to a coordinate cannot overflow: one hundred metres, in EMU.
    const UNBOUNDED: Emu = Emu::from_emu(36_000_000_000);

    let rect = LayoutRect::from_edges(Emu::ZERO, Emu::ZERO, width, UNBOUNDED);
    let placed = body::lay_out(
        engine,
        rect,
        &TextBody {
            geometry: cell_geometry(cell, TextAnchoring::Top),
            paragraphs: body.paragraphs.clone(),
        },
        AutofitPolicy::Disabled,
    )?;
    let bottom = placed
        .lines()
        .map(|line| line.rect.bottom)
        .fold(placed.content.top, Emu::max);
    let resolved = insets(cell);
    Ok(bottom - placed.content.top + resolved.top + resolved.bottom)
}

/// The body properties a cell's `a:tcPr` amounts to.
///
/// A cell states its insets and its anchor on `a:tcPr` rather than on an `a:bodyPr`, so this is the
/// translation between the two — and it is a translation rather than a resolution: every number in
/// it was read from the document, and the four defaults are the schema's own
/// (`@marL`/`@marR` 0.1 inch, `@marT`/`@marB` 0.05 inch, ECMA-376 Part 1 §21.1.3.16).
///
/// Wrapping is [`TextWrapping::Square`] unconditionally, because a cell has no `@wrap`: text in a
/// table cell wraps to the cell, which is what makes a column width mean anything.
fn cell_geometry(cell: &Cell, anchor: TextAnchoring) -> TextBodyPropertiesSpec {
    let insets = insets(cell);
    TextBodyPropertiesSpec::new()
        .with_insets(insets.left, insets.top, insets.right, insets.bottom)
        .with_anchor(anchor)
        .with_wrap(TextWrapping::Square)
        .with_columns(1)
}

/// A cell's four insets, with the schema's defaults substituted for the ones it does not state.
fn insets(cell: &Cell) -> ResolvedInsets {
    let CellInsets {
        left,
        right,
        top,
        bottom,
    } = cell.margins;
    ResolvedInsets {
        left: left.unwrap_or(DEFAULT_HORIZONTAL_MARGIN),
        right: right.unwrap_or(DEFAULT_HORIZONTAL_MARGIN),
        top: top.unwrap_or(DEFAULT_VERTICAL_MARGIN),
        bottom: bottom.unwrap_or(DEFAULT_VERTICAL_MARGIN),
    }
}

/// `@marL` and `@marR`'s schema default — 0.1 inch.
const DEFAULT_HORIZONTAL_MARGIN: Emu = Emu::from_emu(91_440);
/// `@marT` and `@marB`'s schema default — 0.05 inch.
const DEFAULT_VERTICAL_MARGIN: Emu = Emu::from_emu(45_720);

/// A cell's four insets, every one of them stated.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct ResolvedInsets {
    left: Emu,
    top: Emu,
    right: Emu,
    bottom: Emu,
}
