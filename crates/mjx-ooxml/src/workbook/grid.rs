//! The sheet's geometry: merged regions, row heights and column widths, hiding and outlining, and
//! the report of everything a grid says that a well-formed one would not.
//!
//! Every method here names a range or a cell as A1 text and a row or column as a `u32`, so nothing
//! on this surface takes a `(row, column)` pair that could be transposed.

use mjx_sml::{CellRange, CellReference, CellSpan, ColumnWidth, GridAnomaly, RowHeight};

use crate::index::count;

use crate::error::Error;
use crate::index::index;

use super::Workbook;

impl Workbook {
    /// Every merged range on one sheet, in document order, as A1 text.
    ///
    /// # Errors
    /// [`ErrorCode::IndexOutOfRange`](crate::ErrorCode::IndexOutOfRange) if `sheet` names no tab,
    /// [`ErrorCode::NothingToRead`](crate::ErrorCode::NothingToRead) if it holds no worksheet, or
    /// [`ErrorCode::InvalidArgument`](crate::ErrorCode::InvalidArgument) if a `mergeCell@ref` does
    /// not parse.
    pub fn merged_ranges(&self, sheet: u32) -> Result<Vec<String>, Error> {
        Ok(self
            .workbook
            .merged_ranges(index(sheet))?
            .into_iter()
            .map(|range| range.text().as_str().to_owned())
            .collect())
    }

    /// The merged range covering `reference`, or `None` when that cell is not merged.
    ///
    /// # Errors
    /// As [`merged_ranges`](Self::merged_ranges), plus
    /// [`ErrorCode::InvalidArgument`](crate::ErrorCode::InvalidArgument) if `reference` is not an A1
    /// cell.
    pub fn merged_range_containing(
        &self,
        sheet: u32,
        reference: &str,
    ) -> Result<Option<String>, Error> {
        let reference = CellReference::parse(reference)?;
        Ok(self
            .workbook
            .merged_range_containing(index(sheet), reference)?
            .map(|range| range.text().as_str().to_owned()))
    }

    /// Merges `range`.
    ///
    /// The top-left cell keeps its value; the cells the region covers keep theirs, hidden until the
    /// region is unmerged again — **nothing is cleared**, because a merge in SpreadsheetML is a
    /// display statement rather than a destructive edit.
    ///
    /// # Errors
    /// As [`merged_ranges`](Self::merged_ranges), plus
    /// [`ErrorCode::StructureConflict`](crate::ErrorCode::StructureConflict) if the range would
    /// overlap an existing merge, and
    /// [`ErrorCode::InvalidArgument`](crate::ErrorCode::InvalidArgument) if it covers one cell.
    pub fn merge_cells(&mut self, sheet: u32, range: &str) -> Result<(), Error> {
        let range = CellRange::parse(range)?;
        Ok(self.workbook.merge_cells(index(sheet), range)?)
    }

    /// Removes the merge whose `@ref` is exactly `range`, answering whether one was there.
    ///
    /// # Errors
    /// As [`merge_cells`](Self::merge_cells), minus the two conflict cases.
    pub fn unmerge_cells(&mut self, sheet: u32, range: &str) -> Result<bool, Error> {
        let range = CellRange::parse(range)?;
        Ok(self.workbook.unmerge_cells(index(sheet), range)?)
    }

    /// Sets the height, in points, of the row a file numbered `row` — **one-based**, as `row@r` is.
    ///
    /// `custom` says which of the two heights SpreadsheetML distinguishes this is: `true` writes
    /// `customHeight="1"` (a height a person set, which Excel keeps), `false` writes the height
    /// alone (a height a consumer computed to fit the content, which it may compute again). A caller
    /// who wants a particular height wants `true`. `None` removes the height.
    ///
    /// The row must already exist — a height is a property of a row, and creating an empty `<row>`
    /// to carry one would author markup for a row the sheet does not have.
    ///
    /// # Errors
    /// As [`merged_ranges`](Self::merged_ranges).
    pub fn set_row_height(
        &mut self,
        sheet: u32,
        row: u32,
        points: Option<f64>,
        custom: bool,
    ) -> Result<(), Error> {
        let height = points.map(|points| {
            if custom {
                RowHeight::Custom(points)
            } else {
                RowHeight::Fitted(points)
            }
        });
        Ok(self.workbook.set_row_height(index(sheet), row, height)?)
    }

    /// Hides or shows the row a file numbered `row` — **one-based**, as `row@r` is.
    ///
    /// # Errors
    /// As [`merged_ranges`](Self::merged_ranges).
    pub fn set_row_hidden(&mut self, sheet: u32, row: u32, hidden: bool) -> Result<(), Error> {
        Ok(self.workbook.set_row_hidden(index(sheet), row, hidden)?)
    }

