//! [`Document`] — the curated Word surface, from JavaScript and TypeScript.
//!
//! ```js
//! import init, { Document, PageSize } from "@mjx/ooxml";
//!
//! await init();
//! const document = Document.blank(PageSize.a4());
//! try {
//!   document.appendParagraph();
//!   document.appendRun(0, "Hello, document.");
//!   const blob = new Blob([document.save()], {
//!     type: "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
//!   });
//! } finally {
//!   document.free();
//! }
//! ```
//!
//! Mirrors [`crate::deck::Deck`]'s own design exactly — see that module's doc comment for why
//! `free()` is mandatory and why methods are camelCase.
//!
//! # Addressing
//!
//! A paragraph is a `BlockPath`, a number (a top-level paragraph), or an array of numbers (a
//! descent through nested block containers); a run is a `RunPath` or the same two numeric
//! spellings. The numeric spellings allocate nothing and need no `free()`.

use wasm_bindgen::prelude::*;

use mjx_ooxml as ooxml;

use crate::enums::{CellBorderEdge, HeaderFooterType, MergedCellType};
use crate::errors::map_error;
use crate::format::Format;
use crate::support::invalid_argument;
use crate::word::{
    CommentSummary, EffectiveBorder, EffectiveCharacterProperties, EffectiveParagraphProperties,
    EffectiveShading, Field, GridDiscrepancy, HyperlinkTarget, NoteSummary, PageMargins, PageSize,
    RevisionInfo, SectionSummary,
};

// ---------------------------------------------------------------------------------------------
// Addressing: BlockPath, RunPath, SectionLocation
// ---------------------------------------------------------------------------------------------

/// The address of a paragraph within a block container's content.
#[wasm_bindgen]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BlockPath(pub(crate) ooxml::BlockPath);

#[wasm_bindgen]
impl BlockPath {
    /// The top-level paragraph at this index.
    #[wasm_bindgen(js_name = "top")]
    pub fn top(index: u32) -> Self {
        Self(ooxml::BlockPath::from(index))
    }

    /// The paragraph at this address: `[1]` top-level, `[1, 0]` for a nested block container.
    #[wasm_bindgen(js_name = "of")]
    pub fn of(indices: Vec<u32>) -> Result<BlockPath, JsValue> {
        if indices.is_empty() {
            return Err(invalid_argument(
                "a paragraph address needs at least one index",
            ));
        }
        Ok(Self(ooxml::BlockPath::from(indices)))
    }

    /// The address as an array of indices, outermost first.
    #[wasm_bindgen(getter, js_name = "indices")]
    pub fn indices(&self) -> Vec<u32> {
        self.0.indices().to_vec()
    }

    /// How deep the address reaches.
    #[wasm_bindgen(getter, js_name = "depth")]
    pub fn depth(&self) -> u32 {
        self.0.depth()
    }

    /// Whether this addresses a top-level paragraph.
    #[wasm_bindgen(getter, js_name = "isTopLevel")]
    pub fn is_top_level(&self) -> bool {
        self.0.is_top_level()
    }

    /// Whether this addresses the same paragraph as `other`.
    #[wasm_bindgen(js_name = "equals")]
    pub fn equals(&self, other: &BlockPath) -> bool {
        self.0 == other.0
    }

    /// `1` for a top-level paragraph, `[1, 0]` for a nested one.
    #[wasm_bindgen(js_name = "toString")]
    pub fn to_display_string(&self) -> String {
        self.0.to_string()
    }
}

/// The address of a run within one paragraph's content.
#[wasm_bindgen]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RunPath(pub(crate) ooxml::RunPath);

#[wasm_bindgen]
impl RunPath {
    /// The run at this top-level slot.
    #[wasm_bindgen(js_name = "top")]
    pub fn top(index: u32) -> Self {
        Self(ooxml::RunPath::from(index))
    }

    /// The run at this address: `[0]` top-level, `[2, 0]` inside a run container.
    #[wasm_bindgen(js_name = "of")]
    pub fn of(indices: Vec<u32>) -> Result<RunPath, JsValue> {
        if indices.is_empty() {
            return Err(invalid_argument("a run address needs at least one index"));
        }
        Ok(Self(ooxml::RunPath::from(indices)))
    }

    /// The address as an array of indices, outermost first.
    #[wasm_bindgen(getter, js_name = "indices")]
    pub fn indices(&self) -> Vec<u32> {
        self.0.indices().to_vec()
    }

    /// How deep the address reaches.
    #[wasm_bindgen(getter, js_name = "depth")]
    pub fn depth(&self) -> u32 {
        self.0.depth()
    }

    /// Whether this addresses a run directly in the paragraph.
    #[wasm_bindgen(getter, js_name = "isTopLevel")]
    pub fn is_top_level(&self) -> bool {
        self.0.is_top_level()
    }

    /// Whether this addresses the same run as `other`.
    #[wasm_bindgen(js_name = "equals")]
    pub fn equals(&self, other: &RunPath) -> bool {
        self.0 == other.0
    }

