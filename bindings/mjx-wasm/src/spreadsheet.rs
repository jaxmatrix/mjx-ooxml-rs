//! Excel-specific value classes: cell values, the block a range read answers with, the tab and
//! feature reports, and the four cell-format specs — the TypeScript sibling of `mjx-python`'s own
//! `spreadsheet.rs`.
//!
//! # A cell is a value, not an object graph
//!
//! [`CellBlock`] is the one class here that a caller holds a lot of at once, and it is deliberately
//! **flat**: the values live in Rust as a packed row-major vector and cross into JavaScript either
//! one at a time ([`CellBlock::value`]) or all at once as an array of arrays
//! ([`CellBlock::rows`]). Constructing a hundred thousand small wasm objects — each of which the
//! caller would then have to `free()` — is the thing the Excel binding most has to avoid, and the
//! range-shaped surface upstream exists so that the *crossing* is one call; this class exists so the
//! *conversion* can be one call too, into JavaScript's own values and not into wasm handles.
//!
//! `rows()` answers `null`, `number`, `string` and `boolean`, because that is what a caller
//! iterating a table wants. It cannot distinguish a text cell from an error cell, which both arrive
//! as a `string`; [`CellBlock::kinds`] is the disambiguator, built only when asked.
//!
//! # The one shape that differs from Python's
//!
//! The four cell-format specs are **fluent builders** here (`new FontProperties().withBold(true)`)
//! where the Python binding takes keyword arguments. JavaScript has no keyword arguments, and a
//! constructor with fourteen positional optionals is not an API anyone can read — the same forced
//! divergence the crate documentation records for a range argument becoming two numbers.

use wasm_bindgen::prelude::*;

use mjx_ooxml as ooxml;

use crate::enums::{
    ApplyFlag, BorderStyle, CalculationMode, ColorSchemeSlot, FormatAspect, FormatLayer,
    GeometrySource, GridAnomalyKind, HyperlinkKind, PartKind, ReferenceMode, ResizingBehavior,
    SheetKind, SpreadsheetFontScheme, SpreadsheetPatternType, StyleIndexSource, TotalsRowFunction,
    UnderlineType,
};
use crate::errors::map_error;

value_class! {
    /// One cell's value, as the file states it — **stored, not displayed**.
    CellData(ooxml::CellData), derive(PartialEq);

    /// One entry of a `Workbook.writeCells` batch: where, and what.
    ///
    /// **Consumed by `writeCells`**: an array of exported objects crosses the boundary by value, so
    /// a `CellWrite` handed to that call is moved into it and must not be `free()`d afterwards. See
    /// [`Workbook::write_cells`](crate::workbook::Workbook::write_cells).
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

    /// One cell comment, resolved across both of the parts it lives in.
    SheetCommentInfo(ooxml::SheetCommentInfo), derive(PartialEq, Eq);

    /// The `v:shape` that draws one comment's pop-up box.
    CommentBoxInfo(ooxml::CommentBoxInfo), derive(PartialEq, Eq);

    /// One table on a sheet, resolved to its part.
    SheetTableInfo(ooxml::SheetTableInfo), derive(PartialEq, Eq);

    /// One column of a `SheetTableInfo`.
    SheetTableColumnInfo(ooxml::SheetTableColumnInfo), derive(PartialEq, Eq);

    /// One sheet's drawing part, and what is anchored in it.
    SheetDrawingInfo(ooxml::SheetDrawingInfo), derive(PartialEq, Eq);

    /// One anchored object on a sheet.
    SheetDrawingObjectInfo(ooxml::SheetDrawingObjectInfo), derive(PartialEq, Eq);

    /// Where an anchor puts its object, in EMU, and what the answer rests on.
    AnchorBoundsInfo(ooxml::AnchorBoundsInfo), derive(Copy, PartialEq);

    /// What one anchor did when rows or columns moved under it.
    AnchorShiftInfo(ooxml::AnchorShiftInfo), derive(Copy, PartialEq, Eq);

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

    /// One `x:xf`: the four resource indices, the `cellStyleXfs` record beneath it, the
    /// quote-prefix flag and the six `apply*` flags — all twelve readable, as in Rust.
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

/// One cell as JavaScript's own types: `null`, `number`, `string` or `boolean`.
///
/// An error cell arrives as its code (`"#DIV/0!"`), which a text cell holding that same text would
/// too — `CellBlock.kinds` is how the two are told apart when it matters.
fn native(value: &ooxml::CellData) -> JsValue {
    match value {
        ooxml::CellData::Blank => JsValue::NULL,
        ooxml::CellData::Number(number) => JsValue::from_f64(*number),
        ooxml::CellData::Text(text) | ooxml::CellData::Error(text) => JsValue::from_str(text),
        ooxml::CellData::Boolean(value) => JsValue::from_bool(*value),
    }
}

#[wasm_bindgen]
impl CellData {
    /// `"blank"`, `"number"`, `"text"`, `"boolean"` or `"error"`.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> String {
        (kind_of(&self.0)).to_owned()
    }

    /// Whether the cell is not populated, or holds no value element.
    #[wasm_bindgen(getter, js_name = "isBlank")]
    pub fn is_blank(&self) -> bool {
        self.0.is_blank()
    }

    /// The number this cell holds, or `None` for every other kind.
    #[wasm_bindgen(getter, js_name = "number")]
    pub fn number(&self) -> Option<f64> {
        self.0.number()
    }

    /// The string this cell holds, or `None` for every other kind. An error code is **not** a
    /// string here.
    #[wasm_bindgen(getter, js_name = "text")]
    pub fn text(&self) -> Option<String> {
        (self.0.text()).map(str::to_owned)
    }

    /// The boolean this cell holds, or `None` for every other kind.
    #[wasm_bindgen(getter, js_name = "boolean")]
    pub fn boolean(&self) -> Option<bool> {
        self.0.boolean()
    }

    /// The error code this cell holds, or `None` for every other kind.
    #[wasm_bindgen(getter, js_name = "errorCode")]
    pub fn error_code(&self) -> Option<String> {
        (self.0.error_code()).map(str::to_owned)
    }

    /// The value as one of JavaScript's own types: `null`, `number`, `string` or `boolean`.
    #[wasm_bindgen(getter, js_name = "value")]
    pub fn value(&self) -> JsValue {
        native(&self.0)
    }
}

#[wasm_bindgen]
impl CellWrite {
    /// Remove the value, keeping the cell — and therefore its style.
    #[wasm_bindgen(js_name = "blank")]
    pub fn blank(reference: &str) -> Self {
        Self(ooxml::CellWrite::new(reference, ooxml::CellInput::Blank))
    }

    /// A number. `nan` and the infinities are refused: SpreadsheetML has no spelling for them.
    #[wasm_bindgen(js_name = "number")]
    pub fn number(reference: &str, value: f64) -> Self {
        Self(ooxml::CellWrite::new(
            reference,
            ooxml::CellInput::Number(value),
        ))
    }

    /// A string interned into `xl/sharedStrings.xml` — **what Excel itself writes**, and what to
    /// reach for when the same text appears in many cells.
    #[wasm_bindgen(js_name = "sharedText")]
    pub fn shared_text(reference: &str, text: &str) -> Self {
        Self(ooxml::CellWrite::new(
            reference,
            ooxml::CellInput::SharedText(text.to_owned()),
        ))
    }

    /// A string stored in the cell itself as an `inlineStr`. No other part is touched.
    #[wasm_bindgen(js_name = "inlineText")]
    pub fn inline_text(reference: &str, text: &str) -> Self {
        Self(ooxml::CellWrite::new(
            reference,
            ooxml::CellInput::InlineText(text.to_owned()),
        ))
    }

    /// A boolean, written `1` or `0`.
    #[wasm_bindgen(js_name = "boolean")]
    pub fn boolean(reference: &str, value: bool) -> Self {
        Self(ooxml::CellWrite::new(
            reference,
            ooxml::CellInput::Boolean(value),
        ))
    }

    /// An error code — `"#DIV/0!"`, `"#N/A"`. Carried verbatim.
    #[wasm_bindgen(js_name = "error")]
    pub fn error(reference: &str, code: &str) -> Self {
        Self(ooxml::CellWrite::new(
            reference,
            ooxml::CellInput::Error(code.to_owned()),
        ))
    }

    /// The cell this write addresses, in A1 text.
    #[wasm_bindgen(getter, js_name = "reference")]
    pub fn reference(&self) -> String {
        self.0.reference.clone()
    }

    /// `"blank"`, `"number"`, `"shared_text"`, `"inline_text"`, `"boolean"` or `"error"`.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> String {
        (match &self.0.value {
            ooxml::CellInput::Blank => "blank",
            ooxml::CellInput::Number(_) => "number",
            ooxml::CellInput::SharedText(_) => "shared_text",
            ooxml::CellInput::InlineText(_) => "inline_text",
            ooxml::CellInput::Boolean(_) => "boolean",
            ooxml::CellInput::Error(_) => "error",
        })
        .to_owned()
    }
}

#[wasm_bindgen]
impl CellBlock {
    /// The zero-based sheet row the block's first row is: `0` is the row a file spells `1`.
    #[wasm_bindgen(getter, js_name = "firstRow")]
    pub fn first_row(&self) -> u32 {
        self.0.first_row()
    }

    /// The zero-based sheet column the block's first column is: `0` is `A`.
    #[wasm_bindgen(getter, js_name = "firstColumn")]
    pub fn first_column(&self) -> u32 {
        self.0.first_column()
    }

