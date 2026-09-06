//! Excel-specific value classes: cell values, the block a range read answers with, the tab and
//! feature reports, and the four cell-format specs — everything
//! [`crate::workbook::Workbook`]'s methods take or hand back that is not already covered by
//! [`crate::enums`] (payload-free enumerations) or [`crate::errors`].
//!
//! # A cell is a value, not an object graph
//!
//! [`CellBlock`] is the one class here that a caller holds a lot of at once, and it is deliberately
//! **flat**: the values live in Rust as a packed row-major vector and cross into Python either one
//! at a time (`CellBlock.value`) or all at once as native `list[list[object]]`
//! (`CellBlock.rows`). Constructing a hundred thousand small Python objects is the thing the
//! Excel binding most has to avoid, and the range-shaped surface upstream exists so that the
//! *crossing* is one call — this class exists so the *conversion* can be one call too.
//!
//! `CellBlock.rows()` answers Python's own types — `None`, `float`, `str`, `bool` — because that is
//! what a caller iterating a table wants. It cannot distinguish a text cell from an error cell,
//! which both arrive as `str`; `CellBlock.kinds` is the disambiguator, built only when asked.

use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::IntoPyObjectExt;

use mjx_ooxml as ooxml;

use crate::enums::{
    ApplyFlag, BorderStyle, CalculationMode, FormatAspect, FormatLayer, GridAnomalyKind,
    HyperlinkKind, PartKind, ReferenceMode, SheetKind, SpreadsheetFontScheme,
    SpreadsheetPatternType, StyleIndexSource, TotalsRowFunction, UnderlineType,
};
use crate::errors::to_py_err;

value_class! {
    /// One cell's value, as the file states it — **stored, not displayed**.
    CellData(ooxml::CellData), derive(PartialEq);

    /// One entry of a `Workbook.write_cells` batch: where, and what.
    CellWrite(ooxml::CellWrite), derive(PartialEq);

    /// A rectangular block of cell values, read in one pass over one parse of the worksheet.
    CellBlock(ooxml::CellBlock), derive(PartialEq);

    /// One tab of a workbook, as the file states it.
    SheetSummary(ooxml::SheetSummary), derive(PartialEq, Eq);

    /// One `x:workbookView`: where the producer's window sat and what it showed.
    WorkbookWindowInfo(ooxml::WorkbookWindowInfo), derive(Copy, PartialEq, Eq);

    /// One `x:definedName`, with its scope resolved against the tab list.
    DefinedName(ooxml::DefinedName), derive(PartialEq, Eq);

    /// `x:calcPr` — what the producer's calculation engine was told. Reported, never acted on.
    CalculationSettings(ooxml::CalculationSettings), derive(Copy, PartialEq);

    /// One `x:hyperlink` on a sheet, resolved against the sheet's relationships.
    SheetHyperlinkInfo(ooxml::SheetHyperlinkInfo), derive(PartialEq, Eq);

    /// One table on a sheet, resolved to its part.
    SheetTableInfo(ooxml::SheetTableInfo), derive(PartialEq, Eq);

    /// One column of a `SheetTableInfo`.
    SheetTableColumnInfo(ooxml::SheetTableColumnInfo), derive(PartialEq, Eq);

    /// One thing a sheet's grid says that a well-formed one would not.
    GridAnomalyInfo(ooxml::GridAnomalyInfo), derive(PartialEq, Eq);

    /// Every part this project preserves rather than models, by cluster.
    PreservedPartsSummary(ooxml::PreservedPartsSummary), derive(PartialEq, Eq);

    /// One entry of `PreservedPartsSummary.all()`: a part, and which kind of part it is.
    PreservedPart(ooxml::PreservedPart), derive(PartialEq, Eq);

    /// One pivot table, resolved to its part and its cache.
    SheetPivotTableInfo(ooxml::SheetPivotTableInfo), derive(PartialEq, Eq);

    /// One external workbook reference, resolved to its part.
    WorkbookExternalLinkInfo(ooxml::WorkbookExternalLinkInfo), derive(PartialEq, Eq);

    /// One data connection the workbook declares.
    WorkbookConnectionInfo(ooxml::WorkbookConnectionInfo), derive(PartialEq, Eq);

    /// One query table, resolved to its part and the sheet it feeds.
    SheetQueryTableInfo(ooxml::SheetQueryTableInfo), derive(PartialEq, Eq);

    /// The workbook's XML maps — how a custom XML schema is mapped into cells.
    WorkbookXmlMapsInfo(ooxml::WorkbookXmlMapsInfo), derive(PartialEq, Eq);

    /// One `x:Map` of a `WorkbookXmlMapsInfo`.
    XmlMapInfo(ooxml::XmlMapInfo), derive(PartialEq, Eq);

    /// What the workbook says about shared-workbook change tracking.
    WorkbookRevisionState(ooxml::WorkbookRevisionState), derive(PartialEq, Eq);

    /// One recorded editing session of a shared workbook.
    RevisionSessionInfo(ooxml::RevisionSessionInfo), derive(PartialEq, Eq);

    /// One recorded user of a shared workbook.
    SharedWorkbookUserInfo(ooxml::SharedWorkbookUserInfo), derive(PartialEq, Eq);

    /// A SpreadsheetML colour: automatic, an indexed-palette row, an `AARRGGBB` value, or a theme
    /// slot with an optional tint.
    Color(ooxml::Color), derive(PartialEq);

    /// The fifteen font-property children of `CT_Font`/`CT_RPrElt`.
    FontProperties(ooxml::FontProperties), derive(PartialEq);

    /// A cell fill: a pattern and its two colours.
    PatternFillSpec(ooxml::PatternFillSpec), derive(PartialEq);

    /// One border edge: a style and an optional colour.
    BorderEdgeSpec(ooxml::BorderEdgeSpec), derive(PartialEq);

    /// A cell border: up to nine edges, plus the two diagonal flags.
    BorderSpec(ooxml::BorderSpec), derive(PartialEq);

    /// One `x:xf`: the four resource indices and the six `apply*` flags.
    CellFormatSpec(ooxml::CellFormatSpec), derive(PartialEq, Eq);

    /// What one cell's format resolves to, after the `cellXfs` -> `cellStyleXfs` ladder.
    EffectiveCellFormat(ooxml::EffectiveCellFormat), derive(Copy, PartialEq, Eq);

    /// One aspect of an `EffectiveCellFormat`: which layer supplied it, and which resource.
    ResolvedAspect(ooxml::ResolvedAspect), derive(Copy, PartialEq, Eq);
}

// ---------------------------------------------------------------------------------------------
// Cell values
// ---------------------------------------------------------------------------------------------