    /// Sets the outline (grouping) depth of the row a file numbered `row` — **one-based**.
    ///
    /// # Errors
    /// As [`merged_ranges`](Self::merged_ranges).
    pub fn set_row_outline_level(&mut self, sheet: u32, row: u32, level: u8) -> Result<(), Error> {
        Ok(self
            .workbook
            .set_row_outline_level(index(sheet), row, level)?)
    }

    /// Sets the width, in characters of the maximum digit width, of the columns
    /// `first_column..=last_column` — both **zero-based**, so `0` is `A`.
    ///
    /// `custom` distinguishes the two widths SpreadsheetML writes, exactly as it does for
    /// [`set_row_height`](Self::set_row_height). `None` removes the width.
    ///
    /// # Errors
    /// As [`merged_ranges`](Self::merged_ranges), plus
    /// [`ErrorCode::InvalidArgument`](crate::ErrorCode::InvalidArgument) if either column is past
    /// `XFD`.
    pub fn set_column_width(
        &mut self,
        sheet: u32,
        first_column: u32,
        last_column: u32,
        characters: Option<f64>,
        custom: bool,
    ) -> Result<(), Error> {
        let columns = span(first_column, last_column)?;
        let width = characters.map(|width| {
            if custom {
                ColumnWidth::Custom(width)
            } else {
                ColumnWidth::Fitted(width)
            }
        });
        Ok(self
            .workbook
            .set_column_width(index(sheet), columns, width)?)
    }

    /// Hides or shows the columns `first_column..=last_column`, both **zero-based**.
    ///
    /// # Errors
    /// As [`set_column_width`](Self::set_column_width).
    pub fn set_column_hidden(
        &mut self,
        sheet: u32,
        first_column: u32,
        last_column: u32,
        hidden: bool,
    ) -> Result<(), Error> {
        let columns = span(first_column, last_column)?;
        Ok(self
            .workbook
            .set_column_hidden(index(sheet), columns, hidden)?)
    }

    /// Sets the outline (grouping) depth of the columns `first_column..=last_column`, both
    /// **zero-based**.
    ///
    /// # Errors
    /// As [`set_column_width`](Self::set_column_width).
    pub fn set_column_outline_level(
        &mut self,
        sheet: u32,
        first_column: u32,
        last_column: u32,
        level: u8,
    ) -> Result<(), Error> {
        let columns = span(first_column, last_column)?;
        Ok(self
            .workbook
            .set_column_outline_level(index(sheet), columns, level)?)
    }

    /// Everything one sheet's grid says that a well-formed one would not: overlapping merges,
    /// column runs that overlap or are written backwards, and outline maxima the file understates.
    ///
    /// **A report, never a repair.** Every anomaly listed here is still written back verbatim.
    ///
    /// # Errors
    /// As [`merged_ranges`](Self::merged_ranges).
    pub fn grid_anomalies(&self, sheet: u32) -> Result<Vec<GridAnomalyInfo>, Error> {
        Ok(self
            .workbook
            .grid_anomalies(index(sheet))?
            .into_iter()
            .map(anomaly)
            .collect())
    }
}

/// Which of the nine things a grid can say that a well-formed one would not.
///
/// A flat enumeration rather than [`mjx_sml::GridAnomaly`]'s payload-carrying one, because the
/// coordinates each variant carries are the fields of [`GridAnomalyInfo`] beside it — the shape a
/// binding can project, and the shape a caller filtering by kind wants either way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GridAnomalyKind {
    /// A `mergeCell@ref` that is absent or does not parse.
    MergeReferenceUnreadable,
    /// Two merged ranges cover a cell between them.
    MergesOverlap,
    /// A merged range covers one cell, so it merges nothing.
    DegenerateMerge,
    /// A cell inside a merged region — not its anchor — holds a value, which a consumer hides.
    MergeInteriorCellHasValue,
    /// `mergeCells@count` disagrees with how many `mergeCell` children there are.
    MergeCountDisagrees,
    /// A `col` run's `@min` is greater than its `@max`.
    ColumnRunBoundsInverted,
    /// Two `col` runs cover a column between them.
    ColumnRunsOverlap,
    /// A row's `@outlineLevel` is deeper than `sheetFormatPr@outlineLevelRow` declares.
    RowOutlineLevelPastDeclaredMaximum,
    /// The same, for a column against `@outlineLevelCol`.
    ColumnOutlineLevelPastDeclaredMaximum,
}