    /// How many rows the block covers.
    #[wasm_bindgen(getter, js_name = "rowCount")]
    pub fn row_count(&self) -> u32 {
        self.0.row_count()
    }

    /// How many columns the block covers.
    #[wasm_bindgen(getter, js_name = "columnCount")]
    pub fn column_count(&self) -> u32 {
        self.0.column_count()
    }

    /// Whether the block covers no cells at all.
    #[wasm_bindgen(getter, js_name = "isEmpty")]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The A1 text of the range this block covers, or `None` when it covers nothing.
    #[wasm_bindgen(getter, js_name = "range")]
    pub fn range(&self) -> Option<String> {
        self.0.range()
    }

    /// The value at `row`/`column`, **as offsets into the block**.
    #[wasm_bindgen(js_name = "value")]
    pub fn value(&self, row: u32, column: u32) -> Result<CellData, JsValue> {
        map_error(
            self.0
                .value(row, column)
                .map(|value| CellData(value.clone())),
        )
    }

    /// The formula text at `row`/`column`, or `None` when that cell carries none. Exactly as the
    /// file wrote it, never expanded and never evaluated.
    #[wasm_bindgen(js_name = "formula")]
    pub fn formula(&self, row: u32, column: u32) -> Result<Option<String>, JsValue> {
        map_error(
            self.0
                .formula(row, column)
                .map(|text| text.map(str::to_owned)),
        )
    }

    /// The whole block as rows of JavaScript's own types — `null`, `number`, `string`, `boolean` — top to
    /// bottom, left to right.
    ///
    /// **The shape to reach for.** One call converts the whole block; `value` per cell converts one
    /// at a time, which is cheap for a handful and needless for a table.
    #[wasm_bindgen(js_name = "rows")]
    pub fn rows(&self) -> js_sys::Array {
        let outer = js_sys::Array::new();
        for row in self.0.clone().into_rows() {
            let inner = js_sys::Array::new();
            for value in &row {
                inner.push(&native(value));
            }
            outer.push(&inner);
        }
        outer
    }

    /// The whole block as rows of kind names — `"blank"`, `"number"`, `"text"`, `"boolean"`,
    /// `"error"`.
    ///
    /// The disambiguator for [`rows`](Self::rows), which cannot tell a text cell from an error cell
    /// because both arrive as a `string`. Built only when asked.
    #[wasm_bindgen(js_name = "kinds")]
    pub fn kinds(&self) -> js_sys::Array {
        let outer = js_sys::Array::new();
        for row in self.0.clone().into_rows() {
            let inner = js_sys::Array::new();
            for value in &row {
                inner.push(&JsValue::from_str(kind_of(value)));
            }
            outer.push(&inner);
        }
        outer
    }
}

// ---------------------------------------------------------------------------------------------
// Tabs and workbook metadata
// ---------------------------------------------------------------------------------------------

#[wasm_bindgen]
impl SheetSummary {
    /// The tab's name (`@name`), with XML entities decoded.
    #[wasm_bindgen(getter, js_name = "name")]
    pub fn name(&self) -> String {
        self.0.name.clone()
    }

    /// `@sheetId`, or `None` when the attribute is absent or is not an `xsd:unsignedInt`.
    #[wasm_bindgen(getter, js_name = "sheetId")]
    pub fn sheet_id(&self) -> Option<u32> {
        self.0.sheet_id
    }

    /// Whether the tab is shown in a consumer's tab strip.
    #[wasm_bindgen(getter, js_name = "isVisible")]
    pub fn is_visible(&self) -> bool {
        self.0.is_visible
    }

    /// Which of the three sheet kinds the tab's target is, or `None`.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> Result<Option<SheetKind>, JsValue> {
        self.0.kind.map(SheetKind::from_model).transpose()
    }

    /// The part the tab's `r:id` reaches, or `None`.
    #[wasm_bindgen(getter, js_name = "part")]
    pub fn part(&self) -> Option<String> {
        self.0.part.clone()
    }
}

#[wasm_bindgen]
impl WorkbookWindowInfo {
    /// `@activeTab` — the index of the tab that was selected.
    #[wasm_bindgen(getter, js_name = "activeTabIndex")]
    pub fn active_tab_index(&self) -> u32 {
        self.0.active_tab_index
    }

    /// `@firstSheet` — the index of the leftmost tab shown in the tab strip.
    #[wasm_bindgen(getter, js_name = "firstVisibleTabIndex")]
    pub fn first_visible_tab_index(&self) -> u32 {
        self.0.first_visible_tab_index
    }

    /// `@xWindow`, or `None` if the file wrote neither coordinate.
    #[wasm_bindgen(getter, js_name = "windowLeft")]
    pub fn window_left(&self) -> Option<i32> {
        self.0.window_left
    }

    /// `@yWindow`, on the same terms.
    #[wasm_bindgen(getter, js_name = "windowTop")]
    pub fn window_top(&self) -> Option<i32> {
        self.0.window_top
    }

    /// `@windowWidth`, or `None` if the file wrote neither dimension.
    #[wasm_bindgen(getter, js_name = "windowWidth")]
    pub fn window_width(&self) -> Option<u32> {
        self.0.window_width
    }

    /// `@windowHeight`, on the same terms.
    #[wasm_bindgen(getter, js_name = "windowHeight")]
    pub fn window_height(&self) -> Option<u32> {
        self.0.window_height
    }

    /// `@tabRatio`, in thousandths.
    #[wasm_bindgen(getter, js_name = "tabStripRatio")]
    pub fn tab_strip_ratio(&self) -> u32 {
        self.0.tab_strip_ratio
    }

    /// `@showSheetTabs`.
    #[wasm_bindgen(getter, js_name = "showSheetTabs")]
    pub fn show_sheet_tabs(&self) -> bool {
        self.0.show_sheet_tabs
    }
}

#[wasm_bindgen]
impl DefinedName {
    /// `@name`, as a consumer's name manager shows it.
    #[wasm_bindgen(getter, js_name = "name")]
    pub fn name(&self) -> String {
        self.0.name.clone()
    }

    /// The index of the tab this name is local to, or `None` for a workbook-scoped name.
    #[wasm_bindgen(getter, js_name = "sheet")]
    pub fn sheet(&self) -> Option<u32> {
        self.0.sheet
    }

    /// The name of the tab `sheet` indexes, when there is one there.
    #[wasm_bindgen(getter, js_name = "sheetName")]
    pub fn sheet_name(&self) -> Option<String> {
        self.0.sheet_name.clone()
    }

    /// The formula this name stands for, **as text**. Nothing here parses or evaluates it.
    #[wasm_bindgen(getter, js_name = "definition")]
    pub fn definition(&self) -> String {
        self.0.definition.clone()
    }

    /// `@hidden`.
    #[wasm_bindgen(getter, js_name = "hidden")]
    pub fn hidden(&self) -> bool {
        self.0.hidden
    }
}

#[wasm_bindgen]
impl CalculationSettings {
    /// `@calcId`, or `None`. Never derived and never bumped.
    #[wasm_bindgen(getter, js_name = "engineId")]
    pub fn engine_id(&self) -> Option<u32> {
        self.0.engine_id
    }

    /// `@calcMode`.
    #[wasm_bindgen(getter, js_name = "mode")]
    pub fn mode(&self) -> Result<CalculationMode, JsValue> {
        CalculationMode::from_model(self.0.mode)
    }

    /// `@refMode` — A1 or R1C1.
    #[wasm_bindgen(getter, js_name = "referenceMode")]
    pub fn reference_mode(&self) -> Result<ReferenceMode, JsValue> {
        ReferenceMode::from_model(self.0.reference_mode)
    }

    /// `@iterate`.
    #[wasm_bindgen(getter, js_name = "iteratesOnCircularReferences")]
    pub fn iterates_on_circular_references(&self) -> bool {
        self.0.iterates_on_circular_references
    }

    /// `@iterateCount`.
    #[wasm_bindgen(getter, js_name = "iterationLimit")]
    pub fn iteration_limit(&self) -> u32 {
        self.0.iteration_limit
    }

    /// `@iterateDelta`.
    #[wasm_bindgen(getter, js_name = "iterationConvergenceDelta")]
    pub fn iteration_convergence_delta(&self) -> f64 {
        self.0.iteration_convergence_delta
    }

    /// `@fullCalcOnLoad`.
    #[wasm_bindgen(getter, js_name = "fullCalculationOnLoad")]
    pub fn full_calculation_on_load(&self) -> bool {
        self.0.full_calculation_on_load
    }
}

// ---------------------------------------------------------------------------------------------
// Hyperlinks, tables and grid anomalies
// ---------------------------------------------------------------------------------------------

#[wasm_bindgen]
impl SheetCommentInfo {
    /// The cell the comment is attached to, as A1 text.
    #[wasm_bindgen(getter, js_name = "cell")]
    pub fn cell(&self) -> String {
        self.0.cell.clone()
    }

    /// `@authorId` — an index into the part's author list, not a name.
    #[wasm_bindgen(getter, js_name = "authorIndex")]
    pub fn author_index(&self) -> u32 {
        self.0.author_index
    }

    /// The name at that index, or `undefined`.
    #[wasm_bindgen(getter, js_name = "author")]
    pub fn author(&self) -> Option<String> {
        self.0.author.clone()
    }

    /// The displayed text: the plain `t`, then each formatted run's `t`, concatenated.
    #[wasm_bindgen(getter, js_name = "text")]
    pub fn text(&self) -> String {
        self.0.text.clone()
    }

