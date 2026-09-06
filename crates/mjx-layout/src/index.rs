//! [`SpatialIndex`] — where everything on a page is, so that a hit test is a query and not a walk.
//!
//! # Why a walk is not good enough
//!
//! A page of prose holds tens of thousands of fragments. A hit test runs on every mouse move — for
//! the cursor shape, for a hover highlight, for a tooltip — so at sixty frames a second a linear
//! walk is a million rectangle tests a second, on the main thread, for one pointer. A viewport cull
//! runs on every scrolled frame and asks the same question of a rectangle. Both want an index.
//!
//! # Why a uniform grid
//!
//! The candidates were a bounding-volume hierarchy, an R-tree, an interval structure over the `y`
//! axis, and a uniform grid. The grid wins here for reasons that are specific to *this* data:
//!
//! * **The data is a page.** Every entry is inside one bounded rectangle whose size is known before
//!   the first fragment is added, so the grid's one real weakness — unbounded, clustered coordinate
//!   spaces — does not apply.
//! * **The build is two counting passes and two allocations.** A tree is a sort plus a recursion,
//!   and this index is rebuilt for every page of every re-layout, so build time is not amortised
//!   over many queries the way a static scene's would be.
//! * **It has no pathological query.** A tree degenerates when the boxes overlap heavily, which is
//!   exactly what a fragment tree does: every glyph run is inside a line is inside a paragraph is
//!   inside a page, so a hit near the middle of a page is inside four or five nested rectangles by
//!   construction.
//!
//! Storage is a CSR (compressed sparse row) pair — one offset per cell, one entry per
//! cell-membership — rather than a `Vec` per cell, so a 64×64 grid is two allocations rather than
//! four thousand.
//!
//! # The one thing a grid needs a rule for
//!
//! A fragment that covers most of the page would be written into most of the cells, and a page
//! background covers all of them. So a fragment spanning more than [`MAXIMUM_CELL_SPAN`] cells goes
//! into an **oversized** list that every query also scans. That list is short by construction — a
//! page has a handful of page-sized fragments and thousands of line-sized ones — and it is what
//! keeps the entry count linear in the number of fragments rather than quadratic in the page.
//!
//! # What "correct" means here, and how it is checked
//!
//! An index is only ever an accelerator: for any query, it must return **exactly** what a linear
//! scan over the same predicate would. That is a property that can be tested rather than argued, and
//! `tests/spatial_index.rs` tests it against a brute-force scan over randomised fragment sets — the
//! index is never compared against itself.

use crate::fragment::{FragmentId, FragmentTree};
use crate::measure::{LayoutPoint, LayoutRect};
use mjx_ooxml_core::measure::Emu;

/// The most grid cells one fragment is written into before it goes to the oversized list.
///
/// Sixteen: a fragment covering a 4×4 patch of a 64×64 grid is already a sixteenth of the page in
/// each direction, which on a letter page is about two inches. Below that, writing it into every
/// cell it touches is cheaper than scanning it on every query; above, it is not.
pub const MAXIMUM_CELL_SPAN: usize = 16;

/// The most cells the grid is ever divided into, in each direction.
///
/// A page with a million fragments does not want a thousand-column grid: the offset table is
/// `columns × rows + 1` entries whatever the fragment count, so the grid is sized from the fragment
/// count and clamped here.
pub const MAXIMUM_GRID_SIDE: u32 = 64;

/// Where every fragment on one page is.
///
/// Built by [`SpatialIndex::build`] from a finished [`FragmentTree`] and immutable afterwards, which
/// is what makes it safe to hand a query's answer to a caller that still holds the tree.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SpatialIndex {
    /// The area the grid covers: the union of every fragment's page-space bounds.
    covered: LayoutRect,
    columns: u32,
    rows: u32,
    /// Every fragment's page-space bounding box, indexed by [`FragmentId`]. This is the *exact*
    /// predicate's input, so a query never has to go back to the tree.
    bounds: Vec<LayoutRect>,
    /// CSR offsets: cell `c` owns `entries[starts[c]..starts[c + 1]]`.
    starts: Vec<u32>,
    entries: Vec<FragmentId>,
    /// Fragments too big to write into cells; scanned by every query.
    oversized: Vec<FragmentId>,
}