/// The kind name each `CellData` variant answers with, and what `CellBlock.kinds` fills its grid
/// with. Stable; a change here is a breaking change.
fn kind_of(value: &ooxml::CellData) -> &'static str {
    match value {
        ooxml::CellData::Blank => "blank",
        ooxml::CellData::Number(_) => "number",
        ooxml::CellData::Text(_) => "text",
        ooxml::CellData::Boolean(_) => "boolean",
        ooxml::CellData::Error(_) => "error",
    }
}

/// One cell as Python's own types: `None`, `float`, `str` or `bool`.
///
/// An error cell arrives as its code (`"#DIV/0!"`), which a text cell holding that same text would
/// too — `CellBlock.kinds` is how the two are told apart when it matters.
fn native(python: Python<'_>, value: &ooxml::CellData) -> PyResult<Py<PyAny>> {
    match value {
        ooxml::CellData::Blank => python.None().into_bound_py_any(python),
        ooxml::CellData::Number(number) => number.into_bound_py_any(python),
        ooxml::CellData::Text(text) | ooxml::CellData::Error(text) => {
            text.into_bound_py_any(python)
        }
        ooxml::CellData::Boolean(value) => value.into_bound_py_any(python),
    }
    .map(pyo3::Bound::unbind)
}

#[pymethods]
impl CellData {
    /// `"blank"`, `"number"`, `"text"`, `"boolean"` or `"error"`.
    #[getter]
    fn kind(&self) -> &'static str {
        kind_of(&self.0)
    }

    /// Whether the cell is not populated, or holds no value element.
    #[getter]
    fn is_blank(&self) -> bool {
        self.0.is_blank()
    }

    /// The number this cell holds, or `None` for every other kind.
    #[getter]
    fn number(&self) -> Option<f64> {
        self.0.number()
    }

    /// The string this cell holds, or `None` for every other kind. An error code is **not** a
    /// string here.
    #[getter]
    fn text(&self) -> Option<&str> {
        self.0.text()
    }

    /// The boolean this cell holds, or `None` for every other kind.
    #[getter]
    fn boolean(&self) -> Option<bool> {
        self.0.boolean()
    }

    /// The error code this cell holds, or `None` for every other kind.
    #[getter]
    fn error_code(&self) -> Option<&str> {
        self.0.error_code()
    }

    /// The value as one of Python's own types: `None`, `float`, `str` or `bool`.
    #[getter]
    fn value(&self, python: Python<'_>) -> PyResult<Py<PyAny>> {
        native(python, &self.0)
    }

    fn __repr__(&self) -> String {
        format!("CellData({:?})", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl CellWrite {
    /// Remove the value, keeping the cell — and therefore its style.
    #[staticmethod]
    fn blank(reference: &str) -> Self {
        Self(ooxml::CellWrite::new(reference, ooxml::CellInput::Blank))
    }

    /// A number. `nan` and the infinities are refused: SpreadsheetML has no spelling for them.
    #[staticmethod]
    fn number(reference: &str, value: f64) -> Self {
        Self(ooxml::CellWrite::new(
            reference,
            ooxml::CellInput::Number(value),
        ))
    }

    /// A string interned into `xl/sharedStrings.xml` — **what Excel itself writes**, and what to
    /// reach for when the same text appears in many cells.
    #[staticmethod]
    fn shared_text(reference: &str, text: &str) -> Self {
        Self(ooxml::CellWrite::new(
            reference,
            ooxml::CellInput::SharedText(text.to_owned()),
        ))
    }

    /// A string stored in the cell itself as an `inlineStr`. No other part is touched.
    #[staticmethod]
    fn inline_text(reference: &str, text: &str) -> Self {
        Self(ooxml::CellWrite::new(
            reference,
            ooxml::CellInput::InlineText(text.to_owned()),
        ))
    }

    /// A boolean, written `1` or `0`.
    #[staticmethod]
    fn boolean(reference: &str, value: bool) -> Self {
        Self(ooxml::CellWrite::new(
            reference,
            ooxml::CellInput::Boolean(value),
        ))
    }

    /// An error code — `"#DIV/0!"`, `"#N/A"`. Carried verbatim.
    #[staticmethod]
    fn error(reference: &str, code: &str) -> Self {
        Self(ooxml::CellWrite::new(
            reference,
            ooxml::CellInput::Error(code.to_owned()),
        ))
    }

    /// The cell this write addresses, in A1 text.
    #[getter]
    fn reference(&self) -> &str {
        &self.0.reference
    }

    /// `"blank"`, `"number"`, `"shared_text"`, `"inline_text"`, `"boolean"` or `"error"`.
    #[getter]
    fn kind(&self) -> &'static str {
        match &self.0.value {
            ooxml::CellInput::Blank => "blank",
            ooxml::CellInput::Number(_) => "number",
            ooxml::CellInput::SharedText(_) => "shared_text",
            ooxml::CellInput::InlineText(_) => "inline_text",
            ooxml::CellInput::Boolean(_) => "boolean",
            ooxml::CellInput::Error(_) => "error",
        }
    }

    fn __repr__(&self) -> String {
        format!("CellWrite.{}({:?})", self.kind(), self.0.reference)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl CellBlock {
    /// The zero-based sheet row the block's first row is: `0` is the row a file spells `1`.
    #[getter]
    fn first_row(&self) -> u32 {
        self.0.first_row()
    }

    /// The zero-based sheet column the block's first column is: `0` is `A`.
    #[getter]
    fn first_column(&self) -> u32 {
        self.0.first_column()
    }

    /// How many rows the block covers.
    #[getter]
    fn row_count(&self) -> u32 {
        self.0.row_count()
    }

    /// How many columns the block covers.
    #[getter]
    fn column_count(&self) -> u32 {
        self.0.column_count()
    }

    /// Whether the block covers no cells at all.
    #[getter]
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The A1 text of the range this block covers, or `None` when it covers nothing.
    #[getter]
    fn range(&self) -> Option<String> {
        self.0.range()
    }

    /// The value at `row`/`column`, **as offsets into the block**.
    fn value(&self, row: u32, column: u32) -> PyResult<CellData> {
        self.0
            .value(row, column)
            .map(|value| CellData(value.clone()))
            .map_err(to_py_err)
    }

    /// The formula text at `row`/`column`, or `None` when that cell carries none. Exactly as the
    /// file wrote it, never expanded and never evaluated.
    fn formula(&self, row: u32, column: u32) -> PyResult<Option<String>> {
        self.0
            .formula(row, column)
            .map(|text| text.map(str::to_owned))
            .map_err(to_py_err)
    }

    /// The whole block as rows of Python's own types — `None`, `float`, `str`, `bool` — top to
    /// bottom, left to right.
    ///
    /// **The shape to reach for.** One call converts the whole block; `value` per cell converts one
    /// at a time, which is cheap for a handful and needless for a table.
    fn rows<'py>(&self, python: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let block = self.0.clone();
        let outer = PyList::empty(python);
        for row in block.into_rows() {
            let inner = PyList::empty(python);
            for value in &row {
                inner.append(native(python, value)?)?;
            }
            outer.append(inner)?;
        }
        Ok(outer)
    }

    /// The whole block as rows of kind names — `"blank"`, `"number"`, `"text"`, `"boolean"`,
    /// `"error"`.
    ///
    /// The disambiguator for [`rows`](Self::rows), which cannot tell a text cell from an error cell
    /// because both arrive as `str`. Built only when asked.
    fn kinds<'py>(&self, python: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let block = self.0.clone();
        let outer = PyList::empty(python);
        for row in block.into_rows() {
            let inner = PyList::empty(python);
            for value in &row {
                inner.append(kind_of(value))?;
            }
            outer.append(inner)?;
        }
        Ok(outer)
    }

    fn __repr__(&self) -> String {
        format!(
            "CellBlock({} rows x {} columns at ({}, {}))",
            self.0.row_count(),
            self.0.column_count(),
            self.0.first_row(),
            self.0.first_column()
        )
    }
}

// ---------------------------------------------------------------------------------------------
// Tabs and workbook metadata
// ---------------------------------------------------------------------------------------------

#[pymethods]
impl SheetSummary {
    /// The tab's name (`@name`), with XML entities decoded.
    #[getter]
    fn name(&self) -> &str {
        &self.0.name
    }

    /// `@sheetId`, or `None` when the attribute is absent or is not an `xsd:unsignedInt`.
    #[getter]
    fn sheet_id(&self) -> Option<u32> {
        self.0.sheet_id
    }

    /// Whether the tab is shown in a consumer's tab strip.
    #[getter]
    fn is_visible(&self) -> bool {
        self.0.is_visible
    }

    /// Which of the three sheet kinds the tab's target is, or `None`.
    #[getter]
    fn kind(&self) -> PyResult<Option<SheetKind>> {
        self.0.kind.map(SheetKind::from_model).transpose()
    }

    /// The part the tab's `r:id` reaches, or `None`.
    #[getter]
    fn part(&self) -> Option<&str> {
        self.0.part.as_deref()
    }

    fn __repr__(&self) -> String {
        format!("SheetSummary({:?})", self.0.name)
    }
}

#[pymethods]
impl WorkbookWindowInfo {
    /// `@activeTab` — the index of the tab that was selected.
    #[getter]
    fn active_tab_index(&self) -> u32 {
        self.0.active_tab_index
    }

    /// `@firstSheet` — the index of the leftmost tab shown in the tab strip.
    #[getter]
    fn first_visible_tab_index(&self) -> u32 {
        self.0.first_visible_tab_index
    }

    /// `@xWindow`, or `None` if the file wrote neither coordinate.
    #[getter]
    fn window_left(&self) -> Option<i32> {
        self.0.window_left
    }

    /// `@yWindow`, on the same terms.
    #[getter]
    fn window_top(&self) -> Option<i32> {
        self.0.window_top
    }

    /// `@windowWidth`, or `None` if the file wrote neither dimension.
    #[getter]
    fn window_width(&self) -> Option<u32> {
        self.0.window_width
    }

    /// `@windowHeight`, on the same terms.
    #[getter]
    fn window_height(&self) -> Option<u32> {
        self.0.window_height
    }

    /// `@tabRatio`, in thousandths.
    #[getter]
    fn tab_strip_ratio(&self) -> u32 {
        self.0.tab_strip_ratio
    }

    /// `@showSheetTabs`.
    #[getter]
    fn show_sheet_tabs(&self) -> bool {
        self.0.show_sheet_tabs
    }
}

#[pymethods]
impl DefinedName {
    /// `@name`, as a consumer's name manager shows it.
    #[getter]
    fn name(&self) -> &str {
        &self.0.name
    }

    /// The index of the tab this name is local to, or `None` for a workbook-scoped name.
    #[getter]
    fn sheet(&self) -> Option<u32> {
        self.0.sheet
    }

    /// The name of the tab `sheet` indexes, when there is one there.
    #[getter]
    fn sheet_name(&self) -> Option<&str> {
        self.0.sheet_name.as_deref()
    }

    /// The formula this name stands for, **as text**. Nothing here parses or evaluates it.
    #[getter]
    fn definition(&self) -> &str {
        &self.0.definition
    }

    /// `@hidden`.
    #[getter]
    fn hidden(&self) -> bool {
        self.0.hidden
    }

    fn __repr__(&self) -> String {
        format!("DefinedName({:?})", self.0.name)
    }
}

#[pymethods]
impl CalculationSettings {
    /// `@calcId`, or `None`. Never derived and never bumped.
    #[getter]
    fn engine_id(&self) -> Option<u32> {
        self.0.engine_id
    }

    /// `@calcMode`.
    #[getter]
    fn mode(&self) -> PyResult<CalculationMode> {
        CalculationMode::from_model(self.0.mode)
    }

    /// `@refMode` — A1 or R1C1.
    #[getter]
    fn reference_mode(&self) -> PyResult<ReferenceMode> {
        ReferenceMode::from_model(self.0.reference_mode)
    }

    /// `@iterate`.
    #[getter]
    fn iterates_on_circular_references(&self) -> bool {
        self.0.iterates_on_circular_references
    }

    /// `@iterateCount`.
    #[getter]
    fn iteration_limit(&self) -> u32 {
        self.0.iteration_limit
    }

    /// `@iterateDelta`.
    #[getter]
    fn iteration_convergence_delta(&self) -> f64 {
        self.0.iteration_convergence_delta
    }

    /// `@fullCalcOnLoad`.
    #[getter]
    fn full_calculation_on_load(&self) -> bool {
        self.0.full_calculation_on_load
    }
}

// ---------------------------------------------------------------------------------------------
// Hyperlinks, tables and grid anomalies
// ---------------------------------------------------------------------------------------------

#[pymethods]
impl SheetHyperlinkInfo {
    /// `@ref` — the range the link covers, as A1 text.
    #[getter]
    fn range(&self) -> &str {
        &self.0.range
    }

    /// Which of `CT_Hyperlink`'s four shapes this entry is.
    #[getter]
    fn kind(&self) -> PyResult<HyperlinkKind> {
        HyperlinkKind::from_model(self.0.kind)
    }

    /// `@r:id`, or `None` when the entry names no relationship.
    #[getter]
    fn relationship_id(&self) -> Option<&str> {
        self.0.relationship_id.as_deref()
    }

    /// The relationship's `Target`, exactly as the `.rels` wrote it.
    #[getter]
    fn target(&self) -> Option<&str> {
        self.0.target.as_deref()
    }

    /// Whether that relationship's `TargetMode` is `External`.
    #[getter]
    fn target_is_external(&self) -> Option<bool> {
        self.0.target_is_external
    }

    /// `@location` — a cell reference or a defined name inside this workbook.
    #[getter]
    fn location(&self) -> Option<&str> {
        self.0.location.as_deref()
    }

    /// `@tooltip`.
    #[getter]
    fn tooltip(&self) -> Option<&str> {
        self.0.tooltip.as_deref()
    }

    /// `@display` — never kept in step with the cell's own value.
    #[getter]
    fn display(&self) -> Option<&str> {
        self.0.display.as_deref()
    }

    fn __repr__(&self) -> String {
        format!("SheetHyperlinkInfo({:?})", self.0.range)
    }
}

#[pymethods]
impl SheetTableInfo {
    /// The part the table lives in.
    #[getter]
    fn part(&self) -> &str {
        &self.0.part
    }

    /// The `tablePart@r:id` the sheet reached it through.
    #[getter]
    fn relationship_id(&self) -> &str {
        &self.0.relationship_id
    }

    /// `@id` — workbook-unique, never renumbered here.
    #[getter]
    fn id(&self) -> u32 {
        self.0.id
    }

    /// `@displayName` — what a formula references the table by.
    #[getter]
    fn display_name(&self) -> &str {
        &self.0.display_name
    }

    /// `@name`, or `None` when the table writes none.
    #[getter]
    fn name(&self) -> Option<&str> {
        self.0.name.as_deref()
    }

    /// `@ref` as A1 text, header and totals rows included.
    #[getter]
    fn range(&self) -> &str {
        &self.0.range
    }

    /// `@headerRowCount`.
    #[getter]
    fn header_row_count(&self) -> u32 {
        self.0.header_row_count
    }

    /// `@totalsRowCount`.
    #[getter]
    fn totals_row_count(&self) -> u32 {
        self.0.totals_row_count
    }

    /// How many rows of `range` are data rows, or `None` when the counts do not fit inside it.
    #[getter]
    fn data_row_count(&self) -> Option<u32> {
        self.0.data_row_count
    }

    /// `tableStyleInfo@name`, or `None`.
    #[getter]
    fn style_name(&self) -> Option<&str> {
        self.0.style_name.as_deref()
    }

    /// The columns, left to right.
    #[getter]
    fn columns(&self) -> Vec<SheetTableColumnInfo> {
        self.0
            .columns
            .iter()
            .cloned()
            .map(SheetTableColumnInfo)
            .collect()
    }

    fn __repr__(&self) -> String {
        format!("SheetTableInfo({:?})", self.0.display_name)
    }
}

#[pymethods]
impl SheetTableColumnInfo {
    /// `@id`, unique within the table.
    #[getter]
    fn id(&self) -> u32 {
        self.0.id
    }

    /// `@name` — the heading text.
    #[getter]
    fn name(&self) -> &str {
        &self.0.name
    }

    /// `@totalsRowFunction`, or `None`.
    #[getter]
    fn totals_row_function(&self) -> PyResult<Option<TotalsRowFunction>> {
        self.0
            .totals_row_function
            .map(TotalsRowFunction::from_model)
            .transpose()
    }

    /// `@totalsRowLabel`.
    #[getter]
    fn totals_row_label(&self) -> Option<&str> {
        self.0.totals_row_label.as_deref()
    }

    /// `x:calculatedColumnFormula`, exactly as the file wrote it.
    #[getter]
    fn calculated_column_formula(&self) -> Option<&str> {
        self.0.calculated_column_formula.as_deref()
    }

    /// `x:totalsRowFormula`, on the same terms.
    #[getter]
    fn totals_row_formula(&self) -> Option<&str> {
        self.0.totals_row_formula.as_deref()
    }

    fn __repr__(&self) -> String {
        format!("SheetTableColumnInfo({:?})", self.0.name)
    }
}

#[pymethods]
impl GridAnomalyInfo {
    /// Which finding this is.
    #[getter]
    fn kind(&self) -> PyResult<GridAnomalyKind> {
        GridAnomalyKind::from_model(self.0.kind)
    }

    /// The range at fault, as A1 text.
    #[getter]
    fn range(&self) -> Option<&str> {
        self.0.range.as_deref()
    }

    /// The second range of an overlapping pair.
    #[getter]
    fn other_range(&self) -> Option<&str> {
        self.0.other_range.as_deref()
    }

    /// The cell at fault, as A1 text.
    #[getter]
    fn cell(&self) -> Option<&str> {
        self.0.cell.as_deref()
    }

    /// The first column of a `col` run at fault, zero-based.
    #[getter]
    fn first_column(&self) -> Option<u32> {
        self.0.first_column
    }

    /// The last column of that run, zero-based.
    #[getter]
    fn last_column(&self) -> Option<u32> {
        self.0.last_column
    }

    /// The second run's first column, for an overlap.
    #[getter]
    fn other_first_column(&self) -> Option<u32> {
        self.0.other_first_column
    }

    /// The second run's last column, for an overlap.
    #[getter]
    fn other_last_column(&self) -> Option<u32> {
        self.0.other_last_column
    }

    /// What the file declared — a merge count, or an outline maximum.
    #[getter]
    fn declared(&self) -> Option<u32> {
        self.0.declared
    }

    /// What is actually there.
    #[getter]
    fn actual(&self) -> Option<u32> {
        self.0.actual
    }

    /// The index of the `mergeCell` whose `@ref` could not be read.
    #[getter]
    fn index(&self) -> Option<u32> {
        self.0.index
    }

    fn __repr__(&self) -> String {
        format!("GridAnomalyInfo({:?})", self.0.kind)
    }
}

// ---------------------------------------------------------------------------------------------
// The preserved clusters
// ---------------------------------------------------------------------------------------------

#[pymethods]
impl PreservedPartsSummary {
    /// `x:pivotTableDefinition` parts.
    #[getter]
    fn pivot_tables(&self) -> Vec<String> {
        self.0.pivot_tables.clone()
    }

    /// `x:pivotCacheDefinition` parts.
    #[getter]
    fn pivot_cache_definitions(&self) -> Vec<String> {
        self.0.pivot_cache_definitions.clone()
    }

    /// `x:pivotCacheRecords` parts.
    #[getter]
    fn pivot_cache_records(&self) -> Vec<String> {
        self.0.pivot_cache_records.clone()
    }

    /// `x:externalLink` parts.
    #[getter]
    fn external_links(&self) -> Vec<String> {
        self.0.external_links.clone()
    }

    /// The `x:connections` part, if there is one.
    #[getter]
    fn connections(&self) -> Option<&str> {
        self.0.connections.as_deref()
    }

    /// `x:queryTable` parts.
    #[getter]
    fn query_tables(&self) -> Vec<String> {
        self.0.query_tables.clone()
    }

    /// The `x:metadata` part, if there is one.
    #[getter]
    fn metadata(&self) -> Option<&str> {
        self.0.metadata.as_deref()
    }

    /// The `x:volTypes` part, if there is one.
    #[getter]
    fn volatile_dependencies(&self) -> Option<&str> {
        self.0.volatile_dependencies.as_deref()
    }

    /// The `x:MapInfo` part, if there is one.
    #[getter]
    fn custom_xml_mappings(&self) -> Option<&str> {
        self.0.custom_xml_mappings.as_deref()
    }

    /// `x:singleXmlCell` table-definition parts.
    #[getter]
    fn single_cell_table_definitions(&self) -> Vec<String> {
        self.0.single_cell_table_definitions.clone()
    }

    /// The `x:headers` revision-headers part, if there is one.
    #[getter]
    fn revision_headers(&self) -> Option<&str> {
        self.0.revision_headers.as_deref()
    }

    /// `x:revisions` revision-log parts.
    #[getter]
    fn revision_logs(&self) -> Vec<String> {
        self.0.revision_logs.clone()
    }

    /// The shared-workbook user-data part, if there is one.
    #[getter]
    fn shared_workbook_user_data(&self) -> Option<&str> {
        self.0.shared_workbook_user_data.as_deref()
    }

    /// Custom Property parts.
    #[getter]
    fn custom_properties(&self) -> Vec<String> {
        self.0.custom_properties.clone()
    }

    /// Every preserved part with the kind it is.
    fn all(&self) -> Vec<PreservedPart> {
        self.0.all().into_iter().map(PreservedPart).collect()
    }

    /// Whether the workbook carries none of these at all.
    #[getter]
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[pymethods]
impl PreservedPart {
    /// Which SpreadsheetML part this is.
    #[getter]
    fn kind(&self) -> PyResult<PartKind> {
        PartKind::from_model(self.0.kind)
    }

    /// Its part name.
    #[getter]
    fn part(&self) -> &str {
        &self.0.part
    }

    fn __repr__(&self) -> String {
        format!("PreservedPart({:?}, {:?})", self.0.kind, self.0.part)
    }
}

#[pymethods]
impl SheetPivotTableInfo {
    /// The `x:pivotTableDefinition` part.
    #[getter]
    fn part(&self) -> &str {
        &self.0.part
    }

    /// The index of the tab the table sits on.
    #[getter]
    fn sheet(&self) -> u32 {
        self.0.sheet
    }

    /// That tab's name.
    #[getter]
    fn sheet_name(&self) -> &str {
        &self.0.sheet_name
    }

    /// That tab's own part.
    #[getter]
    fn sheet_part(&self) -> &str {
        &self.0.sheet_part
    }

    /// The `r:id` the sheet reached the table through.
    #[getter]
    fn relationship_id(&self) -> &str {
        &self.0.relationship_id
    }

    /// `@name`.
    #[getter]
    fn name(&self) -> &str {
        &self.0.name
    }

    /// `@cacheId`.
    #[getter]
    fn cache_id(&self) -> u32 {
        self.0.cache_id
    }

    /// `@dataCaption`.
    #[getter]
    fn data_caption(&self) -> &str {
        &self.0.data_caption
    }

    /// `x:location/@ref`, exactly as the file wrote it.
    #[getter]
    fn location(&self) -> &str {
        &self.0.location
    }

    /// The cache-definition part `@cacheId` resolves to.
    #[getter]
    fn cache_definition_part(&self) -> Option<&str> {
        self.0.cache_definition_part.as_deref()
    }

    /// The cache-records part that definition names.
    #[getter]
    fn cache_records_part(&self) -> Option<&str> {
        self.0.cache_records_part.as_deref()
    }

    /// `pivotCacheDefinition@recordCount` — the producer's cached number.
    #[getter]
    fn cache_record_count(&self) -> Option<u32> {
        self.0.cache_record_count
    }

    /// `pivotCacheDefinition@refreshedBy`.
    #[getter]
    fn cache_refreshed_by(&self) -> Option<&str> {
        self.0.cache_refreshed_by.as_deref()
    }

    fn __repr__(&self) -> String {
        format!("SheetPivotTableInfo({:?})", self.0.name)
    }
}

#[pymethods]
impl WorkbookExternalLinkInfo {
    /// The `x:externalLink` part.
    #[getter]
    fn part(&self) -> &str {
        &self.0.part
    }

    /// The `r:id` `xl/workbook.xml` reached it through.
    #[getter]
    fn relationship_id(&self) -> Option<&str> {
        self.0.relationship_id.as_deref()
    }

    /// The position in `x:externalReferences` this link is.
    #[getter]
    fn reference_index(&self) -> Option<u32> {
        self.0.reference_index
    }

    /// The relationship target, carried verbatim and never resolved or fetched.
    #[getter]
    fn target(&self) -> Option<&str> {
        self.0.target.as_deref()
    }

    /// `"workbook"`, `"dde"`, `"oleObject"` or `"none"`.
    #[getter]
    fn kind(&self) -> &str {
        self.0.kind
    }

    /// The names of the linked workbook's sheets, as this file cached them.
    #[getter]
    fn sheet_names(&self) -> Vec<String> {
        self.0.sheet_names.clone()
    }
}

#[pymethods]
impl WorkbookConnectionInfo {
    /// The `x:connections` part.
    #[getter]
    fn part(&self) -> &str {
        &self.0.part
    }

    /// `@id` — what a query table's `@connectionId` names.
    #[getter]
    fn id(&self) -> u32 {
        self.0.id
    }

    /// `@name`.
    #[getter]
    fn name(&self) -> Option<&str> {
        self.0.name.as_deref()
    }

    /// `@description`.
    #[getter]
    fn description(&self) -> Option<&str> {
        self.0.description.as_deref()
    }

    /// `@sourceFile` — carried verbatim, never resolved or opened.
    #[getter]
    fn source_file(&self) -> Option<&str> {
        self.0.source_file.as_deref()
    }

    /// `@odcFile`, on the same terms.
    #[getter]
    fn odc_file(&self) -> Option<&str> {
        self.0.odc_file.as_deref()
    }
}

#[pymethods]
impl SheetQueryTableInfo {
    /// The `x:queryTable` part.
    #[getter]
    fn part(&self) -> &str {
        &self.0.part
    }

    /// The index of the tab the query table sits on.
    #[getter]
    fn sheet(&self) -> u32 {
        self.0.sheet
    }

    /// That tab's name.
    #[getter]
    fn sheet_name(&self) -> &str {
        &self.0.sheet_name
    }

    /// That tab's own part.
    #[getter]
    fn sheet_part(&self) -> &str {
        &self.0.sheet_part
    }

    /// The `r:id` the sheet reached it through.
    #[getter]
    fn relationship_id(&self) -> &str {
        &self.0.relationship_id
    }

    /// `@name`.
    #[getter]
    fn name(&self) -> &str {
        &self.0.name
    }

    /// `@connectionId`.
    #[getter]
    fn connection_id(&self) -> u32 {
        self.0.connection_id
    }

    /// `@refreshOnLoad`.
    #[getter]
    fn refresh_on_load(&self) -> bool {
        self.0.refresh_on_load
    }
}

#[pymethods]
impl WorkbookXmlMapsInfo {
    /// The `x:MapInfo` part.
    #[getter]
    fn part(&self) -> &str {
        &self.0.part
    }

    /// `@SelectionNamespaces`.
    #[getter]
    fn selection_namespaces(&self) -> &str {
        &self.0.selection_namespaces
    }

    /// The schema ids declared in the part.
    #[getter]
    fn schema_ids(&self) -> Vec<String> {
        self.0.schema_ids.clone()
    }

    /// The maps themselves.
    #[getter]
    fn maps(&self) -> Vec<XmlMapInfo> {
        self.0.maps.iter().cloned().map(XmlMapInfo).collect()
    }
}

#[pymethods]
impl XmlMapInfo {
    /// `@ID`.
    #[getter]
    fn id(&self) -> u32 {
        self.0.id
    }

    /// `@Name`.
    #[getter]
    fn name(&self) -> &str {
        &self.0.name
    }

    /// `@RootElement`.
    #[getter]
    fn root_element(&self) -> &str {
        &self.0.root_element
    }

    /// `@SchemaID`.
    #[getter]
    fn schema_id(&self) -> &str {
        &self.0.schema_id
    }
}

#[pymethods]
impl WorkbookRevisionState {
    /// Whether `xl/workbook.xml` declares the workbook shared.
    #[getter]
    fn is_shared(&self) -> bool {
        self.0.is_shared
    }

    /// The `x:headers` part, if there is one.
    #[getter]
    fn headers_part(&self) -> Option<&str> {
        self.0.headers_part.as_deref()
    }

    /// `headers@guid`.
    #[getter]
    fn guid(&self) -> Option<&str> {
        self.0.guid.as_deref()
    }

    /// The revision-log parts, in the order the headers name them.
    #[getter]
    fn log_parts(&self) -> Vec<String> {
        self.0.log_parts.clone()
    }

    /// The shared-workbook user-data part, if there is one.
    #[getter]
    fn user_data_part(&self) -> Option<&str> {
        self.0.user_data_part.as_deref()
    }

    /// The recorded editing sessions.
    #[getter]
    fn sessions(&self) -> Vec<RevisionSessionInfo> {
        self.0
            .sessions
            .iter()
            .cloned()
            .map(RevisionSessionInfo)
            .collect()
    }

    /// The recorded users.
    #[getter]
    fn users(&self) -> Vec<SharedWorkbookUserInfo> {
        self.0
            .users
            .iter()
            .cloned()
            .map(SharedWorkbookUserInfo)
            .collect()
    }
}

#[pymethods]
impl RevisionSessionInfo {
    /// `@guid`.
    #[getter]
    fn guid(&self) -> &str {
        &self.0.guid
    }

    /// `@dateTime`, exactly as the file wrote it.
    #[getter]
    fn date_time(&self) -> &str {
        &self.0.date_time
    }

    /// `@userName`.
    #[getter]
    fn user_name(&self) -> &str {
        &self.0.user_name
    }
}

#[pymethods]
impl SharedWorkbookUserInfo {
    /// `@id`.
    #[getter]
    fn id(&self) -> i32 {
        self.0.id
    }

    /// `@name`.
    #[getter]
    fn name(&self) -> &str {
        &self.0.name
    }

    /// `@dateTime`, exactly as the file wrote it.
    #[getter]
    fn date_time(&self) -> &str {
        &self.0.date_time
    }
}

// ---------------------------------------------------------------------------------------------
// The cell-format authoring vocabulary
// ---------------------------------------------------------------------------------------------

#[pymethods]
impl Color {
    /// An opaque sRGB colour, written `rgb="FFRRGGBB"`. `hex` is the six-digit `RRGGBB` form; the
    /// opaque alpha is prefixed for you, because a six-digit `@rgb` is read as transparent.
    #[staticmethod]
    fn from_opaque_rgb(hex: &str) -> Self {
        Self(ooxml::Color::from_opaque_rgb(hex))
    }

    /// A theme colour by index, optionally tinted towards white (positive) or black (negative).
    #[staticmethod]
    #[pyo3(signature = (index, tint = None))]
    fn from_theme(index: u32, tint: Option<f64>) -> Self {
        Self(ooxml::Color::from_theme(index, tint))
    }

    /// The system foreground/background colour, whatever that is at render time.
    #[staticmethod]
    fn automatic() -> Self {
        Self(ooxml::Color {
            automatic: Some(true),
            ..ooxml::Color::default()
        })
    }

    /// A row of the legacy 56-entry indexed palette.
    #[staticmethod]
    fn indexed(index: u32) -> Self {
        Self(ooxml::Color {
            indexed: Some(index),
            ..ooxml::Color::default()
        })
    }

    /// `@auto`.
    #[getter]
    fn is_automatic(&self) -> Option<bool> {
        self.0.automatic
    }

    /// `@indexed`.
    #[getter]
    fn indexed_value(&self) -> Option<u32> {
        self.0.indexed
    }

    /// `@rgb` — eight hex digits, **alpha first**, as the file's own text.
    #[getter]
    fn rgb(&self) -> Option<&str> {
        self.0.rgb.as_deref()
    }

    /// `@theme` — a zero-based index into the theme's colour scheme.
    #[getter]
    fn theme(&self) -> Option<u32> {
        self.0.theme
    }

    /// `@tint`, in `-1.0 ..= 1.0`.
    #[getter]
    fn tint(&self) -> Option<f64> {
        self.0.tint
    }

    fn __repr__(&self) -> String {
        format!("Color({:?})", self.0)
    }
}

#[pymethods]
impl FontProperties {
    /// A font, stated one property at a time. Every argument is optional, and an argument left out
    /// writes **no element at all** — which is a third state beside present-and-false.
    #[new]
    #[pyo3(signature = (
        font_name = None,
        bold = None,
        italic = None,
        strikethrough = None,
        underline = None,
        size_in_points = None,
        color = None,
        scheme = None,
        family = None,
        character_set = None,
        outline = None,
        shadow = None,
        condensed = None,
        extended = None,
    ))]
    #[expect(
        clippy::too_many_arguments,
        reason = "one keyword argument per CT_Font child, which is the shape a Python caller \
                  writes and what makes every property nameable at the call site"
    )]
    fn new(
        font_name: Option<String>,
        bold: Option<bool>,
        italic: Option<bool>,
        strikethrough: Option<bool>,
        underline: Option<UnderlineType>,
        size_in_points: Option<f64>,
        color: Option<Color>,
        scheme: Option<SpreadsheetFontScheme>,
        family: Option<i64>,
        character_set: Option<i64>,
        outline: Option<bool>,
        shadow: Option<bool>,
        condensed: Option<bool>,
        extended: Option<bool>,
    ) -> Self {
        Self(ooxml::FontProperties {
            font_name,
            character_set,
            family,
            bold,
            italic,
            strikethrough,
            outline,
            shadow,
            condensed,
            extended,
            color: color.map(|color| color.0),
            size_in_points,
            underline: underline.map(Into::into),
            vertical_position: None,
            scheme: scheme.map(Into::into),
            extra: Vec::new(),
        })
    }

    /// `rFont`/`name` — the typeface name.
    #[getter]
    fn font_name(&self) -> Option<&str> {
        self.0.font_name.as_deref()
    }

    /// `b`.
    #[getter]
    fn bold(&self) -> Option<bool> {
        self.0.bold
    }

    /// `i`.
    #[getter]
    fn italic(&self) -> Option<bool> {
        self.0.italic
    }

    /// `strike`.
    #[getter]
    fn strikethrough(&self) -> Option<bool> {
        self.0.strikethrough
    }

    /// `sz` — the point size.
    #[getter]
    fn size_in_points(&self) -> Option<f64> {
        self.0.size_in_points
    }

    /// `color`.
    #[getter]
    fn color(&self) -> Option<Color> {
        self.0.color.clone().map(Color)
    }

    /// `u`.
    #[getter]
    fn underline(&self) -> PyResult<Option<UnderlineType>> {
        self.0.underline.map(UnderlineType::from_model).transpose()
    }

    /// `scheme` — whether this is the theme's major or minor font rather than a named one.
    #[getter]
    fn scheme(&self) -> PyResult<Option<SpreadsheetFontScheme>> {
        self.0
            .scheme
            .map(SpreadsheetFontScheme::from_model)
            .transpose()
    }
}