    /// `@shapeId`, when the file states one.
    #[wasm_bindgen(getter, js_name = "shapeId")]
    pub fn shape_id(&self) -> Option<u32> {
        self.0.shape_id
    }

    /// The box that draws it, or `undefined`.
    #[wasm_bindgen(getter, js_name = "commentBox")]
    pub fn comment_box(&self) -> Option<CommentBoxInfo> {
        self.0.comment_box.clone().map(CommentBoxInfo)
    }
}

#[wasm_bindgen]
impl CommentBoxInfo {
    /// The shape's own `@id`, as the file wrote it.
    #[wasm_bindgen(getter, js_name = "identifier")]
    pub fn identifier(&self) -> Option<String> {
        self.0.identifier.clone()
    }

    /// `@o:spid`, the application's identifier for the shape.
    #[wasm_bindgen(getter, js_name = "applicationIdentifier")]
    pub fn application_identifier(&self) -> Option<String> {
        self.0.application_identifier.clone()
    }

    /// Whether the box is showing without the pointer over the cell.
    #[wasm_bindgen(getter, js_name = "isVisible")]
    pub fn is_visible(&self) -> bool {
        self.0.is_visible
    }

    /// `x:ClientData/x:Anchor` exactly as written. Never decoded.
    #[wasm_bindgen(getter, js_name = "anchorText")]
    pub fn anchor_text(&self) -> Option<String> {
        self.0.anchor_text.clone()
    }

    /// `x:ClientData/x:Row` — the zero-based row the box states.
    #[wasm_bindgen(getter, js_name = "row")]
    pub fn row(&self) -> Option<u32> {
        self.0.row
    }

    /// `x:ClientData/x:Column` — the zero-based column.
    #[wasm_bindgen(getter, js_name = "column")]
    pub fn column(&self) -> Option<u32> {
        self.0.column
    }

    /// The shape's CSS2 `@style`, verbatim.
    #[wasm_bindgen(getter, js_name = "style")]
    pub fn style(&self) -> Option<String> {
        self.0.style.clone()
    }
}

#[wasm_bindgen]
impl SheetHyperlinkInfo {
    /// `@ref` — the range the link covers, as A1 text.
    #[wasm_bindgen(getter, js_name = "range")]
    pub fn range(&self) -> String {
        self.0.range.clone()
    }

    /// Which of `CT_Hyperlink`'s four shapes this entry is.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> Result<HyperlinkKind, JsValue> {
        HyperlinkKind::from_model(self.0.kind)
    }

    /// `@r:id`, or `None` when the entry names no relationship.
    #[wasm_bindgen(getter, js_name = "relationshipId")]
    pub fn relationship_id(&self) -> Option<String> {
        self.0.relationship_id.clone()
    }

    /// The relationship's `Target`, exactly as the `.rels` wrote it.
    #[wasm_bindgen(getter, js_name = "target")]
    pub fn target(&self) -> Option<String> {
        self.0.target.clone()
    }

    /// Whether that relationship's `TargetMode` is `External`.
    #[wasm_bindgen(getter, js_name = "targetIsExternal")]
    pub fn target_is_external(&self) -> Option<bool> {
        self.0.target_is_external
    }

    /// `@location` — a cell reference or a defined name inside this workbook.
    #[wasm_bindgen(getter, js_name = "location")]
    pub fn location(&self) -> Option<String> {
        self.0.location.clone()
    }

    /// `@tooltip`.
    #[wasm_bindgen(getter, js_name = "tooltip")]
    pub fn tooltip(&self) -> Option<String> {
        self.0.tooltip.clone()
    }

    /// `@display` — never kept in step with the cell's own value.
    #[wasm_bindgen(getter, js_name = "display")]
    pub fn display(&self) -> Option<String> {
        self.0.display.clone()
    }
}

#[wasm_bindgen]
impl SheetDrawingInfo {
    /// The part the anchors live in.
    #[wasm_bindgen(getter, js_name = "part")]
    pub fn part(&self) -> String {
        self.0.part.clone()
    }

    /// The `x:drawing@r:id` the sheet reached it through.
    #[wasm_bindgen(getter, js_name = "relationshipId")]
    pub fn relationship_id(&self) -> String {
        self.0.relationship_id.clone()
    }

    /// Every anchored object, in paint order.
    #[wasm_bindgen(getter, js_name = "objects")]
    pub fn objects(&self) -> Vec<SheetDrawingObjectInfo> {
        self.0
            .objects
            .iter()
            .cloned()
            .map(SheetDrawingObjectInfo)
            .collect()
    }
}

#[wasm_bindgen]
impl SheetDrawingObjectInfo {
    /// The object's position in the drawing part, which is also its paint order.
    #[wasm_bindgen(getter, js_name = "index")]
    pub fn index(&self) -> u32 {
        self.0.index
    }

    /// Which of the three anchor elements pins it.
    #[wasm_bindgen(getter, js_name = "anchor")]
    pub fn anchor(&self) -> String {
        self.0.anchor.clone()
    }

    /// Which kind of object it holds, or `undefined` for an anchor holding none.
    #[wasm_bindgen(getter, js_name = "object")]
    pub fn object(&self) -> Option<String> {
        self.0.object.clone()
    }

    /// What the anchor promises to do when the cells under it move.
    #[wasm_bindgen(getter, js_name = "resizing")]
    pub fn resizing(&self) -> Result<ResizingBehavior, JsValue> {
        ResizingBehavior::from_model(self.0.resizing)
    }

    /// The object's `cNvPr@id`, or `undefined`.
    #[wasm_bindgen(getter, js_name = "id")]
    pub fn id(&self) -> Option<u32> {
        self.0.id
    }

    /// The object's `cNvPr@name`, or `undefined`.
    #[wasm_bindgen(getter, js_name = "name")]
    pub fn name(&self) -> Option<String> {
        self.0.name.clone()
    }

    /// The image part a picture shows, or `undefined` for every other object kind.
    #[wasm_bindgen(getter, js_name = "image")]
    pub fn image(&self) -> Option<String> {
        self.0.image.clone()
    }

    /// Whether the object prints with the sheet — which **defaults to true**.
    #[wasm_bindgen(getter, js_name = "printsWithSheet")]
    pub fn prints_with_sheet(&self) -> bool {
        self.0.prints_with_sheet
    }
}

#[wasm_bindgen]
impl AnchorBoundsInfo {
    /// The object's left edge, in EMU from the sheet origin.
    #[wasm_bindgen(getter, js_name = "xEmu")]
    pub fn x_emu(&self) -> i64 {
        self.0.x_emu
    }

    /// The object's top edge, in EMU from the sheet origin.
    #[wasm_bindgen(getter, js_name = "yEmu")]
    pub fn y_emu(&self) -> i64 {
        self.0.y_emu
    }

    /// The object's width, in EMU.
    #[wasm_bindgen(getter, js_name = "widthEmu")]
    pub fn width_emu(&self) -> i64 {
        self.0.width_emu
    }

    /// The object's height, in EMU.
    #[wasm_bindgen(getter, js_name = "heightEmu")]
    pub fn height_emu(&self) -> i64 {
        self.0.height_emu
    }

    /// Where the vertical half of this answer came from.
    #[wasm_bindgen(getter, js_name = "rowSource")]
    pub fn row_source(&self) -> Result<GeometrySource, JsValue> {
        GeometrySource::from_model(self.0.row_source)
    }

    /// Where the horizontal half came from.
    #[wasm_bindgen(getter, js_name = "columnSource")]
    pub fn column_source(&self) -> Result<GeometrySource, JsValue> {
        GeometrySource::from_model(self.0.column_source)
    }

    /// The maximum digit width, in pixels, the horizontal half was computed through.
    #[wasm_bindgen(getter, js_name = "maximumDigitWidthPixels")]
    pub fn maximum_digit_width_pixels(&self) -> f64 {
        self.0.maximum_digit_width_pixels
    }

    /// The pixels per inch that width was stated at.
    #[wasm_bindgen(getter, js_name = "pixelsPerInch")]
    pub fn pixels_per_inch(&self) -> f64 {
        self.0.pixels_per_inch
    }
}

#[wasm_bindgen]
impl AnchorShiftInfo {
    /// The anchor's position in the drawing part.
    #[wasm_bindgen(getter, js_name = "index")]
    pub fn index(&self) -> u32 {
        self.0.index
    }

    /// What the anchor promises to do when the cells under it move.
    #[wasm_bindgen(getter, js_name = "promise")]
    pub fn promise(&self) -> Result<ResizingBehavior, JsValue> {
        ResizingBehavior::from_model(self.0.promise)
    }

    /// Whether the object's top-left corner moved.
    #[wasm_bindgen(getter, js_name = "moved")]
    pub fn moved(&self) -> bool {
        self.0.moved
    }

    /// Whether the object's extent changed.
    #[wasm_bindgen(getter, js_name = "resized")]
    pub fn resized(&self) -> bool {
        self.0.resized
    }

    /// Whether the markers alone could keep the anchor's own promise.
    #[wasm_bindgen(getter, js_name = "promiseKept")]
    pub fn promise_kept(&self) -> bool {
        self.0.promise_kept
    }
}

#[wasm_bindgen]
impl SheetTableInfo {
    /// The part the table lives in.
    #[wasm_bindgen(getter, js_name = "part")]
    pub fn part(&self) -> String {
        self.0.part.clone()
    }

    /// The `tablePart@r:id` the sheet reached it through.
    #[wasm_bindgen(getter, js_name = "relationshipId")]
    pub fn relationship_id(&self) -> String {
        self.0.relationship_id.clone()
    }

