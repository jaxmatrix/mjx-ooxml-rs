//! The sheet grid at the package tier: merging, row and column geometry, and the anomaly report.
//!
//! `mjx-sml` answers *what a merged range is* and *what splitting a column run means*; this file
//! answers *which part in this package holds the merge covering B3*, and hands back the one thing
//! the markup tier cannot reach on its own — the **effective format of a merged range**, which
//! needs `xl/styles.xml` as well as the worksheet.
//!
//! # Read, edit, write — once per call, and why that is the right cost
//!
//! Every mutation here is [`Workbook::worksheet_markup`] → one call on the model →
//! [`Workbook::write_worksheet_markup`], exactly as [`Workbook::set_cell_value`] is. That reparses
//! the worksheet per call, which is deliberate: a cached [`WorksheetPart`] would hold the packed
//! store alive for as long as the workbook is open, and `docs/BENCHMARKS.md` is the reason
//! `crates/mjx-xlsx/src/worksheet/grid.rs` refuses to do that for cells. A caller making many
//! geometry edits holds one [`WorksheetPart`] itself and writes it back once; these calls are for
//! the one-off.
//!
//! # Isolation is the model's, not this file's
//!
//! Hiding one row leaves every other row and every other worksheet child byte-identical because
//! [`WorksheetPart`]'s slot-level copy-on-write and the cell store's row-level copy-on-write say so.
//! Nothing here adds isolation and nothing here can take it away — which is why
//! `crates/mjx-xlsx/tests/worksheet_part.rs` asserts it against the committed fixture's own bytes
//! rather than against a second run of this crate's writer.

use mjx_sml::{
    CellRange, CellReference, CellSpan, ColumnWidth, EffectiveCellFormat, GridAnomaly, RowHeight,
    WorksheetPart,
};

use crate::error::XlsxError;
use crate::workbook::Workbook;
use crate::worksheet::formatting::SheetFormatting;

impl Workbook {
    /// Every merged range on the tab at `index`, in document order.
    ///
    /// An empty list for a tab whose part is not an `x:worksheet`, and for a worksheet that writes
    /// no `x:mergeCells`.
    ///
    /// # Errors
    /// [`XlsxError::NoSuchSheet`] if `index` names no tab; [`XlsxError::Sml`] if a `mergeCell@ref`
    /// is absent or does not parse — see
    /// [`WorksheetPart::merged_ranges`](mjx_sml::WorksheetPart::merged_ranges).
    pub fn merged_ranges(&self, index: usize) -> Result<Vec<CellRange>, XlsxError> {
        let Some(markup) = self.worksheet_markup(index)? else {
            return Ok(Vec::new());
        };
        Ok(markup.merged_ranges()?)
    }

    /// The merged range `reference` belongs to on the tab at `index`, or `None`.
    ///
    /// # Errors
    /// As [`merged_ranges`](Self::merged_ranges).
    pub fn merged_range_containing(
        &self,
        index: usize,
        reference: CellReference,
    ) -> Result<Option<CellRange>, XlsxError> {
        let Some(markup) = self.worksheet_markup(index)? else {
            return Ok(None);
        };
        Ok(markup.merged_range_containing(reference)?)
    }

    /// Records `range` as merged on the tab at `index`.
    ///
    /// Touches no cell: see [`mjx_sml::MergedCells`]'s module documentation for why creating the
    /// covered cells and why clearing their values are both refused.
    ///
    /// # Errors
    /// [`XlsxError::NoSuchSheet`] if `index` names no tab, [`XlsxError::MissingWorkbookPart`] if it
    /// reaches no worksheet part, and [`XlsxError::Sml`] wrapping
    /// [`SmlError::MergeOverlapsExistingMerge`](mjx_sml::SmlError::MergeOverlapsExistingMerge) or
    /// [`SmlError::DegenerateMerge`](mjx_sml::SmlError::DegenerateMerge) when the merge is refused.
    pub fn merge_cells(&mut self, index: usize, range: CellRange) -> Result<(), XlsxError> {
        self.edit_worksheet(index, |markup| {
            markup.merge_cells(range)?;
            Ok(())
        })
    }