#[pymethods]
impl PatternFillSpec {
    /// A pattern fill. `foreground` is the colour a **solid** fill actually shows.
    #[new]
    #[pyo3(signature = (pattern = None, foreground = None, background = None))]
    fn new(
        pattern: Option<SpreadsheetPatternType>,
        foreground: Option<Color>,
        background: Option<Color>,
    ) -> Self {
        Self(ooxml::PatternFillSpec {
            pattern: pattern.map(Into::into),
            foreground: foreground.map(|color| color.0),
            background: background.map(|color| color.0),
        })
    }

    /// A solid fill in one opaque colour — the shape a caller filling a cell almost always wants.
    #[staticmethod]
    fn solid(hex: &str) -> Self {
        Self(ooxml::PatternFillSpec::solid(hex))
    }

    /// `@patternType`.
    #[getter]
    fn pattern(&self) -> PyResult<Option<SpreadsheetPatternType>> {
        self.0
            .pattern
            .map(SpreadsheetPatternType::from_model)
            .transpose()
    }

    /// `fgColor`.
    #[getter]
    fn foreground(&self) -> Option<Color> {
        self.0.foreground.clone().map(Color)
    }

    /// `bgColor`.
    #[getter]
    fn background(&self) -> Option<Color> {
        self.0.background.clone().map(Color)
    }
}