    /// `@id` — workbook-unique, never renumbered here.
    #[wasm_bindgen(getter, js_name = "id")]
    pub fn id(&self) -> u32 {
        self.0.id
    }

    /// `@displayName` — what a formula references the table by.
    #[wasm_bindgen(getter, js_name = "displayName")]
    pub fn display_name(&self) -> String {
        self.0.display_name.clone()
    }

    /// `@name`, or `None` when the table writes none.
    #[wasm_bindgen(getter, js_name = "name")]
    pub fn name(&self) -> Option<String> {
        self.0.name.clone()
    }

    /// `@ref` as A1 text, header and totals rows included.
    #[wasm_bindgen(getter, js_name = "range")]
    pub fn range(&self) -> String {
        self.0.range.clone()
    }

    /// `@headerRowCount`.
    #[wasm_bindgen(getter, js_name = "headerRowCount")]
    pub fn header_row_count(&self) -> u32 {
        self.0.header_row_count
    }

    /// `@totalsRowCount`.
    #[wasm_bindgen(getter, js_name = "totalsRowCount")]
    pub fn totals_row_count(&self) -> u32 {
        self.0.totals_row_count
    }

    /// How many rows of `range` are data rows, or `None` when the counts do not fit inside it.
    #[wasm_bindgen(getter, js_name = "dataRowCount")]
    pub fn data_row_count(&self) -> Option<u32> {
        self.0.data_row_count
    }

    /// `tableStyleInfo@name`, or `None`.
    #[wasm_bindgen(getter, js_name = "styleName")]
    pub fn style_name(&self) -> Option<String> {
        self.0.style_name.clone()
    }

    /// The columns, left to right.
    #[wasm_bindgen(getter, js_name = "columns")]
    pub fn columns(&self) -> Vec<SheetTableColumnInfo> {
        self.0
            .columns
            .iter()
            .cloned()
            .map(SheetTableColumnInfo)
            .collect()
    }
}

#[wasm_bindgen]
impl SheetTableColumnInfo {
    /// `@id`, unique within the table.
    #[wasm_bindgen(getter, js_name = "id")]
    pub fn id(&self) -> u32 {
        self.0.id
    }

    /// `@name` — the heading text.
    #[wasm_bindgen(getter, js_name = "name")]
    pub fn name(&self) -> String {
        self.0.name.clone()
    }

    /// `@totalsRowFunction`, or `None`.
    #[wasm_bindgen(getter, js_name = "totalsRowFunction")]
    pub fn totals_row_function(&self) -> Result<Option<TotalsRowFunction>, JsValue> {
        self.0
            .totals_row_function
            .map(TotalsRowFunction::from_model)
            .transpose()
    }

    /// `@totalsRowLabel`.
    #[wasm_bindgen(getter, js_name = "totalsRowLabel")]
    pub fn totals_row_label(&self) -> Option<String> {
        self.0.totals_row_label.clone()
    }

    /// `x:calculatedColumnFormula`, exactly as the file wrote it.
    #[wasm_bindgen(getter, js_name = "calculatedColumnFormula")]
    pub fn calculated_column_formula(&self) -> Option<String> {
        self.0.calculated_column_formula.clone()
    }

    /// `x:totalsRowFormula`, on the same terms.
    #[wasm_bindgen(getter, js_name = "totalsRowFormula")]
    pub fn totals_row_formula(&self) -> Option<String> {
        self.0.totals_row_formula.clone()
    }
}

#[wasm_bindgen]
impl GridAnomalyInfo {
    /// Which finding this is.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> Result<GridAnomalyKind, JsValue> {
        GridAnomalyKind::from_model(self.0.kind)
    }

    /// The range at fault, as A1 text.
    #[wasm_bindgen(getter, js_name = "range")]
    pub fn range(&self) -> Option<String> {
        self.0.range.clone()
    }

    /// The second range of an overlapping pair.
    #[wasm_bindgen(getter, js_name = "otherRange")]
    pub fn other_range(&self) -> Option<String> {
        self.0.other_range.clone()
    }

    /// The cell at fault, as A1 text.
    #[wasm_bindgen(getter, js_name = "cell")]
    pub fn cell(&self) -> Option<String> {
        self.0.cell.clone()
    }

    /// The first column of a `col` run at fault, zero-based.
    #[wasm_bindgen(getter, js_name = "firstColumn")]
    pub fn first_column(&self) -> Option<u32> {
        self.0.first_column
    }

    /// The last column of that run, zero-based.
    #[wasm_bindgen(getter, js_name = "lastColumn")]
    pub fn last_column(&self) -> Option<u32> {
        self.0.last_column
    }

    /// The second run's first column, for an overlap.
    #[wasm_bindgen(getter, js_name = "otherFirstColumn")]
    pub fn other_first_column(&self) -> Option<u32> {
        self.0.other_first_column
    }

    /// The second run's last column, for an overlap.
    #[wasm_bindgen(getter, js_name = "otherLastColumn")]
    pub fn other_last_column(&self) -> Option<u32> {
        self.0.other_last_column
    }

    /// What the file declared — a merge count, or an outline maximum.
    #[wasm_bindgen(getter, js_name = "declared")]
    pub fn declared(&self) -> Option<u32> {
        self.0.declared
    }

    /// What is actually there.
    #[wasm_bindgen(getter, js_name = "actual")]
    pub fn actual(&self) -> Option<u32> {
        self.0.actual
    }

    /// The index of the `mergeCell` whose `@ref` could not be read.
    #[wasm_bindgen(getter, js_name = "index")]
    pub fn index(&self) -> Option<u32> {
        self.0.index
    }
}

// ---------------------------------------------------------------------------------------------
// The preserved clusters
// ---------------------------------------------------------------------------------------------

#[wasm_bindgen]
impl PreservedPartsSummary {
    /// `x:pivotTableDefinition` parts.
    #[wasm_bindgen(getter, js_name = "pivotTables")]
    pub fn pivot_tables(&self) -> Vec<String> {
        self.0.pivot_tables.clone()
    }

    /// `x:pivotCacheDefinition` parts.
    #[wasm_bindgen(getter, js_name = "pivotCacheDefinitions")]
    pub fn pivot_cache_definitions(&self) -> Vec<String> {
        self.0.pivot_cache_definitions.clone()
    }

    /// `x:pivotCacheRecords` parts.
    #[wasm_bindgen(getter, js_name = "pivotCacheRecords")]
    pub fn pivot_cache_records(&self) -> Vec<String> {
        self.0.pivot_cache_records.clone()
    }

    /// `x:externalLink` parts.
    #[wasm_bindgen(getter, js_name = "externalLinks")]
    pub fn external_links(&self) -> Vec<String> {
        self.0.external_links.clone()
    }

    /// The `x:connections` part, if there is one.
    #[wasm_bindgen(getter, js_name = "connections")]
    pub fn connections(&self) -> Option<String> {
        self.0.connections.clone()
    }

    /// `x:queryTable` parts.
    #[wasm_bindgen(getter, js_name = "queryTables")]
    pub fn query_tables(&self) -> Vec<String> {
        self.0.query_tables.clone()
    }

    /// The `x:metadata` part, if there is one.
    #[wasm_bindgen(getter, js_name = "metadata")]
    pub fn metadata(&self) -> Option<String> {
        self.0.metadata.clone()
    }

    /// The `x:volTypes` part, if there is one.
    #[wasm_bindgen(getter, js_name = "volatileDependencies")]
    pub fn volatile_dependencies(&self) -> Option<String> {
        self.0.volatile_dependencies.clone()
    }

    /// The `x:MapInfo` part, if there is one.
    #[wasm_bindgen(getter, js_name = "customXmlMappings")]
    pub fn custom_xml_mappings(&self) -> Option<String> {
        self.0.custom_xml_mappings.clone()
    }

    /// `x:singleXmlCell` table-definition parts.
    #[wasm_bindgen(getter, js_name = "singleCellTableDefinitions")]
    pub fn single_cell_table_definitions(&self) -> Vec<String> {
        self.0.single_cell_table_definitions.clone()
    }

    /// The `x:headers` revision-headers part, if there is one.
    #[wasm_bindgen(getter, js_name = "revisionHeaders")]
    pub fn revision_headers(&self) -> Option<String> {
        self.0.revision_headers.clone()
    }

    /// `x:revisions` revision-log parts.
    #[wasm_bindgen(getter, js_name = "revisionLogs")]
    pub fn revision_logs(&self) -> Vec<String> {
        self.0.revision_logs.clone()
    }

    /// The shared-workbook user-data part, if there is one.
    #[wasm_bindgen(getter, js_name = "sharedWorkbookUserData")]
    pub fn shared_workbook_user_data(&self) -> Option<String> {
        self.0.shared_workbook_user_data.clone()
    }

    /// Custom Property parts.
    #[wasm_bindgen(getter, js_name = "customProperties")]
    pub fn custom_properties(&self) -> Vec<String> {
        self.0.custom_properties.clone()
    }

    /// Every preserved part with the kind it is.
    #[wasm_bindgen(js_name = "all")]
    pub fn all(&self) -> Vec<PreservedPart> {
        self.0.all().into_iter().map(PreservedPart).collect()
    }

    /// Whether the workbook carries none of these at all.
    #[wasm_bindgen(getter, js_name = "isEmpty")]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[wasm_bindgen]
impl PreservedPart {
    /// Which SpreadsheetML part this is.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> Result<PartKind, JsValue> {
        PartKind::from_model(self.0.kind)
    }

    /// Its part name.
    #[wasm_bindgen(getter, js_name = "part")]
    pub fn part(&self) -> String {
        self.0.part.clone()
    }
}