impl SpatialIndex {
    /// Build the index for `tree`.
    ///
    /// Runs once per page, at the end of layout, from the finished tree — which is why
    /// [`PageFragments::new`](crate::PageFragments::new) does it rather than leaving it to each box
    /// model. Building it in bulk rather than incrementally is what makes it two counting passes
    /// instead of a growing structure.
    #[must_use]
    pub fn build(tree: &FragmentTree) -> Self {
        let bounds: Vec<LayoutRect> = tree
            .ids()
            .map(|id| tree.page_bounds(id).unwrap_or(LayoutRect::ZERO))
            .collect();

        let covered = bounds
            .iter()
            .copied()
            .fold(LayoutRect::ZERO, LayoutRect::union);

        // One cell per fragment is the classic sizing: it makes the expected occupancy of a cell one
        // entry. Clamped at both ends — never zero, never past `MAXIMUM_GRID_SIDE`.
        let side = integer_square_root(bounds.len()).clamp(1, MAXIMUM_GRID_SIDE);
        let (columns, rows) = if covered.is_empty() {
            (1, 1)
        } else {
            (side, side)
        };
        let cells = (columns as usize).saturating_mul(rows as usize);

        let mut counts = vec![0_u32; cells];
        let mut oversized = Vec::new();
        // Pass one: count what goes in each cell, and take the oversized aside.
        for (index, rect) in bounds.iter().enumerate() {
            let id = FragmentId::from_index(index);
            match cell_span(covered, columns, rows, *rect) {
                None => oversized.push(id),
                Some(span) => {
                    for cell in span.cells(columns) {
                        if let Some(count) = counts.get_mut(cell) {
                            *count = count.saturating_add(1);
                        }
                    }
                }
            }
        }

        // Pass two: turn the counts into CSR offsets.
        let mut starts = Vec::with_capacity(cells + 1);
        let mut running = 0_u32;
        starts.push(0);
        for count in &counts {
            running = running.saturating_add(*count);
            starts.push(running);
        }

        // Pass three: fill, using a moving cursor per cell.
        let mut cursor = starts.clone();
        let mut entries = vec![FragmentId::from_index(0); running as usize];
        for (index, rect) in bounds.iter().enumerate() {
            let id = FragmentId::from_index(index);
            let Some(span) = cell_span(covered, columns, rows, *rect) else {
                continue;
            };
            for cell in span.cells(columns) {
                let Some(position) = cursor.get_mut(cell) else {
                    continue;
                };
                if let Some(slot) = entries.get_mut(*position as usize) {
                    *slot = id;
                }
                *position = position.saturating_add(1);
            }
        }

        Self {
            covered,
            columns,
            rows,
            bounds,
            starts,
            entries,
            oversized,
        }
    }

