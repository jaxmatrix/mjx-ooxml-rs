//! [`Workbook`] — the curated Excel surface, from JavaScript and TypeScript.
//!
//! ```js
//! import init, { Workbook, CellWrite } from "@mjx/ooxml";
//!
//! await init();
//! const workbook = Workbook.open(bytes);
//! try {
//!   for (const row of workbook.readRange(0, "A1:D20").rows()) {
//!     console.log(row);
//!   }
//! } finally {
//!   workbook.free();
//! }
//! ```
//!
//! Mirrors [`crate::deck::Deck`]'s own design exactly — see that module's doc comment for why
//! `free()` is mandatory and why methods are camelCase.
//!
//! # ⚠ Cells cross a range at a time, and there is no per-cell call
//!
//! This is the one place the Excel binding's shape differs from the other two, and the reason is
//! measured. `mjx_xlsx::Workbook` holds no parsed worksheet, so a per-cell call costs a whole-part
//! parse **every time**: 387 ms to read one cell of a 300,000-cell sheet, against 14.8 ms to open
//! the file. The Rust answer — hold the parsed worksheet yourself between one read and one write —
//! cannot cross this boundary, and a JavaScript caller has no escape hatch to reach it with. So the
//! door here is `readRange` / `readSheet` / `writeCells`, each of which parses once whatever it is
//! asked for.
//!
//! ```js
//! // One call, one parse, every cell.
//! workbook.writeCells(0, [
//!   CellWrite.sharedText("A1", "Region"),
//!   CellWrite.number("B1", 12.5),
//! ]);
//! ```
//!
//! # Addressing
//!
//! A **sheet** is a number — its position in the tab list. A **cell** is A1 text: `"B7"`. A
//! **range** is A1 text too: `"A1:C3"`, `"A:C"` for whole columns, `"1:3"` for whole rows. Nothing
//! here takes a `(row, column)` pair, so nothing here can be transposed by a caller who guessed the
//! order.

use wasm_bindgen::prelude::*;

use mjx_ooxml as ooxml;

use crate::charts::{
    ChartAxisData, ChartData, ChartErrorBarData, ChartLabelScope, ChartLegendData,
    ChartPointFormatData, ChartRangeSeries, ChartSeriesData, ChartSeriesFreshnessInfo,
    ChartSeriesReferences, ChartTrendlineData, DanglingPointReference, DataLabelSettings,
    DataLabelSpec, ErrorBarSpec, ResolvedRangeInfo, SheetChartWorkbookInfo, TrendlineSpec,
};
use crate::enums::{
    AxisOrientation, CellFormatTarget, ChartKind, DateSystem, LegendPosition, ResizingBehavior,
    TableStyleOrigin,
};
use crate::errors::map_error;
use crate::format::Format;
use crate::paint::{FillSpec, LineSpec};
use crate::spreadsheet::{
    AnchorBoundsInfo, AnchorShiftInfo, BorderSpec, CalculationSettings, CellBlock, CellFormatSpec,
    CellWrite, DefinedName, EffectiveCellFormat, FontProperties, GridAnomalyInfo, PatternFillSpec,
    PreservedPartsSummary, SheetCommentInfo, SheetDrawingInfo, SheetHyperlinkInfo,
    SheetPivotTableInfo, SheetQueryTableInfo, SheetSummary, SheetTableInfo, WorkbookConnectionInfo,
    WorkbookExternalLinkInfo, WorkbookRevisionState, WorkbookWindowInfo, WorkbookXmlMapsInfo,
};

/// A length a JavaScript caller stated as a number, as the EMU the model takes.
///
/// See the drawings block below for why the boundary is `f64`. `trunc` rather than a cast alone so
/// that a caller passing `2.5` gets 2 rather than an implementation-defined answer, and a
/// non-finite number becomes zero rather than an unspecified integer.
#[expect(
    clippy::cast_possible_truncation,
    reason = "an EMU length is far inside the range an f64 represents exactly"
)]
fn emu(value: f64) -> i64 {
    if value.is_finite() {
        value.trunc() as i64
    } else {
        0
    }
}

/// An open Excel workbook.
#[wasm_bindgen]
#[derive(Debug)]
pub struct Workbook {
    inner: ooxml::Workbook,
}

#[wasm_bindgen]
impl Workbook {
    /// A new workbook with nothing in it: one empty worksheet named `Sheet1`, a styles part, and the
    /// package around them.
    #[wasm_bindgen(js_name = "blank")]
    pub fn blank() -> Result<Self, JsValue> {
        map_error(ooxml::Workbook::blank().map(|inner| Self { inner }))
    }

    /// Opens a workbook from the bytes of a `.xlsx`, `.xlsm`, `.xltx` or `.xltm`.
    ///
    /// Throws an `OoxmlError` whose `code` is `"Io"` for bytes that are not a readable container,
    /// `"MalformedDocument"` for a package whose markup is not SpreadsheetML, and
    /// `"UnsupportedFormat"` — naming the format — for a PowerPoint or Word document, **and for a
    /// `.xlsb`**, whose main part is the MS-XLSB binary record stream rather than SpreadsheetML.
    /// That last refusal is permanent by design, not a not-yet.
    #[wasm_bindgen(js_name = "open")]
    pub fn open(data: &[u8]) -> Result<Self, JsValue> {
        map_error(ooxml::Workbook::open(data)).map(|inner| Self { inner })
    }

    /// What this workbook's main part says it is.
    #[wasm_bindgen(js_name = "format")]
    pub fn format(&self) -> Result<Format, JsValue> {
        Format::from_model(self.inner.format())
    }

    /// The workbook as the bytes of a `.xlsx`, **validated first**.
    #[wasm_bindgen(js_name = "save")]
    pub fn save(&self) -> Result<Vec<u8>, JsValue> {
        map_error(self.inner.save())
    }

    /// The workbook as bytes, **without** the validation pass.
    #[wasm_bindgen(js_name = "saveUnchecked")]
    pub fn save_unchecked(&self) -> Result<Vec<u8>, JsValue> {
        map_error(self.inner.save_unchecked())
    }

    /// Checks every invariant `save` enforces, without writing anything.
    #[wasm_bindgen(js_name = "validate")]
    pub fn validate(&self) -> Result<(), JsValue> {
        map_error(self.inner.validate())
    }

    // --- tabs -----------------------------------------------------------------------------------

    /// How many tabs the workbook lists.
    #[wasm_bindgen(js_name = "sheetCount")]
    pub fn sheet_count(&self) -> u32 {
        self.inner.sheet_count()
    }

    /// Every tab, in tab order.
    #[wasm_bindgen(js_name = "sheets")]
    pub fn sheets(&self) -> Vec<SheetSummary> {
        self.inner.sheets().into_iter().map(SheetSummary).collect()
    }

    /// One tab.
    #[wasm_bindgen(js_name = "sheet")]
    pub fn sheet(&self, sheet: u32) -> Result<SheetSummary, JsValue> {
        map_error(self.inner.sheet(sheet).map(SheetSummary))
    }

    /// The index of the tab named `name`, or `None` when no tab has that name.
    #[wasm_bindgen(js_name = "sheetIndex")]
    pub fn sheet_index(&self, name: &str) -> Option<u32> {
        self.inner.sheet_index(name)
    }

    /// Appends a new, empty worksheet and answers its index.
    #[wasm_bindgen(js_name = "addSheet")]
    pub fn add_sheet(&mut self, name: &str) -> Result<u32, JsValue> {
        map_error(self.inner.add_sheet(name))
    }

