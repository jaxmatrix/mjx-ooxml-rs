//! The workbook a chart embeds — laid out here, written by `mjx-sml`.
//!
//! A real chart does not only carry cached values: it embeds a whole spreadsheet package at
//! `/ppt/embeddings/*.xlsx`, related from the chart part by `.../relationships/package` and named by
//! the chart's `c:externalData@r:id`. That workbook is what PowerPoint's **Edit Data** opens. A chart
//! with no workbook renders perfectly — the caches are what draw — but Edit Data has nothing to show;
//! a chart whose workbook disagrees with its caches shows the *old* numbers there.
//!
//! # What this module is, and what it deliberately is not
//!
//! This module knows one thing: **which cell a chart's data belongs in**. The header row carries the
//! series names, column `A` the categories, and the values start at `B2` — the same grid the chart's
//! own `c:f` formulas name (`Sheet1!$A$2:$A$4`, `Sheet1!$B$2:$B$4`), which is what makes Edit Data
//! open on the numbers the chart actually draws.
//!
//! It knows nothing whatsoever about SpreadsheetML. Every cell, part, content type and relationship
//! comes from [`mjx_sml::write::WorkbookPackage`]; the two functions here translate a chart into
//! [`AuthoredCellValue`] rows and hand them over. Until MJXOFF-99 this crate carried a minimal
//! spreadsheet writer of its own, because `mjx-sml` did not exist yet and `mjx-chart → mjx-xlsx`
//! would have been an upward edge the layering forbids. `mjx-chart → mjx-sml` points down, so the
//! duplicate is gone and there is exactly one SpreadsheetML writer in the shipped workspace.
//!
//! ```
//! use mjx_chart::{embedded_workbook_for_chart_data, ChartData, ChartKind};
//!
//! # fn main() -> Result<(), mjx_sml::SmlError> {
//! let chart = ChartData::new(ChartKind::Bar)
//!     .categories(["Q1", "Q2"])
//!     .series("Revenue", [10.0, 20.0]);
//! let bytes = embedded_workbook_for_chart_data(&chart)?;
//! assert_eq!(&bytes[..2], b"PK", "a workbook is a ZIP package");
//! # Ok(())
//! # }
//! ```

use mjx_sml::write::{AuthoredCellValue, WorkbookPackage};
use mjx_sml::SmlError;

use crate::author::ChartData;
use crate::space::ChartSpace;

/// The tab a chart's workbook writes into. There is exactly one, and its name is what the chart's
/// `c:f` formulas qualify their ranges with.
const FIRST_TAB: usize = 0;

/// The workbook backing a chart being **authored**: a header row of series names, then one row per
/// category holding its label and each series' value at that position.
///
/// The layout matches the formulas [`ChartData`] synthesizes exactly — column `A` the categories,
/// column `B` onwards one per series, data starting at row 2 — so Edit Data opens on the cells the
/// chart's `c:f` references name. For a plot whose category axis is numeric (scatter, bubble) the
/// `A` column is written as numbers, matching the `c:numRef` the chart uses there.
///
/// # Errors
/// [`SmlError`] if the grid does not fit the sheet — a chart with more series than SpreadsheetML has
/// columns, or more categories than it has rows — or if the packaging layer refuses a part. Every
/// part name and content type is a constant inside `mjx-sml`, so the packaging half cannot fail in
/// practice; the grid half is a real refusal and is why this returns a `Result` at all.
pub fn embedded_workbook_for_chart_data(chart: &ChartData) -> Result<Vec<u8>, SmlError> {
    let mut workbook = WorkbookPackage::new()?;

    let mut header = vec![AuthoredCellValue::Blank];
    for series in chart.series_names() {
        header.push(AuthoredCellValue::SharedText(series.to_owned()));
    }
    workbook.push_row(FIRST_TAB, &header)?;

    let numeric_categories = chart.kind().uses_xy_data();
    let rows = chart.category_count().max(chart.longest_series());
    for index in 0..rows {
        let mut row = vec![match chart.category_label(index) {
            Some(label) if !numeric_categories => AuthoredCellValue::SharedText(label.to_owned()),
            _ => AuthoredCellValue::Number(chart.category_number(index)),
        }];
        for values in chart.series_values() {
            row.push(match values.get(index) {
                Some(&value) => AuthoredCellValue::Number(value),
                None => AuthoredCellValue::Blank,
            });
        }
        workbook.push_row(FIRST_TAB, &row)?;
    }

    finish(workbook)
}

/// The workbook backing an **existing** chart, read from its parsed part — the refresh a data edit
/// needs so the workbook never disagrees with the caches that render.
///
/// The categories come from the first series that declares any (they are shared across a plot's
/// series); each series contributes its name and its values as one column.
///
/// # Errors
/// As [`embedded_workbook_for_chart_data`].
pub fn embedded_workbook_for_chart_space(space: &ChartSpace) -> Result<Vec<u8>, SmlError> {
    let mut workbook = WorkbookPackage::new()?;
    let Some(area) = space.plot_area() else {
        return finish(workbook);
    };

    let mut header = vec![AuthoredCellValue::Blank];
    let mut columns: Vec<Vec<f64>> = Vec::new();
    let mut categories: Vec<AuthoredCellValue> = Vec::new();
    for series in area.all_series() {
        header.push(match series.name() {
            Some(name) => AuthoredCellValue::SharedText(name),
            None => AuthoredCellValue::Blank,
        });
        let source = series.categories().or_else(|| series.x_data());
        if categories.is_empty() {
            if let Some(source) = source {
                categories = if source.is_numeric() {
                    source
                        .values()
                        .into_iter()
                        .map(AuthoredCellValue::Number)
                        .collect()
                } else {
                    source
                        .labels()
                        .into_iter()
                        .map(AuthoredCellValue::SharedText)
                        .collect()
                };
            }
        }
        columns.push(
            series
                .values()
                .map(crate::data::NumericData::values)
                .or_else(|| series.y_data().map(crate::data::NumericData::values))
                .unwrap_or_default(),
        );
    }
    // Every series contributes a header cell, but a series with no `c:tx` contributes a blank one —
    // so this whole row can write nothing at all. It is still row 1, and the categories below it
    // still start at row 2, which is what the chart's own formulas say. `push_row` guarantees that;
    // see `AuthoredWorksheet::appended_row_count`.
    workbook.push_row(FIRST_TAB, &header)?;

    let rows = categories
        .len()
        .max(columns.iter().map(Vec::len).max().unwrap_or(0));
    for index in 0..rows {
        let mut row = vec![categories
            .get(index)
            .cloned()
            .unwrap_or(AuthoredCellValue::Blank)];
        for column in &columns {
            row.push(match column.get(index) {
                Some(&value) => AuthoredCellValue::Number(value),
                None => AuthoredCellValue::Blank,
            });
        }
        workbook.push_row(FIRST_TAB, &row)?;
    }

    finish(workbook)
}

/// Caches the sheet's bounding box and serializes the package.
///
/// `recompute_dimensions` is explicit rather than implicit in `mjx-sml` — a caller that has just
/// authored a sheet has no cached box at all, and `dimension` is what Excel sizes its scroll bars
/// from. A sheet with no populated cell gets no element, because `@ref` is `use="required"` and
/// `ref=""` is not an `ST_Ref`.
fn finish(mut workbook: WorkbookPackage) -> Result<Vec<u8>, SmlError> {
    workbook.recompute_dimensions();
    workbook.to_package_bytes()
}