    /// `0` for a top-level run, `[2, 0]` for a nested one.
    #[wasm_bindgen(js_name = "toString")]
    pub fn to_display_string(&self) -> String {
        self.0.to_string()
    }
}

/// Which `w:sectPr` a section-editing method addresses.
#[wasm_bindgen]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionLocation(pub(crate) ooxml::SectionLocation);

#[wasm_bindgen]
impl SectionLocation {
    /// The body-level `w:sectPr` — the document's last section.
    #[wasm_bindgen(js_name = "body")]
    pub fn body() -> Self {
        Self(ooxml::SectionLocation::Body)
    }

    /// The `w:sectPr` inside this paragraph's own `w:pPr`.
    #[wasm_bindgen(js_name = "paragraph")]
    pub fn paragraph(path: &BlockPathArg) -> Result<SectionLocation, JsValue> {
        Ok(Self(ooxml::SectionLocation::Paragraph(block_path_of(
            path,
        )?)))
    }

    /// `"body"` or `"paragraph"`.
    #[wasm_bindgen(js_name = "toString")]
    pub fn to_display_string(&self) -> String {
        match &self.0 {
            ooxml::SectionLocation::Body => "SectionLocation.body()".to_owned(),
            ooxml::SectionLocation::Paragraph(path) => {
                format!("SectionLocation.paragraph({:?})", path.indices())
            }
        }
    }
}

#[wasm_bindgen]
extern "C" {
    /// A paragraph argument: a `BlockPath`, a number (a top-level paragraph), or an array of
    /// numbers.
    #[wasm_bindgen(typescript_type = "BlockPath | number | number[]")]
    pub type BlockPathArg;

    /// A run argument: a `RunPath`, a number (a top-level run), or an array of numbers.
    #[wasm_bindgen(typescript_type = "RunPath | number | number[]")]
    pub type RunPathArg;
}

/// The model's block path, from whichever spelling the caller used — mirrors
/// `crate::address::path_of` exactly, over `BlockPath` instead of `ShapePath`.
pub(crate) fn block_path_of(argument: &BlockPathArg) -> Result<ooxml::BlockPath, JsValue> {
    let value: &JsValue = argument.as_ref();
    if let Some(index) = as_index(value) {
        return Ok(ooxml::BlockPath::from(index?));
    }
    if let Some(array) = value.dyn_ref::<js_sys::Array>() {
        return indices_of(array, "a paragraph address").map(ooxml::BlockPath::from);
    }
    read_block_path(value)
}

/// As [`block_path_of`], for [`RunPath`].
pub(crate) fn run_path_of(argument: &RunPathArg) -> Result<ooxml::RunPath, JsValue> {
    let value: &JsValue = argument.as_ref();
    if let Some(index) = as_index(value) {
        return Ok(ooxml::RunPath::from(index?));
    }
    if let Some(array) = value.dyn_ref::<js_sys::Array>() {
        return indices_of(array, "a run address").map(ooxml::RunPath::from);
    }
    read_run_path(value)
}

/// One property of a JavaScript object, or `None` if reading it failed or it was absent — the same
/// helper `crate::address` uses.
fn property(value: &JsValue, name: &str) -> Option<JsValue> {
    if !value.is_object() {
        return None;
    }
    js_sys::Reflect::get(value, &JsValue::from_str(name))
        .ok()
        .filter(|found| !found.is_undefined() && !found.is_null())
}

/// A `BlockPath`, rebuilt from the `indices` it publishes (read, not consumed — see
/// `crate::address`'s own doc comment for why).
fn read_block_path(value: &JsValue) -> Result<ooxml::BlockPath, JsValue> {
    let refuse =
        || invalid_argument("a paragraph address is a BlockPath, a number, or an array of numbers");
    let indices = property(value, "indices").ok_or_else(refuse)?;
    let array = js_sys::Uint32Array::new(&indices).to_vec();
    if array.is_empty() {
        return Err(refuse());
    }
    Ok(ooxml::BlockPath::from(array))
}

/// As [`read_block_path`], for [`RunPath`].
fn read_run_path(value: &JsValue) -> Result<ooxml::RunPath, JsValue> {
    let refuse =
        || invalid_argument("a run address is a RunPath, a number, or an array of numbers");
    let indices = property(value, "indices").ok_or_else(refuse)?;
    let array = js_sys::Uint32Array::new(&indices).to_vec();
    if array.is_empty() {
        return Err(refuse());
    }
    Ok(ooxml::RunPath::from(array))
}

/// The whole, non-negative indices an array of numbers holds.
fn indices_of(array: &js_sys::Array, what: &str) -> Result<Vec<u32>, JsValue> {
    if array.length() == 0 {
        return Err(invalid_argument(format!("{what} needs at least one index")));
    }
    let mut indices = Vec::with_capacity(array.length() as usize);
    for entry in array.iter() {
        match as_index(&entry) {
            Some(index) => indices.push(index?),
            None => {
                return Err(invalid_argument(format!(
                    "{what} given as an array holds whole, non-negative numbers"
                )))
            }
        }
    }
    Ok(indices)
}