#[pymethods]
impl BorderEdgeSpec {
    /// One edge: a style, and optionally a colour. No colour means *automatic*.
    #[new]
    #[pyo3(signature = (style = None, color = None))]
    fn new(style: Option<BorderStyle>, color: Option<Color>) -> Self {
        Self(ooxml::BorderEdgeSpec {
            style: style.map(Into::into),
            color: color.map(|color| color.0),
        })
    }

    /// `@style`.
    #[getter]
    fn style(&self) -> PyResult<Option<BorderStyle>> {
        self.0.style.map(BorderStyle::from_model).transpose()
    }

    /// `color`.
    #[getter]
    fn color(&self) -> Option<Color> {
        self.0.color.clone().map(Color)
    }
}

#[pymethods]
impl BorderSpec {
    /// A border, stated one edge at a time. `left`/`right` are the physical edges;
    /// `leading`/`trailing` are the reading-direction ones (`start`/`end`).
    #[new]
    #[pyo3(signature = (
        left = None,
        right = None,
        top = None,
        bottom = None,
        diagonal = None,
        leading = None,
        trailing = None,
        vertical_inner = None,
        horizontal_inner = None,
        diagonal_up = None,
        diagonal_down = None,
    ))]
    #[expect(
        clippy::too_many_arguments,
        reason = "CT_Border declares nine independent edges and two diagonal flags; one keyword \
                  argument each is what makes them nameable at the call site"
    )]
    fn new(
        left: Option<BorderEdgeSpec>,
        right: Option<BorderEdgeSpec>,
        top: Option<BorderEdgeSpec>,
        bottom: Option<BorderEdgeSpec>,
        diagonal: Option<BorderEdgeSpec>,
        leading: Option<BorderEdgeSpec>,
        trailing: Option<BorderEdgeSpec>,
        vertical_inner: Option<BorderEdgeSpec>,
        horizontal_inner: Option<BorderEdgeSpec>,
        diagonal_up: Option<bool>,
        diagonal_down: Option<bool>,
    ) -> Self {
        let edge = |value: Option<BorderEdgeSpec>| value.map(|edge| edge.0);
        Self(ooxml::BorderSpec {
            leading: edge(leading),
            trailing: edge(trailing),
            left: edge(left),
            right: edge(right),
            top: edge(top),
            bottom: edge(bottom),
            diagonal: edge(diagonal),
            vertical_inner: edge(vertical_inner),
            horizontal_inner: edge(horizontal_inner),
            diagonal_up,
            diagonal_down,
            ..ooxml::BorderSpec::default()
        })
    }

    /// `left`.
    #[getter]
    fn left(&self) -> Option<BorderEdgeSpec> {
        self.0.left.clone().map(BorderEdgeSpec)
    }

    /// `right`.
    #[getter]
    fn right(&self) -> Option<BorderEdgeSpec> {
        self.0.right.clone().map(BorderEdgeSpec)
    }

    /// `top`.
    #[getter]
    fn top(&self) -> Option<BorderEdgeSpec> {
        self.0.top.clone().map(BorderEdgeSpec)
    }

    /// `bottom`.
    #[getter]
    fn bottom(&self) -> Option<BorderEdgeSpec> {
        self.0.bottom.clone().map(BorderEdgeSpec)
    }

    /// `diagonal`.
    #[getter]
    fn diagonal(&self) -> Option<BorderEdgeSpec> {
        self.0.diagonal.clone().map(BorderEdgeSpec)
    }
}