#[wasm_bindgen]
impl SheetPivotTableInfo {
    /// The `x:pivotTableDefinition` part.
    #[wasm_bindgen(getter, js_name = "part")]
    pub fn part(&self) -> String {
        self.0.part.clone()
    }

    /// The index of the tab the table sits on.
    #[wasm_bindgen(getter, js_name = "sheet")]
    pub fn sheet(&self) -> u32 {
        self.0.sheet
    }

    /// That tab's name.
    #[wasm_bindgen(getter, js_name = "sheetName")]
    pub fn sheet_name(&self) -> String {
        self.0.sheet_name.clone()
    }

    /// That tab's own part.
    #[wasm_bindgen(getter, js_name = "sheetPart")]
    pub fn sheet_part(&self) -> String {
        self.0.sheet_part.clone()
    }

    /// The `r:id` the sheet reached the table through.
    #[wasm_bindgen(getter, js_name = "relationshipId")]
    pub fn relationship_id(&self) -> String {
        self.0.relationship_id.clone()
    }

    /// `@name`.
    #[wasm_bindgen(getter, js_name = "name")]
    pub fn name(&self) -> String {
        self.0.name.clone()
    }

    /// `@cacheId`.
    #[wasm_bindgen(getter, js_name = "cacheId")]
    pub fn cache_id(&self) -> u32 {
        self.0.cache_id
    }

    /// `@dataCaption`.
    #[wasm_bindgen(getter, js_name = "dataCaption")]
    pub fn data_caption(&self) -> String {
        self.0.data_caption.clone()
    }

    /// `x:location/@ref`, exactly as the file wrote it.
    #[wasm_bindgen(getter, js_name = "location")]
    pub fn location(&self) -> String {
        self.0.location.clone()
    }

    /// The cache-definition part `@cacheId` resolves to.
    #[wasm_bindgen(getter, js_name = "cacheDefinitionPart")]
    pub fn cache_definition_part(&self) -> Option<String> {
        self.0.cache_definition_part.clone()
    }

    /// The cache-records part that definition names.
    #[wasm_bindgen(getter, js_name = "cacheRecordsPart")]
    pub fn cache_records_part(&self) -> Option<String> {
        self.0.cache_records_part.clone()
    }

    /// `pivotCacheDefinition@recordCount` — the producer's cached number.
    #[wasm_bindgen(getter, js_name = "cacheRecordCount")]
    pub fn cache_record_count(&self) -> Option<u32> {
        self.0.cache_record_count
    }

    /// `pivotCacheDefinition@refreshedBy`.
    #[wasm_bindgen(getter, js_name = "cacheRefreshedBy")]
    pub fn cache_refreshed_by(&self) -> Option<String> {
        self.0.cache_refreshed_by.clone()
    }
}

#[wasm_bindgen]
impl WorkbookExternalLinkInfo {
    /// The `x:externalLink` part.
    #[wasm_bindgen(getter, js_name = "part")]
    pub fn part(&self) -> String {
        self.0.part.clone()
    }

    /// The `r:id` `xl/workbook.xml` reached it through.
    #[wasm_bindgen(getter, js_name = "relationshipId")]
    pub fn relationship_id(&self) -> Option<String> {
        self.0.relationship_id.clone()
    }

    /// The position in `x:externalReferences` this link is.
    #[wasm_bindgen(getter, js_name = "referenceIndex")]
    pub fn reference_index(&self) -> Option<u32> {
        self.0.reference_index
    }

    /// The relationship target, carried verbatim and never resolved or fetched.
    #[wasm_bindgen(getter, js_name = "target")]
    pub fn target(&self) -> Option<String> {
        self.0.target.clone()
    }

    /// `"workbook"`, `"dde"`, `"oleObject"` or `"none"`.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> String {
        (self.0.kind).to_owned()
    }

    /// The names of the linked workbook's sheets, as this file cached them.
    #[wasm_bindgen(getter, js_name = "sheetNames")]
    pub fn sheet_names(&self) -> Vec<String> {
        self.0.sheet_names.clone()
    }
}

#[wasm_bindgen]
impl WorkbookConnectionInfo {
    /// The `x:connections` part.
    #[wasm_bindgen(getter, js_name = "part")]
    pub fn part(&self) -> String {
        self.0.part.clone()
    }

    /// `@id` — what a query table's `@connectionId` names.
    #[wasm_bindgen(getter, js_name = "id")]
    pub fn id(&self) -> u32 {
        self.0.id
    }

    /// `@name`.
    #[wasm_bindgen(getter, js_name = "name")]
    pub fn name(&self) -> Option<String> {
        self.0.name.clone()
    }

    /// `@description`.
    #[wasm_bindgen(getter, js_name = "description")]
    pub fn description(&self) -> Option<String> {
        self.0.description.clone()
    }

    /// `@sourceFile` — carried verbatim, never resolved or opened.
    #[wasm_bindgen(getter, js_name = "sourceFile")]
    pub fn source_file(&self) -> Option<String> {
        self.0.source_file.clone()
    }

    /// `@odcFile`, on the same terms.
    #[wasm_bindgen(getter, js_name = "odcFile")]
    pub fn odc_file(&self) -> Option<String> {
        self.0.odc_file.clone()
    }
}

#[wasm_bindgen]
impl SheetQueryTableInfo {
    /// The `x:queryTable` part.
    #[wasm_bindgen(getter, js_name = "part")]
    pub fn part(&self) -> String {
        self.0.part.clone()
    }

    /// The index of the tab the query table sits on.
    #[wasm_bindgen(getter, js_name = "sheet")]
    pub fn sheet(&self) -> u32 {
        self.0.sheet
    }

    /// That tab's name.
    #[wasm_bindgen(getter, js_name = "sheetName")]
    pub fn sheet_name(&self) -> String {
        self.0.sheet_name.clone()
    }

    /// That tab's own part.
    #[wasm_bindgen(getter, js_name = "sheetPart")]
    pub fn sheet_part(&self) -> String {
        self.0.sheet_part.clone()
    }

    /// The `r:id` the sheet reached it through.
    #[wasm_bindgen(getter, js_name = "relationshipId")]
    pub fn relationship_id(&self) -> String {
        self.0.relationship_id.clone()
    }

    /// `@name`.
    #[wasm_bindgen(getter, js_name = "name")]
    pub fn name(&self) -> String {
        self.0.name.clone()
    }

    /// `@connectionId`.
    #[wasm_bindgen(getter, js_name = "connectionId")]
    pub fn connection_id(&self) -> u32 {
        self.0.connection_id
    }

    /// `@refreshOnLoad`.
    #[wasm_bindgen(getter, js_name = "refreshOnLoad")]
    pub fn refresh_on_load(&self) -> bool {
        self.0.refresh_on_load
    }
}

#[wasm_bindgen]
impl WorkbookXmlMapsInfo {
    /// The `x:MapInfo` part.
    #[wasm_bindgen(getter, js_name = "part")]
    pub fn part(&self) -> String {
        self.0.part.clone()
    }

    /// `@SelectionNamespaces`.
    #[wasm_bindgen(getter, js_name = "selectionNamespaces")]
    pub fn selection_namespaces(&self) -> String {
        self.0.selection_namespaces.clone()
    }

    /// The schema ids declared in the part.
    #[wasm_bindgen(getter, js_name = "schemaIds")]
    pub fn schema_ids(&self) -> Vec<String> {
        self.0.schema_ids.clone()
    }

    /// The maps themselves.
    #[wasm_bindgen(getter, js_name = "maps")]
    pub fn maps(&self) -> Vec<XmlMapInfo> {
        self.0.maps.iter().cloned().map(XmlMapInfo).collect()
    }
}

#[wasm_bindgen]
impl XmlMapInfo {
    /// `@ID`.
    #[wasm_bindgen(getter, js_name = "id")]
    pub fn id(&self) -> u32 {
        self.0.id
    }

    /// `@Name`.
    #[wasm_bindgen(getter, js_name = "name")]
    pub fn name(&self) -> String {
        self.0.name.clone()
    }

    /// `@RootElement`.
    #[wasm_bindgen(getter, js_name = "rootElement")]
    pub fn root_element(&self) -> String {
        self.0.root_element.clone()
    }

    /// `@SchemaID`.
    #[wasm_bindgen(getter, js_name = "schemaId")]
    pub fn schema_id(&self) -> String {
        self.0.schema_id.clone()
    }
}

#[wasm_bindgen]
impl WorkbookRevisionState {
    /// Whether `xl/workbook.xml` declares the workbook shared.
    #[wasm_bindgen(getter, js_name = "isShared")]
    pub fn is_shared(&self) -> bool {
        self.0.is_shared
    }

    /// The `x:headers` part, if there is one.
    #[wasm_bindgen(getter, js_name = "headersPart")]
    pub fn headers_part(&self) -> Option<String> {
        self.0.headers_part.clone()
    }

    /// `headers@guid`.
    #[wasm_bindgen(getter, js_name = "guid")]
    pub fn guid(&self) -> Option<String> {
        self.0.guid.clone()
    }

    /// The revision-log parts, in the order the headers name them.
    #[wasm_bindgen(getter, js_name = "logParts")]
    pub fn log_parts(&self) -> Vec<String> {
        self.0.log_parts.clone()
    }

    /// The shared-workbook user-data part, if there is one.
    #[wasm_bindgen(getter, js_name = "userDataPart")]
    pub fn user_data_part(&self) -> Option<String> {
        self.0.user_data_part.clone()
    }

