//! [`Workbook`] — the curated Excel surface, from Python.
//!
//! ```python
//! import mjx_ooxml
//!
//! workbook = mjx_ooxml.Workbook.open(open("book.xlsx", "rb").read())
//! for row in workbook.read_range(0, "A1:D20").rows():
//!     print(row)
//! ```
//!
//! Mirrors [`crate::deck::Deck`]'s own design exactly — see that module's doc comment for the "one
//! workbook, one thread" discipline, which applies here unchanged.
//!
//! # ⚠ Cells cross a range at a time, and there is no per-cell call
//!
//! This is the one place the Excel binding's shape differs from the other two, and the reason is
//! measured. `mjx_xlsx::Workbook` holds no parsed worksheet, so a per-cell call costs a whole-part
//! parse **every time**: 387 ms to read one cell of a 300,000-cell sheet, against 14.8 ms to open
//! the file. The Rust answer — hold the parsed worksheet yourself between one read and one write —
//! cannot cross this boundary, and a Python caller has no escape hatch to reach it with. So the door
//! here is `read_range` / `read_sheet` / `write_cells`, each of which parses once whatever it is
//! asked for.
//!
//! ```python
//! # One call, one parse, every cell.
//! workbook.write_cells(0, [
//!     mjx_ooxml.CellWrite.shared_text("A1", "Region"),
//!     mjx_ooxml.CellWrite.number("B1", 12.5),
//! ])
//! ```
//!
//! # Addressing
//!
//! A **sheet** is an `int` — its position in the tab list. A **cell** is A1 text: `"B7"`. A
//! **range** is A1 text too: `"A1:C3"`, `"A:C"` for whole columns, `"1:3"` for whole rows. Nothing
//! here takes a `(row, column)` pair, so nothing here can be transposed by a caller who guessed the
//! order.

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyModule};

use mjx_ooxml as ooxml;

use crate::enums::{CellFormatTarget, DateSystem, ResizingBehavior, TableStyleOrigin};
use crate::errors::to_py_err;
use crate::format::Format;
use crate::spreadsheet::{
    AnchorBoundsInfo, AnchorShiftInfo, BorderSpec, CalculationSettings, CellBlock, CellFormatSpec,
    CellWrite, DefinedName, EffectiveCellFormat, FontProperties, GridAnomalyInfo, PatternFillSpec,
    PreservedPartsSummary, SheetDrawingInfo, SheetHyperlinkInfo, SheetPivotTableInfo,
    SheetQueryTableInfo, SheetSummary, SheetTableInfo, WorkbookConnectionInfo,
    WorkbookExternalLinkInfo, WorkbookRevisionState, WorkbookWindowInfo, WorkbookXmlMapsInfo,
};

/// An open Excel workbook.
#[pyclass(module = "mjx_ooxml")]
#[derive(Debug)]
pub struct Workbook {
    inner: ooxml::Workbook,
}

#[pymethods]
impl Workbook {
    /// A new workbook with nothing in it: one empty worksheet named `Sheet1`, a styles part, and the
    /// package around them.
    #[staticmethod]
    fn blank() -> PyResult<Self> {
        ooxml::Workbook::blank()
            .map(|inner| Self { inner })
            .map_err(to_py_err)
    }

    /// Opens a workbook from the bytes of a `.xlsx`, `.xlsm`, `.xltx` or `.xltm`.
    ///
    /// The interpreter lock is released for the parse. Raises `IoError` for bytes that are not a
    /// readable container, `MalformedDocumentError` for a package whose markup is not
    /// SpreadsheetML, and `UnsupportedFormatError` — naming the format — for a PowerPoint or Word
    /// document, **and for a `.xlsb`**, whose main part is the MS-XLSB binary record stream rather
    /// than SpreadsheetML. That last refusal is permanent by design, not a not-yet.
    #[staticmethod]
    fn open(python: Python<'_>, data: &[u8]) -> PyResult<Self> {
        python
            .detach(|| ooxml::Workbook::open(data))
            .map(|inner| Self { inner })
            .map_err(to_py_err)
    }

    /// What this workbook's main part says it is.
    fn format(&self) -> PyResult<Format> {
        Format::from_model(self.inner.format())
    }