#[pymethods]
impl CellFormatSpec {
    /// An `x:xf`: the four resource indices, the `cellStyleXfs` record beneath it, and the six
    /// `apply*` flags.
    #[new]
    #[pyo3(signature = (
        number_format_id = None,
        font_index = None,
        fill_index = None,
        border_index = None,
        cell_style_format_index = None,
        applies_number_format = None,
        applies_font = None,
        applies_fill = None,
        applies_border = None,
        applies_alignment = None,
        applies_protection = None,
        text_is_quote_prefixed = None,
    ))]
    #[expect(
        clippy::too_many_arguments,
        reason = "one keyword argument per CT_Xf attribute, which is what makes each nameable"
    )]
    fn new(
        number_format_id: Option<u32>,
        font_index: Option<u32>,
        fill_index: Option<u32>,
        border_index: Option<u32>,
        cell_style_format_index: Option<u32>,
        applies_number_format: Option<bool>,
        applies_font: Option<bool>,
        applies_fill: Option<bool>,
        applies_border: Option<bool>,
        applies_alignment: Option<bool>,
        applies_protection: Option<bool>,
        text_is_quote_prefixed: Option<bool>,
    ) -> Self {
        Self(ooxml::CellFormatSpec {
            number_format_id,
            font_index,
            fill_index,
            border_index,
            cell_style_format_index,
            text_is_quote_prefixed,
            applies_number_format,
            applies_font,
            applies_fill,
            applies_border,
            applies_alignment,
            applies_protection,
        })
    }

    /// The record every workbook's `cellXfs[0]` is: `numFmtId="0" fontId="0" fillId="0"
    /// borderId="0" xfId="0"`. The base to build a real format on.
    #[staticmethod]
    fn skeleton_cell_format() -> Self {
        Self(ooxml::CellFormatSpec::skeleton_cell_format())
    }

    /// The same record without `@xfId` — a `cellStyleXfs` entry, which has nothing beneath it.
    #[staticmethod]
    fn skeleton_cell_style_format() -> Self {
        Self(ooxml::CellFormatSpec::skeleton_cell_style_format())
    }

    /// A copy of this record with the resource indices replaced. Values left `None` are kept.
    #[pyo3(signature = (font_index = None, fill_index = None, border_index = None, number_format_id = None))]
    fn with_resources(
        &self,
        font_index: Option<u32>,
        fill_index: Option<u32>,
        border_index: Option<u32>,
        number_format_id: Option<u32>,
    ) -> Self {
        Self(ooxml::CellFormatSpec {
            font_index: font_index.or(self.0.font_index),
            fill_index: fill_index.or(self.0.fill_index),
            border_index: border_index.or(self.0.border_index),
            number_format_id: number_format_id.or(self.0.number_format_id),
            applies_font: font_index.map(|_| true).or(self.0.applies_font),
            applies_fill: fill_index.map(|_| true).or(self.0.applies_fill),
            applies_border: border_index.map(|_| true).or(self.0.applies_border),
            applies_number_format: number_format_id
                .map(|_| true)
                .or(self.0.applies_number_format),
            ..self.0
        })
    }

    /// `@numFmtId`.
    #[getter]
    fn number_format_id(&self) -> Option<u32> {
        self.0.number_format_id
    }

    /// `@fontId`.
    #[getter]
    fn font_index(&self) -> Option<u32> {
        self.0.font_index
    }

    /// `@fillId`.
    #[getter]
    fn fill_index(&self) -> Option<u32> {
        self.0.fill_index
    }

    /// `@borderId`.
    #[getter]
    fn border_index(&self) -> Option<u32> {
        self.0.border_index
    }
}

