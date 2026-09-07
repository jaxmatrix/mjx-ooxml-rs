//! [`Document`] — the curated Word surface, from Python.
//!
//! ```python
//! import mjx_ooxml
//!
//! document = mjx_ooxml.Document.blank(mjx_ooxml.PageSize.a4())
//! document.append_paragraph()
//! document.append_run(0, "Hello, document.")
//! open("out.docx", "wb").write(document.save())
//! ```
//!
//! Mirrors [`crate::deck::Deck`]'s own design exactly — see that module's doc comment for the "one
//! document, one thread" discipline, which applies here unchanged: almost every method takes
//! `&mut self`, nothing returns a view into the document, and nothing takes a callback.
//!
//! # Addressing
//!
//! A paragraph is an `int` (a top-level paragraph) or a [`BlockPath`]; a run is an `int` or a
//! [`RunPath`] — both accept a `list[int]` too, for the rare nested address (a run inside a
//! hyperlink, once `w:tbl` block nesting exists). `SectionLocation.body()` or
//! `SectionLocation.paragraph(...)` says which `w:sectPr` a section-editing call addresses.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyModule, PySequence, PyString};
use pyo3::Borrowed;

use mjx_ooxml as ooxml;

use crate::charts::{
    ChartAxisData, ChartData, ChartErrorBarData, ChartLabelScope, ChartLegendData,
    ChartPointFormatData, ChartSeriesData, ChartSeriesReferences, ChartTrendlineData, ChartWrap,
    DanglingPointReference, DataLabelSettings, DataLabelSpec, DocumentChartWorkbook, ErrorBarSpec,
    TrendlineSpec,
};
use crate::enums::{
    AxisOrientation, CellBorderEdge, ChartKind, HeaderFooterType, LegendPosition, MergedCellType,
};
use crate::errors::to_py_err;
use crate::format::Format;
use crate::paint::{FillSpec, LineSpec};
use crate::word::{
    CommentSummary, EffectiveBorder, EffectiveCharacterProperties, EffectiveParagraphProperties,
    EffectiveShading, Field, GridDiscrepancy, HyperlinkTarget, NoteSummary, PageMargins, PageSize,
    RevisionInfo, SectionSummary,
};

// ---------------------------------------------------------------------------------------------
// Addressing: BlockPath, RunPath, SectionLocation
// ---------------------------------------------------------------------------------------------

/// The address of a paragraph within a block container's content.
#[pyclass(frozen, from_py_object, module = "mjx_ooxml")]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BlockPath(pub(crate) ooxml::BlockPath);

#[pymethods]
impl BlockPath {
    /// The top-level paragraph at this index.
    #[staticmethod]
    fn top(index: u32) -> Self {
        Self(ooxml::BlockPath::from(index))
    }

    /// The paragraph at this address: `[1]` top-level, `[1, 0]` for a nested block container.
    #[staticmethod]
    fn of(indices: Vec<u32>) -> Self {
        Self(ooxml::BlockPath::from(indices))
    }

    /// The address as a list of indices, outermost first.
    #[getter]
    fn indices(&self) -> Vec<u32> {
        self.0.indices().to_vec()
    }

    /// How deep the address reaches.
    #[getter]
    fn depth(&self) -> u32 {
        self.0.depth()
    }

    /// Whether this addresses a top-level paragraph.
    #[getter]
    fn is_top_level(&self) -> bool {
        self.0.is_top_level()
    }

    fn __repr__(&self) -> String {
        format!("BlockPath.of({:?})", self.0.indices())
    }

    fn __str__(&self) -> String {
        self.0.to_string()
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn __hash__(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.0.hash(&mut hasher);
        hasher.finish()
    }
}

/// The address of a run within one paragraph's content.
#[pyclass(frozen, from_py_object, module = "mjx_ooxml")]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RunPath(pub(crate) ooxml::RunPath);

#[pymethods]
impl RunPath {
    /// The run at this top-level slot.
    #[staticmethod]
    fn top(index: u32) -> Self {
        Self(ooxml::RunPath::from(index))
    }

    /// The run at this address: `[0]` top-level, `[2, 0]` inside a run container (e.g. a
    /// hyperlink).
    #[staticmethod]
    fn of(indices: Vec<u32>) -> Self {
        Self(ooxml::RunPath::from(indices))
    }

    /// The address as a list of indices, outermost first.
    #[getter]
    fn indices(&self) -> Vec<u32> {
        self.0.indices().to_vec()
    }

    /// How deep the address reaches.
    #[getter]
    fn depth(&self) -> u32 {
        self.0.depth()
    }

    /// Whether this addresses a run directly in the paragraph.
    #[getter]
    fn is_top_level(&self) -> bool {
        self.0.is_top_level()
    }

    fn __repr__(&self) -> String {
        format!("RunPath.of({:?})", self.0.indices())
    }

    fn __str__(&self) -> String {
        self.0.to_string()
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn __hash__(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.0.hash(&mut hasher);
        hasher.finish()
    }
}

/// Which `w:sectPr` a section-editing method addresses.
#[pyclass(frozen, from_py_object, module = "mjx_ooxml")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionLocation(pub(crate) ooxml::SectionLocation);

#[pymethods]
impl SectionLocation {
    /// The body-level `w:sectPr` — the document's last section.
    #[staticmethod]
    fn body() -> Self {
        Self(ooxml::SectionLocation::Body)
    }

    /// The `w:sectPr` inside this paragraph's own `w:pPr`.
    #[staticmethod]
    fn paragraph(path: &Bound<'_, PyAny>) -> PyResult<Self> {
        let path = BlockPathArg::from_object(&path.as_borrowed())?;
        Ok(Self(ooxml::SectionLocation::Paragraph(path)))
    }