    /// Renames one tab. Formulas that reference the old name are **not** rewritten.
    #[wasm_bindgen(js_name = "renameSheet")]
    pub fn rename_sheet(&mut self, sheet: u32, name: &str) -> Result<(), JsValue> {
        map_error(self.inner.rename_sheet(sheet, name))
    }

    /// The index of the tab a consumer opens the workbook on, or `None`.
    #[wasm_bindgen(js_name = "activeSheet")]
    pub fn active_sheet(&mut self) -> Result<Option<u32>, JsValue> {
        map_error(self.inner.active_sheet())
    }

    /// Every `x:workbookView`, in document order.
    #[wasm_bindgen(js_name = "windowViews")]
    pub fn window_views(&mut self) -> Result<Vec<WorkbookWindowInfo>, JsValue> {
        map_error(
            self.inner
                .window_views()
                .map(|views| views.into_iter().map(WorkbookWindowInfo).collect()),
        )
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
    #[wasm_bindgen(js_name = "readRange")]
    pub fn read_range(&self, sheet: u32, range: &str) -> Result<CellBlock, JsValue> {
        map_error(self.inner.read_range(sheet, range).map(CellBlock))
    }

    /// Reads every populated cell of one sheet, parsing the worksheet **once**.
    #[wasm_bindgen(js_name = "readSheet")]
    pub fn read_sheet(&self, sheet: u32) -> Result<CellBlock, JsValue> {
        map_error(self.inner.read_sheet(sheet).map(CellBlock))
    }

    /// The A1 range of one sheet's populated extent, or `None` when nothing is populated.
    #[wasm_bindgen(js_name = "usedRange")]
    pub fn used_range(&self, sheet: u32) -> Result<Option<String>, JsValue> {
        map_error(self.inner.used_range(sheet))
    }

    /// Writes every entry of `cells` into one sheet, parsing **once** and serializing **once**.
    ///
    /// Entries are applied in the order given. Prefer top-to-bottom, left-to-right: that is
    /// append-only in the cell arena. A batch with a bad address writes **nothing**.
    ///
    /// # ⚠ The entries are **moved into this call**, not borrowed
    ///
    /// An array of exported objects crosses the boundary by value, so each `CellWrite` in `cells` is
    /// consumed here and its handle is dead when the call returns. **Do not call `free()` on them**,
    /// and build a fresh array to write the same batch twice. This is the one place in this binding
    /// where the "free everything you were handed" rule does not apply — it is what makes a batch of
    /// a hundred thousand writes one crossing and one copy rather than two.
    #[wasm_bindgen(js_name = "writeCells")]
    pub fn write_cells(&mut self, sheet: u32, cells: Vec<CellWrite>) -> Result<(), JsValue> {
        let cells: Vec<ooxml::CellWrite> = cells.into_iter().map(|write| write.0).collect();
        map_error(self.inner.write_cells(sheet, &cells))
    }

    // --- geometry ---------------------------------------------------------------------------------

    /// Every merged range on one sheet, as A1 text.
    #[wasm_bindgen(js_name = "mergedRanges")]
    pub fn merged_ranges(&self, sheet: u32) -> Result<Vec<String>, JsValue> {
        map_error(self.inner.merged_ranges(sheet))
    }

    /// The merged range covering `reference`, or `None` when that cell is not merged.
    #[wasm_bindgen(js_name = "mergedRangeContaining")]
    pub fn merged_range_containing(
        &self,
        sheet: u32,
        reference: &str,
    ) -> Result<Option<String>, JsValue> {
        map_error(self.inner.merged_range_containing(sheet, reference))
    }

    /// Merges `range`. Nothing is cleared: a merge is a display statement, not a destructive edit.
    #[wasm_bindgen(js_name = "mergeCells")]
    pub fn merge_cells(&mut self, sheet: u32, range: &str) -> Result<(), JsValue> {
        map_error(self.inner.merge_cells(sheet, range))
    }

    /// Removes the merge whose `@ref` is exactly `range`, answering whether one was there.
    #[wasm_bindgen(js_name = "unmergeCells")]
    pub fn unmerge_cells(&mut self, sheet: u32, range: &str) -> Result<bool, JsValue> {
        map_error(self.inner.unmerge_cells(sheet, range))
    }

    /// Sets a row's height in points. `row` is **one-based**, as `row@r` is. `custom=True` is the
    /// height a person set (Excel keeps it); `False` is one a consumer computed and may recompute.
    #[wasm_bindgen(js_name = "setRowHeight")]
    pub fn set_row_height(
        &mut self,
        sheet: u32,
        row: u32,
        points: Option<f64>,
        custom: bool,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_row_height(sheet, row, points, custom))
    }

    /// Hides or shows a row. `row` is **one-based**.
    #[wasm_bindgen(js_name = "setRowHidden")]
    pub fn set_row_hidden(&mut self, sheet: u32, row: u32, hidden: bool) -> Result<(), JsValue> {
        map_error(self.inner.set_row_hidden(sheet, row, hidden))
    }