#[pymethods]
impl EffectiveCellFormat {
    /// The `cellXfs` index this cell resolved to.
    #[getter]
    fn style_index(&self) -> u32 {
        self.0.style_index()
    }

    /// Which of the four layers supplied that index — the cell, its row, its column, or the default.
    #[getter]
    fn style_index_source(&self) -> PyResult<StyleIndexSource> {
        StyleIndexSource::from_model(self.0.style_index_source())
    }

    /// The `cellStyleXfs` record beneath the resolved `x:xf`, when it names one.
    #[getter]
    fn cell_style_format_index(&self) -> Option<u32> {
        self.0.cell_style_format_index()
    }

    /// One aspect's resolution: the number format, the font, the fill, the border, the alignment or
    /// the protection.
    fn aspect(&self, aspect: FormatAspect) -> ResolvedAspect {
        ResolvedAspect(self.0.aspect(aspect.into()))
    }
}

#[pymethods]
impl ResolvedAspect {
    /// Whether the resolved `x:xf` applies this aspect, suppresses it, or says nothing.
    #[getter]
    fn apply_flag(&self) -> PyResult<ApplyFlag> {
        ApplyFlag::from_model(self.0.apply_flag)
    }

    /// The same, for the record that supplied the value.
    #[getter]
    fn supplying_apply_flag(&self) -> PyResult<ApplyFlag> {
        ApplyFlag::from_model(self.0.supplying_apply_flag)
    }

