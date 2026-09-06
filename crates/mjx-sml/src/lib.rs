//! `mjx-sml` — the SpreadsheetML **markup** model: cells, rows, sheet data, shared strings, styles,
//! number formats and formulas-as-text.
//!
//! # Why SpreadsheetML is shared markup, not a format
//!
//! A `.xlsx` package is Excel's, but SpreadsheetML is not. An authored PowerPoint chart embeds a
//! whole workbook at `/ppt/embeddings/*.xlsx` — that package is what **Edit Data** opens — and a
//! Word document that carries a chart does the same. `mjx-chart` therefore needed a SpreadsheetML
//! writer before Excel existed, and wrote a minimal one of its own
//! ([`mjx_chart::EmbeddedWorkbook`](https://docs.rs/mjx-chart)) with a note naming its executioner.
//!
//! That deletion was **illegal as specified**: `mjx-chart` sits in the shared-markup tier, so an
//! edge from it to `mjx-xlsx` (a format) would point *upward*, and `mjx-pptx → mjx-xlsx` would point
//! *sideways*. Both are forbidden by the layering rule in `CLAUDE.md`. The layer was not missing by
//! accident — it was missing because nobody had split Excel the way DrawingML was already split:
//!
//! * **`mjx-sml`** (here) — the `sml.xsd` *markup*: what a cell, a row, a shared string, an `xf` or
//!   a number format **is**. Shared-markup tier, beside `mjx-dml`.
//! * **`mjx-xlsx`** — the `Workbook` surface, the package and part graph, `open`/`save`/`blank`,
//!   relationships. Format tier, beside `mjx-pptx` and `mjx-docx`.
//!
//! With the split, `mjx-chart → mjx-sml → mjx-dml` is a chain of downward edges and
//! `EmbeddedWorkbook` can finally be deleted (MJXOFF-112, then MJXOFF-99).
//!
//! # Where this crate sits, exactly
//!
//! Shared markup is not flat. This crate is **rank 2.1**: below `mjx-chart`, `mjx-omml` and
//! `mjx-vml` (2.2), above `mjx-dml` (2.0), and far below the format crates (3.0).
//!
//! | Rank | Crates |
//! |---|---|
//! | 0.0 — foundations, core | `mjx-ooxml-core`, `mjx-derive` |
//! | 0.1 — foundations, XML | `mjx-xml` |
//! | 1.0 — packaging / compatibility | `mjx-ooxml-types`, `mjx-opc`, `mjx-mce` |
//! | 2.0 — shared markup, base | `mjx-dml` |
//! | **2.1 — shared markup, spreadsheet** | **`mjx-sml`** |
//! | 2.2 — shared markup, upper | `mjx-chart`, `mjx-omml`, `mjx-vml` |
//! | 3.0 — formats | `mjx-pptx`, `mjx-docx`, `mjx-xlsx` |
//! | 4.0 — facade | `mjx-ooxml` |
//! | 5.0 — bindings | `bindings/mjx-python`, `bindings/mjx-wasm` |
//!
//! An edge is legal **iff** it points to a strictly lower rank. So `mjx-chart → mjx-sml` (2.2 → 2.1)
//! and `mjx-sml → mjx-dml` (2.1 → 2.0) are both legal, while `mjx-dml → mjx-sml` and
//! `mjx-sml → mjx-chart` are not, and the graph stays acyclic. This is not a comment anyone has to
//! trust: `xtask/tests/layering.rs` reads the real dependency graph out of `cargo metadata` and
//! fails, naming both crates and both ranks, on any edge that does not point down.
//!
//! # Status — the crate spine, plus the addressing vocabulary
//!
//! MJXOFF-132 creates the crate, the module tree, the `sml` child-order table and the layering test;
//! MJXOFF-93 fills [`address`], which is markup vocabulary rather than package structure and which
//! eleven later children consume. Everything else below is still a named home with the work item
//! that fills it, so that no later child has to invent a place to put its model.
//!
//! | Module | Filled by |
//! |---|---|
//! | [`address`] | **MJXOFF-93 (D03) — done**: references, ranges, `sqref`, `spans`, A1 and R1C1 |
//! | [`cells`] | **MJXOFF-95 (D04) — done**: the cell store, and the hybrid memory model made real |
//! | [`strings`] | **MJXOFF-97 (D05) — done**: `sharedStrings.xml`, rich-text runs, inline strings |
//! | [`font`] | **MJXOFF-97 (D05) — done**: `CT_RPrElt`/`CT_Font`'s shared property family, reused by D08 |
//! | [`styles`] | **MJXOFF-105 (D08) + MJXOFF-108 (D09) — done**: fonts, fills, borders, dxfs, the indexed palette; the `xf` indirection, number formats, named styles and [`EffectiveCellFormat`]; MJXOFF-125 (D15) adds the `tableStyles` slot, the last of `CT_Stylesheet`'s eleven to be modelled |
//! | [`formula`] | **MJXOFF-115 (D11) — done**: `CT_CellFormula`'s twelve attributes, shared/array/data-table formulas, cached values, `calcChain` — and the written-down guarantee that nothing here recalculates; MJXOFF-123 (D14) adds [`FormulaElement`], the `ST_Formula` *element* three slots share |
//! | [`sheets`] | **MJXOFF-129 (D17) — done**: `CT_Chartsheet`, `CT_Dialogsheet` and `CT_Macrosheet` — the three sheet kinds that are not worksheets, and the one part frame they share |
//! | [`worksheet`] | **MJXOFF-102 (D07) — done**: `CT_Worksheet`'s 39 slots, the widest content model in the schema; MJXOFF-117 (D12) adds the sheet grid, MJXOFF-120 (D13) the `conditionalFormatting` slot, MJXOFF-123 (D14) the `autoFilter` and `dataValidations` slots, MJXOFF-125 (D15) the `tableParts` slot, MJXOFF-127 (D16) the `hyperlinks`, `dataConsolidate`, `customProperties`, `cellWatches`, `ignoredErrors`, `smartTags` and `webPublishItems` slots |
//! | [`workbook`] | **MJXOFF-100 (D06) — done**: `CT_Workbook`'s nineteen slots, the sheet list, properties, views, defined names |
//! | [`features`] | **MJXOFF-120 (D13) — done**: conditional formatting, the cross-block priority order and the `dxf` layer; **MJXOFF-123 (D14) — done**: data validation, autofilters and sort state; **MJXOFF-125 (D15) — done**: worksheet tables, their columns and the `tableParts` list; **MJXOFF-127 (D16) — done**: hyperlinks, the object-anchor vocabulary three Phase E children share, and the six small worksheet clusters nothing else claimed; **MJXOFF-129 (D17) — done**: the print block every sheet *kind* shares, and custom sheet views |
//! | [`mod@write`] | **MJXOFF-112 (D10) — done**: [`WorkbookPackage`], the package writer that replaces `EmbeddedWorkbook`, and the `styles.xml` skeleton behind it |
//! | [`error`] | MJXOFF-132 (D01) — this child; every later one adds its variants |
//!
//! The half of `sml.xsd` this workspace deliberately does **not** model — pivot tables, external
//! links, metadata, connections and revisions — is written down by MJXOFF-133 (D18) in
//! [`preserved`] and in `crates/mjx-xlsx/docs/guide/fidelity_and_the_part_graph.md`. Everything
//! unmodelled is still preserved, by the unknown bucket and by `mjx-opc`'s copy-on-write, exactly as
//! it is for every other schema here.
//!
//! # The prose that explains this crate to a caller
//!
//! `crates/mjx-xlsx/docs/guide/` is where Excel is documented for someone using it rather than
//! extending it, and two of its pages are about this crate specifically:
//! **Deliberate limitations** says which of the two crates to reach for and why the split exists,
//! and **Large workbooks** is the memory model of [`cells`] in a caller's terms — what a sparse
//! sheet costs, what a populated cell costs, and what the parse behind
//! [`WorksheetPart::read_part`] costs at three hundred thousand cells.
//!
//! # Ordering
//!
//! `sml` is in `xtask`'s `CHILD_ORDER_SCHEMAS` as of this child, so
//! [`mjx_ooxml_types::child_order`] carries the `xsd:sequence` position of every child of all 367
//! SpreadsheetML complex types — `CT_Worksheet`'s 39 slots among them, the largest sequence in the
//! workspace. Every writer added here places children through that table rather than by hand, and
//! `mjx-schema-gate` audits every `x:`-rooted part of every package it inspects.