    /// The recorded editing sessions.
    #[wasm_bindgen(getter, js_name = "sessions")]
    pub fn sessions(&self) -> Vec<RevisionSessionInfo> {
        self.0
            .sessions
            .iter()
            .cloned()
            .map(RevisionSessionInfo)
            .collect()
    }

    /// The recorded users.
    #[wasm_bindgen(getter, js_name = "users")]
    pub fn users(&self) -> Vec<SharedWorkbookUserInfo> {
        self.0
            .users
            .iter()
            .cloned()
            .map(SharedWorkbookUserInfo)
            .collect()
    }
}

#[wasm_bindgen]
impl RevisionSessionInfo {
    /// `@guid`.
    #[wasm_bindgen(getter, js_name = "guid")]
    pub fn guid(&self) -> String {
        self.0.guid.clone()
    }

    /// `@dateTime`, exactly as the file wrote it.
    #[wasm_bindgen(getter, js_name = "dateTime")]
    pub fn date_time(&self) -> String {
        self.0.date_time.clone()
    }

    /// `@userName`.
    #[wasm_bindgen(getter, js_name = "userName")]
    pub fn user_name(&self) -> String {
        self.0.user_name.clone()
    }
}

#[wasm_bindgen]
impl SharedWorkbookUserInfo {
    /// `@id`.
    #[wasm_bindgen(getter, js_name = "id")]
    pub fn id(&self) -> i32 {
        self.0.id
    }

    /// `@name`.
    #[wasm_bindgen(getter, js_name = "name")]
    pub fn name(&self) -> String {
        self.0.name.clone()
    }

    /// `@dateTime`, exactly as the file wrote it.
    #[wasm_bindgen(getter, js_name = "dateTime")]
    pub fn date_time(&self) -> String {
        self.0.date_time.clone()
    }
}

// ---------------------------------------------------------------------------------------------
// The cell-format authoring vocabulary
// ---------------------------------------------------------------------------------------------

#[wasm_bindgen]
impl Color {
    /// An opaque sRGB colour, written `rgb="FFRRGGBB"`. `hex` is the six-digit `RRGGBB` form; the
    /// opaque alpha is prefixed for you, because a six-digit `@rgb` is read as transparent.
    #[wasm_bindgen(js_name = "fromOpaqueRgb")]
    pub fn from_opaque_rgb(hex: &str) -> Self {
        Self(ooxml::Color::from_opaque_rgb(hex))
    }

    /// A theme colour by index, optionally tinted towards white (positive) or black (negative).
    ///
    /// The index is a position in `theme1.xml`'s colour scheme, which is what a *file* states. An
    /// author should reach for `fromThemeSlot`, which names the slot instead of numbering it.
    #[wasm_bindgen(js_name = "fromTheme")]
    pub fn from_theme(index: u32, tint: Option<f64>) -> Self {
        Self(ooxml::Color::from_theme(index, tint))
    }

    /// A theme colour by **slot**, optionally tinted — `fromTheme` with the position spelled out,
    /// and the constructor an author should reach for.
    #[wasm_bindgen(js_name = "fromThemeSlot")]
    pub fn from_theme_slot(slot: ColorSchemeSlot, tint: Option<f64>) -> Self {
        Self(ooxml::Color::from_theme_slot(slot.into(), tint))
    }

    /// The system foreground/background colour, whatever that is at render time.
    #[wasm_bindgen(js_name = "automatic")]
    pub fn automatic() -> Self {
        Self(ooxml::Color {
            automatic: Some(true),
            ..ooxml::Color::default()
        })
    }

    /// A row of the legacy 56-entry indexed palette.
    #[wasm_bindgen(js_name = "indexed")]
    pub fn indexed(index: u32) -> Self {
        Self(ooxml::Color {
            indexed: Some(index),
            ..ooxml::Color::default()
        })
    }

    /// `@auto`.
    #[wasm_bindgen(getter, js_name = "isAutomatic")]
    pub fn is_automatic(&self) -> Option<bool> {
        self.0.automatic
    }

    /// `@indexed`.
    #[wasm_bindgen(getter, js_name = "indexedValue")]
    pub fn indexed_value(&self) -> Option<u32> {
        self.0.indexed
    }

    /// `@rgb` — eight hex digits, **alpha first**, as the file's own text.
    #[wasm_bindgen(getter, js_name = "rgb")]
    pub fn rgb(&self) -> Option<String> {
        self.0.rgb.clone()
    }

    /// `@theme` — a zero-based index into the theme's colour scheme.
    #[wasm_bindgen(getter, js_name = "theme")]
    pub fn theme(&self) -> Option<u32> {
        self.0.theme
    }

    /// `@tint`, in `-1.0 ..= 1.0`.
    #[wasm_bindgen(getter, js_name = "tint")]
    pub fn tint(&self) -> Option<f64> {
        self.0.tint
    }
}

#[wasm_bindgen]
impl FontProperties {
    /// A font that states nothing. Every `with…` below adds one property and returns a **new**
    /// value, so a specification can be built once and appended many times.
    ///
    /// A property never set writes **no element at all**, which is a third state beside
    /// present-and-false: a cell that inherits its boldness is not a cell that switches it off.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self(ooxml::FontProperties::default())
    }

    /// `rFont`/`name` — the typeface name.
    #[wasm_bindgen(js_name = "withFontName")]
    pub fn with_font_name(&self, font_name: String) -> Self {
        let mut next = self.0.clone();
        next.font_name = Some(font_name);
        Self(next)
    }

    /// `b`.
    #[wasm_bindgen(js_name = "withBold")]
    pub fn with_bold(&self, bold: bool) -> Self {
        let mut next = self.0.clone();
        next.bold = Some(bold);
        Self(next)
    }

    /// `i`.
    #[wasm_bindgen(js_name = "withItalic")]
    pub fn with_italic(&self, italic: bool) -> Self {
        let mut next = self.0.clone();
        next.italic = Some(italic);
        Self(next)
    }

    /// `strike`.
    #[wasm_bindgen(js_name = "withStrikethrough")]
    pub fn with_strikethrough(&self, strikethrough: bool) -> Self {
        let mut next = self.0.clone();
        next.strikethrough = Some(strikethrough);
        Self(next)
    }

    /// `sz` — the point size.
    #[wasm_bindgen(js_name = "withSizePoints")]
    pub fn with_size_points(&self, points: f64) -> Self {
        let mut next = self.0.clone();
        next.size_in_points = Some(points);
        Self(next)
    }

    /// `color`.
    #[wasm_bindgen(js_name = "withColor")]
    pub fn with_color(&self, color: &Color) -> Self {
        let mut next = self.0.clone();
        next.color = Some(color.0.clone());
        Self(next)
    }

    /// `u`.
    #[wasm_bindgen(js_name = "withUnderline")]
    pub fn with_underline(&self, underline: UnderlineType) -> Self {
        let mut next = self.0.clone();
        next.underline = Some(underline.into());
        Self(next)
    }

    /// `scheme` — whether this is the theme's major or minor font rather than a named one.
    #[wasm_bindgen(js_name = "withScheme")]
    pub fn with_scheme(&self, scheme: SpreadsheetFontScheme) -> Self {
        let mut next = self.0.clone();
        next.scheme = Some(scheme.into());
        Self(next)
    }

    /// `family` — the legacy classification number (0 unknown, 1 roman, 2 swiss, 3 modern,
    /// 4 script, 5 decorative).
    #[wasm_bindgen(js_name = "withFamily")]
    pub fn with_family(&self, family: i32) -> Self {
        let mut next = self.0.clone();
        next.family = Some(i64::from(family));
        Self(next)
    }

    /// `charset` — the legacy Windows character-set number.
    #[wasm_bindgen(js_name = "withCharacterSet")]
    pub fn with_character_set(&self, character_set: i32) -> Self {
        let mut next = self.0.clone();
        next.character_set = Some(i64::from(character_set));
        Self(next)
    }

    /// `rFont`/`name` — the typeface name.
    #[wasm_bindgen(getter, js_name = "fontName")]
    pub fn font_name(&self) -> Option<String> {
        self.0.font_name.clone()
    }

    /// `b`.
    #[wasm_bindgen(getter, js_name = "bold")]
    pub fn bold(&self) -> Option<bool> {
        self.0.bold
    }

    /// `i`.
    #[wasm_bindgen(getter, js_name = "italic")]
    pub fn italic(&self) -> Option<bool> {
        self.0.italic
    }

    /// `strike`.
    #[wasm_bindgen(getter, js_name = "strikethrough")]
    pub fn strikethrough(&self) -> Option<bool> {
        self.0.strikethrough
    }

    /// `sz` — the point size.
    #[wasm_bindgen(getter, js_name = "sizeInPoints")]
    pub fn size_in_points(&self) -> Option<f64> {
        self.0.size_in_points
    }

    /// `color`.
    #[wasm_bindgen(getter, js_name = "color")]
    pub fn color(&self) -> Option<Color> {
        self.0.color.clone().map(Color)
    }

    /// `u`.
    #[wasm_bindgen(getter, js_name = "underline")]
    pub fn underline(&self) -> Result<Option<UnderlineType>, JsValue> {
        self.0.underline.map(UnderlineType::from_model).transpose()
    }

    /// `scheme`.
    #[wasm_bindgen(getter, js_name = "scheme")]
    pub fn scheme(&self) -> Result<Option<SpreadsheetFontScheme>, JsValue> {
        self.0
            .scheme
            .map(SpreadsheetFontScheme::from_model)
            .transpose()
    }
}