    fn __repr__(&self) -> String {
        match &self.0 {
            ooxml::SectionLocation::Body => "SectionLocation.body()".to_owned(),
            ooxml::SectionLocation::Paragraph(path) => {
                format!("SectionLocation.paragraph({:?})", path.indices())
            }
        }
    }
}

/// A paragraph argument: an `int`, a sequence of `int`, or a [`BlockPath`].
pub(crate) struct BlockPathArg;

impl BlockPathArg {
    fn from_object(object: &Borrowed<'_, '_, PyAny>) -> PyResult<ooxml::BlockPath> {
        if let Ok(path) = object.extract::<BlockPath>() {
            return Ok(path.0);
        }
        if let Some(index) = extract_index(object, "a paragraph address")? {
            return Ok(ooxml::BlockPath::from(index));
        }
        extract_indices(object, "a paragraph address").map(ooxml::BlockPath::from)
    }
}

/// A run argument: an `int`, a sequence of `int`, or a [`RunPath`].
pub(crate) struct RunPathArg;

impl RunPathArg {
    fn from_object(object: &Borrowed<'_, '_, PyAny>) -> PyResult<ooxml::RunPath> {
        if let Ok(path) = object.extract::<RunPath>() {
            return Ok(path.0);
        }
        if let Some(index) = extract_index(object, "a run address")? {
            return Ok(ooxml::RunPath::from(index));
        }
        extract_indices(object, "a run address").map(ooxml::RunPath::from)
    }
}

/// Tries the `int` case shared by every path argument. `Ok(None)` means "not a bool, not an int",
/// so the caller falls through to the sequence case; `Err` is a `bool`, rejected outright.
fn extract_index(object: &Borrowed<'_, '_, PyAny>, what: &str) -> PyResult<Option<u32>> {
    if object.is_instance_of::<pyo3::types::PyBool>() {
        return Err(pyo3::exceptions::PyTypeError::new_err(format!(
            "{what} is an int, a sequence of int, or a path object, not a bool"
        )));
    }
    Ok(object.extract::<u32>().ok())
}

/// The sequence case shared by every path argument.
fn extract_indices(object: &Borrowed<'_, '_, PyAny>, what: &str) -> PyResult<Vec<u32>> {
    if object.is_instance_of::<PyString>() {
        return Err(pyo3::exceptions::PyTypeError::new_err(format!(
            "{what} is an int, a sequence of int, or a path object, not a str"
        )));
    }
    if object.cast::<PySequence>().is_ok() {
        if let Ok(indices) = object.extract::<Vec<u32>>() {
            if indices.is_empty() {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "{what} needs at least one index"
                )));
            }
            return Ok(indices);
        }
    }
    Err(pyo3::exceptions::PyTypeError::new_err(format!(
        "{what} is an int, a sequence of int, or a path object, not {}",
        object
            .get_type()
            .name()
            .map(|name| name.to_string())
            .unwrap_or_else(|_| "an object of unknown type".to_owned())
    )))
}

// ---------------------------------------------------------------------------------------------
// Document
// ---------------------------------------------------------------------------------------------

/// An open Word document.
#[pyclass(module = "mjx_ooxml")]
#[derive(Debug)]
pub struct Document {
    inner: ooxml::Document,
}

#[pymethods]
impl Document {
    /// A new document with nothing in it beyond one empty paragraph and a body-level `w:sectPr`.
    #[staticmethod]
    fn blank(size: PageSize) -> PyResult<Self> {
        ooxml::Document::blank(size.0)
            .map(|inner| Self { inner })
            .map_err(to_py_err)
    }

    /// Opens a document from the bytes of a `.docx`, `.docm`, `.dotx` or `.dotm`.
    ///
    /// The interpreter lock is released for the parse. Raises `IoError` for bytes that are not a
    /// readable container, `MalformedDocumentError` for a package whose markup is not
    /// WordprocessingML, and `UnsupportedFormatError` — naming the format — for a PowerPoint or
    /// Excel document.
    #[staticmethod]
    fn open(python: Python<'_>, data: &[u8]) -> PyResult<Self> {
        python
            .detach(|| ooxml::Document::open(data))
            .map(|inner| Self { inner })
            .map_err(to_py_err)
    }

    /// What this document's main part says it is.
    fn format(&self) -> PyResult<Format> {
        Format::from_model(self.inner.format())
    }