pub(crate) mod arena;
pub(crate) mod leaf;

pub mod address;
pub mod cells;
pub mod error;
pub mod features;
pub mod font;
pub mod formula;
pub mod preserved;
pub mod sheets;
pub mod strings;
pub mod styles;
pub mod workbook;
pub mod worksheet;
pub mod write;

pub use address::{
    AddressError, AddressText, Anchoring, CellRange, CellRangeList, CellReference, CellSpan,
    CellSpans, ColumnBound, GridBounds, R1C1Coordinate, R1C1Range, R1C1Reference, ReferenceMode,
    RowBound, SheetName, SheetQualifiedReference,
};
pub use cells::{Cell, CellValue, PayloadShape, Row, SheetData, SheetDataAnomaly};
pub use error::SmlError;
pub use features::{
    AppliedConditionalRule, AutoFilter, AutoFilterContent, AutoFilterSpec, CellSmartTag,
    CellSmartTagContent, CellSmartTagProperty, CellSmartTags, CellSmartTagsContent, CellWatch,
    CellWatches, CellWatchesContent, ColorFilter, ColorScale, ColorScaleContent, ColorScaleSpec,
    ConditionalCellFormat, ConditionalFormatLayer, ConditionalFormatting,
    ConditionalFormattingContent, ConditionalFormattingRule, ConditionalFormattingRuleContent,
    ConditionalRuleChain, ConditionalRuleSpec, ConditionalRuleSpecKind, ConditionalValueObject,
    ConditionalValueObjectSpec, CustomFilter, CustomFilterSpec, CustomFilters,
    CustomFiltersContent, CustomProperties, CustomPropertiesContent, CustomProperty, DataBar,
    DataBarContent, DataBarSpec, DataConsolidation, DataConsolidationContent, DataReference,
    DataReferences, DataReferencesContent, DataValidation, DataValidationContent,
    DataValidationSpec, DataValidations, DataValidationsContent, DateGroupItem,
    DifferentialFormatSpec, DynamicFilter, Filter, FilterColumn, FilterColumnSpec, FilterKind,
    FilterSpecKind, Filters, FiltersContent, Hyperlink, Hyperlinks, HyperlinksContent, IconFilter,
    IconSet, IconSetContent, IconSetSpec, IgnoredError, IgnoredErrors, IgnoredErrorsContent,
    ObjectAnchor, ObjectProperties, ObjectPropertiesContent, SmartTags, SmartTagsContent,
    SortCondition, SortConditionSpec, SortState, SortStateContent, SortStateSpec, TableColumn,
    TableColumnContent, TableColumnSpec, TableColumns, TableColumnsContent, TableFormula,
    TablePart, TableParts, TablePartsContent, TableStyleReference, TableStyleReferenceSpec,
    Top10Filter, WebPublishItem, WebPublishItems, WebPublishItemsContent, WorksheetTable,
    WorksheetTableContent, WorksheetTableSpec, XmlColumnProperties,
};
pub use features::{
    ChartSheetPageSetup, CustomSheetView, CustomSheetViewContent, CustomSheetViews,
    CustomSheetViewsContent, HeaderFooter, HeaderFooterContent, HeaderFooterSection,
    HeaderFooterSlot, HeaderFooterText, PageMargins, PageSetup, PrintOptions,
    SheetBackgroundPicture,
};
pub use font::{Color, ColorElement, FontProperties, FontPropertyOwner};
pub use formula::{
    CachedValue, CalculationChain, CalculationChainCell, CalculationChainContent, CellFormula,
    FormulaElement, FormulaKind, ResolvedCalculationChainCell, SharedFormulaGroup,
    SharedFormulaGroups,
};
pub use preserved::{
    relationship_prefix, ConnectionIdentity, ExternalLinkIdentity, ExternalLinkTarget,
    PivotCacheIdentity, PivotCacheSource, PivotTableIdentity, QueryTableIdentity,
    RevisionHeadersIdentity, RevisionSession, SharedWorkbookUser, SharedWorkbookUsersIdentity,
    XmlMapIdentity, XmlMapsIdentity,
};
pub use sheets::{
    ChartSheetContent, ChartSheetPart, ChartSheetProperties, ChartSheetPropertiesContent,
    ChartSheetProtection, ChartSheetView, ChartSheetViews, ChartSheetViewsContent,
    CustomChartSheetView, CustomChartSheetViewContent, CustomChartSheetViews,
    CustomChartSheetViewsContent, DialogSheetContent, DialogSheetPart, MacroSheetContent,
    MacroSheetPart, SheetDrawing,
};
pub use strings::{
    InlineString, PhoneticProperties, PhoneticRun, RichTextRun, RichTextRunSpec, SharedStringTable,
    StringItem,
};
pub use styles::{
    apply_tint, apply_tint_to_luminance, builtin_cell_style_name, builtin_format_code,
    builtin_format_code_in, builtin_table_style_name, cell_style_index, column_style_index,
    is_locale_dependent, ApplyFlag, Border, BorderContent, BorderEdge, BorderEdgeContent,
    BorderTable, BorderTableContent, BuiltInCellStyleName, BuiltInTableStyle,
    BuiltInTableStyleFamily, CellAlignment, CellFormat, CellFormatContent, CellFormatResolver,
    CellFormatTable, CellFormatTableContent, CellFormatTableKind, CellProtection, ColorTable,
    ColorTableContent, ColumnStyles, DifferentialFormat, DifferentialFormatContent,
    DifferentialFormats, DifferentialFormatsContent, EffectiveCellFormat, Fill, FillContent,
    FillTable, FillTableContent, Font, FontTable, FontTableContent, FormatAspect, FormatLayer,
    GradientFill, GradientFillContent, GradientStop, GradientStopContent, IndexedColor,
    IndexedColorPalette, IndexedColors, IndexedColorsContent, MruColors, MruColorsContent,
    NamedCellStyle, NamedCellStyles, NamedCellStylesContent, NumberFormat, NumberFormatLanguage,
    NumberFormatTable, NumberFormatTableContent, PatternFill, PatternFillContent, ResolvedAspect,
    RgbColor, StyleIndexSource, StylesheetContent, StylesheetPart, TableStyleDefinition,
    TableStyleDefinitionContent, TableStyleLookup, TableStyleOrigin, TableStyleRegion, TableStyles,
    TableStylesContent,
};
pub use workbook::{
    BookViews, BuiltInName, CalculationProperties, CustomWorkbookView, CustomWorkbookViews,
    DefinedName, DefinedNames, EmbeddedObjectSize, ExternalReference, ExternalReferences,
    FileRecoveryProperties, FileSharing, FileVersion, FunctionGroup, FunctionGroups, PivotCache,
    PivotCaches, SheetEntry, SheetList, SmartTagProperties, SmartTagType, SmartTagTypes,
    WebPublishObject, WebPublishObjects, WebPublishing, WorkbookContent, WorkbookPart,
    WorkbookProperties, WorkbookProtection, WorkbookView,
};
pub use worksheet::{
    BreakAxis, ColumnBlock, ColumnBlockContent, ColumnRun, ColumnWidth, GridAnomaly, MergedCells,
    MergedCellsContent, MergedRange, OutlineProperties, PageBreak, PageBreaks, PageBreaksContent,
    PageSetupProperties, PivotSelection, ProtectedRange, ProtectedRanges, ProtectedRangesContent,
    RowHeight, Scenario, ScenarioContent, ScenarioInputCells, Scenarios, ScenariosContent,
    Selection, SheetCalculationProperties, SheetDimension, SheetFormatProperties, SheetPane,
    SheetProperties, SheetPropertiesContent, SheetProtection, SheetView, SheetViewContent,
    SheetViews, SheetViewsContent, WorksheetContent, WorksheetPart,
};
pub use write::{
    AuthoredCellValue, AuthoredStylesheet, AuthoredTable, AuthoredWorkbook, AuthoredWorksheet,
    BorderEdgeSpec, BorderSpec, CellFormatSpec, CellFormatTarget, PatternFillSpec, WorkbookPackage,
};