/// One thing a sheet's grid says that a well-formed one would not, with whatever coordinates the
/// finding carries.
///
/// Every field beside [`kind`](Self::kind) is `None` for the variants that do not name it, and which
/// those are is stated on [`GridAnomalyKind`]'s own variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridAnomalyInfo {
    /// Which finding this is.
    pub kind: GridAnomalyKind,
    /// The range at fault, as A1 text — the first of an overlapping pair, or the single range a
    /// degenerate or interior-value finding names.
    pub range: Option<String>,
    /// The second range of an overlapping pair.
    pub other_range: Option<String>,
    /// The cell at fault, as A1 text.
    pub cell: Option<String>,
    /// The first column of a `col` run at fault, zero-based.
    pub first_column: Option<u32>,
    /// The last column of that run, zero-based.
    pub last_column: Option<u32>,
    /// The second run's first column, for an overlap.
    pub other_first_column: Option<u32>,
    /// The second run's last column, for an overlap.
    pub other_last_column: Option<u32>,
    /// What the file declared — a merge count, or an outline maximum.
    pub declared: Option<u32>,
    /// What is actually there — a merge count, or the deepest outline level found.
    pub actual: Option<u32>,
    /// The index of the `mergeCell` whose `@ref` could not be read.
    pub index: Option<u32>,
}

/// One model anomaly as the facade states it.
fn anomaly(anomaly: GridAnomaly) -> GridAnomalyInfo {
    let empty = GridAnomalyInfo {
        kind: GridAnomalyKind::MergeReferenceUnreadable,
        range: None,
        other_range: None,
        cell: None,
        first_column: None,
        last_column: None,
        other_first_column: None,
        other_last_column: None,
        declared: None,
        actual: None,
        index: None,
    };
    let text = |range: CellRange| range.text().as_str().to_owned();
    match anomaly {
        GridAnomaly::MergeReferenceUnreadable { index } => GridAnomalyInfo {
            kind: GridAnomalyKind::MergeReferenceUnreadable,
            index: Some(count(index)),
            ..empty
        },
        GridAnomaly::MergesOverlap { first, second } => GridAnomalyInfo {
            kind: GridAnomalyKind::MergesOverlap,
            range: Some(text(first)),
            other_range: Some(text(second)),
            ..empty
        },
        GridAnomaly::DegenerateMerge { range } => GridAnomalyInfo {
            kind: GridAnomalyKind::DegenerateMerge,
            range: Some(text(range)),
            ..empty
        },
        GridAnomaly::MergeInteriorCellHasValue { merge, cell } => GridAnomalyInfo {
            kind: GridAnomalyKind::MergeInteriorCellHasValue,
            range: Some(text(merge)),
            cell: Some(cell.text().as_str().to_owned()),
            ..empty
        },
        GridAnomaly::MergeCountDisagrees { declared, actual } => GridAnomalyInfo {
            kind: GridAnomalyKind::MergeCountDisagrees,
            declared: Some(declared),
            actual: Some(count(actual)),
            ..empty
        },
        GridAnomaly::ColumnRunBoundsInverted {
            first_column,
            last_column,
        } => GridAnomalyInfo {
            kind: GridAnomalyKind::ColumnRunBoundsInverted,
            first_column: Some(first_column),
            last_column: Some(last_column),
            ..empty
        },
        GridAnomaly::ColumnRunsOverlap { first, second } => GridAnomalyInfo {
            kind: GridAnomalyKind::ColumnRunsOverlap,
            first_column: Some(first.0),
            last_column: Some(first.1),
            other_first_column: Some(second.0),
            other_last_column: Some(second.1),
            ..empty
        },
        GridAnomaly::RowOutlineLevelPastDeclaredMaximum { deepest, declared } => GridAnomalyInfo {
            kind: GridAnomalyKind::RowOutlineLevelPastDeclaredMaximum,
            declared: Some(u32::from(declared)),
            actual: Some(u32::from(deepest)),
            ..empty
        },
        GridAnomaly::ColumnOutlineLevelPastDeclaredMaximum { deepest, declared } => {
            GridAnomalyInfo {
                kind: GridAnomalyKind::ColumnOutlineLevelPastDeclaredMaximum,
                declared: Some(u32::from(declared)),
                actual: Some(u32::from(deepest)),
                ..empty
            }
        }
    }
}

/// A zero-based inclusive column pair as the [`CellSpan`] the model addresses with.
fn span(first_column: u32, last_column: u32) -> Result<CellSpan, Error> {
    let first = u16::try_from(first_column).map_err(|_| mjx_sml::AddressError::ColumnOutOfGrid)?;
    let last = u16::try_from(last_column).map_err(|_| mjx_sml::AddressError::ColumnOutOfGrid)?;
    Ok(CellSpan::new(first, last)?)
}