impl Default for FontProperties {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl PatternFillSpec {
    /// A fill that states nothing.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self(ooxml::PatternFillSpec {
            pattern: None,
            foreground: None,
            background: None,
        })
    }

    /// A solid fill in one opaque colour — the shape a caller filling a cell almost always wants.
    #[wasm_bindgen(js_name = "solid")]
    pub fn solid(hex: &str) -> Self {
        Self(ooxml::PatternFillSpec::solid(hex))
    }

    /// A solid fill in one of the **workbook's own theme colours**, optionally tinted. Reach for
    /// this one unless the colour itself is the point: a hex literal survives into a document whose
    /// owner has rebranded everything around it.
    #[wasm_bindgen(js_name = "solidFromTheme")]
    pub fn solid_from_theme(slot: ColorSchemeSlot, tint: Option<f64>) -> Self {
        Self(ooxml::PatternFillSpec::solid_from_theme(slot.into(), tint))
    }

    /// `@patternType`.
    #[wasm_bindgen(js_name = "withPattern")]
    pub fn with_pattern(&self, pattern: SpreadsheetPatternType) -> Self {
        let mut next = self.0.clone();
        next.pattern = Some(pattern.into());
        Self(next)
    }

    /// `fgColor` — the colour a **solid** fill actually shows.
    #[wasm_bindgen(js_name = "withForeground")]
    pub fn with_foreground(&self, color: &Color) -> Self {
        let mut next = self.0.clone();
        next.foreground = Some(color.0.clone());
        Self(next)
    }

    /// `bgColor`.
    #[wasm_bindgen(js_name = "withBackground")]
    pub fn with_background(&self, color: &Color) -> Self {
        let mut next = self.0.clone();
        next.background = Some(color.0.clone());
        Self(next)
    }

    /// `@patternType`.
    #[wasm_bindgen(getter, js_name = "pattern")]
    pub fn pattern(&self) -> Result<Option<SpreadsheetPatternType>, JsValue> {
        self.0
            .pattern
            .map(SpreadsheetPatternType::from_model)
            .transpose()
    }

    /// `fgColor`.
    #[wasm_bindgen(getter, js_name = "foreground")]
    pub fn foreground(&self) -> Option<Color> {
        self.0.foreground.clone().map(Color)
    }

    /// `bgColor`.
    #[wasm_bindgen(getter, js_name = "background")]
    pub fn background(&self) -> Option<Color> {
        self.0.background.clone().map(Color)
    }
}

impl Default for PatternFillSpec {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl BorderEdgeSpec {
    /// An edge that states nothing.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self(ooxml::BorderEdgeSpec {
            style: None,
            color: None,
        })
    }

    /// An edge in `style`, with no colour of its own — which means *automatic*.
    #[wasm_bindgen(js_name = "styled")]
    pub fn styled(style: BorderStyle) -> Self {
        Self(ooxml::BorderEdgeSpec::styled(style.into()))
    }

    /// `@style`.
    #[wasm_bindgen(js_name = "withStyle")]
    pub fn with_style(&self, style: BorderStyle) -> Self {
        let mut next = self.0.clone();
        next.style = Some(style.into());
        Self(next)
    }

    /// `color`. Absent means *automatic*.
    #[wasm_bindgen(js_name = "withColor")]
    pub fn with_color(&self, color: &Color) -> Self {
        let mut next = self.0.clone();
        next.color = Some(color.0.clone());
        Self(next)
    }

    /// `@style`.
    #[wasm_bindgen(getter, js_name = "style")]
    pub fn style(&self) -> Result<Option<BorderStyle>, JsValue> {
        self.0.style.map(BorderStyle::from_model).transpose()
    }

    /// `color`.
    #[wasm_bindgen(getter, js_name = "color")]
    pub fn color(&self) -> Option<Color> {
        self.0.color.clone().map(Color)
    }
}

impl Default for BorderEdgeSpec {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl BorderSpec {
    /// A border that states no edge at all.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self(ooxml::BorderSpec::default())
    }

    /// `left` — the physical left edge.
    #[wasm_bindgen(js_name = "withLeft")]
    pub fn with_left(&self, edge: &BorderEdgeSpec) -> Self {
        let mut next = self.0.clone();
        next.left = Some(edge.0.clone());
        Self(next)
    }

    /// `right` — the physical right edge.
    #[wasm_bindgen(js_name = "withRight")]
    pub fn with_right(&self, edge: &BorderEdgeSpec) -> Self {
        let mut next = self.0.clone();
        next.right = Some(edge.0.clone());
        Self(next)
    }

    /// `top`.
    #[wasm_bindgen(js_name = "withTop")]
    pub fn with_top(&self, edge: &BorderEdgeSpec) -> Self {
        let mut next = self.0.clone();
        next.top = Some(edge.0.clone());
        Self(next)
    }

    /// `bottom`.
    #[wasm_bindgen(js_name = "withBottom")]
    pub fn with_bottom(&self, edge: &BorderEdgeSpec) -> Self {
        let mut next = self.0.clone();
        next.bottom = Some(edge.0.clone());
        Self(next)
    }

    /// `diagonal` — the line's style and colour. **Which** diagonals are drawn is
    /// `withDiagonalUp`/`withDiagonalDown`.
    #[wasm_bindgen(js_name = "withDiagonal")]
    pub fn with_diagonal(&self, edge: &BorderEdgeSpec) -> Self {
        let mut next = self.0.clone();
        next.diagonal = Some(edge.0.clone());
        Self(next)
    }

    /// `start` — the leading edge in the reading direction.
    #[wasm_bindgen(js_name = "withLeading")]
    pub fn with_leading(&self, edge: &BorderEdgeSpec) -> Self {
        let mut next = self.0.clone();
        next.leading = Some(edge.0.clone());
        Self(next)
    }

    /// `end` — the trailing edge in the reading direction.
    #[wasm_bindgen(js_name = "withTrailing")]
    pub fn with_trailing(&self, edge: &BorderEdgeSpec) -> Self {
        let mut next = self.0.clone();
        next.trailing = Some(edge.0.clone());
        Self(next)
    }

    /// `vertical` — the inner vertical border of a range.
    #[wasm_bindgen(js_name = "withVerticalInner")]
    pub fn with_vertical_inner(&self, edge: &BorderEdgeSpec) -> Self {
        let mut next = self.0.clone();
        next.vertical_inner = Some(edge.0.clone());
        Self(next)
    }

    /// `horizontal` — the inner horizontal border of a range.
    #[wasm_bindgen(js_name = "withHorizontalInner")]
    pub fn with_horizontal_inner(&self, edge: &BorderEdgeSpec) -> Self {
        let mut next = self.0.clone();
        next.horizontal_inner = Some(edge.0.clone());
        Self(next)
    }

    /// `@diagonalUp` — draw the bottom-left to top-right diagonal.
    #[wasm_bindgen(js_name = "withDiagonalUp")]
    pub fn with_diagonal_up(&self, draw: bool) -> Self {
        let mut next = self.0.clone();
        next.diagonal_up = Some(draw);
        Self(next)
    }

    /// `@diagonalDown` — draw the top-left to bottom-right diagonal.
    #[wasm_bindgen(js_name = "withDiagonalDown")]
    pub fn with_diagonal_down(&self, draw: bool) -> Self {
        let mut next = self.0.clone();
        next.diagonal_down = Some(draw);
        Self(next)
    }

    /// `left`.
    #[wasm_bindgen(getter, js_name = "left")]
    pub fn left(&self) -> Option<BorderEdgeSpec> {
        self.0.left.clone().map(BorderEdgeSpec)
    }

    /// `right`.
    #[wasm_bindgen(getter, js_name = "right")]
    pub fn right(&self) -> Option<BorderEdgeSpec> {
        self.0.right.clone().map(BorderEdgeSpec)
    }

    /// `top`.
    #[wasm_bindgen(getter, js_name = "top")]
    pub fn top(&self) -> Option<BorderEdgeSpec> {
        self.0.top.clone().map(BorderEdgeSpec)
    }

    /// `bottom`.
    #[wasm_bindgen(getter, js_name = "bottom")]
    pub fn bottom(&self) -> Option<BorderEdgeSpec> {
        self.0.bottom.clone().map(BorderEdgeSpec)
    }

    /// `diagonal`.
    #[wasm_bindgen(getter, js_name = "diagonal")]
    pub fn diagonal(&self) -> Option<BorderEdgeSpec> {
        self.0.diagonal.clone().map(BorderEdgeSpec)
    }
}

impl Default for BorderSpec {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl CellFormatSpec {
    /// An `x:xf` that states nothing.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self(ooxml::CellFormatSpec::default())
    }

    /// The record every workbook's `cellXfs[0]` is: `numFmtId="0" fontId="0" fillId="0"
    /// borderId="0" xfId="0"`. The base to build a real format on.
    #[wasm_bindgen(js_name = "skeletonCellFormat")]
    pub fn skeleton_cell_format() -> Self {
        Self(ooxml::CellFormatSpec::skeleton_cell_format())
    }

    /// The same record without `@xfId` — a `cellStyleXfs` entry, which has nothing beneath it.
    #[wasm_bindgen(js_name = "skeletonCellStyleFormat")]
    pub fn skeleton_cell_style_format() -> Self {
        Self(ooxml::CellFormatSpec::skeleton_cell_style_format())
    }

    /// `@fontId`, and the `@applyFont` that makes a consumer honour it.
    #[wasm_bindgen(js_name = "withFontIndex")]
    pub fn with_font_index(&self, index: u32) -> Self {
        let mut next = self.0.clone();
        next.font_index = Some(index);
        next.applies_font = Some(true);
        Self(next)
    }