    /// Removes the merge covering exactly the cells of `range` from the tab at `index`, reporting
    /// whether one was there.
    ///
    /// # Errors
    /// As [`merge_cells`](Self::merge_cells).
    pub fn unmerge_cells(&mut self, index: usize, range: CellRange) -> Result<bool, XlsxError> {
        let mut removed = false;
        self.edit_worksheet(index, |markup| {
            removed = markup.unmerge_cells(range)?;
            Ok(())
        })?;
        Ok(removed)
    }

    /// Sets the height of the row a file numbered `row` on the tab at `index`.
    ///
    /// The height and its `customHeight` flag arrive as one [`RowHeight`], which is the whole point
    /// of that type: a height whose provenance is unsaid is a height Excel may recompute.
    ///
    /// # Errors
    /// As [`merge_cells`](Self::merge_cells), without the merge refusals.
    pub fn set_row_height(
        &mut self,
        index: usize,
        row: u32,
        height: Option<RowHeight>,
    ) -> Result<(), XlsxError> {
        self.edit_worksheet(index, |markup| {
            markup.set_row_height(row, height)?;
            Ok(())
        })
    }

    /// Hides or shows the row a file numbered `row` on the tab at `index`.
    ///
    /// # Errors
    /// As [`set_row_height`](Self::set_row_height).
    pub fn set_row_hidden(
        &mut self,
        index: usize,
        row: u32,
        hidden: bool,
    ) -> Result<(), XlsxError> {
        self.edit_worksheet(index, |markup| {
            markup.set_row_hidden(row, hidden)?;
            Ok(())
        })
    }

    /// Sets the outline level of the row a file numbered `row` on the tab at `index`, raising
    /// `sheetFormatPr@outlineLevelRow` when the level is deeper than the sheet declares.
    ///
    /// # Errors
    /// As [`set_row_height`](Self::set_row_height).
    pub fn set_row_outline_level(
        &mut self,
        index: usize,
        row: u32,
        level: u8,
    ) -> Result<(), XlsxError> {
        self.edit_worksheet(index, |markup| {
            markup.set_row_outline_level(row, level)?;
            Ok(())
        })
    }

    /// Sets the width of every column of `columns` on the tab at `index`, splitting any `col` run
    /// that reaches outside them.
    ///
    /// See
    /// [`WorksheetPart::set_column_width`](mjx_sml::WorksheetPart::set_column_width) for the four
    /// cases the split has, and [`ColumnWidth`] for why the width and its `customWidth` flag are one
    /// value.
    ///
    /// # Errors
    /// As [`set_row_height`](Self::set_row_height), plus [`XlsxError::Sml`] if a `col` is missing
    /// `@min` or `@max`.
    pub fn set_column_width(
        &mut self,
        index: usize,
        columns: CellSpan,
        width: Option<ColumnWidth>,
    ) -> Result<(), XlsxError> {
        self.edit_worksheet(index, |markup| {
            markup.set_column_width(columns, width)?;
            Ok(())
        })
    }

    /// Hides or shows every column of `columns` on the tab at `index`, splitting any run that
    /// reaches outside them.
    ///
    /// # Errors
    /// As [`set_column_width`](Self::set_column_width).
    pub fn set_column_hidden(
        &mut self,
        index: usize,
        columns: CellSpan,
        hidden: bool,
    ) -> Result<(), XlsxError> {
        self.edit_worksheet(index, |markup| {
            markup.set_column_hidden(columns, hidden)?;
            Ok(())
        })
    }

    /// Sets the outline level of every column of `columns` on the tab at `index`, splitting any run
    /// that reaches outside them and raising `sheetFormatPr@outlineLevelCol` when the level is
    /// deeper than the sheet declares.
    ///
    /// # Errors
    /// As [`set_column_width`](Self::set_column_width).
    pub fn set_column_outline_level(
        &mut self,
        index: usize,
        columns: CellSpan,
        level: u8,
    ) -> Result<(), XlsxError> {
        self.edit_worksheet(index, |markup| {
            markup.set_column_outline_level(columns, level)?;
            Ok(())
        })
    }