    /// How many fragments are indexed.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bounds.len()
    }

    /// Whether nothing is indexed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bounds.is_empty()
    }

    /// The area the grid covers — the union of every fragment's page-space bounds.
    #[must_use]
    pub fn covered_area(&self) -> LayoutRect {
        self.covered
    }

    /// How the grid is divided, as columns by rows. Diagnostic; the answers do not depend on it, and
    /// `tests/spatial_index.rs` asserts exactly that by comparing against a brute-force scan across
    /// a range of fragment counts that move it.
    #[must_use]
    pub fn grid(&self) -> (u32, u32) {
        (self.columns, self.rows)
    }

    /// A fragment's page-space bounding box, as the index recorded it.
    #[must_use]
    pub fn bounds_of(&self, id: FragmentId) -> Option<LayoutRect> {
        self.bounds.get(id.index() as usize).copied()
    }

    /// How much memory the index holds.
    #[must_use]
    pub fn heap_bytes(&self) -> usize {
        self.bounds.capacity() * std::mem::size_of::<LayoutRect>()
            + self.starts.capacity() * std::mem::size_of::<u32>()
            + self.entries.capacity() * std::mem::size_of::<FragmentId>()
            + self.oversized.capacity() * std::mem::size_of::<FragmentId>()
    }

    /// Every fragment whose page-space bounds contain `point`, in ascending [`FragmentId`] order —
    /// which is paint order, so the **last** is the one on top and the one a click belongs to.
    ///
    /// This is the broad phase. A rotated fragment's bounds are bigger than the fragment, so a caller
    /// that cares confirms with
    /// [`FragmentTree::contains_page_point`]; a caller on a
    /// page with no rotation — which is every slide without `a:xfrm@rot` and every page of prose —
    /// needs no narrow phase at all, because there the bounds *are* the fragment.
    #[must_use]
    pub fn fragments_at(&self, point: LayoutPoint) -> Vec<FragmentId> {
        let mut found = Vec::new();
        for id in &self.oversized {
            if self.contains(*id, point) {
                found.push(*id);
            }
        }
        if let Some(cell) = self.cell_of(point) {
            for id in self.cell_entries(cell) {
                if self.contains(*id, point) {
                    found.push(*id);
                }
            }
        }
        found.sort_unstable();
        found.dedup();
        found
    }

    /// The topmost fragment whose bounds contain `point`, or `None` when none does.
    ///
    /// "Topmost" is the largest [`FragmentId`], because children are added after their parents and
    /// siblings in paint order — so the innermost, last-painted fragment wins, which is the one a
    /// click is about.
    #[must_use]
    pub fn topmost_at(&self, point: LayoutPoint) -> Option<FragmentId> {
        let mut best: Option<FragmentId> = None;
        for id in &self.oversized {
            if self.contains(*id, point) && best.is_none_or(|current| *id > current) {
                best = Some(*id);
            }
        }
        if let Some(cell) = self.cell_of(point) {
            for id in self.cell_entries(cell) {
                if self.contains(*id, point) && best.is_none_or(|current| *id > current) {
                    best = Some(*id);
                }
            }
        }
        best
    }

    /// Every fragment whose page-space bounds meet `rect`, in ascending [`FragmentId`] order — a
    /// viewport cull, and the query a repaint of a scrolled page is built on.
    #[must_use]
    pub fn fragments_intersecting(&self, rect: LayoutRect) -> Vec<FragmentId> {
        let mut found = Vec::new();
        for id in &self.oversized {
            if self.meets(*id, rect) {
                found.push(*id);
            }
        }
        if let Some(span) = cell_span(self.covered, self.columns, self.rows, rect) {
            for cell in span.cells(self.columns) {
                for id in self.cell_entries(cell) {
                    if self.meets(*id, rect) {
                        found.push(*id);
                    }
                }
            }
        } else {
            // The query rectangle itself spans more cells than the span cap allows, so there is no
            // cheaper answer than every cell. This is the whole-page repaint, and it is linear in
            // the number of fragments rather than in the number of cells because each fragment is
            // visited once per cell it occupies and then deduplicated.
            for id in &self.entries {
                if self.meets(*id, rect) {
                    found.push(*id);
                }
            }
        }
        found.sort_unstable();
        found.dedup();
        found
    }

    fn contains(&self, id: FragmentId, point: LayoutPoint) -> bool {
        self.bounds
            .get(id.index() as usize)
            .is_some_and(|rect| rect.contains(point))
    }

    fn meets(&self, id: FragmentId, rect: LayoutRect) -> bool {
        self.bounds
            .get(id.index() as usize)
            .is_some_and(|bounds| bounds.intersects(rect))
    }

    fn cell_entries(&self, cell: usize) -> &[FragmentId] {
        let (Some(start), Some(end)) = (self.starts.get(cell), self.starts.get(cell + 1)) else {
            return &[];
        };
        self.entries
            .get(*start as usize..*end as usize)
            .unwrap_or(&[])
    }

    fn cell_of(&self, point: LayoutPoint) -> Option<usize> {
        if !self.covered.contains(point) {
            return None;
        }
        let column = axis_cell(self.covered.left, self.covered.right, self.columns, point.x);
        let row = axis_cell(self.covered.top, self.covered.bottom, self.rows, point.y);
        Some((row as usize).saturating_mul(self.columns as usize) + column as usize)
    }
}