    /// `@fillId`, and `@applyFill`.
    #[wasm_bindgen(js_name = "withFillIndex")]
    pub fn with_fill_index(&self, index: u32) -> Self {
        let mut next = self.0.clone();
        next.fill_index = Some(index);
        next.applies_fill = Some(true);
        Self(next)
    }

    /// `@borderId`, and `@applyBorder`.
    #[wasm_bindgen(js_name = "withBorderIndex")]
    pub fn with_border_index(&self, index: u32) -> Self {
        let mut next = self.0.clone();
        next.border_index = Some(index);
        next.applies_border = Some(true);
        Self(next)
    }

    /// `@numFmtId`, and `@applyNumberFormat`.
    #[wasm_bindgen(js_name = "withNumberFormatId")]
    pub fn with_number_format_id(&self, id: u32) -> Self {
        let mut next = self.0.clone();
        next.number_format_id = Some(id);
        next.applies_number_format = Some(true);
        Self(next)
    }

    /// `@xfId` — the `cellStyleXfs` record beneath this one.
    #[wasm_bindgen(js_name = "withCellStyleFormatIndex")]
    pub fn with_cell_style_format_index(&self, index: u32) -> Self {
        let mut next = self.0.clone();
        next.cell_style_format_index = Some(index);
        Self(next)
    }

    /// `@quotePrefix` — the value is text because it was typed with a leading apostrophe.
    #[wasm_bindgen(js_name = "withQuotePrefix")]
    pub fn with_quote_prefix(&self, quoted: bool) -> Self {
        let mut next = self.0.clone();
        next.text_is_quote_prefixed = Some(quoted);
        Self(next)
    }

    /// `@numFmtId`.
    #[wasm_bindgen(getter, js_name = "numberFormatId")]
    pub fn number_format_id(&self) -> Option<u32> {
        self.0.number_format_id
    }

    /// `@fontId`.
    #[wasm_bindgen(getter, js_name = "fontIndex")]
    pub fn font_index(&self) -> Option<u32> {
        self.0.font_index
    }

    /// `@fillId`.
    #[wasm_bindgen(getter, js_name = "fillIndex")]
    pub fn fill_index(&self) -> Option<u32> {
        self.0.fill_index
    }

    /// `@borderId`.
    #[wasm_bindgen(getter, js_name = "borderIndex")]
    pub fn border_index(&self) -> Option<u32> {
        self.0.border_index
    }

    /// `@xfId` — the `cellStyleXfs` record beneath this one.
    #[wasm_bindgen(getter, js_name = "cellStyleFormatIndex")]
    pub fn cell_style_format_index(&self) -> Option<u32> {
        self.0.cell_style_format_index
    }

    /// `@quotePrefix` — the value is text because it was typed with a leading apostrophe. Spelled
    /// as the facade's own field so that the readable attribute reads the same in both bindings;
    /// the builder beside it keeps the shorter `withQuotePrefix` it shipped with.
    #[wasm_bindgen(getter, js_name = "textIsQuotePrefixed")]
    pub fn text_is_quote_prefixed(&self) -> Option<bool> {
        self.0.text_is_quote_prefixed
    }

    /// `@applyNumberFormat`.
    #[wasm_bindgen(getter, js_name = "appliesNumberFormat")]
    pub fn applies_number_format(&self) -> Option<bool> {
        self.0.applies_number_format
    }

    /// `@applyFont`.
    #[wasm_bindgen(getter, js_name = "appliesFont")]
    pub fn applies_font(&self) -> Option<bool> {
        self.0.applies_font
    }

    /// `@applyFill`.
    #[wasm_bindgen(getter, js_name = "appliesFill")]
    pub fn applies_fill(&self) -> Option<bool> {
        self.0.applies_fill
    }

    /// `@applyBorder`.
    #[wasm_bindgen(getter, js_name = "appliesBorder")]
    pub fn applies_border(&self) -> Option<bool> {
        self.0.applies_border
    }

    /// `@applyAlignment`.
    #[wasm_bindgen(getter, js_name = "appliesAlignment")]
    pub fn applies_alignment(&self) -> Option<bool> {
        self.0.applies_alignment
    }

    /// `@applyProtection`.
    #[wasm_bindgen(getter, js_name = "appliesProtection")]
    pub fn applies_protection(&self) -> Option<bool> {
        self.0.applies_protection
    }

    /// `@applyNumberFormat`, stated on its own.
    ///
    /// The six `withApplies…` builders exist because the flag is **three-valued** — §18.8.9 makes
    /// an absent flag *participate* and a `0` *suppress*, which is not the same thing — and because
    /// `withNumberFormatId` and its three siblings can only ever say `1`. Pass `undefined` to write
    /// no attribute at all. Call this *after* the index builder, whose implied `1` it replaces.
    #[wasm_bindgen(js_name = "withAppliesNumberFormat")]
    pub fn with_applies_number_format(&self, applies: Option<bool>) -> Self {
        let mut next = self.0.clone();
        next.applies_number_format = applies;
        Self(next)
    }

    /// `@applyFont`, stated on its own. See `withAppliesNumberFormat`.
    #[wasm_bindgen(js_name = "withAppliesFont")]
    pub fn with_applies_font(&self, applies: Option<bool>) -> Self {
        let mut next = self.0.clone();
        next.applies_font = applies;
        Self(next)
    }

    /// `@applyFill`, stated on its own. See `withAppliesNumberFormat`.
    #[wasm_bindgen(js_name = "withAppliesFill")]
    pub fn with_applies_fill(&self, applies: Option<bool>) -> Self {
        let mut next = self.0.clone();
        next.applies_fill = applies;
        Self(next)
    }

    /// `@applyBorder`, stated on its own. See `withAppliesNumberFormat`.
    #[wasm_bindgen(js_name = "withAppliesBorder")]
    pub fn with_applies_border(&self, applies: Option<bool>) -> Self {
        let mut next = self.0.clone();
        next.applies_border = applies;
        Self(next)
    }

    /// `@applyAlignment`, stated on its own — the one `x:xf` attribute no index builder implies,
    /// because the alignment it governs is a child element rather than a resource index.
    #[wasm_bindgen(js_name = "withAppliesAlignment")]
    pub fn with_applies_alignment(&self, applies: Option<bool>) -> Self {
        let mut next = self.0.clone();
        next.applies_alignment = applies;
        Self(next)
    }

    /// `@applyProtection`, stated on its own. The other attribute no index builder implies.
    #[wasm_bindgen(js_name = "withAppliesProtection")]
    pub fn with_applies_protection(&self, applies: Option<bool>) -> Self {
        let mut next = self.0.clone();
        next.applies_protection = applies;
        Self(next)
    }
}

impl Default for CellFormatSpec {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl EffectiveCellFormat {
    /// The `cellXfs` index this cell resolved to.
    #[wasm_bindgen(getter, js_name = "styleIndex")]
    pub fn style_index(&self) -> u32 {
        self.0.style_index()
    }

    /// Which of the four layers supplied that index — the cell, its row, its column, or the default.
    #[wasm_bindgen(getter, js_name = "styleIndexSource")]
    pub fn style_index_source(&self) -> Result<StyleIndexSource, JsValue> {
        StyleIndexSource::from_model(self.0.style_index_source())
    }

    /// The `cellStyleXfs` record beneath the resolved `x:xf`, when it names one.
    #[wasm_bindgen(getter, js_name = "cellStyleFormatIndex")]
    pub fn cell_style_format_index(&self) -> Option<u32> {
        self.0.cell_style_format_index()
    }

    /// One aspect's resolution: the number format, the font, the fill, the border, the alignment or
    /// the protection.
    #[wasm_bindgen(js_name = "aspect")]
    pub fn aspect(&self, aspect: FormatAspect) -> ResolvedAspect {
        ResolvedAspect(self.0.aspect(aspect.into()))
    }
}

#[wasm_bindgen]
impl ResolvedAspect {
    /// Whether the resolved `x:xf` applies this aspect, suppresses it, or says nothing.
    #[wasm_bindgen(getter, js_name = "applyFlag")]
    pub fn apply_flag(&self) -> Result<ApplyFlag, JsValue> {
        ApplyFlag::from_model(self.0.apply_flag)
    }

    /// The same, for the record that supplied the value.
    #[wasm_bindgen(getter, js_name = "supplyingApplyFlag")]
    pub fn supplying_apply_flag(&self) -> Result<ApplyFlag, JsValue> {
        ApplyFlag::from_model(self.0.supplying_apply_flag)
    }

    /// Which layer supplied it: the direct `cellXfs` record, the `cellStyleXfs` one beneath it, or
    /// neither.
    #[wasm_bindgen(getter, js_name = "layer")]
    pub fn layer(&self) -> Result<FormatLayer, JsValue> {
        FormatLayer::from_model(self.0.layer)
    }

    /// The index of the `x:xf` that supplied the value.
    #[wasm_bindgen(getter, js_name = "formatIndex")]
    pub fn format_index(&self) -> Option<u32> {
        self.0.format_index
    }

    /// The index into the resource table this aspect names — a font, a fill, a border.
    #[wasm_bindgen(getter, js_name = "resourceIndex")]
    pub fn resource_index(&self) -> Option<u32> {
        self.0.resource_index
    }

    /// Whether anything stated this aspect at all.
    #[wasm_bindgen(getter, js_name = "isStated")]
    pub fn is_stated(&self) -> bool {
        self.0.is_stated
    }
}
