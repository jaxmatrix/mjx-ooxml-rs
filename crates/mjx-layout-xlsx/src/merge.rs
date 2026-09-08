//! [`MergeIndex`] — which merged region covers a cell, and what its four edges are drawn from.
//!
//! # Why an index rather than the list
//!
//! `WorksheetPart::merged_range_containing` is a linear scan of `x:mergeCells`, which is right for a
//! one-off question and wrong for a viewport: a screen of two thousand cells against a sheet with a
//! thousand merges is two million comparisons a frame. This sorts the ranges by their first row once
//! and answers each cell in `O(log m + overlap)`.
//!
//! # The anchor owns the content, and the covered cells own nothing
//!
//! That is not this crate's invention: `mjx-xlsx` already answers
//! [`SheetFormatting::effective_merged_cell_format`](mjx_xlsx::SheetFormatting::effective_merged_cell_format)
//! with *the anchor's* format for every covered position, and `mjx-sml`'s own merge model refuses to
//! create or clear the covered cells. So a merged region renders **once**, at the union rectangle,
//! from the top-left cell — and every other position it covers draws nothing at all, not even its
//! own background.
//!
//! A naive per-cell walk gets this wrong in a way that is instantly visible: it draws the anchor's
//! text and border inside the anchor's own small rectangle, leaves gridlines through the middle of
//! the region, and paints the covered cells' own fills over the top.
//!
//! # ⚠ Where the four borders come from is a reading, not a rule
//!
//! ECMA-376 says what `mergeCell` is and says nothing about how a consumer draws the region's
//! outline. Two authoring styles exist in real files: Excel's own *Format Cells* on a merged region
//! writes the border onto the perimeter cells' `xf`s, while a file written by a library often states
//! it only on the anchor. Honouring one and not the other loses a border a person can see.
//!
//! GUESS: so each edge of the union takes **the first stated border along that edge**, scanning from
//! the anchor outward, and falls back to the anchor's own corresponding edge when no perimeter cell
//! states one. A region whose left column states a left border therefore gets it, and one whose
//! anchor alone states it gets it too. Which of the two Excel actually draws when they *disagree* is
//! for the Windows sitting.

use mjx_sml::{CellRange, CellReference};

/// One merged region, as a rectangle of zero-based indices.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MergedRegion {
    /// The anchor's row — the top of the region, and the cell whose content and format render.
    pub first_row: u32,
    /// The anchor's column.
    pub first_column: u16,
    /// The last row, inclusive.
    pub last_row: u32,
    /// The last column, inclusive.
    pub last_column: u16,
}

impl MergedRegion {
    /// The region a `mergeCell@ref` names, normalised so that `first` really is the top-left.
    #[must_use]
    pub fn from_range(range: CellRange) -> Self {
        let bounds = range.normalized_bounds();
        Self {
            first_row: bounds.first_row(),
            first_column: bounds.first_column(),
            last_row: bounds.last_row(),
            last_column: bounds.last_column(),
        }
    }

    /// Whether `row`/`column` is inside the region.
    #[must_use]
    pub fn contains(&self, row: u32, column: u16) -> bool {
        row >= self.first_row
            && row <= self.last_row
            && column >= self.first_column
            && column <= self.last_column
    }

    /// Whether `row`/`column` is the anchor — the one position that renders.
    #[must_use]
    pub fn is_anchor(&self, row: u32, column: u16) -> bool {
        row == self.first_row && column == self.first_column
    }

    /// How many rows it spans, never zero.
    #[must_use]
    pub fn row_span(&self) -> u32 {
        self.last_row
            .saturating_sub(self.first_row)
            .saturating_add(1)
    }

    /// How many columns it spans, never zero.
    #[must_use]
    pub fn column_span(&self) -> u16 {
        self.last_column
            .saturating_sub(self.first_column)
            .saturating_add(1)
    }

    /// The anchor as a reference.
    ///
    /// # Errors
    /// [`mjx_sml::AddressError`] when the anchor is outside the grid, which a parsed `mergeCell@ref`
    /// never is.
    pub fn anchor(&self) -> Result<CellReference, mjx_sml::AddressError> {
        CellReference::relative(self.first_column, self.first_row)
    }