/// A rectangular block of grid cells.
#[derive(Clone, Copy, Debug)]
struct CellSpan {
    first_column: u32,
    last_column: u32,
    first_row: u32,
    last_row: u32,
}

impl CellSpan {
    fn cells(self, columns: u32) -> impl Iterator<Item = usize> {
        (self.first_row..=self.last_row).flat_map(move |row| {
            (self.first_column..=self.last_column).map(move |column| {
                (row as usize).saturating_mul(columns as usize) + column as usize
            })
        })
    }
}

/// Which cells `rect` covers, or `None` when it covers more than [`MAXIMUM_CELL_SPAN`] of them or
/// none at all.
fn cell_span(covered: LayoutRect, columns: u32, rows: u32, rect: LayoutRect) -> Option<CellSpan> {
    if rect.is_empty() || covered.is_empty() {
        return None;
    }
    let Some(overlap) = covered.intersection(rect) else {
        // Nothing in the grid can meet it, so it belongs in no cell — but it must still be
        // reachable, or a query with a rectangle outside the covered area would miss a fragment
        // outside it too. `covered` is the union of every fragment's bounds, so a *fragment* can
        // never land here; a *query* rectangle can, and the `None` sends it to the exhaustive path.
        return None;
    };
    let first_column = axis_cell(covered.left, covered.right, columns, overlap.left);
    let last_column = axis_cell(
        covered.left,
        covered.right,
        columns,
        overlap.right - Emu::from_emu(1),
    );
    let first_row = axis_cell(covered.top, covered.bottom, rows, overlap.top);
    let last_row = axis_cell(
        covered.top,
        covered.bottom,
        rows,
        overlap.bottom - Emu::from_emu(1),
    );
    let width = (last_column.saturating_sub(first_column) as usize).saturating_add(1);
    let height = (last_row.saturating_sub(first_row) as usize).saturating_add(1);
    if width.saturating_mul(height) > MAXIMUM_CELL_SPAN {
        return None;
    }
    Some(CellSpan {
        first_column,
        last_column,
        first_row,
        last_row,
    })
}

/// Which cell along one axis `value` falls in, clamped into the grid.
fn axis_cell(low: Emu, high: Emu, cells: u32, value: Emu) -> u32 {
    let cells = cells.max(1);
    let extent = (high - low).emu();
    if extent <= 0 {
        return 0;
    }
    let offset = (value - low).emu().max(0);
    // `offset` and `extent` are both non-negative `i64`, and `cells` is small, so the product is
    // computed in `i128` to keep a page the size of the universe from wrapping.
    let scaled = (offset as i128) * i128::from(cells) / (extent as i128);
    let index = scaled.clamp(0, i128::from(cells) - 1);
    // Clamped into `0..cells`, so the cast is exact.
    index as u32
}

/// The integer square root of `value`, for sizing the grid. Small and exact; `f64::sqrt` on a
/// `usize` past 2^53 would not be.
fn integer_square_root(value: usize) -> u32 {
    if value == 0 {
        return 0;
    }
    let mut low = 1_u64;
    let mut high = (value as u64).min(u64::from(u32::MAX));
    while low < high {
        let middle = low + (high - low).div_ceil(2);
        if middle.saturating_mul(middle) <= value as u64 {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    // `low` is bounded by `u32::MAX` above, so the cast is exact.
    low as u32
}