    /// The effective format of the cell that actually **renders** at `reference` on the tab at
    /// `index`.
    ///
    /// For a cell outside every merge this is [`effective_cell_format`](Self::effective_cell_format).
    /// For a cell inside one it is the format of the merge's **top-left**, because ECMA-376 Part 1
    /// §18.3.1.55 says *"The formatting and content for the merged range is always stored in the top
    /// left cell."* Resolving the covered cell's own record instead would report the format of a
    /// cell nothing draws.
    ///
    /// This is the one thing this tier adds to the merge model: a merge is in the worksheet part and
    /// a format is in `xl/styles.xml`, so no crate that has never heard of a package can put the two
    /// together.
    ///
    /// `Ok(None)` when [`sheet_formatting`](Self::sheet_formatting) answers `None`.
    ///
    /// # Errors
    /// As [`sheet_formatting`](Self::sheet_formatting), plus [`XlsxError::Sml`] if a `mergeCell@ref`
    /// will not parse or the style index in force names no record in `cellXfs`.
    pub fn effective_merged_cell_format(
        &self,
        index: usize,
        reference: CellReference,
    ) -> Result<Option<EffectiveCellFormat>, XlsxError> {
        let Some(formatting) = self.sheet_formatting(index)? else {
            return Ok(None);
        };
        formatting.effective_merged_cell_format(reference).map(Some)
    }

    /// Everything the grid of the tab at `index` says that a well-formed sheet would not.
    ///
    /// An empty list for a tab whose part is not an `x:worksheet`. Nothing is repaired; see
    /// [`GridAnomaly`].
    ///
    /// # Errors
    /// [`XlsxError::NoSuchSheet`] if `index` names no tab, [`XlsxError::Sml`] if a `col` is missing
    /// `@min` or `@max`.
    pub fn grid_anomalies(&self, index: usize) -> Result<Vec<GridAnomaly>, XlsxError> {
        let Some(markup) = self.worksheet_markup(index)? else {
            return Ok(Vec::new());
        };
        Ok(markup.grid_anomalies()?)
    }

    /// Reads the worksheet part behind the tab at `index`, hands it to `edit`, and writes it back.
    ///
    /// The one shape every mutation on this surface has. A model `edit` leaves untouched — because
    /// it returned early, or because the change was a no-op — writes back the buffer it was read
    /// from, so a failed edit is not a rewritten part.
    pub(crate) fn edit_worksheet(
        &mut self,
        index: usize,
        edit: impl FnOnce(&mut WorksheetPart) -> Result<(), XlsxError>,
    ) -> Result<(), XlsxError> {
        let mut markup = self
            .worksheet_markup(index)?
            .ok_or_else(|| XlsxError::MissingWorkbookPart(format!("sheet {index}")))?;
        edit(&mut markup)?;
        self.write_worksheet_markup(index, &markup)
    }
}

impl SheetFormatting {
    /// The effective format of the cell that actually renders at `reference` — the merge anchor's
    /// when `reference` is covered, and its own otherwise.
    ///
    /// [`Workbook::effective_merged_cell_format`] is this for a single lookup; hold a
    /// [`SheetFormatting`] and call this when there are many, because the two parts are then parsed
    /// once rather than once per cell.
    ///
    /// # Errors
    /// [`XlsxError::Sml`] if a `mergeCell@ref` will not parse, or if the style index in force names
    /// no record in `cellXfs`.
    pub fn effective_merged_cell_format(
        &self,
        reference: CellReference,
    ) -> Result<EffectiveCellFormat, XlsxError> {
        let anchor = self.worksheet().merge_anchor(reference)?;
        self.resolver()?.effective_cell_format(anchor)
    }
}