    /// The workbook as the bytes of a `.xlsx`, **validated first**. The interpreter lock is released
    /// for the write.
    fn save<'py>(&self, python: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        python
            .detach(|| self.inner.save())
            .map(|bytes| PyBytes::new(python, &bytes))
            .map_err(to_py_err)
    }

    /// The workbook as bytes, **without** the validation pass.
    fn save_unchecked<'py>(&self, python: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        python
            .detach(|| self.inner.save_unchecked())
            .map(|bytes| PyBytes::new(python, &bytes))
            .map_err(to_py_err)
    }

    /// Checks every invariant `save` enforces, without writing anything.
    fn validate(&self) -> PyResult<()> {
        self.inner.validate().map_err(to_py_err)
    }

    // --- tabs -----------------------------------------------------------------------------------

    /// How many tabs the workbook lists.
    fn sheet_count(&self) -> u32 {
        self.inner.sheet_count()
    }

    /// Every tab, in tab order.
    fn sheets(&self) -> Vec<SheetSummary> {
        self.inner.sheets().into_iter().map(SheetSummary).collect()
    }

    /// One tab.
    fn sheet(&self, sheet: u32) -> PyResult<SheetSummary> {
        self.inner.sheet(sheet).map(SheetSummary).map_err(to_py_err)
    }

    /// The index of the tab named `name`, or `None` when no tab has that name.
    fn sheet_index(&self, name: &str) -> Option<u32> {
        self.inner.sheet_index(name)
    }

    /// Appends a new, empty worksheet and answers its index.
    fn add_sheet(&mut self, name: &str) -> PyResult<u32> {
        self.inner.add_sheet(name).map_err(to_py_err)
    }

    /// Renames one tab. Formulas that reference the old name are **not** rewritten.
    fn rename_sheet(&mut self, sheet: u32, name: &str) -> PyResult<()> {
        self.inner.rename_sheet(sheet, name).map_err(to_py_err)
    }

    /// The index of the tab a consumer opens the workbook on, or `None`.
    fn active_sheet(&mut self) -> PyResult<Option<u32>> {
        self.inner.active_sheet().map_err(to_py_err)
    }

    /// Every `x:workbookView`, in document order.
    fn window_views(&mut self) -> PyResult<Vec<WorkbookWindowInfo>> {
        self.inner
            .window_views()
            .map(|views| views.into_iter().map(WorkbookWindowInfo).collect())
            .map_err(to_py_err)
    }

    // --- cells: the range crossing ----------------------------------------------------------------

    /// Reads every cell of one rectangle, parsing the worksheet **once**.
    ///
    /// `range` is A1 text: `"A1"` for one cell, `"A1:C3"` for a rectangle, `"A:C"` for whole
    /// columns, `"1:3"` for whole rows. The two open-ended forms are clamped to the sheet's
    /// populated extent, so `"A:A"` costs the rows the file actually has.
    ///
    /// **This is the way to read cells**, and there is deliberately no per-cell call beside it: see
    /// this module's own documentation for the measurements that decided that.
    fn read_range(&self, sheet: u32, range: &str) -> PyResult<CellBlock> {
        self.inner
            .read_range(sheet, range)
            .map(CellBlock)
            .map_err(to_py_err)
    }

    /// Reads every populated cell of one sheet, parsing the worksheet **once**.
    fn read_sheet(&self, sheet: u32) -> PyResult<CellBlock> {
        self.inner
            .read_sheet(sheet)
            .map(CellBlock)
            .map_err(to_py_err)
    }

    /// The A1 range of one sheet's populated extent, or `None` when nothing is populated.
    fn used_range(&self, sheet: u32) -> PyResult<Option<String>> {
        self.inner.used_range(sheet).map_err(to_py_err)
    }

    /// Writes every entry of `cells` into one sheet, parsing **once** and serializing **once**.
    ///
    /// Entries are applied in the order given. Prefer top-to-bottom, left-to-right: that is
    /// append-only in the cell arena. A batch with a bad address writes **nothing**.
    fn write_cells(&mut self, sheet: u32, cells: Vec<CellWrite>) -> PyResult<()> {
        let cells: Vec<ooxml::CellWrite> = cells.into_iter().map(|write| write.0).collect();
        self.inner.write_cells(sheet, &cells).map_err(to_py_err)
    }

    // --- geometry ---------------------------------------------------------------------------------

    /// Every merged range on one sheet, as A1 text.
    fn merged_ranges(&self, sheet: u32) -> PyResult<Vec<String>> {
        self.inner.merged_ranges(sheet).map_err(to_py_err)
    }

    /// The merged range covering `reference`, or `None` when that cell is not merged.
    fn merged_range_containing(&self, sheet: u32, reference: &str) -> PyResult<Option<String>> {
        self.inner
            .merged_range_containing(sheet, reference)
            .map_err(to_py_err)
    }

    /// Merges `range`. Nothing is cleared: a merge is a display statement, not a destructive edit.
    fn merge_cells(&mut self, sheet: u32, range: &str) -> PyResult<()> {
        self.inner.merge_cells(sheet, range).map_err(to_py_err)
    }

    /// Removes the merge whose `@ref` is exactly `range`, answering whether one was there.
    fn unmerge_cells(&mut self, sheet: u32, range: &str) -> PyResult<bool> {
        self.inner.unmerge_cells(sheet, range).map_err(to_py_err)
    }

    /// Sets a row's height in points. `row` is **one-based**, as `row@r` is. `custom=True` is the
    /// height a person set (Excel keeps it); `False` is one a consumer computed and may recompute.
    #[pyo3(signature = (sheet, row, points, custom = true))]
    fn set_row_height(
        &mut self,
        sheet: u32,
        row: u32,
        points: Option<f64>,
        custom: bool,
    ) -> PyResult<()> {
        self.inner
            .set_row_height(sheet, row, points, custom)
            .map_err(to_py_err)
    }

    /// Hides or shows a row. `row` is **one-based**.
    fn set_row_hidden(&mut self, sheet: u32, row: u32, hidden: bool) -> PyResult<()> {
        self.inner
            .set_row_hidden(sheet, row, hidden)
            .map_err(to_py_err)
    }

    /// Sets a row's outline (grouping) depth. `row` is **one-based**.
    fn set_row_outline_level(&mut self, sheet: u32, row: u32, level: u8) -> PyResult<()> {
        self.inner
            .set_row_outline_level(sheet, row, level)
            .map_err(to_py_err)
    }

    /// Sets the width of the columns `first_column..=last_column`, both **zero-based**, in
    /// characters of the maximum digit width.
    #[pyo3(signature = (sheet, first_column, last_column, characters, custom = true))]
    fn set_column_width(
        &mut self,
        sheet: u32,
        first_column: u32,
        last_column: u32,
        characters: Option<f64>,
        custom: bool,
    ) -> PyResult<()> {
        self.inner
            .set_column_width(sheet, first_column, last_column, characters, custom)
            .map_err(to_py_err)
    }

    /// Hides or shows the columns `first_column..=last_column`, both **zero-based**.
    fn set_column_hidden(
        &mut self,
        sheet: u32,
        first_column: u32,
        last_column: u32,
        hidden: bool,
    ) -> PyResult<()> {
        self.inner
            .set_column_hidden(sheet, first_column, last_column, hidden)
            .map_err(to_py_err)
    }

    /// Sets the outline depth of the columns `first_column..=last_column`, both **zero-based**.
    fn set_column_outline_level(
        &mut self,
        sheet: u32,
        first_column: u32,
        last_column: u32,
        level: u8,
    ) -> PyResult<()> {
        self.inner
            .set_column_outline_level(sheet, first_column, last_column, level)
            .map_err(to_py_err)
    }

    /// Everything one sheet's grid says that a well-formed one would not. A report, never a repair.
    fn grid_anomalies(&self, sheet: u32) -> PyResult<Vec<GridAnomalyInfo>> {
        self.inner
            .grid_anomalies(sheet)
            .map(|found| found.into_iter().map(GridAnomalyInfo).collect())
            .map_err(to_py_err)
    }

    // --- cell formats -----------------------------------------------------------------------------

    /// Appends a font to `xl/styles.xml`'s `fonts` table and answers its index.
    fn append_font(&mut self, properties: &FontProperties) -> PyResult<u32> {
        self.inner.append_font(&properties.0).map_err(to_py_err)
    }

    /// Appends a pattern fill to the `fills` table and answers its index.
    fn append_pattern_fill(&mut self, spec: &PatternFillSpec) -> PyResult<u32> {
        self.inner.append_pattern_fill(&spec.0).map_err(to_py_err)
    }

    /// Appends a border to the `borders` table and answers its index.
    fn append_border(&mut self, spec: &BorderSpec) -> PyResult<u32> {
        self.inner.append_border(&spec.0).map_err(to_py_err)
    }

    /// Appends an `x:xf` to `cellXfs` or `cellStyleXfs` and answers its index.
    fn append_cell_format(
        &mut self,
        target: CellFormatTarget,
        spec: &CellFormatSpec,
    ) -> PyResult<u32> {
        self.inner
            .append_cell_format(target.into(), &spec.0)
            .map_err(to_py_err)
    }

    /// Points one cell at `cellXfs[style]`, or removes its `@s` with `None`. The cell must already
    /// exist — write the value first.
    #[pyo3(signature = (sheet, reference, style = None))]
    fn set_cell_style(&mut self, sheet: u32, reference: &str, style: Option<u32>) -> PyResult<()> {
        self.inner
            .set_cell_style(sheet, reference, style)
            .map_err(to_py_err)
    }

    /// Interns `text` into `xl/sharedStrings.xml` and answers its index.
    fn intern_shared_string(&mut self, text: &str) -> PyResult<u32> {
        self.inner.intern_shared_string(text).map_err(to_py_err)
    }

    /// What one cell's format resolves to, after the `cellXfs` -> `cellStyleXfs` ladder and the
    /// column and row defaults above it. **What the file states, not what a renderer shows.**
    fn effective_cell_format(
        &self,
        sheet: u32,
        reference: &str,
    ) -> PyResult<Option<EffectiveCellFormat>> {
        self.inner
            .effective_cell_format(sheet, reference)
            .map(|found| found.map(EffectiveCellFormat))
            .map_err(to_py_err)
    }

    /// The same ladder, answered for the **anchor** of the merged region `reference` falls in.
    fn effective_merged_cell_format(
        &self,
        sheet: u32,
        reference: &str,
    ) -> PyResult<Option<EffectiveCellFormat>> {
        self.inner
            .effective_merged_cell_format(sheet, reference)
            .map(|found| found.map(EffectiveCellFormat))
            .map_err(to_py_err)
    }

    // --- hyperlinks -------------------------------------------------------------------------------

    /// Every hyperlink on one sheet, in document order.
    fn sheet_hyperlinks(&self, sheet: u32) -> PyResult<Vec<SheetHyperlinkInfo>> {
        self.inner
            .sheet_hyperlinks(sheet)
            .map(|links| links.into_iter().map(SheetHyperlinkInfo).collect())
            .map_err(to_py_err)
    }

    /// The hyperlink whose range covers `reference`, or `None`.
    fn cell_hyperlink(&self, sheet: u32, reference: &str) -> PyResult<Option<SheetHyperlinkInfo>> {
        self.inner
            .cell_hyperlink(sheet, reference)
            .map(|link| link.map(SheetHyperlinkInfo))
            .map_err(to_py_err)
    }

    /// Points `range` at an external URL, writing the entry **and** its `External` relationship.
    fn set_cell_hyperlink_url(&mut self, sheet: u32, range: &str, url: &str) -> PyResult<()> {
        self.inner
            .set_cell_hyperlink_url(sheet, range, url)
            .map_err(to_py_err)
    }

    /// Points `range` at a location inside this workbook — `"Sheet2!A1"`, or a defined name.
    fn set_cell_hyperlink_location(
        &mut self,
        sheet: u32,
        range: &str,
        location: &str,
    ) -> PyResult<()> {
        self.inner
            .set_cell_hyperlink_location(sheet, range, location)
            .map_err(to_py_err)
    }

    /// Removes the hyperlink covering `reference`, and the relationship it named.
    fn remove_cell_hyperlink(&mut self, sheet: u32, reference: &str) -> PyResult<bool> {
        self.inner
            .remove_cell_hyperlink(sheet, reference)
            .map_err(to_py_err)
    }

    // --- drawings ---------------------------------------------------------------------------------

    /// The drawing part behind one sheet, and everything anchored in it.
    fn sheet_drawing(&self, sheet: u32) -> PyResult<Option<SheetDrawingInfo>> {
        self.inner
            .sheet_drawing(sheet)
            .map(|drawing| drawing.map(SheetDrawingInfo))
            .map_err(to_py_err)
    }

    /// Where the anchor at `anchor` puts its object, in EMU. `7.0` and `96.0` are ECMA-376's own
    /// worked example, for 11-point Calibri.
    fn sheet_anchor_bounds(
        &self,
        sheet: u32,
        anchor: u32,
        maximum_digit_width_pixels: f64,
        pixels_per_inch: f64,
    ) -> PyResult<Option<AnchorBoundsInfo>> {
        self.inner
            .sheet_anchor_bounds(sheet, anchor, maximum_digit_width_pixels, pixels_per_inch)
            .map(|bounds| bounds.map(AnchorBoundsInfo))
            .map_err(to_py_err)
    }

    /// Anchors a picture between two cells, and answers its position in the paint order.
    #[allow(clippy::too_many_arguments)]
    fn add_two_cell_anchored_picture(
        &mut self,
        sheet: u32,
        image_bytes: Vec<u8>,
        name: &str,
        from_column: u32,
        from_column_offset_emu: i64,
        from_row: u32,
        from_row_offset_emu: i64,
        to_column: u32,
        to_column_offset_emu: i64,
        to_row: u32,
        to_row_offset_emu: i64,
        resizing: ResizingBehavior,
    ) -> PyResult<u32> {
        self.inner
            .add_two_cell_anchored_picture(
                sheet,
                &image_bytes,
                name,
                from_column,
                from_column_offset_emu,
                from_row,
                from_row_offset_emu,
                to_column,
                to_column_offset_emu,
                to_row,
                to_row_offset_emu,
                resizing.into(),
            )
            .map_err(to_py_err)
    }

    /// Anchors a picture to one cell, at its own size.
    #[allow(clippy::too_many_arguments)]
    fn add_one_cell_anchored_picture(
        &mut self,
        sheet: u32,
        image_bytes: Vec<u8>,
        name: &str,
        from_column: u32,
        from_column_offset_emu: i64,
        from_row: u32,
        from_row_offset_emu: i64,
        width_emu: i64,
        height_emu: i64,
    ) -> PyResult<u32> {
        self.inner
            .add_one_cell_anchored_picture(
                sheet,
                &image_bytes,
                name,
                from_column,
                from_column_offset_emu,
                from_row,
                from_row_offset_emu,
                width_emu,
                height_emu,
            )
            .map_err(to_py_err)
    }

    /// Anchors a picture to the sheet, at an absolute position and size in EMU.
    #[allow(clippy::too_many_arguments)]
    fn add_absolute_anchored_picture(
        &mut self,
        sheet: u32,
        image_bytes: Vec<u8>,
        name: &str,
        x_emu: i64,
        y_emu: i64,
        width_emu: i64,
        height_emu: i64,
    ) -> PyResult<u32> {
        self.inner
            .add_absolute_anchored_picture(
                sheet,
                &image_bytes,
                name,
                x_emu,
                y_emu,
                width_emu,
                height_emu,
            )
            .map_err(to_py_err)
    }

    /// Removes one anchored object from a sheet, reporting whether there was one.
    fn remove_sheet_drawing_object(&mut self, sheet: u32, anchor: u32) -> PyResult<bool> {
        self.inner
            .remove_sheet_drawing_object(sheet, anchor)
            .map_err(to_py_err)
    }

    /// Moves every anchor on a sheet for `rows` inserted at the zero-based `at`.
    fn insert_rows_into_drawing(
        &mut self,
        sheet: u32,
        at: u32,
        rows: u32,
    ) -> PyResult<Vec<AnchorShiftInfo>> {
        self.inner
            .insert_rows_into_drawing(sheet, at, rows)
            .map(|report| report.into_iter().map(AnchorShiftInfo).collect())
            .map_err(to_py_err)
    }

    /// Moves every anchor on a sheet for `rows` removed at the zero-based `at`.
    fn remove_rows_from_drawing(
        &mut self,
        sheet: u32,
        at: u32,
        rows: u32,
    ) -> PyResult<Vec<AnchorShiftInfo>> {
        self.inner
            .remove_rows_from_drawing(sheet, at, rows)
            .map(|report| report.into_iter().map(AnchorShiftInfo).collect())
            .map_err(to_py_err)
    }

    /// Moves every anchor on a sheet for `columns` inserted at the zero-based `at`.
    fn insert_columns_into_drawing(
        &mut self,
        sheet: u32,
        at: u32,
        columns: u32,
    ) -> PyResult<Vec<AnchorShiftInfo>> {
        self.inner
            .insert_columns_into_drawing(sheet, at, columns)
            .map(|report| report.into_iter().map(AnchorShiftInfo).collect())
            .map_err(to_py_err)
    }

    /// Moves every anchor on a sheet for `columns` removed at the zero-based `at`.
    fn remove_columns_from_drawing(
        &mut self,
        sheet: u32,
        at: u32,
        columns: u32,
    ) -> PyResult<Vec<AnchorShiftInfo>> {
        self.inner
            .remove_columns_from_drawing(sheet, at, columns)
            .map(|report| report.into_iter().map(AnchorShiftInfo).collect())
            .map_err(to_py_err)
    }

    // --- tables -----------------------------------------------------------------------------------

    /// Every table on one sheet.
    fn sheet_tables(&self, sheet: u32) -> PyResult<Vec<SheetTableInfo>> {
        self.inner
            .sheet_tables(sheet)
            .map(|tables| tables.into_iter().map(SheetTableInfo).collect())
            .map_err(to_py_err)
    }

    /// Where the table style `name` comes from: this workbook, Excel's built-in set, or nowhere.
    fn table_style_origin(&self, name: &str) -> PyResult<TableStyleOrigin> {
        self.inner
            .table_style_origin(name)
            .map_err(to_py_err)
            .and_then(TableStyleOrigin::from_model)
    }

    /// The lowest `@id` no table in the workbook uses.
    fn next_table_id(&self) -> PyResult<u32> {
        self.inner.next_table_id().map_err(to_py_err)
    }

    // --- features, reported ------------------------------------------------------------------------

    /// The range one sheet's autofilter covers, or `None`.
    fn auto_filter_range(&self, sheet: u32) -> PyResult<Option<String>> {
        self.inner.auto_filter_range(sheet).map_err(to_py_err)
    }

    /// Removes one sheet's autofilter, answering whether there was one. Rows it hid stay hidden.
    fn remove_auto_filter(&mut self, sheet: u32) -> PyResult<bool> {
        self.inner.remove_auto_filter(sheet).map_err(to_py_err)
    }

    /// The ranges every data-validation rule on one sheet claims, one entry per rule.
    fn data_validation_ranges(&self, sheet: u32) -> PyResult<Vec<String>> {
        self.inner.data_validation_ranges(sheet).map_err(to_py_err)
    }

    /// Removes the `rule`-th data-validation rule of one sheet.
    fn remove_data_validation(&mut self, sheet: u32, rule: u32) -> PyResult<bool> {
        self.inner
            .remove_data_validation(sheet, rule)
            .map_err(to_py_err)
    }

    /// The ranges every conditional-formatting block on one sheet claims, one entry per block.
    fn conditional_formatting_ranges(&self, sheet: u32) -> PyResult<Vec<String>> {
        self.inner
            .conditional_formatting_ranges(sheet)
            .map_err(to_py_err)
    }

    /// How many rules the conditional-formatting block at `block` holds, or `None`.
    fn conditional_formatting_rule_count(&self, sheet: u32, block: u32) -> PyResult<Option<u32>> {
        self.inner
            .conditional_formatting_rule_count(sheet, block)
            .map_err(to_py_err)
    }

    // --- workbook metadata --------------------------------------------------------------------------

    /// Every defined name, with its scope resolved against the tab list.
    fn defined_names(&mut self) -> PyResult<Vec<DefinedName>> {
        self.inner
            .defined_names()
            .map(|names| names.into_iter().map(DefinedName).collect())
            .map_err(to_py_err)
    }

    /// One defined name by its `@name`, or `None`.
    fn defined_name(&mut self, name: &str) -> PyResult<Option<DefinedName>> {
        self.inner
            .defined_name(name)
            .map(|found| found.map(DefinedName))
            .map_err(to_py_err)
    }

    /// The print area of one tab, as the text the file wrote, or `None`.
    fn print_area(&mut self, sheet: u32) -> PyResult<Option<String>> {
        self.inner.print_area(sheet).map_err(to_py_err)
    }

    /// Which epoch this workbook's date serials count from. The two are 1,462 days apart.
    fn date_system(&mut self) -> PyResult<DateSystem> {
        self.inner
            .date_system()
            .map_err(to_py_err)
            .and_then(DateSystem::from_model)
    }

    /// `x:calcPr` — what the producer's calculation engine was told. Reported, never acted on.
    fn calculation_settings(&mut self) -> PyResult<CalculationSettings> {
        self.inner
            .calculation_settings()
            .map(CalculationSettings)
            .map_err(to_py_err)
    }

    // --- print setup and the preserved clusters --------------------------------------------------

    /// The printer-settings part one sheet reaches, or `None`. Opaque bytes, never XML.
    fn sheet_printer_settings(&self, sheet: u32) -> PyResult<Option<String>> {
        self.inner.sheet_printer_settings(sheet).map_err(to_py_err)
    }

    /// The background-picture part one sheet reaches, or `None`.
    fn sheet_background_image(&self, sheet: u32) -> PyResult<Option<String>> {
        self.inner.sheet_background_image(sheet).map_err(to_py_err)
    }

    /// Every part this project preserves rather than models. Resolved from the part graph alone.
    fn preserved_parts(&self) -> PyResult<PreservedPartsSummary> {
        self.inner
            .preserved_parts()
            .map(PreservedPartsSummary)
            .map_err(to_py_err)
    }

    /// Every pivot table in the workbook, with its cache resolved.
    fn pivot_tables(&self) -> PyResult<Vec<SheetPivotTableInfo>> {
        self.inner
            .pivot_tables()
            .map(|tables| tables.into_iter().map(SheetPivotTableInfo).collect())
            .map_err(to_py_err)
    }

    /// Every external workbook reference, with the target it names carried verbatim.
    fn external_links(&self) -> PyResult<Vec<WorkbookExternalLinkInfo>> {
        self.inner
            .external_links()
            .map(|links| links.into_iter().map(WorkbookExternalLinkInfo).collect())
            .map_err(to_py_err)
    }

    /// Every data connection the workbook declares.
    fn connections(&self) -> PyResult<Vec<WorkbookConnectionInfo>> {
        self.inner
            .connections()
            .map(|found| found.into_iter().map(WorkbookConnectionInfo).collect())
            .map_err(to_py_err)
    }

    /// Every query table, sheet by sheet.
    fn query_tables(&self) -> PyResult<Vec<SheetQueryTableInfo>> {
        self.inner
            .query_tables()
            .map(|tables| tables.into_iter().map(SheetQueryTableInfo).collect())
            .map_err(to_py_err)
    }

    /// The workbook's XML maps, or `None` when it declares none.
    fn xml_maps(&self) -> PyResult<Option<WorkbookXmlMapsInfo>> {
        self.inner
            .xml_maps()
            .map(|maps| maps.map(WorkbookXmlMapsInfo))
            .map_err(to_py_err)
    }

    /// What the workbook says about shared-workbook change tracking.
    fn revision_state(&self) -> PyResult<WorkbookRevisionState> {
        self.inner
            .revision_state()
            .map(WorkbookRevisionState)
            .map_err(to_py_err)
    }

    // --- the part graph ---------------------------------------------------------------------------

    /// The workbook part the package's `officeDocument` relationship names.
    fn workbook_part(&self) -> String {
        self.inner.workbook_part()
    }

    /// Every part in the package, in the order the container holds them.
    fn part_names(&self) -> Vec<String> {
        self.inner.part_names()
    }

    /// The content type of one part, or `None` when the package holds no such part.
    fn content_type_of(&self, part: &str) -> PyResult<Option<String>> {
        self.inner.content_type_of(part).map_err(to_py_err)
    }

    /// The bytes of one part, exactly as the package holds them. Reading never dirties a part.
    fn part_bytes<'py>(&self, python: Python<'py>, part: &str) -> PyResult<Bound<'py, PyBytes>> {
        self.inner
            .part_bytes(part)
            .map(|bytes| PyBytes::new(python, &bytes))
            .map_err(to_py_err)
    }

    fn __repr__(&self) -> String {
        format!("Workbook({} sheet(s))", self.inner.sheet_count())
    }
}

/// Adds `Workbook` to the extension module.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<Workbook>()
}