    /// Sets a row's outline (grouping) depth. `row` is **one-based**.
    #[wasm_bindgen(js_name = "setRowOutlineLevel")]
    pub fn set_row_outline_level(
        &mut self,
        sheet: u32,
        row: u32,
        level: u8,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_row_outline_level(sheet, row, level))
    }

    /// Sets the width of the columns `first_column..=last_column`, both **zero-based**, in
    /// characters of the maximum digit width.
    #[wasm_bindgen(js_name = "setColumnWidth")]
    pub fn set_column_width(
        &mut self,
        sheet: u32,
        first_column: u32,
        last_column: u32,
        characters: Option<f64>,
        custom: bool,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .set_column_width(sheet, first_column, last_column, characters, custom),
        )
    }

    /// Hides or shows the columns `first_column..=last_column`, both **zero-based**.
    #[wasm_bindgen(js_name = "setColumnHidden")]
    pub fn set_column_hidden(
        &mut self,
        sheet: u32,
        first_column: u32,
        last_column: u32,
        hidden: bool,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .set_column_hidden(sheet, first_column, last_column, hidden),
        )
    }

    /// Sets the outline depth of the columns `first_column..=last_column`, both **zero-based**.
    #[wasm_bindgen(js_name = "setColumnOutlineLevel")]
    pub fn set_column_outline_level(
        &mut self,
        sheet: u32,
        first_column: u32,
        last_column: u32,
        level: u8,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .set_column_outline_level(sheet, first_column, last_column, level),
        )
    }

    /// Everything one sheet's grid says that a well-formed one would not. A report, never a repair.
    #[wasm_bindgen(js_name = "gridAnomalies")]
    pub fn grid_anomalies(&self, sheet: u32) -> Result<Vec<GridAnomalyInfo>, JsValue> {
        map_error(
            self.inner
                .grid_anomalies(sheet)
                .map(|found| found.into_iter().map(GridAnomalyInfo).collect()),
        )
    }

    // --- cell formats -----------------------------------------------------------------------------

    /// Appends a font to `xl/styles.xml`'s `fonts` table and answers its index.
    #[wasm_bindgen(js_name = "appendFont")]
    pub fn append_font(&mut self, properties: &FontProperties) -> Result<u32, JsValue> {
        map_error(self.inner.append_font(&properties.0))
    }

    /// Appends a pattern fill to the `fills` table and answers its index.
    #[wasm_bindgen(js_name = "appendPatternFill")]
    pub fn append_pattern_fill(&mut self, spec: &PatternFillSpec) -> Result<u32, JsValue> {
        map_error(self.inner.append_pattern_fill(&spec.0))
    }

    /// Appends a border to the `borders` table and answers its index.
    #[wasm_bindgen(js_name = "appendBorder")]
    pub fn append_border(&mut self, spec: &BorderSpec) -> Result<u32, JsValue> {
        map_error(self.inner.append_border(&spec.0))
    }

    /// Appends an `x:xf` to `cellXfs` or `cellStyleXfs` and answers its index.
    #[wasm_bindgen(js_name = "appendCellFormat")]
    pub fn append_cell_format(
        &mut self,
        target: CellFormatTarget,
        spec: &CellFormatSpec,
    ) -> Result<u32, JsValue> {
        map_error(self.inner.append_cell_format(target.into(), &spec.0))
    }

    /// Points one cell at `cellXfs[style]`, or removes its `@s` with `None`. The cell must already
    /// exist — write the value first.
    #[wasm_bindgen(js_name = "setCellStyle")]
    pub fn set_cell_style(
        &mut self,
        sheet: u32,
        reference: &str,
        style: Option<u32>,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_cell_style(sheet, reference, style))
    }

    /// Interns `text` into `xl/sharedStrings.xml` and answers its index.
    #[wasm_bindgen(js_name = "internSharedString")]
    pub fn intern_shared_string(&mut self, text: &str) -> Result<u32, JsValue> {
        map_error(self.inner.intern_shared_string(text))
    }

    /// What one cell's format resolves to, after the `cellXfs` -> `cellStyleXfs` ladder and the
    /// column and row defaults above it. **What the file states, not what a renderer shows.**
    #[wasm_bindgen(js_name = "effectiveCellFormat")]
    pub fn effective_cell_format(
        &self,
        sheet: u32,
        reference: &str,
    ) -> Result<Option<EffectiveCellFormat>, JsValue> {
        map_error(
            self.inner
                .effective_cell_format(sheet, reference)
                .map(|found| found.map(EffectiveCellFormat)),
        )
    }

    /// The same ladder, answered for the **anchor** of the merged region `reference` falls in.
    #[wasm_bindgen(js_name = "effectiveMergedCellFormat")]
    pub fn effective_merged_cell_format(
        &self,
        sheet: u32,
        reference: &str,
    ) -> Result<Option<EffectiveCellFormat>, JsValue> {
        map_error(
            self.inner
                .effective_merged_cell_format(sheet, reference)
                .map(|found| found.map(EffectiveCellFormat)),
        )
    }

    // --- hyperlinks -------------------------------------------------------------------------------

    /// Every hyperlink on one sheet, in document order.
    #[wasm_bindgen(js_name = "sheetHyperlinks")]
    pub fn sheet_hyperlinks(&self, sheet: u32) -> Result<Vec<SheetHyperlinkInfo>, JsValue> {
        map_error(
            self.inner
                .sheet_hyperlinks(sheet)
                .map(|links| links.into_iter().map(SheetHyperlinkInfo).collect()),
        )
    }

    /// The hyperlink whose range covers `reference`, or `None`.
    #[wasm_bindgen(js_name = "cellHyperlink")]
    pub fn cell_hyperlink(
        &self,
        sheet: u32,
        reference: &str,
    ) -> Result<Option<SheetHyperlinkInfo>, JsValue> {
        map_error(
            self.inner
                .cell_hyperlink(sheet, reference)
                .map(|link| link.map(SheetHyperlinkInfo)),
        )
    }

    /// Points `range` at an external URL, writing the entry **and** its `External` relationship.
    #[wasm_bindgen(js_name = "setCellHyperlinkUrl")]
    pub fn set_cell_hyperlink_url(
        &mut self,
        sheet: u32,
        range: &str,
        url: &str,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_cell_hyperlink_url(sheet, range, url))
    }

    /// Points `range` at a location inside this workbook — `"Sheet2!A1"`, or a defined name.
    #[wasm_bindgen(js_name = "setCellHyperlinkLocation")]
    pub fn set_cell_hyperlink_location(
        &mut self,
        sheet: u32,
        range: &str,
        location: &str,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .set_cell_hyperlink_location(sheet, range, location),
        )
    }

    /// Removes the hyperlink covering `reference`, and the relationship it named.
    #[wasm_bindgen(js_name = "removeCellHyperlink")]
    pub fn remove_cell_hyperlink(&mut self, sheet: u32, reference: &str) -> Result<bool, JsValue> {
        map_error(self.inner.remove_cell_hyperlink(sheet, reference))
    }

    // --- cell comments ----------------------------------------------------------------------------

    /// Every comment on one sheet, in the order the comments part lists them.
    #[wasm_bindgen(js_name = "sheetComments")]
    pub fn sheet_comments(&self, sheet: u32) -> Result<Vec<SheetCommentInfo>, JsValue> {
        map_error(
            self.inner
                .sheet_comments(sheet)
                .map(|comments| comments.into_iter().map(SheetCommentInfo).collect()),
        )
    }

    /// The comment attached to `reference`, or `undefined`.
    #[wasm_bindgen(js_name = "cellComment")]
    pub fn cell_comment(
        &self,
        sheet: u32,
        reference: &str,
    ) -> Result<Option<SheetCommentInfo>, JsValue> {
        map_error(
            self.inner
                .cell_comment(sheet, reference)
                .map(|comment| comment.map(SheetCommentInfo)),
        )
    }

    /// Attaches a comment to `reference`, writing both halves, and answers its shape identifier.
    #[wasm_bindgen(js_name = "addCellComment")]
    pub fn add_cell_comment(
        &mut self,
        sheet: u32,
        reference: &str,
        author: &str,
        text: &str,
    ) -> Result<u32, JsValue> {
        map_error(self.inner.add_cell_comment(sheet, reference, author, text))
    }

    /// Replaces the text of the comment on `reference`, leaving its box as it was.
    #[wasm_bindgen(js_name = "setCellCommentText")]
    pub fn set_cell_comment_text(
        &mut self,
        sheet: u32,
        reference: &str,
        text: &str,
    ) -> Result<bool, JsValue> {
        map_error(self.inner.set_cell_comment_text(sheet, reference, text))
    }

    /// Removes the comment on `reference` — both halves.
    #[wasm_bindgen(js_name = "removeCellComment")]
    pub fn remove_cell_comment(&mut self, sheet: u32, reference: &str) -> Result<bool, JsValue> {
        map_error(self.inner.remove_cell_comment(sheet, reference))
    }

    /// The `@id` of the legacy VML shape an OLE object on a sheet is drawn as, or `undefined`.
    #[wasm_bindgen(js_name = "vmlShapeIdForOleObject")]
    pub fn vml_shape_id_for_ole_object(
        &self,
        sheet: u32,
        object: u32,
    ) -> Result<Option<String>, JsValue> {
        map_error(self.inner.vml_shape_id_for_ole_object(sheet, object))
    }

    /// The `@id` of the legacy VML shape a form control on a sheet is drawn as, or `undefined`.
    #[wasm_bindgen(js_name = "vmlShapeIdForFormControl")]
    pub fn vml_shape_id_for_form_control(
        &self,
        sheet: u32,
        control: u32,
    ) -> Result<Option<String>, JsValue> {
        map_error(self.inner.vml_shape_id_for_form_control(sheet, control))
    }

    /// The verbatim bytes of the legacy VML drawing part behind one sheet, or `undefined`.
    #[wasm_bindgen(js_name = "sheetVmlPartBytes")]
    pub fn sheet_vml_part_bytes(&self, sheet: u32) -> Result<Option<Vec<u8>>, JsValue> {
        map_error(self.inner.sheet_vml_part_bytes(sheet))
    }

    // --- drawings ---------------------------------------------------------------------------------
    //
    // Every EMU argument here is an `f64`, not an `i64`. That is the shape MJXOFF-137 settled for
    // this binding and `Document::addInlinePicture` already follows: a JavaScript number *is* an
    // `f64`, and taking an `i64` would hand a TypeScript caller a `bigint` for a length that never
    // needs one — an EMU is 1/914,400 of an inch, so the whole of a sheet fits inside 2^53 with
    // eleven orders of magnitude to spare.

    /// The drawing part behind one sheet, and everything anchored in it.
    #[wasm_bindgen(js_name = "sheetDrawing")]
    pub fn sheet_drawing(&self, sheet: u32) -> Result<Option<SheetDrawingInfo>, JsValue> {
        map_error(
            self.inner
                .sheet_drawing(sheet)
                .map(|drawing| drawing.map(SheetDrawingInfo)),
        )
    }

    /// Where the anchor at `anchor` puts its object, in EMU. `7.0` and `96.0` are ECMA-376's own
    /// worked example, for 11-point Calibri.
    #[wasm_bindgen(js_name = "sheetAnchorBounds")]
    pub fn sheet_anchor_bounds(
        &self,
        sheet: u32,
        anchor: u32,
        maximum_digit_width_pixels: f64,
        pixels_per_inch: f64,
    ) -> Result<Option<AnchorBoundsInfo>, JsValue> {
        map_error(
            self.inner
                .sheet_anchor_bounds(sheet, anchor, maximum_digit_width_pixels, pixels_per_inch)
                .map(|bounds| bounds.map(AnchorBoundsInfo)),
        )
    }

    /// Anchors a picture between two cells, and answers its position in the paint order.
    #[wasm_bindgen(js_name = "addTwoCellAnchoredPicture")]
    #[allow(clippy::too_many_arguments)]
    pub fn add_two_cell_anchored_picture(
        &mut self,
        sheet: u32,
        image_bytes: Vec<u8>,
        name: &str,
        from_column: u32,
        from_column_offset_emu: f64,
        from_row: u32,
        from_row_offset_emu: f64,
        to_column: u32,
        to_column_offset_emu: f64,
        to_row: u32,
        to_row_offset_emu: f64,
        resizing: ResizingBehavior,
    ) -> Result<u32, JsValue> {
        map_error(self.inner.add_two_cell_anchored_picture(
            sheet,
            &image_bytes,
            name,
            from_column,
            emu(from_column_offset_emu),
            from_row,
            emu(from_row_offset_emu),
            to_column,
            emu(to_column_offset_emu),
            to_row,
            emu(to_row_offset_emu),
            resizing.into(),
        ))
    }

    /// Anchors a picture to one cell, at its own size.
    #[wasm_bindgen(js_name = "addOneCellAnchoredPicture")]
    #[allow(clippy::too_many_arguments)]
    pub fn add_one_cell_anchored_picture(
        &mut self,
        sheet: u32,
        image_bytes: Vec<u8>,
        name: &str,
        from_column: u32,
        from_column_offset_emu: f64,
        from_row: u32,
        from_row_offset_emu: f64,
        width_emu: f64,
        height_emu: f64,
    ) -> Result<u32, JsValue> {
        map_error(self.inner.add_one_cell_anchored_picture(
            sheet,
            &image_bytes,
            name,
            from_column,
            emu(from_column_offset_emu),
            from_row,
            emu(from_row_offset_emu),
            emu(width_emu),
            emu(height_emu),
        ))
    }

    /// Anchors a picture to the sheet, at an absolute position and size in EMU.
    #[wasm_bindgen(js_name = "addAbsoluteAnchoredPicture")]
    #[allow(clippy::too_many_arguments)]
    pub fn add_absolute_anchored_picture(
        &mut self,
        sheet: u32,
        image_bytes: Vec<u8>,
        name: &str,
        x_emu: f64,
        y_emu: f64,
        width_emu: f64,
        height_emu: f64,
    ) -> Result<u32, JsValue> {
        map_error(self.inner.add_absolute_anchored_picture(
            sheet,
            &image_bytes,
            name,
            emu(x_emu),
            emu(y_emu),
            emu(width_emu),
            emu(height_emu),
        ))
    }

    /// Removes one anchored object from a sheet, reporting whether there was one.
    #[wasm_bindgen(js_name = "removeSheetDrawingObject")]
    pub fn remove_sheet_drawing_object(
        &mut self,
        sheet: u32,
        anchor: u32,
    ) -> Result<bool, JsValue> {
        map_error(self.inner.remove_sheet_drawing_object(sheet, anchor))
    }

    /// Moves every anchor on a sheet for `rows` inserted at the zero-based `at`.
    #[wasm_bindgen(js_name = "insertRowsIntoDrawing")]
    pub fn insert_rows_into_drawing(
        &mut self,
        sheet: u32,
        at: u32,
        rows: u32,
    ) -> Result<Vec<AnchorShiftInfo>, JsValue> {
        map_error(
            self.inner
                .insert_rows_into_drawing(sheet, at, rows)
                .map(|report| report.into_iter().map(AnchorShiftInfo).collect()),
        )
    }

    /// Moves every anchor on a sheet for `rows` removed at the zero-based `at`.
    #[wasm_bindgen(js_name = "removeRowsFromDrawing")]
    pub fn remove_rows_from_drawing(
        &mut self,
        sheet: u32,
        at: u32,
        rows: u32,
    ) -> Result<Vec<AnchorShiftInfo>, JsValue> {
        map_error(
            self.inner
                .remove_rows_from_drawing(sheet, at, rows)
                .map(|report| report.into_iter().map(AnchorShiftInfo).collect()),
        )
    }

    /// Moves every anchor on a sheet for `columns` inserted at the zero-based `at`.
    #[wasm_bindgen(js_name = "insertColumnsIntoDrawing")]
    pub fn insert_columns_into_drawing(
        &mut self,
        sheet: u32,
        at: u32,
        columns: u32,
    ) -> Result<Vec<AnchorShiftInfo>, JsValue> {
        map_error(
            self.inner
                .insert_columns_into_drawing(sheet, at, columns)
                .map(|report| report.into_iter().map(AnchorShiftInfo).collect()),
        )
    }

    /// Moves every anchor on a sheet for `columns` removed at the zero-based `at`.
    #[wasm_bindgen(js_name = "removeColumnsFromDrawing")]
    pub fn remove_columns_from_drawing(
        &mut self,
        sheet: u32,
        at: u32,
        columns: u32,
    ) -> Result<Vec<AnchorShiftInfo>, JsValue> {
        map_error(
            self.inner
                .remove_columns_from_drawing(sheet, at, columns)
                .map(|report| report.into_iter().map(AnchorShiftInfo).collect()),
        )
    }

    // --- tables -----------------------------------------------------------------------------------

    /// Every table on one sheet.
    #[wasm_bindgen(js_name = "sheetTables")]
    pub fn sheet_tables(&self, sheet: u32) -> Result<Vec<SheetTableInfo>, JsValue> {
        map_error(
            self.inner
                .sheet_tables(sheet)
                .map(|tables| tables.into_iter().map(SheetTableInfo).collect()),
        )
    }

    /// Where the table style `name` comes from: this workbook, Excel's built-in set, or nowhere.
    #[wasm_bindgen(js_name = "tableStyleOrigin")]
    pub fn table_style_origin(&self, name: &str) -> Result<TableStyleOrigin, JsValue> {
        map_error(self.inner.table_style_origin(name)).and_then(TableStyleOrigin::from_model)
    }

    /// The lowest `@id` no table in the workbook uses.
    #[wasm_bindgen(js_name = "nextTableId")]
    pub fn next_table_id(&self) -> Result<u32, JsValue> {
        map_error(self.inner.next_table_id())
    }

    // --- features, reported ------------------------------------------------------------------------

    /// The range one sheet's autofilter covers, or `None`.
    #[wasm_bindgen(js_name = "autoFilterRange")]
    pub fn auto_filter_range(&self, sheet: u32) -> Result<Option<String>, JsValue> {
        map_error(self.inner.auto_filter_range(sheet))
    }

    /// Removes one sheet's autofilter, answering whether there was one. Rows it hid stay hidden.
    #[wasm_bindgen(js_name = "removeAutoFilter")]
    pub fn remove_auto_filter(&mut self, sheet: u32) -> Result<bool, JsValue> {
        map_error(self.inner.remove_auto_filter(sheet))
    }

    /// The ranges every data-validation rule on one sheet claims, one entry per rule.
    #[wasm_bindgen(js_name = "dataValidationRanges")]
    pub fn data_validation_ranges(&self, sheet: u32) -> Result<Vec<String>, JsValue> {
        map_error(self.inner.data_validation_ranges(sheet))
    }

    /// Removes the `rule`-th data-validation rule of one sheet.
    #[wasm_bindgen(js_name = "removeDataValidation")]
    pub fn remove_data_validation(&mut self, sheet: u32, rule: u32) -> Result<bool, JsValue> {
        map_error(self.inner.remove_data_validation(sheet, rule))
    }

    /// The ranges every conditional-formatting block on one sheet claims, one entry per block.
    #[wasm_bindgen(js_name = "conditionalFormattingRanges")]
    pub fn conditional_formatting_ranges(&self, sheet: u32) -> Result<Vec<String>, JsValue> {
        map_error(self.inner.conditional_formatting_ranges(sheet))
    }

    /// How many rules the conditional-formatting block at `block` holds, or `None`.
    #[wasm_bindgen(js_name = "conditionalFormattingRuleCount")]
    pub fn conditional_formatting_rule_count(
        &self,
        sheet: u32,
        block: u32,
    ) -> Result<Option<u32>, JsValue> {
        map_error(self.inner.conditional_formatting_rule_count(sheet, block))
    }

    // --- workbook metadata --------------------------------------------------------------------------

    /// Every defined name, with its scope resolved against the tab list.
    #[wasm_bindgen(js_name = "definedNames")]
    pub fn defined_names(&mut self) -> Result<Vec<DefinedName>, JsValue> {
        map_error(
            self.inner
                .defined_names()
                .map(|names| names.into_iter().map(DefinedName).collect()),
        )
    }

    /// One defined name by its `@name`, or `None`.
    #[wasm_bindgen(js_name = "definedName")]
    pub fn defined_name(&mut self, name: &str) -> Result<Option<DefinedName>, JsValue> {
        map_error(
            self.inner
                .defined_name(name)
                .map(|found| found.map(DefinedName)),
        )
    }

    /// The print area of one tab, as the text the file wrote, or `None`.
    #[wasm_bindgen(js_name = "printArea")]
    pub fn print_area(&mut self, sheet: u32) -> Result<Option<String>, JsValue> {
        map_error(self.inner.print_area(sheet))
    }

    /// Which epoch this workbook's date serials count from. The two are 1,462 days apart.
    #[wasm_bindgen(js_name = "dateSystem")]
    pub fn date_system(&mut self) -> Result<DateSystem, JsValue> {
        map_error(self.inner.date_system()).and_then(DateSystem::from_model)
    }

    /// `x:calcPr` — what the producer's calculation engine was told. Reported, never acted on.
    #[wasm_bindgen(js_name = "calculationSettings")]
    pub fn calculation_settings(&mut self) -> Result<CalculationSettings, JsValue> {
        map_error(self.inner.calculation_settings().map(CalculationSettings))
    }

    // --- print setup and the preserved clusters --------------------------------------------------

    /// The printer-settings part one sheet reaches, or `None`. Opaque bytes, never XML.
    #[wasm_bindgen(js_name = "sheetPrinterSettings")]
    pub fn sheet_printer_settings(&self, sheet: u32) -> Result<Option<String>, JsValue> {
        map_error(self.inner.sheet_printer_settings(sheet))
    }

    /// The background-picture part one sheet reaches, or `None`.
    #[wasm_bindgen(js_name = "sheetBackgroundImage")]
    pub fn sheet_background_image(&self, sheet: u32) -> Result<Option<String>, JsValue> {
        map_error(self.inner.sheet_background_image(sheet))
    }

    /// Every part this project preserves rather than models. Resolved from the part graph alone.
    #[wasm_bindgen(js_name = "preservedParts")]
    pub fn preserved_parts(&self) -> Result<PreservedPartsSummary, JsValue> {
        map_error(self.inner.preserved_parts().map(PreservedPartsSummary))
    }

    /// Every pivot table in the workbook, with its cache resolved.
    #[wasm_bindgen(js_name = "pivotTables")]
    pub fn pivot_tables(&self) -> Result<Vec<SheetPivotTableInfo>, JsValue> {
        map_error(
            self.inner
                .pivot_tables()
                .map(|tables| tables.into_iter().map(SheetPivotTableInfo).collect()),
        )
    }

    /// Every external workbook reference, with the target it names carried verbatim.
    #[wasm_bindgen(js_name = "externalLinks")]
    pub fn external_links(&self) -> Result<Vec<WorkbookExternalLinkInfo>, JsValue> {
        map_error(
            self.inner
                .external_links()
                .map(|links| links.into_iter().map(WorkbookExternalLinkInfo).collect()),
        )
    }

    /// Every data connection the workbook declares.
    #[wasm_bindgen(js_name = "connections")]
    pub fn connections(&self) -> Result<Vec<WorkbookConnectionInfo>, JsValue> {
        map_error(
            self.inner
                .connections()
                .map(|found| found.into_iter().map(WorkbookConnectionInfo).collect()),
        )
    }

    /// Every query table, sheet by sheet.
    #[wasm_bindgen(js_name = "queryTables")]
    pub fn query_tables(&self) -> Result<Vec<SheetQueryTableInfo>, JsValue> {
        map_error(
            self.inner
                .query_tables()
                .map(|tables| tables.into_iter().map(SheetQueryTableInfo).collect()),
        )
    }

    /// The workbook's XML maps, or `None` when it declares none.
    #[wasm_bindgen(js_name = "xmlMaps")]
    pub fn xml_maps(&self) -> Result<Option<WorkbookXmlMapsInfo>, JsValue> {
        map_error(
            self.inner
                .xml_maps()
                .map(|maps| maps.map(WorkbookXmlMapsInfo)),
        )
    }

    /// What the workbook says about shared-workbook change tracking.
    #[wasm_bindgen(js_name = "revisionState")]
    pub fn revision_state(&self) -> Result<WorkbookRevisionState, JsValue> {
        map_error(self.inner.revision_state().map(WorkbookRevisionState))
    }

    // --- the part graph ---------------------------------------------------------------------------

    /// The workbook part the package's `officeDocument` relationship names.
    #[wasm_bindgen(js_name = "workbookPart")]
    pub fn workbook_part(&self) -> String {
        self.inner.workbook_part()
    }

    /// Every part in the package, in the order the container holds them.
    #[wasm_bindgen(js_name = "partNames")]
    pub fn part_names(&self) -> Vec<String> {
        self.inner.part_names()
    }

    /// The content type of one part, or `None` when the package holds no such part.
    #[wasm_bindgen(js_name = "contentTypeOf")]
    pub fn content_type_of(&self, part: &str) -> Result<Option<String>, JsValue> {
        map_error(self.inner.content_type_of(part))
    }

    /// The bytes of one part, exactly as the package holds them. Reading never dirties a part.
    #[wasm_bindgen(js_name = "partBytes")]
    pub fn part_bytes(&self, part: &str) -> Result<Vec<u8>, JsValue> {
        map_error(self.inner.part_bytes(part))
    }

    // -----------------------------------------------------------------------------------------
    // Charts (MJXOFF-111)
    // -----------------------------------------------------------------------------------------

    /// The index of every anchor on `sheet` that frames a chart, in paint order.
    #[wasm_bindgen(js_name = "chartAnchorIndices")]
    pub fn chart_anchor_indices(&mut self, sheet: u32) -> Result<Vec<u32>, JsValue> {
        map_error(self.inner.chart_anchor_indices(sheet))
    }

    /// The relationship id the anchor names as its chart part, or `undefined` when it frames no
    /// chart.
    #[wasm_bindgen(js_name = "chartRelId")]
    pub fn chart_rel_id(&mut self, sheet: u32, anchor: u32) -> Result<Option<String>, JsValue> {
        map_error(self.inner.chart_rel_id(sheet, anchor))
    }

    /// The raw XML of the chart part the anchor frames, or `undefined` when it frames no chart.
    #[wasm_bindgen(js_name = "chartPartBytes")]
    pub fn chart_part_bytes(
        &mut self,
        sheet: u32,
        anchor: u32,
    ) -> Result<Option<Vec<u8>>, JsValue> {
        map_error(self.inner.chart_part_bytes(sheet, anchor))
    }

    /// Anchors `chart` between two cells, with the embedded workbook Office's *Edit Data* opens.
    /// Answers its position in the paint order.
    #[wasm_bindgen(js_name = "addChart")]
    #[allow(clippy::too_many_arguments)]
    pub fn add_chart(
        &mut self,
        sheet: u32,
        chart: &ChartData,
        from_column: u32,
        from_row: u32,
        to_column: u32,
        to_row: u32,
        name: &str,
        resizing: ResizingBehavior,
    ) -> Result<u32, JsValue> {
        map_error(self.inner.add_chart(
            sheet,
            &chart.0,
            from_column,
            from_row,
            to_column,
            to_row,
            name,
            resizing.into(),
        ))
    }

    /// Anchors a chart taking its data from cells in this workbook — no embedded copy at all.
    /// Answers its position in the paint order.
    #[wasm_bindgen(js_name = "addRangeChart")]
    #[allow(clippy::too_many_arguments)]
    pub fn add_range_chart(
        &mut self,
        sheet: u32,
        kind: ChartKind,
        categories: Option<String>,
        series: Vec<ChartRangeSeries>,
        from_column: u32,
        from_row: u32,
        to_column: u32,
        to_row: u32,
        name: &str,
        resizing: ResizingBehavior,
    ) -> Result<u32, JsValue> {
        let series: Vec<ooxml::ChartRangeSeries> =
            series.into_iter().map(|entry| entry.0).collect();
        map_error(self.inner.add_range_chart(
            sheet,
            kind.into(),
            categories.as_deref(),
            &series,
            from_column,
            from_row,
            to_column,
            to_row,
            name,
            resizing.into(),
        ))
    }

    /// Every chart in the workbook that references a backing workbook. A chart whose data is a live
    /// range has none, and is absent from this list.
    #[wasm_bindgen(js_name = "chartWorkbooks")]
    pub fn chart_workbooks(&mut self) -> Result<Vec<SheetChartWorkbookInfo>, JsValue> {
        map_error(self.inner.chart_workbooks())
            .map(|values| values.into_iter().map(SheetChartWorkbookInfo).collect())
    }

    /// Rewrites the embedded workbook of the chart. Answers `false` — changing nothing — when there
    /// is none, which is the ordinary state of a chart on a sheet.
    #[wasm_bindgen(js_name = "refreshChartWorkbook")]
    pub fn refresh_chart_workbook(&mut self, sheet: u32, anchor: u32) -> Result<bool, JsValue> {
        map_error(self.inner.refresh_chart_workbook(sheet, anchor))
    }

    /// Detaches the backing workbook, leaving the chart to render from its cached values. The
    /// workbook part goes with it unless another chart still names it.
    #[wasm_bindgen(js_name = "detachChartWorkbook")]
    pub fn detach_chart_workbook(&mut self, sheet: u32, anchor: u32) -> Result<(), JsValue> {
        map_error(self.inner.detach_chart_workbook(sheet, anchor))
    }

    /// Where every series says its data lives — the formula beside each cache, as written.
    #[wasm_bindgen(js_name = "chartSeriesReferences")]
    pub fn chart_series_references(
        &mut self,
        sheet: u32,
        anchor: u32,
    ) -> Result<Vec<ChartSeriesReferences>, JsValue> {
        map_error(self.inner.chart_series_references(sheet, anchor))
            .map(|values| values.into_iter().map(ChartSeriesReferences).collect())
    }

    /// Every series of the chart, read from the cells its formulas name rather than from its
    /// caches.
    #[wasm_bindgen(js_name = "chartSeriesFromCells")]
    pub fn chart_series_from_cells(
        &mut self,
        sheet: u32,
        anchor: u32,
    ) -> Result<Vec<ChartSeriesData>, JsValue> {
        map_error(self.inner.chart_series_from_cells(sheet, anchor))
            .map(|values| values.into_iter().map(ChartSeriesData).collect())
    }

    /// Every series' cache set beside what its cells say, with each named. The cache is what draws
    /// until a consumer recalculates; neither is silently preferred.
    #[wasm_bindgen(js_name = "chartSeriesFreshness")]
    pub fn chart_series_freshness(
        &mut self,
        sheet: u32,
        anchor: u32,
    ) -> Result<Vec<ChartSeriesFreshnessInfo>, JsValue> {
        map_error(self.inner.chart_series_freshness(sheet, anchor))
            .map(|values| values.into_iter().map(ChartSeriesFreshnessInfo).collect())
    }

    /// Rewrites the chart's caches from the cells its formulas name, answering how many series
    /// changed. The opt-in repair — writing a cell never does this for you.
    #[wasm_bindgen(js_name = "refreshChartCacheFromCells")]
    pub fn refresh_chart_cache_from_cells(
        &mut self,
        sheet: u32,
        anchor: u32,
    ) -> Result<u32, JsValue> {
        map_error(self.inner.refresh_chart_cache_from_cells(sheet, anchor))
    }

    /// Resolves a reference — a chart's formula, a defined name — against this workbook's cells,
    /// with `sheet` as the tab an area that names none means.
    #[wasm_bindgen(js_name = "resolveRangeReference")]
    pub fn resolve_range_reference(
        &mut self,
        sheet: u32,
        reference: &str,
    ) -> Result<ResolvedRangeInfo, JsValue> {
        map_error(self.inner.resolve_range_reference(sheet, reference)).map(ResolvedRangeInfo)
    }

    /// The series of the chart, from its caches.
    #[wasm_bindgen(js_name = "chartSeries")]
    pub fn chart_series(
        &mut self,
        sheet: u32,
        anchor: u32,
    ) -> Result<Vec<ChartSeriesData>, JsValue> {
        map_error(self.inner.chart_series(sheet, anchor))
            .map(|values| values.into_iter().map(ChartSeriesData).collect())
    }

    /// The kind of every plot the chart draws, in document order.
    #[wasm_bindgen(js_name = "chartKinds")]
    pub fn chart_kinds(&mut self, sheet: u32, anchor: u32) -> Result<Vec<ChartKind>, JsValue> {
        map_error(self.inner.chart_kinds(sheet, anchor))?
            .into_iter()
            .map(ChartKind::from_model)
            .collect()
    }

    /// The axes of the chart, in document order.
    #[wasm_bindgen(js_name = "chartAxes")]
    pub fn chart_axes(&mut self, sheet: u32, anchor: u32) -> Result<Vec<ChartAxisData>, JsValue> {
        map_error(self.inner.chart_axes(sheet, anchor))
            .map(|values| values.into_iter().map(ChartAxisData).collect())
    }

    /// The heading of the chart, or `undefined` when it has none.
    #[wasm_bindgen(js_name = "chartTitle")]
    pub fn chart_title(&mut self, sheet: u32, anchor: u32) -> Result<Option<String>, JsValue> {
        map_error(self.inner.chart_title(sheet, anchor))
    }

    /// The legend of the chart, or `undefined` when it has none.
    #[wasm_bindgen(js_name = "chartLegend")]
    pub fn chart_legend(
        &mut self,
        sheet: u32,
        anchor: u32,
    ) -> Result<Option<ChartLegendData>, JsValue> {
        map_error(self.inner.chart_legend(sheet, anchor)).map(|value| value.map(ChartLegendData))
    }

    /// The built-in style id the chart names, or `undefined`.
    #[wasm_bindgen(js_name = "chartStyleId")]
    pub fn chart_style_id(&mut self, sheet: u32, anchor: u32) -> Result<Option<u32>, JsValue> {
        map_error(self.inner.chart_style_id(sheet, anchor))
    }

    /// The fill of series `seriesIdx`, or `undefined` when it takes its colour from the chart style.
    #[wasm_bindgen(js_name = "chartSeriesFill")]
    pub fn chart_series_fill(
        &mut self,
        sheet: u32,
        anchor: u32,
        series_idx: u32,
    ) -> Result<Option<FillSpec>, JsValue> {
        map_error(self.inner.chart_series_fill(sheet, anchor, series_idx))
            .map(|value| value.map(FillSpec))
    }

    /// The data-label settings in force for one point of series `seriesIdx`.
    #[wasm_bindgen(js_name = "chartDataLabels")]
    pub fn chart_data_labels(
        &mut self,
        sheet: u32,
        anchor: u32,
        series_idx: u32,
        point_idx: Option<u32>,
    ) -> Result<DataLabelSettings, JsValue> {
        map_error(
            self.inner
                .chart_data_labels(sheet, anchor, series_idx, point_idx),
        )
        .map(DataLabelSettings)
    }

    /// The data-label settings one tier states in its own right.
    #[wasm_bindgen(js_name = "chartDataLabelTier")]
    pub fn chart_data_label_tier(
        &mut self,
        sheet: u32,
        anchor: u32,
        scope: &ChartLabelScope,
    ) -> Result<Option<DataLabelSettings>, JsValue> {
        map_error(self.inner.chart_data_label_tier(sheet, anchor, scope.0))
            .map(|value| value.map(DataLabelSettings))
    }

    /// The words one point's label shows in place of its value, or `undefined`.
    #[wasm_bindgen(js_name = "chartPointLabelText")]
    pub fn chart_point_label_text(
        &mut self,
        sheet: u32,
        anchor: u32,
        series_idx: u32,
        point_idx: u32,
    ) -> Result<Option<String>, JsValue> {
        map_error(
            self.inner
                .chart_point_label_text(sheet, anchor, series_idx, point_idx),
        )
    }

    /// Every point of series `seriesIdx` that carries its own formatting.
    #[wasm_bindgen(js_name = "chartPointFormats")]
    pub fn chart_point_formats(
        &mut self,
        sheet: u32,
        anchor: u32,
        series_idx: u32,
    ) -> Result<Vec<ChartPointFormatData>, JsValue> {
        map_error(self.inner.chart_point_formats(sheet, anchor, series_idx))
            .map(|values| values.into_iter().map(ChartPointFormatData).collect())
    }

    /// Every trendline fitted through series `seriesIdx`.
    #[wasm_bindgen(js_name = "chartTrendlines")]
    pub fn chart_trendlines(
        &mut self,
        sheet: u32,
        anchor: u32,
        series_idx: u32,
    ) -> Result<Vec<ChartTrendlineData>, JsValue> {
        map_error(self.inner.chart_trendlines(sheet, anchor, series_idx))
            .map(|values| values.into_iter().map(ChartTrendlineData).collect())
    }

    /// Every set of error bars series `seriesIdx` carries.
    #[wasm_bindgen(js_name = "chartErrorBars")]
    pub fn chart_error_bars(
        &mut self,
        sheet: u32,
        anchor: u32,
        series_idx: u32,
    ) -> Result<Vec<ChartErrorBarData>, JsValue> {
        map_error(self.inner.chart_error_bars(sheet, anchor, series_idx))
            .map(|values| values.into_iter().map(ChartErrorBarData).collect())
    }

    /// Every decoration of series `seriesIdx` naming a point the series no longer has.
    #[wasm_bindgen(js_name = "chartDanglingDecoration")]
    pub fn chart_dangling_decoration(
        &mut self,
        sheet: u32,
        anchor: u32,
        series_idx: u32,
    ) -> Result<Vec<DanglingPointReference>, JsValue> {
        map_error(
            self.inner
                .chart_dangling_decoration(sheet, anchor, series_idx),
        )
        .map(|values| values.into_iter().map(DanglingPointReference).collect())
    }

    /// Rewrites the values of series `seriesIdx`, refreshing the embedded workbook in the same call.
    #[wasm_bindgen(js_name = "setChartSeriesValues")]
    pub fn set_chart_series_values(
        &mut self,
        sheet: u32,
        anchor: u32,
        series_idx: u32,
        values: Vec<f64>,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .set_chart_series_values(sheet, anchor, series_idx, &values),
        )
    }

    /// Rewrites the category labels of series `seriesIdx`, refreshing the workbook alongside.
    #[wasm_bindgen(js_name = "setChartSeriesCategories")]
    pub fn set_chart_series_categories(
        &mut self,
        sheet: u32,
        anchor: u32,
        series_idx: u32,
        labels: Vec<String>,
    ) -> Result<(), JsValue> {
        let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
        map_error(
            self.inner
                .set_chart_series_categories(sheet, anchor, series_idx, &labels),
        )
    }

    /// Sets or clears the explicit bounds of axis `axisIdx`.
    #[wasm_bindgen(js_name = "setChartAxisScale")]
    pub fn set_chart_axis_scale(
        &mut self,
        sheet: u32,
        anchor: u32,
        axis_idx: u32,
        minimum: Option<f64>,
        maximum: Option<f64>,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .set_chart_axis_scale(sheet, anchor, axis_idx, minimum, maximum),
        )
    }

    /// Sets the direction of axis `axisIdx`.
    #[wasm_bindgen(js_name = "setChartAxisOrientation")]
    pub fn set_chart_axis_orientation(
        &mut self,
        sheet: u32,
        anchor: u32,
        axis_idx: u32,
        orientation: AxisOrientation,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_chart_axis_orientation(
            sheet,
            anchor,
            axis_idx,
            orientation.into(),
        ))
    }

    /// Sets or removes the title of axis `axisIdx`.
    #[wasm_bindgen(js_name = "setChartAxisTitle")]
    pub fn set_chart_axis_title(
        &mut self,
        sheet: u32,
        anchor: u32,
        axis_idx: u32,
        text: Option<String>,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .set_chart_axis_title(sheet, anchor, axis_idx, text.as_deref()),
        )
    }

    /// Turns the gridlines of axis `axisIdx` on or off.
    #[wasm_bindgen(js_name = "setChartAxisGridlines")]
    pub fn set_chart_axis_gridlines(
        &mut self,
        sheet: u32,
        anchor: u32,
        axis_idx: u32,
        major: bool,
        minor: bool,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .set_chart_axis_gridlines(sheet, anchor, axis_idx, major, minor),
        )
    }

    /// Sets or removes the chart's heading.
    #[wasm_bindgen(js_name = "setChartTitle")]
    pub fn set_chart_title(
        &mut self,
        sheet: u32,
        anchor: u32,
        text: Option<String>,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_chart_title(sheet, anchor, text.as_deref()))
    }

    /// Places the chart's legend at `position`, or removes it.
    #[wasm_bindgen(js_name = "setChartLegend")]
    pub fn set_chart_legend(
        &mut self,
        sheet: u32,
        anchor: u32,
        position: Option<LegendPosition>,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .set_chart_legend(sheet, anchor, position.map(Into::into)),
        )
    }

    /// Sets the fill of series `seriesIdx`.
    #[wasm_bindgen(js_name = "setChartSeriesFill")]
    pub fn set_chart_series_fill(
        &mut self,
        sheet: u32,
        anchor: u32,
        series_idx: u32,
        fill: &FillSpec,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .set_chart_series_fill(sheet, anchor, series_idx, &fill.0),
        )
    }

    /// Sets the outline of series `seriesIdx`.
    #[wasm_bindgen(js_name = "setChartSeriesLine")]
    pub fn set_chart_series_line(
        &mut self,
        sheet: u32,
        anchor: u32,
        series_idx: u32,
        line: &LineSpec,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .set_chart_series_line(sheet, anchor, series_idx, &line.0),
        )
    }

    /// Applies `spec` at one tier of the chart's data labels.
    #[wasm_bindgen(js_name = "setChartDataLabels")]
    pub fn set_chart_data_labels(
        &mut self,
        sheet: u32,
        anchor: u32,
        scope: &ChartLabelScope,
        spec: &DataLabelSpec,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .set_chart_data_labels(sheet, anchor, scope.0, &spec.0),
        )
    }

    /// Suppresses the labels at one tier.
    #[wasm_bindgen(js_name = "suppressChartDataLabels")]
    pub fn suppress_chart_data_labels(
        &mut self,
        sheet: u32,
        anchor: u32,
        scope: &ChartLabelScope,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .suppress_chart_data_labels(sheet, anchor, scope.0),
        )
    }

    /// Removes the labels at one tier entirely. Answers whether one was there.
    #[wasm_bindgen(js_name = "removeChartDataLabels")]
    pub fn remove_chart_data_labels(
        &mut self,
        sheet: u32,
        anchor: u32,
        scope: &ChartLabelScope,
    ) -> Result<bool, JsValue> {
        map_error(self.inner.remove_chart_data_labels(sheet, anchor, scope.0))
    }

    /// Colours point `pointIdx` of series `seriesIdx` differently from the rest of its series.
    #[wasm_bindgen(js_name = "setChartPointFill")]
    pub fn set_chart_point_fill(
        &mut self,
        sheet: u32,
        anchor: u32,
        series_idx: u32,
        point_idx: u32,
        fill: &FillSpec,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .set_chart_point_fill(sheet, anchor, series_idx, point_idx, &fill.0),
        )
    }

    /// Outlines point `pointIdx` of series `seriesIdx` differently from the rest of its series.
    #[wasm_bindgen(js_name = "setChartPointLine")]
    pub fn set_chart_point_line(
        &mut self,
        sheet: u32,
        anchor: u32,
        series_idx: u32,
        point_idx: u32,
        line: &LineSpec,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .set_chart_point_line(sheet, anchor, series_idx, point_idx, &line.0),
        )
    }

    /// Pulls slice `pointIdx` of series `seriesIdx` out of its pie or doughnut, or puts it back.
    #[wasm_bindgen(js_name = "setChartPointExplosion")]
    pub fn set_chart_point_explosion(
        &mut self,
        sheet: u32,
        anchor: u32,
        series_idx: u32,
        point_idx: u32,
        percent: Option<u32>,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .set_chart_point_explosion(sheet, anchor, series_idx, point_idx, percent),
        )
    }

    /// Removes the formatting of point `pointIdx` of series `seriesIdx`.
    #[wasm_bindgen(js_name = "removeChartPointFormat")]
    pub fn remove_chart_point_format(
        &mut self,
        sheet: u32,
        anchor: u32,
        series_idx: u32,
        point_idx: u32,
    ) -> Result<bool, JsValue> {
        map_error(
            self.inner
                .remove_chart_point_format(sheet, anchor, series_idx, point_idx),
        )
    }

    /// Fits a trendline through series `seriesIdx`, appending to any it already carries.
    #[wasm_bindgen(js_name = "addChartTrendline")]
    pub fn add_chart_trendline(
        &mut self,
        sheet: u32,
        anchor: u32,
        series_idx: u32,
        spec: &TrendlineSpec,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .add_chart_trendline(sheet, anchor, series_idx, &spec.0),
        )
    }

    /// Rewrites trendline `trendlineIdx` of series `seriesIdx` from `spec`, in place.
    #[wasm_bindgen(js_name = "setChartTrendline")]
    pub fn set_chart_trendline(
        &mut self,
        sheet: u32,
        anchor: u32,
        series_idx: u32,
        trendline_idx: u32,
        spec: &TrendlineSpec,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .set_chart_trendline(sheet, anchor, series_idx, trendline_idx, &spec.0),
        )
    }

    /// Removes every trendline from series `seriesIdx`, answering how many went.
    #[wasm_bindgen(js_name = "removeChartTrendlines")]
    pub fn remove_chart_trendlines(
        &mut self,
        sheet: u32,
        anchor: u32,
        series_idx: u32,
    ) -> Result<u32, JsValue> {
        map_error(
            self.inner
                .remove_chart_trendlines(sheet, anchor, series_idx),
        )
    }

    /// Gives series `seriesIdx` error bars, replacing an existing set along the same axis.
    #[wasm_bindgen(js_name = "setChartErrorBars")]
    pub fn set_chart_error_bars(
        &mut self,
        sheet: u32,
        anchor: u32,
        series_idx: u32,
        spec: &ErrorBarSpec,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .set_chart_error_bars(sheet, anchor, series_idx, &spec.0),
        )
    }

    /// Removes every set of error bars from series `seriesIdx`, answering how many went.
    #[wasm_bindgen(js_name = "removeChartErrorBars")]
    pub fn remove_chart_error_bars(
        &mut self,
        sheet: u32,
        anchor: u32,
        series_idx: u32,
    ) -> Result<u32, JsValue> {
        map_error(
            self.inner
                .remove_chart_error_bars(sheet, anchor, series_idx),
        )
    }

    /// Removes every decoration of series `seriesIdx` past the end of its data, answering how many
    /// went.
    #[wasm_bindgen(js_name = "dropChartDanglingDecoration")]
    pub fn drop_chart_dangling_decoration(
        &mut self,
        sheet: u32,
        anchor: u32,
        series_idx: u32,
    ) -> Result<u32, JsValue> {
        map_error(
            self.inner
                .drop_chart_dangling_decoration(sheet, anchor, series_idx),
        )
    }
}