    /// Every cell along one edge of the region, from the anchor outward — the order the border
    /// resolution scans in.
    #[must_use]
    pub fn edge_cells(&self, edge: RegionEdge) -> Vec<(u32, u16)> {
        match edge {
            RegionEdge::Left => (self.first_row..=self.last_row)
                .map(|row| (row, self.first_column))
                .collect(),
            RegionEdge::Right => (self.first_row..=self.last_row)
                .map(|row| (row, self.last_column))
                .collect(),
            RegionEdge::Top => (self.first_column..=self.last_column)
                .map(|column| (self.first_row, column))
                .collect(),
            RegionEdge::Bottom => (self.first_column..=self.last_column)
                .map(|column| (self.last_row, column))
                .collect(),
        }
    }
}

/// Which side of a merged region a border is being resolved for.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum RegionEdge {
    /// The left of the union.
    Left,
    /// The right.
    Right,
    /// The top.
    Top,
    /// The bottom.
    Bottom,
}

impl RegionEdge {
    /// All four, in the order a decoration lists them.
    pub const ALL: [Self; 4] = [Self::Left, Self::Right, Self::Top, Self::Bottom];
}

/// Every merged region of a sheet, indexed for a viewport.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct MergeIndex {
    /// Sorted by `first_row`, then by `first_column`.
    regions: Vec<MergedRegion>,
    /// The deepest a region reaches below its own first row, so a search can start early enough.
    ///
    /// Without it a binary search on `first_row` would miss a tall region that starts far above the
    /// row being asked about; with it, the scan starts at `row - tallest` and is still bounded.
    tallest: u32,
}

impl MergeIndex {
    /// The index over a worksheet's `x:mergeCells`.
    ///
    /// # Errors
    /// [`mjx_sml::SmlError`] when a `mergeCell@ref` is absent or will not parse.
    pub fn read(worksheet: &mjx_sml::WorksheetPart) -> Result<Self, mjx_sml::SmlError> {
        let ranges = worksheet.merged_ranges()?;
        let mut regions: Vec<MergedRegion> =
            ranges.into_iter().map(MergedRegion::from_range).collect();
        regions.sort_unstable_by_key(|region| (region.first_row, region.first_column));
        let tallest = regions
            .iter()
            .map(|region| region.last_row.saturating_sub(region.first_row))
            .max()
            .unwrap_or(0);
        Ok(Self { regions, tallest })
    }

    /// How many merged regions the sheet has.
    #[must_use]
    pub fn len(&self) -> usize {
        self.regions.len()
    }

    /// Whether the sheet has none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.regions.is_empty()
    }

    /// Every region, sorted by top-left.
    #[must_use]
    pub fn regions(&self) -> &[MergedRegion] {
        &self.regions
    }

    /// The region covering `row`/`column`, or `None`.
    #[must_use]
    pub fn covering(&self, row: u32, column: u16) -> Option<MergedRegion> {
        if self.regions.is_empty() {
            return None;
        }
        let from = row.saturating_sub(self.tallest);
        let at = self
            .regions
            .partition_point(|region| region.first_row < from);
        for region in &self.regions[at..] {
            if region.first_row > row {
                break;
            }
            if region.contains(row, column) {
                return Some(*region);
            }
        }
        None
    }

    /// Every region whose rows overlap `rows` — what a band of the sheet has to draw, including the
    /// regions whose anchors are above the band.
    #[must_use]
    pub fn overlapping(&self, rows: std::ops::Range<u32>) -> Vec<MergedRegion> {
        if self.regions.is_empty() || rows.is_empty() {
            return Vec::new();
        }
        let from = rows.start.saturating_sub(self.tallest);
        let at = self
            .regions
            .partition_point(|region| region.first_row < from);
        self.regions[at..]
            .iter()
            .take_while(|region| region.first_row < rows.end)
            .filter(|region| region.last_row >= rows.start)
            .copied()
            .collect()
    }
}