/// A JavaScript number as the whole, non-negative index an address is made of — the same helper
/// `crate::address` uses.
fn as_index(value: &JsValue) -> Option<Result<u32, JsValue>> {
    let number = value.as_f64()?;
    if number.fract() != 0.0 || number < 0.0 || number > f64::from(u32::MAX) {
        return Some(Err(invalid_argument(format!(
            "an index is a whole number between 0 and {}, not {number}",
            u32::MAX
        ))));
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the range and integrality were just checked"
    )]
    Some(Ok(number as u32))
}

// ---------------------------------------------------------------------------------------------
// CellExtent-shaped pairs
// ---------------------------------------------------------------------------------------------

/// How many rows and columns a table (or a cell's span) has — reuses
/// [`crate::deck::CellExtent`], the same "two named getters, not a tuple" shape `Deck` already
/// uses for `mjx_pptx`'s own `(rows, columns)` pairs.
pub use crate::deck::CellExtent;

/// Which cell of a table a merge anchor resolves to — reuses [`crate::deck::CellAddress`].
pub use crate::deck::CellAddress;

// ---------------------------------------------------------------------------------------------
// Document
// ---------------------------------------------------------------------------------------------

/// An open Word document.
///
/// **Call `free()` when you are done with it.** See the module documentation.
#[wasm_bindgen]
#[derive(Debug)]
pub struct Document {
    inner: ooxml::Document,
}

#[wasm_bindgen]
impl Document {
    /// A new document with nothing in it beyond one empty paragraph and a body-level `w:sectPr`.
    #[wasm_bindgen(js_name = "blank")]
    pub fn blank(size: &PageSize) -> Result<Document, JsValue> {
        map_error(ooxml::Document::blank(size.0)).map(|inner| Self { inner })
    }

    /// Opens a document from the bytes of a `.docx`, `.docm`, `.dotx` or `.dotm`.
    #[wasm_bindgen(js_name = "open")]
    pub fn open(data: &[u8]) -> Result<Document, JsValue> {
        map_error(ooxml::Document::open(data)).map(|inner| Self { inner })
    }

    /// What this document's main part says it is.
    #[wasm_bindgen(js_name = "format")]
    pub fn format(&self) -> Result<Format, JsValue> {
        Format::from_model(self.inner.format())
    }

    /// The document as the bytes of a `.docx`, **validated first**.
    #[wasm_bindgen(js_name = "save")]
    pub fn save(&self) -> Result<Vec<u8>, JsValue> {
        map_error(self.inner.save())
    }

    /// The document as bytes, **without** the validation pass.
    #[wasm_bindgen(js_name = "saveUnchecked")]
    pub fn save_unchecked(&self) -> Result<Vec<u8>, JsValue> {
        map_error(self.inner.save_unchecked())
    }

    /// Checks the packaging invariants `save` enforces, without writing anything.
    #[wasm_bindgen(js_name = "validate")]
    pub fn validate(&self) -> Result<(), JsValue> {
        map_error(self.inner.validate())
    }

    /// The document's conformance class (`"strict"`/`"transitional"`), or `undefined` if absent.
    #[wasm_bindgen(js_name = "conformance")]
    pub fn conformance(&mut self) -> Result<Option<String>, JsValue> {
        map_error(self.inner.conformance()).map(|value| value.map(conformance_str))
    }

    /// Sets (or, given `undefined`, removes) `w:document/@conformance`. `value` is `"strict"` or
    /// `"transitional"`.
    #[wasm_bindgen(js_name = "setConformance")]
    pub fn set_conformance(&mut self, value: Option<String>) -> Result<(), JsValue> {
        let value = value
            .map(|value| conformance_from_str(&value))
            .transpose()?;
        map_error(self.inner.set_conformance(value))
    }

    // --- text: paragraphs and runs ---------------------------------------------------------------

    /// How many paragraphs the document body holds.
    #[wasm_bindgen(js_name = "paragraphCount")]
    pub fn paragraph_count(&mut self) -> Result<u32, JsValue> {
        map_error(self.inner.paragraph_count())
    }

    /// How many run-or-hyperlink slots the given paragraph holds at its own top level.
    #[wasm_bindgen(js_name = "runCount")]
    pub fn run_count(&mut self, paragraph: &BlockPathArg) -> Result<u32, JsValue> {
        map_error(self.inner.run_count(block_path_of(paragraph)?))
    }

    /// The whole text of a paragraph.
    #[wasm_bindgen(js_name = "paragraphText")]
    pub fn paragraph_text(&mut self, paragraph: &BlockPathArg) -> Result<String, JsValue> {
        map_error(self.inner.paragraph_text(block_path_of(paragraph)?))
    }