    /// Which layer supplied it: the direct `cellXfs` record, the `cellStyleXfs` one beneath it, or
    /// neither.
    #[getter]
    fn layer(&self) -> PyResult<FormatLayer> {
        FormatLayer::from_model(self.0.layer)
    }

    /// The index of the `x:xf` that supplied the value.
    #[getter]
    fn format_index(&self) -> Option<u32> {
        self.0.format_index
    }

    /// The index into the resource table this aspect names — a font, a fill, a border.
    #[getter]
    fn resource_index(&self) -> Option<u32> {
        self.0.resource_index
    }

    /// Whether anything stated this aspect at all.
    #[getter]
    fn is_stated(&self) -> bool {
        self.0.is_stated
    }
}

/// Adds every class in this module to the extension module.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<CellData>()?;
    module.add_class::<CellWrite>()?;
    module.add_class::<CellBlock>()?;
    module.add_class::<SheetSummary>()?;
    module.add_class::<WorkbookWindowInfo>()?;
    module.add_class::<DefinedName>()?;
    module.add_class::<CalculationSettings>()?;
    module.add_class::<SheetHyperlinkInfo>()?;
    module.add_class::<SheetTableInfo>()?;
    module.add_class::<SheetTableColumnInfo>()?;
    module.add_class::<GridAnomalyInfo>()?;
    module.add_class::<PreservedPartsSummary>()?;
    module.add_class::<PreservedPart>()?;
    module.add_class::<SheetPivotTableInfo>()?;
    module.add_class::<WorkbookExternalLinkInfo>()?;
    module.add_class::<WorkbookConnectionInfo>()?;
    module.add_class::<SheetQueryTableInfo>()?;
    module.add_class::<WorkbookXmlMapsInfo>()?;
    module.add_class::<XmlMapInfo>()?;
    module.add_class::<WorkbookRevisionState>()?;
    module.add_class::<RevisionSessionInfo>()?;
    module.add_class::<SharedWorkbookUserInfo>()?;
    module.add_class::<Color>()?;
    module.add_class::<FontProperties>()?;
    module.add_class::<PatternFillSpec>()?;
    module.add_class::<BorderEdgeSpec>()?;
    module.add_class::<BorderSpec>()?;
    module.add_class::<CellFormatSpec>()?;
    module.add_class::<EffectiveCellFormat>()?;
    module.add_class::<ResolvedAspect>()
}