    /// The document as the bytes of a `.docx`, **validated first**. The interpreter lock is
    /// released for the write.
    fn save<'py>(&self, python: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        python
            .detach(|| self.inner.save())
            .map(|bytes| PyBytes::new(python, &bytes))
            .map_err(to_py_err)
    }

    /// The document as bytes, **without** the validation pass.
    fn save_unchecked<'py>(&self, python: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        python
            .detach(|| self.inner.save_unchecked())
            .map(|bytes| PyBytes::new(python, &bytes))
            .map_err(to_py_err)
    }

    /// Checks the packaging invariants `save` enforces, without writing anything.
    fn validate(&self) -> PyResult<()> {
        self.inner.validate().map_err(to_py_err)
    }

    /// The document's conformance class (`"strict"`/`"transitional"`), or `None` if absent.
    fn conformance(&mut self) -> PyResult<Option<&'static str>> {
        self.inner
            .conformance()
            .map_err(to_py_err)
            .map(|value| value.map(conformance_str))
    }

    /// Sets (or, given `None`, removes) `w:document/@conformance`. `value` is `"strict"` or
    /// `"transitional"`.
    #[pyo3(signature = (value = None))]
    fn set_conformance(&mut self, value: Option<&str>) -> PyResult<()> {
        let value = value.map(conformance_from_str).transpose()?;
        self.inner.set_conformance(value).map_err(to_py_err)
    }

    // --- text: paragraphs and runs ---------------------------------------------------------------

    /// How many paragraphs the document body holds.
    fn paragraph_count(&mut self) -> PyResult<u32> {
        self.inner.paragraph_count().map_err(to_py_err)
    }

    /// How many run-or-hyperlink slots the given paragraph holds at its own top level.
    fn run_count(&mut self, paragraph: &Bound<'_, PyAny>) -> PyResult<u32> {
        let paragraph = BlockPathArg::from_object(&paragraph.as_borrowed())?;
        self.inner.run_count(paragraph).map_err(to_py_err)
    }

    /// The whole text of a paragraph, every run concatenated in document order.
    fn paragraph_text(&mut self, paragraph: &Bound<'_, PyAny>) -> PyResult<String> {
        let paragraph = BlockPathArg::from_object(&paragraph.as_borrowed())?;
        self.inner.paragraph_text(paragraph).map_err(to_py_err)
    }

    /// The text of one run.
    fn run_text(
        &mut self,
        paragraph: &Bound<'_, PyAny>,
        run: &Bound<'_, PyAny>,
    ) -> PyResult<String> {
        let paragraph = BlockPathArg::from_object(&paragraph.as_borrowed())?;
        let run = RunPathArg::from_object(&run.as_borrowed())?;
        self.inner.run_text(paragraph, run).map_err(to_py_err)
    }

    /// Sets the text of one run.
    fn set_run_text(
        &mut self,
        paragraph: &Bound<'_, PyAny>,
        run: &Bound<'_, PyAny>,
        text: &str,
    ) -> PyResult<()> {
        let paragraph = BlockPathArg::from_object(&paragraph.as_borrowed())?;
        let run = RunPathArg::from_object(&run.as_borrowed())?;
        self.inner
            .set_run_text(paragraph, run, text)
            .map_err(to_py_err)
    }

    /// Inserts a new, empty paragraph at `at`, shifting every paragraph at or after it later.
    fn insert_paragraph(&mut self, at: &Bound<'_, PyAny>) -> PyResult<()> {
        let at = BlockPathArg::from_object(&at.as_borrowed())?;
        self.inner.insert_paragraph(at).map_err(to_py_err)
    }

    /// Appends a new, empty paragraph as the body's new last paragraph.
    fn append_paragraph(&mut self) -> PyResult<()> {
        self.inner.append_paragraph().map_err(to_py_err)
    }

    /// Removes the paragraph at `at`.
    fn remove_paragraph(&mut self, at: &Bound<'_, PyAny>) -> PyResult<()> {
        let at = BlockPathArg::from_object(&at.as_borrowed())?;
        self.inner.remove_paragraph(at).map_err(to_py_err)
    }

    /// Inserts a new run holding `text` at slot `at` within `paragraph`.
    fn insert_run(
        &mut self,
        paragraph: &Bound<'_, PyAny>,
        at: &Bound<'_, PyAny>,
        text: &str,
    ) -> PyResult<()> {
        let paragraph = BlockPathArg::from_object(&paragraph.as_borrowed())?;
        let at = RunPathArg::from_object(&at.as_borrowed())?;
        self.inner
            .insert_run(paragraph, at, text)
            .map_err(to_py_err)
    }

    /// Appends a new run holding `text` as the paragraph's new last top-level run.
    fn append_run(&mut self, paragraph: &Bound<'_, PyAny>, text: &str) -> PyResult<()> {
        let paragraph = BlockPathArg::from_object(&paragraph.as_borrowed())?;
        self.inner.append_run(paragraph, text).map_err(to_py_err)
    }

    /// Removes the run at `run` within `paragraph`.
    fn remove_run(&mut self, paragraph: &Bound<'_, PyAny>, run: &Bound<'_, PyAny>) -> PyResult<()> {
        let paragraph = BlockPathArg::from_object(&paragraph.as_borrowed())?;
        let run = RunPathArg::from_object(&run.as_borrowed())?;
        self.inner.remove_run(paragraph, run).map_err(to_py_err)
    }

    // --- effective properties -----------------------------------------------------------------

    /// The effective character formatting of one run.
    fn effective_run_properties(
        &mut self,
        paragraph: &Bound<'_, PyAny>,
        run: &Bound<'_, PyAny>,
    ) -> PyResult<EffectiveCharacterProperties> {
        let paragraph = BlockPathArg::from_object(&paragraph.as_borrowed())?;
        let run = RunPathArg::from_object(&run.as_borrowed())?;
        self.inner
            .effective_run_properties(paragraph, run)
            .map_err(to_py_err)
            .map(EffectiveCharacterProperties)
    }

    /// The effective paragraph layout of one paragraph.
    fn effective_paragraph_properties(
        &mut self,
        paragraph: &Bound<'_, PyAny>,
    ) -> PyResult<EffectiveParagraphProperties> {
        let paragraph = BlockPathArg::from_object(&paragraph.as_borrowed())?;
        self.inner
            .effective_paragraph_properties(paragraph)
            .map_err(to_py_err)
            .map(EffectiveParagraphProperties)
    }

    /// The effective fill of one table cell.
    fn effective_cell_fill(
        &mut self,
        table: u32,
        row: u32,
        column: u32,
    ) -> PyResult<Option<EffectiveShading>> {
        self.inner
            .effective_cell_fill(table, row, column)
            .map_err(to_py_err)
            .map(|value| value.map(EffectiveShading))
    }

    /// The effective border on one edge of one table cell.
    fn effective_cell_border(
        &mut self,
        table: u32,
        row: u32,
        column: u32,
        edge: CellBorderEdge,
    ) -> PyResult<Option<EffectiveBorder>> {
        self.inner
            .effective_cell_border(table, row, column, edge.into())
            .map_err(to_py_err)
            .map(|value| value.map(EffectiveBorder))
    }

    /// The effective character formatting of a run addressed inside a table cell.
    fn effective_cell_run_properties(
        &mut self,
        table: u32,
        row: u32,
        column: u32,
        paragraph: u32,
        run: u32,
    ) -> PyResult<EffectiveCharacterProperties> {
        self.inner
            .effective_cell_run_properties(table, row, column, paragraph, run)
            .map_err(to_py_err)
            .map(EffectiveCharacterProperties)
    }

    // --- styles (read-only) --------------------------------------------------------------------

    /// Every `styleId` this document's `word/styles.xml` defines.
    fn style_ids(&mut self) -> PyResult<Vec<String>> {
        self.inner.style_ids().map_err(to_py_err)
    }

    /// The display name of the style identified by `style_id`, or `None`.
    fn style_name(&mut self, style_id: &str) -> PyResult<Option<String>> {
        self.inner.style_name(style_id).map_err(to_py_err)
    }

    // --- numbering ------------------------------------------------------------------------------

    /// Attaches a paragraph to numbering instance `numbering_id` at `level`.
    fn attach_paragraph_to_list(
        &mut self,
        paragraph: &Bound<'_, PyAny>,
        numbering_id: i64,
        level: i64,
    ) -> PyResult<()> {
        let paragraph = BlockPathArg::from_object(&paragraph.as_borrowed())?;
        self.inner
            .attach_paragraph_to_list(paragraph, numbering_id, level)
            .map_err(to_py_err)
    }

    /// Removes a paragraph's own numbering reference, if it carries one.
    fn detach_paragraph_from_list(&mut self, paragraph: &Bound<'_, PyAny>) -> PyResult<()> {
        let paragraph = BlockPathArg::from_object(&paragraph.as_borrowed())?;
        self.inner
            .detach_paragraph_from_list(paragraph)
            .map_err(to_py_err)
    }

    // --- sections and headers/footers ------------------------------------------------------------

    /// How many sections the document has.
    fn section_count(&mut self) -> PyResult<u32> {
        self.inner.section_count().map_err(to_py_err)
    }

    /// Every section, in document order, with its own resolved page geometry.
    fn sections(&mut self) -> PyResult<Vec<SectionSummary>> {
        self.inner
            .sections()
            .map_err(to_py_err)
            .map(|sections| sections.into_iter().map(SectionSummary).collect())
    }

    /// Sets (or removes) the page size of the `w:sectPr` at `location`.
    #[pyo3(signature = (location, size = None))]
    fn set_section_page_size(
        &mut self,
        location: &SectionLocation,
        size: Option<PageSize>,
    ) -> PyResult<()> {
        self.inner
            .set_section_page_size(location.0.clone(), size.map(|value| value.0))
            .map_err(to_py_err)
    }

    /// Sets (or removes) the page margins of the `w:sectPr` at `location`.
    #[pyo3(signature = (location, margins = None))]
    fn set_section_page_margins(
        &mut self,
        location: &SectionLocation,
        margins: Option<PageMargins>,
    ) -> PyResult<()> {
        self.inner
            .set_section_page_margins(location.0.clone(), margins.map(|value| value.0))
            .map_err(to_py_err)
    }

    /// Removes the `w:sectPr` at `location`, if it carries one.
    fn remove_section_properties(&mut self, location: &SectionLocation) -> PyResult<()> {
        self.inner
            .remove_section_properties(location.0.clone())
            .map_err(to_py_err)
    }

    /// Whether this document's sections use different headers/footers for even and odd pages.
    fn even_and_odd_headers(&mut self) -> PyResult<bool> {
        self.inner.even_and_odd_headers().map_err(to_py_err)
    }

    /// The text of the header of `kind` that applies to `section`'s pages, or `None`.
    fn header_text(&mut self, section: u32, kind: HeaderFooterType) -> PyResult<Option<String>> {
        self.inner
            .header_text(section, kind.into())
            .map_err(to_py_err)
    }

    /// As `header_text`, for footers.
    fn footer_text(&mut self, section: u32, kind: HeaderFooterType) -> PyResult<Option<String>> {
        self.inner
            .footer_text(section, kind.into())
            .map_err(to_py_err)
    }

    /// Creates (or replaces) a header holding one paragraph of `text` for the section at
    /// `location`.
    fn set_header_text(
        &mut self,
        location: &SectionLocation,
        kind: HeaderFooterType,
        text: &str,
    ) -> PyResult<()> {
        self.inner
            .set_header_text(location.0.clone(), kind.into(), text)
            .map_err(to_py_err)
    }

    /// As `set_header_text`, for footers.
    fn set_footer_text(
        &mut self,
        location: &SectionLocation,
        kind: HeaderFooterType,
        text: &str,
    ) -> PyResult<()> {
        self.inner
            .set_footer_text(location.0.clone(), kind.into(), text)
            .map_err(to_py_err)
    }

    /// Removes the section at `location`'s own `kind` header reference, if it states one.
    fn remove_header(
        &mut self,
        location: &SectionLocation,
        kind: HeaderFooterType,
    ) -> PyResult<()> {
        self.inner
            .remove_header(location.0.clone(), kind.into())
            .map_err(to_py_err)
    }

    /// As `remove_header`, for footers.
    fn remove_footer(
        &mut self,
        location: &SectionLocation,
        kind: HeaderFooterType,
    ) -> PyResult<()> {
        self.inner
            .remove_footer(location.0.clone(), kind.into())
            .map_err(to_py_err)
    }

    // --- tables ---------------------------------------------------------------------------------

    /// How many top-level tables the document body holds.
    fn table_count(&mut self) -> PyResult<u32> {
        self.inner.table_count().map_err(to_py_err)
    }

    /// The shape of a table, as `(rows, columns)`.
    fn table_dimensions(&mut self, table: u32) -> PyResult<(u32, u32)> {
        self.inner.table_dimensions(table).map_err(to_py_err)
    }

    /// How many rows and columns a cell spans, as `(rows, columns)`.
    fn cell_span(&mut self, table: u32, row: u32, column: u32) -> PyResult<(u32, u32)> {
        self.inner.cell_span(table, row, column).map_err(to_py_err)
    }

    /// Which cell actually renders at `(row, column)`, resolving any merge.
    fn merged_cell_anchor(&mut self, table: u32, row: u32, column: u32) -> PyResult<(u32, u32)> {
        self.inner
            .merged_cell_anchor(table, row, column)
            .map_err(to_py_err)
    }

    /// Every grid discrepancy a table currently has.
    fn table_grid_discrepancies(&mut self, table: u32) -> PyResult<Vec<GridDiscrepancy>> {
        self.inner
            .table_grid_discrepancies(table)
            .map_err(to_py_err)
            .map(|values| values.into_iter().map(GridDiscrepancy).collect())
    }

    /// The text of a table cell.
    fn cell_text(&mut self, table: u32, row: u32, column: u32) -> PyResult<String> {
        self.inner.cell_text(table, row, column).map_err(to_py_err)
    }

    /// Sets the text of a table cell.
    fn set_cell_text(&mut self, table: u32, row: u32, column: u32, text: &str) -> PyResult<()> {
        self.inner
            .set_cell_text(table, row, column, text)
            .map_err(to_py_err)
    }

    /// Sets (or, given `None`/`1`, removes) a cell's `w:gridSpan`.
    #[pyo3(signature = (table, row, column, span = None))]
    fn set_cell_span(
        &mut self,
        table: u32,
        row: u32,
        column: u32,
        span: Option<u32>,
    ) -> PyResult<()> {
        self.inner
            .set_cell_span(table, row, column, span)
            .map_err(to_py_err)
    }

    /// Sets (or, given `None`, removes) a cell's `w:vMerge`.
    #[pyo3(signature = (table, row, column, kind = None))]
    fn set_cell_vertical_merge(
        &mut self,
        table: u32,
        row: u32,
        column: u32,
        kind: Option<MergedCellType>,
    ) -> PyResult<()> {
        self.inner
            .set_cell_vertical_merge(table, row, column, kind.map(Into::into))
            .map_err(to_py_err)
    }

    /// Appends a new `rows` x `columns` table, and returns its new index.
    fn append_table(&mut self, rows: u32, columns: u32) -> PyResult<u32> {
        self.inner.append_table(rows, columns).map_err(to_py_err)
    }

    /// Removes the top-level table at `table`.
    fn remove_table(&mut self, table: u32) -> PyResult<()> {
        self.inner.remove_table(table).map_err(to_py_err)
    }

    /// Inserts a row into `table` so it becomes row `at`.
    fn insert_row(&mut self, table: u32, at: u32) -> PyResult<()> {
        self.inner.insert_row(table, at).map_err(to_py_err)
    }

    /// Removes row `at` from `table`.
    fn remove_row(&mut self, table: u32, at: u32) -> PyResult<()> {
        self.inner.remove_row(table, at).map_err(to_py_err)
    }

    /// Inserts a column into `table` so it becomes column `at`.
    fn insert_column(&mut self, table: u32, at: u32) -> PyResult<()> {
        self.inner.insert_column(table, at).map_err(to_py_err)
    }

    /// Removes column `at` from `table`.
    fn remove_column(&mut self, table: u32, at: u32) -> PyResult<()> {
        self.inner.remove_column(table, at).map_err(to_py_err)
    }

    // --- fields -----------------------------------------------------------------------------------

    /// Every field a paragraph holds, at its own top level and nested, in document order.
    fn fields(&mut self, paragraph: &Bound<'_, PyAny>) -> PyResult<Vec<Field>> {
        let paragraph = BlockPathArg::from_object(&paragraph.as_borrowed())?;
        self.inner
            .fields(paragraph)
            .map_err(to_py_err)
            .map(|fields| fields.into_iter().map(Field).collect())
    }

    /// Sets a field's own instruction. `field` is the sequence of indices from `fields`'s own top
    /// level down to the target field: `[0]` for the paragraph's first field.
    fn set_field_instruction(
        &mut self,
        paragraph: &Bound<'_, PyAny>,
        field: Vec<u32>,
        text: &str,
    ) -> PyResult<()> {
        let paragraph = BlockPathArg::from_object(&paragraph.as_borrowed())?;
        self.inner
            .set_field_instruction(paragraph, &field, text)
            .map_err(to_py_err)
    }

    /// Sets a field's own cached result. See `set_field_instruction` for how `field` addresses one.
    fn set_field_cached_result_text(
        &mut self,
        paragraph: &Bound<'_, PyAny>,
        field: Vec<u32>,
        text: &str,
    ) -> PyResult<()> {
        let paragraph = BlockPathArg::from_object(&paragraph.as_borrowed())?;
        self.inner
            .set_field_cached_result_text(paragraph, &field, text)
            .map_err(to_py_err)
    }

    // --- hyperlinks -----------------------------------------------------------------------------

    /// The click target of the hyperlink at slot `at` within `paragraph`, or `None`.
    fn hyperlink_target(
        &mut self,
        paragraph: &Bound<'_, PyAny>,
        at: &Bound<'_, PyAny>,
    ) -> PyResult<Option<HyperlinkTarget>> {
        let paragraph = BlockPathArg::from_object(&paragraph.as_borrowed())?;
        let at = RunPathArg::from_object(&at.as_borrowed())?;
        self.inner
            .hyperlink_target(paragraph, at)
            .map_err(to_py_err)
            .map(|value| value.map(HyperlinkTarget))
    }

    /// Inserts a new hyperlink wrapping one run of `text` at slot `at` within `paragraph`.
    fn insert_hyperlink(
        &mut self,
        paragraph: &Bound<'_, PyAny>,
        at: &Bound<'_, PyAny>,
        text: &str,
        target: &HyperlinkTarget,
    ) -> PyResult<()> {
        let paragraph = BlockPathArg::from_object(&paragraph.as_borrowed())?;
        let at = RunPathArg::from_object(&at.as_borrowed())?;
        self.inner
            .insert_hyperlink(paragraph, at, text, &target.0)
            .map_err(to_py_err)
    }

    /// Removes the hyperlink at slot `at` within `paragraph`, together with every run it wraps.
    fn remove_hyperlink(
        &mut self,
        paragraph: &Bound<'_, PyAny>,
        at: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        let paragraph = BlockPathArg::from_object(&paragraph.as_borrowed())?;
        let at = RunPathArg::from_object(&at.as_borrowed())?;
        self.inner
            .remove_hyperlink(paragraph, at)
            .map_err(to_py_err)
    }

    // --- comments -------------------------------------------------------------------------------

    /// Every comment this document's `word/comments.xml` holds.
    fn comments(&mut self) -> PyResult<Vec<CommentSummary>> {
        self.inner
            .comments()
            .map_err(to_py_err)
            .map(|values| values.into_iter().map(CommentSummary).collect())
    }

    /// Adds a new comment on the whole paragraph at `paragraph`. Returns the comment's own id.
    #[pyo3(signature = (paragraph, author, initials = None, text = ""))]
    fn add_comment(
        &mut self,
        paragraph: &Bound<'_, PyAny>,
        author: &str,
        initials: Option<&str>,
        text: &str,
    ) -> PyResult<i64> {
        let paragraph = BlockPathArg::from_object(&paragraph.as_borrowed())?;
        self.inner
            .add_comment(paragraph, author, initials, text)
            .map_err(to_py_err)
    }

    /// Removes the comment with `id`.
    fn remove_comment(&mut self, id: i64) -> PyResult<()> {
        self.inner.remove_comment(id).map_err(to_py_err)
    }

    /// The resolved text between a comment's own range markers, or `None`.
    fn comment_range_text(&mut self, id: i64) -> PyResult<Option<String>> {
        self.inner.comment_range_text(id).map_err(to_py_err)
    }

    // --- footnotes, endnotes and revisions --------------------------------------------------------

    /// Every user-visible footnote this document holds.
    fn footnotes(&mut self) -> PyResult<Vec<NoteSummary>> {
        self.inner
            .footnotes()
            .map_err(to_py_err)
            .map(|values| values.into_iter().map(NoteSummary).collect())
    }

    /// Adds a new user footnote referenced from the end of `paragraph`. Returns its own id.
    fn add_footnote(&mut self, paragraph: &Bound<'_, PyAny>, text: &str) -> PyResult<i64> {
        let paragraph = BlockPathArg::from_object(&paragraph.as_borrowed())?;
        self.inner.add_footnote(paragraph, text).map_err(to_py_err)
    }

    /// Removes the user footnote with `id`.
    fn remove_footnote(&mut self, id: i64) -> PyResult<()> {
        self.inner.remove_footnote(id).map_err(to_py_err)
    }

    /// As `footnotes`, for endnotes.
    fn endnotes(&mut self) -> PyResult<Vec<NoteSummary>> {
        self.inner
            .endnotes()
            .map_err(to_py_err)
            .map(|values| values.into_iter().map(NoteSummary).collect())
    }

    /// As `add_footnote`, for endnotes.
    fn add_endnote(&mut self, paragraph: &Bound<'_, PyAny>, text: &str) -> PyResult<i64> {
        let paragraph = BlockPathArg::from_object(&paragraph.as_borrowed())?;
        self.inner.add_endnote(paragraph, text).map_err(to_py_err)
    }

    /// As `remove_footnote`, for endnotes.
    fn remove_endnote(&mut self, id: i64) -> PyResult<()> {
        self.inner.remove_endnote(id).map_err(to_py_err)
    }

    /// Every tracked-change marker the document body holds.
    fn revisions(&mut self) -> PyResult<Vec<RevisionInfo>> {
        self.inner
            .revisions()
            .map_err(to_py_err)
            .map(|values| values.into_iter().map(RevisionInfo).collect())
    }

    /// The document body's text with tracked insertions kept and tracked deletions dropped.
    fn text_with_revisions_accepted(&mut self) -> PyResult<String> {
        self.inner.text_with_revisions_accepted().map_err(to_py_err)
    }

    /// As `text_with_revisions_accepted`, the rejected-text counterpart.
    fn text_with_revisions_rejected(&mut self) -> PyResult<String> {
        self.inner.text_with_revisions_rejected().map_err(to_py_err)
    }

    // --- drawings ---------------------------------------------------------------------------------

    /// Adds an inline picture as a new run at the end of `paragraph`. Returns its `wp:docPr` id.
    #[allow(clippy::too_many_arguments)]
    fn add_inline_picture(
        &mut self,
        paragraph: &Bound<'_, PyAny>,
        image_bytes: Vec<u8>,
        content_type: &str,
        extension: &str,
        width_emu: i64,
        height_emu: i64,
        name: &str,
    ) -> PyResult<u32> {
        let paragraph = BlockPathArg::from_object(&paragraph.as_borrowed())?;
        self.inner
            .add_inline_picture(
                paragraph,
                image_bytes,
                content_type,
                extension,
                width_emu,
                height_emu,
                name,
            )
            .map_err(to_py_err)
    }

    /// Removes the drawing whose `wp:docPr@id` is `doc_pr_id`. Returns whether one was removed.
    fn remove_drawing(&mut self, doc_pr_id: u32) -> PyResult<bool> {
        self.inner.remove_drawing(doc_pr_id).map_err(to_py_err)
    }

    // --- charts (MJXOFF-103) ----------------------------------------------------------------------
    //
    // Every method below is named exactly as its `Deck` counterpart is, takes the same arguments in
    // the same order after the address, and delegates to exactly one `mjx_ooxml::Document` method.
    // The address is the drawing's own `wp:docPr` id rather than `(surface, shape)`, because a
    // document has no shape tree — see `mjx_ooxml::document::charts` for that argument in full.

    /// The `wp:docPr` id of every drawing in the document body that frames a chart, in document
    /// order.
    fn chart_drawing_ids(&mut self) -> PyResult<Vec<u32>> {
        self.inner.chart_drawing_ids().map_err(to_py_err)
    }

    /// The relationship id the drawing `drawing_id` names as its chart part, or `None` when that
    /// drawing frames no chart.
    fn chart_rel_id(&mut self, drawing_id: u32) -> PyResult<Option<String>> {
        self.inner.chart_rel_id(drawing_id).map_err(to_py_err)
    }

    /// The raw XML bytes of the chart part the drawing `drawing_id` references, or `None`.
    fn chart_part_bytes<'py>(
        &mut self,
        py: Python<'py>,
        drawing_id: u32,
    ) -> PyResult<Option<Bound<'py, PyBytes>>> {
        Ok(self
            .inner
            .chart_part_bytes(drawing_id)
            .map_err(to_py_err)?
            .map(|bytes| PyBytes::new(py, &bytes)))
    }

    /// Adds `chart` as a new inline chart at the end of `paragraph`. Returns its `wp:docPr` id.
    fn add_chart(
        &mut self,
        paragraph: &Bound<'_, PyAny>,
        chart: &ChartData,
        width_emu: i64,
        height_emu: i64,
        name: &str,
    ) -> PyResult<u32> {
        let paragraph = BlockPathArg::from_object(&paragraph.as_borrowed())?;
        self.inner
            .add_chart(paragraph, &chart.0, width_emu, height_emu, name)
            .map_err(to_py_err)
    }

    /// Adds `chart` as a floating chart, offset from the paragraph's own origin, with the text
    /// wrapping around it as `wrap` says. Returns its `wp:docPr` id.
    #[allow(clippy::too_many_arguments)]
    fn add_floating_chart(
        &mut self,
        paragraph: &Bound<'_, PyAny>,
        chart: &ChartData,
        offset_x_emu: i64,
        offset_y_emu: i64,
        width_emu: i64,
        height_emu: i64,
        wrap: &ChartWrap,
        name: &str,
    ) -> PyResult<u32> {
        let paragraph = BlockPathArg::from_object(&paragraph.as_borrowed())?;
        self.inner
            .add_floating_chart(
                paragraph,
                &chart.0,
                offset_x_emu,
                offset_y_emu,
                width_emu,
                height_emu,
                wrap.0,
                name,
            )
            .map_err(to_py_err)
    }

    /// Every chart in the document that references a backing workbook.
    fn chart_workbooks(&mut self) -> PyResult<Vec<DocumentChartWorkbook>> {
        Ok(self
            .inner
            .chart_workbooks()
            .map_err(to_py_err)?
            .into_iter()
            .map(DocumentChartWorkbook)
            .collect())
    }

    /// Rewrites the embedded workbook of the chart `drawing_id` frames. Answers whether it rewrote
    /// one.
    fn refresh_chart_workbook(&mut self, drawing_id: u32) -> PyResult<bool> {
        self.inner
            .refresh_chart_workbook(drawing_id)
            .map_err(to_py_err)
    }

    /// Replaces the embedded workbook of the chart `drawing_id` frames with a freshly built one,
    /// discarding whatever it held. Answers whether it replaced one.
    fn regenerate_chart_workbook(&mut self, drawing_id: u32) -> PyResult<bool> {
        self.inner
            .regenerate_chart_workbook(drawing_id)
            .map_err(to_py_err)
    }

    /// Detaches the backing workbook, leaving the chart to render from its cached values.
    fn detach_chart_workbook(&mut self, drawing_id: u32) -> PyResult<()> {
        self.inner
            .detach_chart_workbook(drawing_id)
            .map_err(to_py_err)
    }

    /// The series of the chart the drawing `drawing_id` frames.
    fn chart_series(&mut self, drawing_id: u32) -> PyResult<Vec<ChartSeriesData>> {
        Ok(self
            .inner
            .chart_series(drawing_id)
            .map_err(to_py_err)?
            .into_iter()
            .map(ChartSeriesData)
            .collect())
    }

    /// Where every series says its data lives — the formula beside each cache, as written. The
    /// companion of `chart_series`: that answers what the caches *hold*, this answers what the
    /// references *name*.
    fn chart_series_references(&mut self, drawing_id: u32) -> PyResult<Vec<ChartSeriesReferences>> {
        Ok(self
            .inner
            .chart_series_references(drawing_id)
            .map_err(to_py_err)?
            .into_iter()
            .map(ChartSeriesReferences)
            .collect())
    }

    /// The kind of every plot the chart draws, in document order.
    fn chart_kinds(&mut self, drawing_id: u32) -> PyResult<Vec<ChartKind>> {
        self.inner
            .chart_kinds(drawing_id)
            .map_err(to_py_err)?
            .into_iter()
            .map(ChartKind::from_model)
            .collect()
    }

    /// The axes of the chart, in document order.
    fn chart_axes(&mut self, drawing_id: u32) -> PyResult<Vec<ChartAxisData>> {
        Ok(self
            .inner
            .chart_axes(drawing_id)
            .map_err(to_py_err)?
            .into_iter()
            .map(ChartAxisData)
            .collect())
    }

    /// The heading of the chart, or `None` when it has none.
    fn chart_title(&mut self, drawing_id: u32) -> PyResult<Option<String>> {
        self.inner.chart_title(drawing_id).map_err(to_py_err)
    }

    /// The legend of the chart, or `None` when it has none.
    fn chart_legend(&mut self, drawing_id: u32) -> PyResult<Option<ChartLegendData>> {
        Ok(self
            .inner
            .chart_legend(drawing_id)
            .map_err(to_py_err)?
            .map(ChartLegendData))
    }

    /// The built-in style id the chart names, or `None`.
    fn chart_style_id(&mut self, drawing_id: u32) -> PyResult<Option<u32>> {
        self.inner.chart_style_id(drawing_id).map_err(to_py_err)
    }

    /// The fill of series `series_idx`, or `None` when it takes its colour from the chart style.
    fn chart_series_fill(
        &mut self,
        drawing_id: u32,
        series_idx: u32,
    ) -> PyResult<Option<FillSpec>> {
        Ok(self
            .inner
            .chart_series_fill(drawing_id, series_idx)
            .map_err(to_py_err)?
            .map(FillSpec))
    }

    /// The data-label settings in force for one point of series `series_idx`.
    #[pyo3(signature = (drawing_id, series_idx, point_idx=None))]
    fn chart_data_labels(
        &mut self,
        drawing_id: u32,
        series_idx: u32,
        point_idx: Option<u32>,
    ) -> PyResult<DataLabelSettings> {
        self.inner
            .chart_data_labels(drawing_id, series_idx, point_idx)
            .map_err(to_py_err)
            .map(DataLabelSettings)
    }

    /// The data-label settings one tier states in its own right.
    fn chart_data_label_tier(
        &mut self,
        drawing_id: u32,
        scope: &ChartLabelScope,
    ) -> PyResult<Option<DataLabelSettings>> {
        Ok(self
            .inner
            .chart_data_label_tier(drawing_id, scope.0)
            .map_err(to_py_err)?
            .map(DataLabelSettings))
    }

    /// The words one point's label shows in place of its value, or `None`.
    fn chart_point_label_text(
        &mut self,
        drawing_id: u32,
        series_idx: u32,
        point_idx: u32,
    ) -> PyResult<Option<String>> {
        self.inner
            .chart_point_label_text(drawing_id, series_idx, point_idx)
            .map_err(to_py_err)
    }

    /// Every point of series `series_idx` that carries its own formatting.
    fn chart_point_formats(
        &mut self,
        drawing_id: u32,
        series_idx: u32,
    ) -> PyResult<Vec<ChartPointFormatData>> {
        Ok(self
            .inner
            .chart_point_formats(drawing_id, series_idx)
            .map_err(to_py_err)?
            .into_iter()
            .map(ChartPointFormatData)
            .collect())
    }

    /// Every trendline fitted through series `series_idx`.
    fn chart_trendlines(
        &mut self,
        drawing_id: u32,
        series_idx: u32,
    ) -> PyResult<Vec<ChartTrendlineData>> {
        Ok(self
            .inner
            .chart_trendlines(drawing_id, series_idx)
            .map_err(to_py_err)?
            .into_iter()
            .map(ChartTrendlineData)
            .collect())
    }

    /// Every set of error bars series `series_idx` carries.
    fn chart_error_bars(
        &mut self,
        drawing_id: u32,
        series_idx: u32,
    ) -> PyResult<Vec<ChartErrorBarData>> {
        Ok(self
            .inner
            .chart_error_bars(drawing_id, series_idx)
            .map_err(to_py_err)?
            .into_iter()
            .map(ChartErrorBarData)
            .collect())
    }

    /// Every decoration of series `series_idx` naming a point the series no longer has.
    fn chart_dangling_decoration(
        &mut self,
        drawing_id: u32,
        series_idx: u32,
    ) -> PyResult<Vec<DanglingPointReference>> {
        Ok(self
            .inner
            .chart_dangling_decoration(drawing_id, series_idx)
            .map_err(to_py_err)?
            .into_iter()
            .map(DanglingPointReference)
            .collect())
    }

    /// Rewrites the values of series `series_idx`, refreshing the embedded workbook in the same
    /// call.
    fn set_chart_series_values(
        &mut self,
        drawing_id: u32,
        series_idx: u32,
        values: Vec<f64>,
    ) -> PyResult<()> {
        self.inner
            .set_chart_series_values(drawing_id, series_idx, &values)
            .map_err(to_py_err)
    }

    /// Rewrites the category labels of series `series_idx`, refreshing the workbook alongside.
    fn set_chart_series_categories(
        &mut self,
        drawing_id: u32,
        series_idx: u32,
        labels: Vec<String>,
    ) -> PyResult<()> {
        let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
        self.inner
            .set_chart_series_categories(drawing_id, series_idx, &labels)
            .map_err(to_py_err)
    }

    /// Sets or clears the explicit bounds of axis `axis_idx`.
    #[pyo3(signature = (drawing_id, axis_idx, minimum=None, maximum=None))]
    fn set_chart_axis_scale(
        &mut self,
        drawing_id: u32,
        axis_idx: u32,
        minimum: Option<f64>,
        maximum: Option<f64>,
    ) -> PyResult<()> {
        self.inner
            .set_chart_axis_scale(drawing_id, axis_idx, minimum, maximum)
            .map_err(to_py_err)
    }

    /// Sets the direction of axis `axis_idx`.
    fn set_chart_axis_orientation(
        &mut self,
        drawing_id: u32,
        axis_idx: u32,
        orientation: AxisOrientation,
    ) -> PyResult<()> {
        self.inner
            .set_chart_axis_orientation(drawing_id, axis_idx, orientation.into())
            .map_err(to_py_err)
    }

    /// Sets or removes the title of axis `axis_idx`.
    #[pyo3(signature = (drawing_id, axis_idx, text=None))]
    fn set_chart_axis_title(
        &mut self,
        drawing_id: u32,
        axis_idx: u32,
        text: Option<&str>,
    ) -> PyResult<()> {
        self.inner
            .set_chart_axis_title(drawing_id, axis_idx, text)
            .map_err(to_py_err)
    }

    /// Turns the gridlines of axis `axis_idx` on or off.
    fn set_chart_axis_gridlines(
        &mut self,
        drawing_id: u32,
        axis_idx: u32,
        major: bool,
        minor: bool,
    ) -> PyResult<()> {
        self.inner
            .set_chart_axis_gridlines(drawing_id, axis_idx, major, minor)
            .map_err(to_py_err)
    }

    /// Sets or removes the chart's heading.
    #[pyo3(signature = (drawing_id, text=None))]
    fn set_chart_title(&mut self, drawing_id: u32, text: Option<&str>) -> PyResult<()> {
        self.inner
            .set_chart_title(drawing_id, text)
            .map_err(to_py_err)
    }

    /// Places the chart's legend at `position`, or removes it.
    #[pyo3(signature = (drawing_id, position=None))]
    fn set_chart_legend(
        &mut self,
        drawing_id: u32,
        position: Option<LegendPosition>,
    ) -> PyResult<()> {
        self.inner
            .set_chart_legend(drawing_id, position.map(Into::into))
            .map_err(to_py_err)
    }

    /// Sets the fill of series `series_idx`.
    fn set_chart_series_fill(
        &mut self,
        drawing_id: u32,
        series_idx: u32,
        fill: &FillSpec,
    ) -> PyResult<()> {
        self.inner
            .set_chart_series_fill(drawing_id, series_idx, &fill.0)
            .map_err(to_py_err)
    }

    /// Sets the outline of series `series_idx`.
    fn set_chart_series_line(
        &mut self,
        drawing_id: u32,
        series_idx: u32,
        line: &LineSpec,
    ) -> PyResult<()> {
        self.inner
            .set_chart_series_line(drawing_id, series_idx, &line.0)
            .map_err(to_py_err)
    }

    /// Applies `spec` at one tier of the chart's data labels.
    fn set_chart_data_labels(
        &mut self,
        drawing_id: u32,
        scope: &ChartLabelScope,
        spec: &DataLabelSpec,
    ) -> PyResult<()> {
        self.inner
            .set_chart_data_labels(drawing_id, scope.0, &spec.0)
            .map_err(to_py_err)
    }

    /// Suppresses the labels at one tier.
    fn suppress_chart_data_labels(
        &mut self,
        drawing_id: u32,
        scope: &ChartLabelScope,
    ) -> PyResult<()> {
        self.inner
            .suppress_chart_data_labels(drawing_id, scope.0)
            .map_err(to_py_err)
    }

    /// Removes the labels at one tier entirely. Answers whether one was there.
    fn remove_chart_data_labels(
        &mut self,
        drawing_id: u32,
        scope: &ChartLabelScope,
    ) -> PyResult<bool> {
        self.inner
            .remove_chart_data_labels(drawing_id, scope.0)
            .map_err(to_py_err)
    }

    /// Colours point `point_idx` of series `series_idx` differently from the rest of its series.
    fn set_chart_point_fill(
        &mut self,
        drawing_id: u32,
        series_idx: u32,
        point_idx: u32,
        fill: &FillSpec,
    ) -> PyResult<()> {
        self.inner
            .set_chart_point_fill(drawing_id, series_idx, point_idx, &fill.0)
            .map_err(to_py_err)
    }

    /// Outlines point `point_idx` of series `series_idx` differently from the rest of its series.
    fn set_chart_point_line(
        &mut self,
        drawing_id: u32,
        series_idx: u32,
        point_idx: u32,
        line: &LineSpec,
    ) -> PyResult<()> {
        self.inner
            .set_chart_point_line(drawing_id, series_idx, point_idx, &line.0)
            .map_err(to_py_err)
    }

    /// Pulls slice `point_idx` of series `series_idx` out of its pie or doughnut, or puts it back.
    #[pyo3(signature = (drawing_id, series_idx, point_idx, percent=None))]
    fn set_chart_point_explosion(
        &mut self,
        drawing_id: u32,
        series_idx: u32,
        point_idx: u32,
        percent: Option<u32>,
    ) -> PyResult<()> {
        self.inner
            .set_chart_point_explosion(drawing_id, series_idx, point_idx, percent)
            .map_err(to_py_err)
    }

    /// Removes the formatting of point `point_idx` of series `series_idx`.
    fn remove_chart_point_format(
        &mut self,
        drawing_id: u32,
        series_idx: u32,
        point_idx: u32,
    ) -> PyResult<bool> {
        self.inner
            .remove_chart_point_format(drawing_id, series_idx, point_idx)
            .map_err(to_py_err)
    }

    /// Fits a trendline through series `series_idx`, appending to any it already carries.
    fn add_chart_trendline(
        &mut self,
        drawing_id: u32,
        series_idx: u32,
        spec: &TrendlineSpec,
    ) -> PyResult<()> {
        self.inner
            .add_chart_trendline(drawing_id, series_idx, &spec.0)
            .map_err(to_py_err)
    }

    /// Rewrites trendline `trendline_idx` of series `series_idx` from `spec`, in place.
    fn set_chart_trendline(
        &mut self,
        drawing_id: u32,
        series_idx: u32,
        trendline_idx: u32,
        spec: &TrendlineSpec,
    ) -> PyResult<()> {
        self.inner
            .set_chart_trendline(drawing_id, series_idx, trendline_idx, &spec.0)
            .map_err(to_py_err)
    }

    /// Removes every trendline from series `series_idx`, answering how many went.
    fn remove_chart_trendlines(&mut self, drawing_id: u32, series_idx: u32) -> PyResult<u32> {
        self.inner
            .remove_chart_trendlines(drawing_id, series_idx)
            .map_err(to_py_err)
    }

    /// Gives series `series_idx` error bars, replacing an existing set along the same axis.
    fn set_chart_error_bars(
        &mut self,
        drawing_id: u32,
        series_idx: u32,
        spec: &ErrorBarSpec,
    ) -> PyResult<()> {
        self.inner
            .set_chart_error_bars(drawing_id, series_idx, &spec.0)
            .map_err(to_py_err)
    }

    /// Removes every set of error bars from series `series_idx`, answering how many went.
    fn remove_chart_error_bars(&mut self, drawing_id: u32, series_idx: u32) -> PyResult<u32> {
        self.inner
            .remove_chart_error_bars(drawing_id, series_idx)
            .map_err(to_py_err)
    }

    /// Removes every decoration of series `series_idx` past the end of its data, answering how many
    /// went.
    fn drop_chart_dangling_decoration(
        &mut self,
        drawing_id: u32,
        series_idx: u32,
    ) -> PyResult<u32> {
        self.inner
            .drop_chart_dangling_decoration(drawing_id, series_idx)
            .map_err(to_py_err)
    }
}

/// `w:document/@conformance`'s two wire values, as the strings this binding raises/accepts —
/// `mjx_ooxml_types::shared::ConformanceClass` carries no Python projection of its own since this
/// is its only use on the curated surface.
fn conformance_str(value: ooxml::ConformanceClass) -> &'static str {
    match value {
        ooxml::ConformanceClass::Strict => "strict",
        ooxml::ConformanceClass::Transitional => "transitional",
    }
}

fn conformance_from_str(value: &str) -> PyResult<ooxml::ConformanceClass> {
    match value {
        "strict" => Ok(ooxml::ConformanceClass::Strict),
        "transitional" => Ok(ooxml::ConformanceClass::Transitional),
        other => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "conformance is \"strict\" or \"transitional\", not {other:?}"
        ))),
    }
}

/// Adds [`Document`] and its addressing classes to the extension module.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<BlockPath>()?;
    module.add_class::<RunPath>()?;
    module.add_class::<SectionLocation>()?;
    module.add_class::<Document>()
}