    /// The text of one run.
    #[wasm_bindgen(js_name = "runText")]
    pub fn run_text(
        &mut self,
        paragraph: &BlockPathArg,
        run: &RunPathArg,
    ) -> Result<String, JsValue> {
        map_error(
            self.inner
                .run_text(block_path_of(paragraph)?, run_path_of(run)?),
        )
    }

    /// Sets the text of one run.
    #[wasm_bindgen(js_name = "setRunText")]
    pub fn set_run_text(
        &mut self,
        paragraph: &BlockPathArg,
        run: &RunPathArg,
        text: &str,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .set_run_text(block_path_of(paragraph)?, run_path_of(run)?, text),
        )
    }

    /// Inserts a new, empty paragraph at `at`, shifting every paragraph at or after it later.
    #[wasm_bindgen(js_name = "insertParagraph")]
    pub fn insert_paragraph(&mut self, at: &BlockPathArg) -> Result<(), JsValue> {
        map_error(self.inner.insert_paragraph(block_path_of(at)?))
    }

    /// Appends a new, empty paragraph as the body's new last paragraph.
    #[wasm_bindgen(js_name = "appendParagraph")]
    pub fn append_paragraph(&mut self) -> Result<(), JsValue> {
        map_error(self.inner.append_paragraph())
    }

    /// Removes the paragraph at `at`.
    #[wasm_bindgen(js_name = "removeParagraph")]
    pub fn remove_paragraph(&mut self, at: &BlockPathArg) -> Result<(), JsValue> {
        map_error(self.inner.remove_paragraph(block_path_of(at)?))
    }

    /// Inserts a new run holding `text` at slot `at` within `paragraph`.
    #[wasm_bindgen(js_name = "insertRun")]
    pub fn insert_run(
        &mut self,
        paragraph: &BlockPathArg,
        at: &RunPathArg,
        text: &str,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .insert_run(block_path_of(paragraph)?, run_path_of(at)?, text),
        )
    }

    /// Appends a new run holding `text` as the paragraph's new last top-level run.
    #[wasm_bindgen(js_name = "appendRun")]
    pub fn append_run(&mut self, paragraph: &BlockPathArg, text: &str) -> Result<(), JsValue> {
        map_error(self.inner.append_run(block_path_of(paragraph)?, text))
    }

    /// Removes the run at `run` within `paragraph`.
    #[wasm_bindgen(js_name = "removeRun")]
    pub fn remove_run(
        &mut self,
        paragraph: &BlockPathArg,
        run: &RunPathArg,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .remove_run(block_path_of(paragraph)?, run_path_of(run)?),
        )
    }

    // --- effective properties -----------------------------------------------------------------

    /// The effective character formatting of one run.
    #[wasm_bindgen(js_name = "effectiveRunProperties")]
    pub fn effective_run_properties(
        &mut self,
        paragraph: &BlockPathArg,
        run: &RunPathArg,
    ) -> Result<EffectiveCharacterProperties, JsValue> {
        map_error(
            self.inner
                .effective_run_properties(block_path_of(paragraph)?, run_path_of(run)?),
        )
        .map(EffectiveCharacterProperties)
    }

    /// The effective paragraph layout of one paragraph.
    #[wasm_bindgen(js_name = "effectiveParagraphProperties")]
    pub fn effective_paragraph_properties(
        &mut self,
        paragraph: &BlockPathArg,
    ) -> Result<EffectiveParagraphProperties, JsValue> {
        map_error(
            self.inner
                .effective_paragraph_properties(block_path_of(paragraph)?),
        )
        .map(EffectiveParagraphProperties)
    }

    /// The effective fill of one table cell.
    #[wasm_bindgen(js_name = "effectiveCellFill")]
    pub fn effective_cell_fill(
        &mut self,
        table: u32,
        row: u32,
        column: u32,
    ) -> Result<Option<EffectiveShading>, JsValue> {
        map_error(self.inner.effective_cell_fill(table, row, column))
            .map(|value| value.map(EffectiveShading))
    }

    /// The effective border on one edge of one table cell.
    #[wasm_bindgen(js_name = "effectiveCellBorder")]
    pub fn effective_cell_border(
        &mut self,
        table: u32,
        row: u32,
        column: u32,
        edge: CellBorderEdge,
    ) -> Result<Option<EffectiveBorder>, JsValue> {
        map_error(
            self.inner
                .effective_cell_border(table, row, column, edge.into()),
        )
        .map(|value| value.map(EffectiveBorder))
    }

    /// The effective character formatting of a run addressed inside a table cell.
    #[wasm_bindgen(js_name = "effectiveCellRunProperties")]
    #[allow(clippy::too_many_arguments)]
    pub fn effective_cell_run_properties(
        &mut self,
        table: u32,
        row: u32,
        column: u32,
        paragraph: u32,
        run: u32,
    ) -> Result<EffectiveCharacterProperties, JsValue> {
        map_error(
            self.inner
                .effective_cell_run_properties(table, row, column, paragraph, run),
        )
        .map(EffectiveCharacterProperties)
    }

    // --- styles (read-only) --------------------------------------------------------------------

    /// Every `styleId` this document's `word/styles.xml` defines.
    #[wasm_bindgen(js_name = "styleIds")]
    pub fn style_ids(&mut self) -> Result<Vec<String>, JsValue> {
        map_error(self.inner.style_ids())
    }

    /// The display name of the style identified by `styleId`, or `undefined`.
    #[wasm_bindgen(js_name = "styleName")]
    pub fn style_name(&mut self, style_id: &str) -> Result<Option<String>, JsValue> {
        map_error(self.inner.style_name(style_id))
    }

    // --- numbering ------------------------------------------------------------------------------

    /// Attaches a paragraph to numbering instance `numberingId` at `level`.
    #[wasm_bindgen(js_name = "attachParagraphToList")]
    pub fn attach_paragraph_to_list(
        &mut self,
        paragraph: &BlockPathArg,
        numbering_id: f64,
        level: f64,
    ) -> Result<(), JsValue> {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "numbering and level ids are small signed integers in practice"
        )]
        map_error(self.inner.attach_paragraph_to_list(
            block_path_of(paragraph)?,
            numbering_id as i64,
            level as i64,
        ))
    }

    /// Removes a paragraph's own numbering reference, if it carries one.
    #[wasm_bindgen(js_name = "detachParagraphFromList")]
    pub fn detach_paragraph_from_list(&mut self, paragraph: &BlockPathArg) -> Result<(), JsValue> {
        map_error(
            self.inner
                .detach_paragraph_from_list(block_path_of(paragraph)?),
        )
    }

    // --- sections and headers/footers ------------------------------------------------------------

    /// How many sections the document has.
    #[wasm_bindgen(js_name = "sectionCount")]
    pub fn section_count(&mut self) -> Result<u32, JsValue> {
        map_error(self.inner.section_count())
    }

    /// Every section, in document order, with its own resolved page geometry.
    #[wasm_bindgen(js_name = "sections")]
    pub fn sections(&mut self) -> Result<Vec<SectionSummary>, JsValue> {
        map_error(self.inner.sections())
            .map(|sections| sections.into_iter().map(SectionSummary).collect())
    }

    /// Sets (or, given `undefined`, removes) the page size of the `w:sectPr` at `location`.
    #[wasm_bindgen(js_name = "setSectionPageSize")]
    pub fn set_section_page_size(
        &mut self,
        location: &SectionLocation,
        size: Option<PageSize>,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .set_section_page_size(location.0.clone(), size.map(|value| value.0)),
        )
    }

    /// Sets (or, given `undefined`, removes) the page margins of the `w:sectPr` at `location`.
    #[wasm_bindgen(js_name = "setSectionPageMargins")]
    pub fn set_section_page_margins(
        &mut self,
        location: &SectionLocation,
        margins: Option<PageMargins>,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .set_section_page_margins(location.0.clone(), margins.map(|value| value.0)),
        )
    }

    /// Removes the `w:sectPr` at `location`, if it carries one.
    #[wasm_bindgen(js_name = "removeSectionProperties")]
    pub fn remove_section_properties(&mut self, location: &SectionLocation) -> Result<(), JsValue> {
        map_error(self.inner.remove_section_properties(location.0.clone()))
    }

    /// Whether this document's sections use different headers/footers for even and odd pages.
    #[wasm_bindgen(js_name = "evenAndOddHeaders")]
    pub fn even_and_odd_headers(&mut self) -> Result<bool, JsValue> {
        map_error(self.inner.even_and_odd_headers())
    }

    /// The text of the header of `kind` that applies to `section`'s pages, or `undefined`.
    #[wasm_bindgen(js_name = "headerText")]
    pub fn header_text(
        &mut self,
        section: u32,
        kind: HeaderFooterType,
    ) -> Result<Option<String>, JsValue> {
        map_error(self.inner.header_text(section, kind.into()))
    }

    /// As `headerText`, for footers.
    #[wasm_bindgen(js_name = "footerText")]
    pub fn footer_text(
        &mut self,
        section: u32,
        kind: HeaderFooterType,
    ) -> Result<Option<String>, JsValue> {
        map_error(self.inner.footer_text(section, kind.into()))
    }

    /// Creates (or replaces) a header holding one paragraph of `text` for the section at
    /// `location`.
    #[wasm_bindgen(js_name = "setHeaderText")]
    pub fn set_header_text(
        &mut self,
        location: &SectionLocation,
        kind: HeaderFooterType,
        text: &str,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .set_header_text(location.0.clone(), kind.into(), text),
        )
    }

    /// As `setHeaderText`, for footers.
    #[wasm_bindgen(js_name = "setFooterText")]
    pub fn set_footer_text(
        &mut self,
        location: &SectionLocation,
        kind: HeaderFooterType,
        text: &str,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .set_footer_text(location.0.clone(), kind.into(), text),
        )
    }

    /// Removes the section at `location`'s own `kind` header reference, if it states one.
    #[wasm_bindgen(js_name = "removeHeader")]
    pub fn remove_header(
        &mut self,
        location: &SectionLocation,
        kind: HeaderFooterType,
    ) -> Result<(), JsValue> {
        map_error(self.inner.remove_header(location.0.clone(), kind.into()))
    }

    /// As `removeHeader`, for footers.
    #[wasm_bindgen(js_name = "removeFooter")]
    pub fn remove_footer(
        &mut self,
        location: &SectionLocation,
        kind: HeaderFooterType,
    ) -> Result<(), JsValue> {
        map_error(self.inner.remove_footer(location.0.clone(), kind.into()))
    }

    // --- tables ---------------------------------------------------------------------------------

    /// How many top-level tables the document body holds.
    #[wasm_bindgen(js_name = "tableCount")]
    pub fn table_count(&mut self) -> Result<u32, JsValue> {
        map_error(self.inner.table_count())
    }

    /// The shape of a table.
    #[wasm_bindgen(js_name = "tableDimensions")]
    pub fn table_dimensions(&mut self, table: u32) -> Result<CellExtent, JsValue> {
        map_error(self.inner.table_dimensions(table))
            .map(|(rows, columns)| CellExtent::new(rows, columns))
    }

    /// How many rows and columns a cell spans.
    #[wasm_bindgen(js_name = "cellSpan")]
    pub fn cell_span(&mut self, table: u32, row: u32, column: u32) -> Result<CellExtent, JsValue> {
        map_error(self.inner.cell_span(table, row, column))
            .map(|(rows, columns)| CellExtent::new(rows, columns))
    }

    /// Which cell actually renders at `(row, column)`, resolving any merge.
    #[wasm_bindgen(js_name = "mergedCellAnchor")]
    pub fn merged_cell_anchor(
        &mut self,
        table: u32,
        row: u32,
        column: u32,
    ) -> Result<CellAddress, JsValue> {
        map_error(self.inner.merged_cell_anchor(table, row, column))
            .map(|(row, column)| CellAddress::new(row, column))
    }

    /// Every grid discrepancy a table currently has.
    #[wasm_bindgen(js_name = "tableGridDiscrepancies")]
    pub fn table_grid_discrepancies(
        &mut self,
        table: u32,
    ) -> Result<Vec<GridDiscrepancy>, JsValue> {
        map_error(self.inner.table_grid_discrepancies(table))
            .map(|values| values.into_iter().map(GridDiscrepancy).collect())
    }

    /// The text of a table cell.
    #[wasm_bindgen(js_name = "cellText")]
    pub fn cell_text(&mut self, table: u32, row: u32, column: u32) -> Result<String, JsValue> {
        map_error(self.inner.cell_text(table, row, column))
    }

    /// Sets the text of a table cell.
    #[wasm_bindgen(js_name = "setCellText")]
    pub fn set_cell_text(
        &mut self,
        table: u32,
        row: u32,
        column: u32,
        text: &str,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_cell_text(table, row, column, text))
    }

    /// Sets (or, given `undefined`/`1`, removes) a cell's `w:gridSpan`.
    #[wasm_bindgen(js_name = "setCellSpan")]
    pub fn set_cell_span(
        &mut self,
        table: u32,
        row: u32,
        column: u32,
        span: Option<u32>,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_cell_span(table, row, column, span))
    }

    /// Sets (or, given `undefined`, removes) a cell's `w:vMerge`.
    #[wasm_bindgen(js_name = "setCellVerticalMerge")]
    pub fn set_cell_vertical_merge(
        &mut self,
        table: u32,
        row: u32,
        column: u32,
        kind: Option<MergedCellType>,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .set_cell_vertical_merge(table, row, column, kind.map(Into::into)),
        )
    }

    /// Appends a new `rows` x `columns` table, and returns its new index.
    #[wasm_bindgen(js_name = "appendTable")]
    pub fn append_table(&mut self, rows: u32, columns: u32) -> Result<u32, JsValue> {
        map_error(self.inner.append_table(rows, columns))
    }

    /// Removes the top-level table at `table`.
    #[wasm_bindgen(js_name = "removeTable")]
    pub fn remove_table(&mut self, table: u32) -> Result<(), JsValue> {
        map_error(self.inner.remove_table(table))
    }

    /// Inserts a row into `table` so it becomes row `at`.
    #[wasm_bindgen(js_name = "insertRow")]
    pub fn insert_row(&mut self, table: u32, at: u32) -> Result<(), JsValue> {
        map_error(self.inner.insert_row(table, at))
    }

    /// Removes row `at` from `table`.
    #[wasm_bindgen(js_name = "removeRow")]
    pub fn remove_row(&mut self, table: u32, at: u32) -> Result<(), JsValue> {
        map_error(self.inner.remove_row(table, at))
    }

    /// Inserts a column into `table` so it becomes column `at`.
    #[wasm_bindgen(js_name = "insertColumn")]
    pub fn insert_column(&mut self, table: u32, at: u32) -> Result<(), JsValue> {
        map_error(self.inner.insert_column(table, at))
    }

    /// Removes column `at` from `table`.
    #[wasm_bindgen(js_name = "removeColumn")]
    pub fn remove_column(&mut self, table: u32, at: u32) -> Result<(), JsValue> {
        map_error(self.inner.remove_column(table, at))
    }

    // --- fields -----------------------------------------------------------------------------------

    /// Every field a paragraph holds, at its own top level and nested, in document order.
    #[wasm_bindgen(js_name = "fields")]
    pub fn fields(&mut self, paragraph: &BlockPathArg) -> Result<Vec<Field>, JsValue> {
        map_error(self.inner.fields(block_path_of(paragraph)?))
            .map(|fields| fields.into_iter().map(Field).collect())
    }

    /// Sets a field's own instruction. `field` is the sequence of indices from `fields`'s own top
    /// level down to the target field: `[0]` for the paragraph's first field.
    #[wasm_bindgen(js_name = "setFieldInstruction")]
    pub fn set_field_instruction(
        &mut self,
        paragraph: &BlockPathArg,
        field: Vec<u32>,
        text: &str,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .set_field_instruction(block_path_of(paragraph)?, &field, text),
        )
    }

    /// Sets a field's own cached result. See `setFieldInstruction` for how `field` addresses one.
    #[wasm_bindgen(js_name = "setFieldCachedResultText")]
    pub fn set_field_cached_result_text(
        &mut self,
        paragraph: &BlockPathArg,
        field: Vec<u32>,
        text: &str,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .set_field_cached_result_text(block_path_of(paragraph)?, &field, text),
        )
    }

    // --- hyperlinks -----------------------------------------------------------------------------

    /// The click target of the hyperlink at slot `at` within `paragraph`, or `undefined`.
    #[wasm_bindgen(js_name = "hyperlinkTarget")]
    pub fn hyperlink_target(
        &mut self,
        paragraph: &BlockPathArg,
        at: &RunPathArg,
    ) -> Result<Option<HyperlinkTarget>, JsValue> {
        map_error(
            self.inner
                .hyperlink_target(block_path_of(paragraph)?, run_path_of(at)?),
        )
        .map(|value| value.map(HyperlinkTarget))
    }

    /// Inserts a new hyperlink wrapping one run of `text` at slot `at` within `paragraph`.
    #[wasm_bindgen(js_name = "insertHyperlink")]
    pub fn insert_hyperlink(
        &mut self,
        paragraph: &BlockPathArg,
        at: &RunPathArg,
        text: &str,
        target: &HyperlinkTarget,
    ) -> Result<(), JsValue> {
        map_error(self.inner.insert_hyperlink(
            block_path_of(paragraph)?,
            run_path_of(at)?,
            text,
            &target.0,
        ))
    }

    /// Removes the hyperlink at slot `at` within `paragraph`, together with every run it wraps.
    #[wasm_bindgen(js_name = "removeHyperlink")]
    pub fn remove_hyperlink(
        &mut self,
        paragraph: &BlockPathArg,
        at: &RunPathArg,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .remove_hyperlink(block_path_of(paragraph)?, run_path_of(at)?),
        )
    }

    // --- comments -------------------------------------------------------------------------------

    /// Every comment this document's `word/comments.xml` holds.
    #[wasm_bindgen(js_name = "comments")]
    pub fn comments(&mut self) -> Result<Vec<CommentSummary>, JsValue> {
        map_error(self.inner.comments())
            .map(|values| values.into_iter().map(CommentSummary).collect())
    }

    /// Adds a new comment on the whole paragraph at `paragraph`. Returns the comment's own id.
    #[wasm_bindgen(js_name = "addComment")]
    pub fn add_comment(
        &mut self,
        paragraph: &BlockPathArg,
        author: &str,
        initials: Option<String>,
        text: &str,
    ) -> Result<f64, JsValue> {
        let id = map_error(self.inner.add_comment(
            block_path_of(paragraph)?,
            author,
            initials.as_deref(),
            text,
        ))?;
        #[expect(clippy::cast_precision_loss, reason = "a comment id is far below 2^53")]
        Ok(id as f64)
    }

    /// Removes the comment with `id`.
    #[wasm_bindgen(js_name = "removeComment")]
    pub fn remove_comment(&mut self, id: f64) -> Result<(), JsValue> {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a comment id is a small integer"
        )]
        map_error(self.inner.remove_comment(id as i64))
    }

    /// The resolved text between a comment's own range markers, or `undefined`.
    #[wasm_bindgen(js_name = "commentRangeText")]
    pub fn comment_range_text(&mut self, id: f64) -> Result<Option<String>, JsValue> {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a comment id is a small integer"
        )]
        map_error(self.inner.comment_range_text(id as i64))
    }

    // --- footnotes, endnotes and revisions --------------------------------------------------------

    /// Every user-visible footnote this document holds.
    #[wasm_bindgen(js_name = "footnotes")]
    pub fn footnotes(&mut self) -> Result<Vec<NoteSummary>, JsValue> {
        map_error(self.inner.footnotes())
            .map(|values| values.into_iter().map(NoteSummary).collect())
    }

    /// Adds a new user footnote referenced from the end of `paragraph`. Returns its own id.
    #[wasm_bindgen(js_name = "addFootnote")]
    pub fn add_footnote(&mut self, paragraph: &BlockPathArg, text: &str) -> Result<f64, JsValue> {
        let id = map_error(self.inner.add_footnote(block_path_of(paragraph)?, text))?;
        #[expect(
            clippy::cast_precision_loss,
            reason = "a footnote id is far below 2^53"
        )]
        Ok(id as f64)
    }

    /// Removes the user footnote with `id`.
    #[wasm_bindgen(js_name = "removeFootnote")]
    pub fn remove_footnote(&mut self, id: f64) -> Result<(), JsValue> {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a footnote id is a small integer"
        )]
        map_error(self.inner.remove_footnote(id as i64))
    }

    /// As `footnotes`, for endnotes.
    #[wasm_bindgen(js_name = "endnotes")]
    pub fn endnotes(&mut self) -> Result<Vec<NoteSummary>, JsValue> {
        map_error(self.inner.endnotes()).map(|values| values.into_iter().map(NoteSummary).collect())
    }

    /// As `addFootnote`, for endnotes.
    #[wasm_bindgen(js_name = "addEndnote")]
    pub fn add_endnote(&mut self, paragraph: &BlockPathArg, text: &str) -> Result<f64, JsValue> {
        let id = map_error(self.inner.add_endnote(block_path_of(paragraph)?, text))?;
        #[expect(
            clippy::cast_precision_loss,
            reason = "an endnote id is far below 2^53"
        )]
        Ok(id as f64)
    }

    /// As `removeFootnote`, for endnotes.
    #[wasm_bindgen(js_name = "removeEndnote")]
    pub fn remove_endnote(&mut self, id: f64) -> Result<(), JsValue> {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "an endnote id is a small integer"
        )]
        map_error(self.inner.remove_endnote(id as i64))
    }

    /// Every tracked-change marker the document body holds.
    #[wasm_bindgen(js_name = "revisions")]
    pub fn revisions(&mut self) -> Result<Vec<RevisionInfo>, JsValue> {
        map_error(self.inner.revisions())
            .map(|values| values.into_iter().map(RevisionInfo).collect())
    }

    /// The document body's text with tracked insertions kept and tracked deletions dropped.
    #[wasm_bindgen(js_name = "textWithRevisionsAccepted")]
    pub fn text_with_revisions_accepted(&mut self) -> Result<String, JsValue> {
        map_error(self.inner.text_with_revisions_accepted())
    }

    /// As `textWithRevisionsAccepted`, the rejected-text counterpart.
    #[wasm_bindgen(js_name = "textWithRevisionsRejected")]
    pub fn text_with_revisions_rejected(&mut self) -> Result<String, JsValue> {
        map_error(self.inner.text_with_revisions_rejected())
    }

    // --- drawings ---------------------------------------------------------------------------------

    /// Adds an inline picture as a new run at the end of `paragraph`. Returns its `wp:docPr` id.
    #[wasm_bindgen(js_name = "addInlinePicture")]
    #[allow(clippy::too_many_arguments)]
    pub fn add_inline_picture(
        &mut self,
        paragraph: &BlockPathArg,
        image_bytes: Vec<u8>,
        content_type: &str,
        extension: &str,
        width_emu: f64,
        height_emu: f64,
        name: &str,
    ) -> Result<u32, JsValue> {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "an EMU extent is far below i64::MAX in practice"
        )]
        map_error(self.inner.add_inline_picture(
            block_path_of(paragraph)?,
            image_bytes,
            content_type,
            extension,
            width_emu as i64,
            height_emu as i64,
            name,
        ))
    }

    /// Removes the drawing whose `wp:docPr@id` is `docPrId`. Returns whether one was removed.
    #[wasm_bindgen(js_name = "removeDrawing")]
    pub fn remove_drawing(&mut self, doc_pr_id: u32) -> Result<bool, JsValue> {
        map_error(self.inner.remove_drawing(doc_pr_id))
    }
}

/// `w:document/@conformance`'s two wire values, as the strings this binding raises/accepts.
fn conformance_str(value: ooxml::ConformanceClass) -> String {
    match value {
        ooxml::ConformanceClass::Strict => "strict",
        ooxml::ConformanceClass::Transitional => "transitional",
    }
    .to_owned()
}

fn conformance_from_str(value: &str) -> Result<ooxml::ConformanceClass, JsValue> {
    match value {
        "strict" => Ok(ooxml::ConformanceClass::Strict),
        "transitional" => Ok(ooxml::ConformanceClass::Transitional),
        other => Err(invalid_argument(format!(
            "conformance is \"strict\" or \"transitional\", not {other:?}"
        ))),
    }
}
