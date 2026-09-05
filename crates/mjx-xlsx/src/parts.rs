//! The SpreadsheetML part graph: content types, relationship types, and the parts a workbook or a
//! worksheet relates to.
//!
//! # Where the strings come from
//!
//! Every constant here is quoted from **ECMA-376 Part 1 (5th edition), §12.3 "Part Summary"**,
//! except the two the spec keeps elsewhere and which say so in their own doc comments
//! ([`REL_THEME`]/[`CONTENT_TYPE_THEME`], DrawingML §14.2.7; [`REL_VML_DRAWING`]/
//! [`CONTENT_TYPE_VML_DRAWING`], **Part 4 §8.2**). Part 1 states every relationship type in its
//! *Strict* form (`http://purl.oclc.org/ooxml/officeDocument/relationships/...`); every fixture in
//! this workspace is Transitional, so the prefix is substituted for
//! `http://schemas.openxmlformats.org/officeDocument/2006/relationships/...`, exactly as
//! `mjx_docx::constants` documents for WordprocessingML. The four constants a workbook package
//! cannot do without — [`CONTENT_TYPE_WORKBOOK`], [`CONTENT_TYPE_WORKSHEET`],
//! [`CONTENT_TYPE_SHARED_STRINGS`], [`CONTENT_TYPE_STYLES`] — additionally match `mjx-chart`'s
//! `workbook.rs` string for string, which is a package both LibreOffice and PowerPoint accept.
//! MJXOFF-112 (D10) is where that duplicate goes away.
//!
//! # What is deliberately *not* here
//!
//! The macro-enabled workbook content types (`…spreadsheetml.sheet.macroEnabled.main+xml` and its
//! siblings) are **not** declared. `macroEnabled` appears nowhere in ECMA-376 — not in Part 1, 2, 3
//! or 4 — so writing one here would be guessing a wire token, which this project does not do (see
//! `CLAUDE.md`, *"Wire tokens are preserved exactly, never guessed"*). A `.xlsm` still opens: the
//! workbook part is found through its `officeDocument` relationship and identified by its **root
//! element**, never by its content type, and [`crate::PartClassification`] reports the part as
//! preserved-and-unclassified rather than rejecting it.
//!
//! # This file is the part graph, not a model
//!
//! [`PartKind`] answers "what is this part, and how is it reached"; [`WorkbookParts`] and
//! [`WorksheetParts`] answer "what does this one relate to". Nothing here parses SpreadsheetML
//! content — that is `mjx-sml`'s, and the Phase D children listed in
//! `crates/mjx-xlsx/src/workbook/mod.rs`'s own module documentation fill it in.

use mjx_opc::{Package, PartName, Relationship, Relationships, TargetMode};

use crate::error::XlsxError;

// ---------------------------------------------------------------------------------------------
// Relationship types (ECMA-376 Part 1 §12.3, Transitional spellings)
// ---------------------------------------------------------------------------------------------

/// The relationship type from the package root to the workbook part (§12.3.23).
pub const REL_OFFICE_DOCUMENT: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";

/// The relationship type from the workbook part to a worksheet part (§12.3.24).
pub const REL_WORKSHEET: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet";

/// The relationship type from the workbook part to a chartsheet part (§12.3.2).
pub const REL_CHARTSHEET: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/chartsheet";

/// The relationship type from the workbook part to a dialogsheet part (§12.3.7).
pub const REL_DIALOGSHEET: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/dialogsheet";

/// The relationship type from the workbook part to the shared string table (§12.3.15).
pub const REL_SHARED_STRINGS: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings";

/// The relationship type from the workbook part to the styles part (§12.3.20).
pub const REL_STYLES: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles";

/// The relationship type from the workbook part to the calculation chain (§12.3.1).
pub const REL_CALCULATION_CHAIN: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/calcChain";

/// The relationship type from the workbook part to the connections part (§12.3.4).
pub const REL_CONNECTIONS: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/connections";

/// The relationship type from the workbook part to the cell metadata part (§12.3.10).
///
/// The part is named "Metadata" in the spec's own heading and reached through a relationship type
/// spelled `sheetMetadata`; both spellings are the spec's, quoted rather than reconciled.
pub const REL_METADATA: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/sheetMetadata";

/// The relationship type from the workbook part to the volatile dependencies part (§12.3.22).
pub const REL_VOLATILE_DEPENDENCIES: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/volatileDependencies";

/// The relationship type from the workbook part to an external workbook references part (§12.3.9).
pub const REL_EXTERNAL_LINK: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/externalLink";

/// The relationship type from the workbook part to a pivot table cache definition (§12.3.12).
pub const REL_PIVOT_CACHE_DEFINITION: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/pivotCacheDefinition";

/// The relationship type from a pivot table cache definition to its cache records (§12.3.13).
pub const REL_PIVOT_CACHE_RECORDS: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/pivotCacheRecords";

/// The relationship type from a sheet part to a pivot table part (§12.3.11).
pub const REL_PIVOT_TABLE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/pivotTable";

/// The relationship type from a worksheet part to a query table part (§12.3.14).
pub const REL_QUERY_TABLE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/queryTable";

/// The relationship type from a worksheet part to a table definition part (§12.3.21).
pub const REL_TABLE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/table";

/// The relationship type from a sheet part to its comments part (§12.3.3).
pub const REL_COMMENTS: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments";

/// The relationship type from a sheet part to a DrawingML drawings part (§12.3.8).
pub const REL_DRAWING: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing";

/// The relationship type from a worksheet or dialogsheet part to a legacy VML drawing.
///
/// **ECMA-376 Part 4 §8.2 "VML Drawing Part"**, not Part 1: VML is a Transitional-only feature, and
/// Part 4 already states this URI in the Transitional form quoted here rather than the Strict form
/// every Part 1 constant above is substituted from. In a workbook this is what carries a comment's
/// pop-up box and a form control's appearance.
pub const REL_VML_DRAWING: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/vmlDrawing";

/// The relationship type from a sheet part to its printer settings part (Part 1 §15.2.13, the
/// *shared* part summary — one Printer Settings part per chartsheet, dialogsheet or worksheet).
pub const REL_PRINTER_SETTINGS: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/printerSettings";

/// The relationship type from the workbook part to a theme part (DrawingML, Part 1 §14.2.7).
///
/// Not SpreadsheetML — the same URI and the same OPC concept `mjx-pptx` and `mjx-docx` each declare
/// for themselves, declared again here for the same reason they do: reaching across for it would be
/// a sideways crate edge.
pub const REL_THEME: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme";

// ---------------------------------------------------------------------------------------------
// Content types (ECMA-376 Part 1 §12.3)
// ---------------------------------------------------------------------------------------------

/// The content type of the workbook part of a spreadsheet document (§12.3.23, first of the two the
/// clause lists). Identical to `mjx_chart`'s own `CONTENT_TYPE_WORKBOOK`.
pub const CONTENT_TYPE_WORKBOOK: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml";

/// The content type of the workbook part of a spreadsheet *template* (§12.3.23, second of the two).
pub const CONTENT_TYPE_WORKBOOK_TEMPLATE: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.template.main+xml";

/// The content type of a worksheet part (§12.3.24).
pub const CONTENT_TYPE_WORKSHEET: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml";

/// The content type of a chartsheet part (§12.3.2).
pub const CONTENT_TYPE_CHARTSHEET: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.chartsheet+xml";

/// The content type of a dialogsheet part (§12.3.7).
pub const CONTENT_TYPE_DIALOGSHEET: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.dialogsheet+xml";

/// The content type of the shared string table (§12.3.15).
pub const CONTENT_TYPE_SHARED_STRINGS: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml";

/// The content type of the styles part (§12.3.20).
pub const CONTENT_TYPE_STYLES: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml";

/// The content type of the calculation chain (§12.3.1).
pub const CONTENT_TYPE_CALCULATION_CHAIN: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.calcChain+xml";

/// The content type of the connections part (§12.3.4).
pub const CONTENT_TYPE_CONNECTIONS: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.connections+xml";

/// The content type of the cell metadata part (§12.3.10).
pub const CONTENT_TYPE_METADATA: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheetMetadata+xml";

/// The content type of the volatile dependencies part (§12.3.22).
pub const CONTENT_TYPE_VOLATILE_DEPENDENCIES: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.volatileDependencies+xml";

/// The content type of an external workbook references part (§12.3.9).
pub const CONTENT_TYPE_EXTERNAL_LINK: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.externalLink+xml";

/// The content type of a pivot table cache definition part (§12.3.12).
pub const CONTENT_TYPE_PIVOT_CACHE_DEFINITION: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.pivotCacheDefinition+xml";

/// The content type of a pivot table cache records part (§12.3.13).
pub const CONTENT_TYPE_PIVOT_CACHE_RECORDS: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.pivotCacheRecords+xml";

/// The content type of a pivot table part (§12.3.11).
pub const CONTENT_TYPE_PIVOT_TABLE: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.pivotTable+xml";

/// The content type of a query table part (§12.3.14).
pub const CONTENT_TYPE_QUERY_TABLE: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.queryTable+xml";

/// The content type of a table definition part (§12.3.21).
pub const CONTENT_TYPE_TABLE: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.table+xml";

/// The content type of a comments part (§12.3.3).
pub const CONTENT_TYPE_COMMENTS: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.comments+xml";

/// The content type of a DrawingML drawings part (§12.3.8). Not a SpreadsheetML content type —
/// `…officedocument.drawing+xml` is DrawingML's, and the same string a `.docx` uses.
pub const CONTENT_TYPE_DRAWING: &str = "application/vnd.openxmlformats-officedocument.drawing+xml";

/// The content type of a legacy VML drawing part (**Part 4 §8.2**). Binary as far as XML validation
/// is concerned: its root is a bare `<xml>` wrapper in no namespace, which no OOXML schema declares.
pub const CONTENT_TYPE_VML_DRAWING: &str =
    "application/vnd.openxmlformats-officedocument.vmlDrawing";

/// The content type of a printer settings part in a *SpreadsheetML* document (Part 1 §15.2.13,
/// which lists one such content type per format). Never XML — the spec places no requirement at all
/// on this part's content — so it is registered by extension in real output and preserved verbatim
/// here.
pub const CONTENT_TYPE_PRINTER_SETTINGS: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.printerSettings";

/// The content type of a theme part (DrawingML, Part 1 §14.2.7) — the same string every format uses.
pub const CONTENT_TYPE_THEME: &str = "application/vnd.openxmlformats-officedocument.theme+xml";

// ---------------------------------------------------------------------------------------------
// Part kinds
// ---------------------------------------------------------------------------------------------

/// One kind of part a workbook package holds, in the sense that it names both a relationship type
/// (how the graph reaches it) and one or more content types (how `[Content_Types].xml` registers
/// it).
///
/// The set is the part list of ECMA-376 Part 1 §12.3 that a `.xlsx` written by a real producer
/// actually carries, plus the three non-SpreadsheetML parts a workbook nevertheless relates to
/// ([`Theme`](Self::Theme), [`Drawing`](Self::Drawing), [`VmlDrawing`](Self::VmlDrawing)) and
/// [`PrinterSettings`](Self::PrinterSettings). The five §12.3 parts left out — Custom Property,
/// Custom XML Mappings, and the three Shared Workbook revision parts — are left out on purpose:
/// two of them have no content type of their own to classify by (`application/xml`, and "any
/// content, support for which is application-defined"), and the revision trio belongs to the
/// shared-workbook feature MJXOFF-133 (D18) writes down as deliberately unmodelled. All five are
/// still **preserved** — see [`crate::PartClassification`], which reports any part this enum does
/// not name rather than rejecting it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PartKind {
    /// `x:workbook` — the workbook part (§12.3.23).
    Workbook,
    /// `x:worksheet` — a worksheet part (§12.3.24).
    Worksheet,
    /// `x:chartsheet` — a chartsheet part (§12.3.2).
    Chartsheet,
    /// `x:dialogsheet` — a dialogsheet part (§12.3.7).
    Dialogsheet,
    /// `x:sst` — the shared string table (§12.3.15).
    SharedStrings,
    /// `x:styleSheet` — the styles part (§12.3.20).
    Styles,
    /// `x:calcChain` — the calculation chain (§12.3.1).
    CalculationChain,
    /// `x:connections` — the connections part (§12.3.4).
    Connections,
    /// `x:metadata` — the cell metadata part (§12.3.10).
    Metadata,
    /// `x:volTypes` — the volatile dependencies part (§12.3.22).
    VolatileDependencies,
    /// `x:externalLink` — an external workbook references part (§12.3.9).
    ExternalLink,
    /// `x:pivotCacheDefinition` — a pivot table cache definition (§12.3.12).
    PivotCacheDefinition,
    /// `x:pivotCacheRecords` — a pivot table cache records part (§12.3.13).
    PivotCacheRecords,
    /// `x:pivotTableDefinition` — a pivot table part (§12.3.11).
    PivotTable,
    /// `x:queryTable` — a query table part (§12.3.14).
    QueryTable,
    /// `x:table` — a table definition part (§12.3.21).
    Table,
    /// `x:comments` — a comments part (§12.3.3).
    Comments,
    /// `xdr:wsDr` — a DrawingML drawings part (§12.3.8). DrawingML, not SpreadsheetML.
    Drawing,
    /// A legacy VML drawing part (Part 4 §8.2) — a comment's pop-up box, a form control's look.
    VmlDrawing,
    /// A printer settings part (§15.2.13). Opaque bytes, never XML.
    PrinterSettings,
    /// `a:theme` — a theme part (§14.2.7). DrawingML, not SpreadsheetML.
    Theme,
}

impl PartKind {
    /// The relationship type a part graph reaches this kind through.
    ///
    /// From the package root for [`Workbook`](Self::Workbook); from the workbook part for the
    /// workbook-level kinds; from a sheet part for the sheet-level ones; and from a pivot cache
    /// definition for [`PivotCacheRecords`](Self::PivotCacheRecords). [`WorkbookParts`] and
    /// [`WorksheetParts`] are what say which is which.
    #[must_use]
    pub fn relationship_type(self) -> &'static str {
        match self {
            Self::Workbook => REL_OFFICE_DOCUMENT,
            Self::Worksheet => REL_WORKSHEET,
            Self::Chartsheet => REL_CHARTSHEET,
            Self::Dialogsheet => REL_DIALOGSHEET,
            Self::SharedStrings => REL_SHARED_STRINGS,
            Self::Styles => REL_STYLES,
            Self::CalculationChain => REL_CALCULATION_CHAIN,
            Self::Connections => REL_CONNECTIONS,
            Self::Metadata => REL_METADATA,
            Self::VolatileDependencies => REL_VOLATILE_DEPENDENCIES,
            Self::ExternalLink => REL_EXTERNAL_LINK,
            Self::PivotCacheDefinition => REL_PIVOT_CACHE_DEFINITION,
            Self::PivotCacheRecords => REL_PIVOT_CACHE_RECORDS,
            Self::PivotTable => REL_PIVOT_TABLE,
            Self::QueryTable => REL_QUERY_TABLE,
            Self::Table => REL_TABLE,
            Self::Comments => REL_COMMENTS,
            Self::Drawing => REL_DRAWING,
            Self::VmlDrawing => REL_VML_DRAWING,
            Self::PrinterSettings => REL_PRINTER_SETTINGS,
            Self::Theme => REL_THEME,
        }
    }

    /// Every content type `[Content_Types].xml` may register this kind under, most common first.
    ///
    /// Plural rather than singular because [`Workbook`](Self::Workbook) genuinely has two — §12.3.23
    /// lists the workbook and the template — and collapsing that to one would make a `.xltx` an
    /// unclassified part for no reason. Every other kind returns exactly one.
    #[must_use]
    pub fn content_types(self) -> &'static [&'static str] {
        match self {
            Self::Workbook => &[CONTENT_TYPE_WORKBOOK, CONTENT_TYPE_WORKBOOK_TEMPLATE],
            Self::Worksheet => &[CONTENT_TYPE_WORKSHEET],
            Self::Chartsheet => &[CONTENT_TYPE_CHARTSHEET],
            Self::Dialogsheet => &[CONTENT_TYPE_DIALOGSHEET],
            Self::SharedStrings => &[CONTENT_TYPE_SHARED_STRINGS],
            Self::Styles => &[CONTENT_TYPE_STYLES],
            Self::CalculationChain => &[CONTENT_TYPE_CALCULATION_CHAIN],
            Self::Connections => &[CONTENT_TYPE_CONNECTIONS],
            Self::Metadata => &[CONTENT_TYPE_METADATA],
            Self::VolatileDependencies => &[CONTENT_TYPE_VOLATILE_DEPENDENCIES],
            Self::ExternalLink => &[CONTENT_TYPE_EXTERNAL_LINK],
            Self::PivotCacheDefinition => &[CONTENT_TYPE_PIVOT_CACHE_DEFINITION],
            Self::PivotCacheRecords => &[CONTENT_TYPE_PIVOT_CACHE_RECORDS],
            Self::PivotTable => &[CONTENT_TYPE_PIVOT_TABLE],
            Self::QueryTable => &[CONTENT_TYPE_QUERY_TABLE],
            Self::Table => &[CONTENT_TYPE_TABLE],
            Self::Comments => &[CONTENT_TYPE_COMMENTS],
            Self::Drawing => &[CONTENT_TYPE_DRAWING],
            Self::VmlDrawing => &[CONTENT_TYPE_VML_DRAWING],
            Self::PrinterSettings => &[CONTENT_TYPE_PRINTER_SETTINGS],
            Self::Theme => &[CONTENT_TYPE_THEME],
        }
    }

    /// Every kind this crate classifies, in the order [`ALL`](Self::ALL) declares them.
    ///
    /// Exhaustive by construction: `every_part_kind_is_in_all` walks it and compares against the
    /// discriminant count, so a variant added without a row here fails the suite.
    pub const ALL: &'static [Self] = &[
        Self::Workbook,
        Self::Worksheet,
        Self::Chartsheet,
        Self::Dialogsheet,
        Self::SharedStrings,
        Self::Styles,
        Self::CalculationChain,
        Self::Connections,
        Self::Metadata,
        Self::VolatileDependencies,
        Self::ExternalLink,
        Self::PivotCacheDefinition,
        Self::PivotCacheRecords,
        Self::PivotTable,
        Self::QueryTable,
        Self::Table,
        Self::Comments,
        Self::Drawing,
        Self::VmlDrawing,
        Self::PrinterSettings,
        Self::Theme,
    ];

    /// The kind a part with this content type is, or `None` for a content type this crate does not
    /// classify.
    ///
    /// `None` is never a rejection — see [`crate::PartClassification`]. It is what a `.xlsm`'s
    /// macro-enabled workbook, a custom XML mapping (`application/xml`) and an image all report.
    #[must_use]
    pub fn from_content_type(content_type: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|kind| kind.content_types().contains(&content_type))
    }
}

/// Which of the three sheet kinds a `x:sheet` entry leads to.
///
/// A workbook's `x:sheets` list is one list over three part kinds — ECMA-376 Part 1 §12.3.23 names
/// Chartsheet, Dialogsheet and Worksheet as the three explicit relationships a workbook part may
/// have to a sheet — so the entry says *which* only through the content type of the part its
/// `r:id` reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SheetKind {
    /// A worksheet — the ordinary grid of cells (§12.3.24).
    Worksheet,
    /// A chartsheet — one chart occupying a whole sheet tab (§12.3.2).
    Chartsheet,
    /// A dialogsheet — a legacy Excel 5.0 dialog (§12.3.7).
    Dialogsheet,
}

impl SheetKind {
    /// The sheet kind a part with this content type is, or `None` if it is not a sheet at all.
    #[must_use]
    pub fn from_content_type(content_type: &str) -> Option<Self> {
        match PartKind::from_content_type(content_type)? {
            PartKind::Worksheet => Some(Self::Worksheet),
            PartKind::Chartsheet => Some(Self::Chartsheet),
            PartKind::Dialogsheet => Some(Self::Dialogsheet),
            _ => None,
        }
    }

    /// The [`PartKind`] this sheet kind's part is.
    #[must_use]
    pub fn part_kind(self) -> PartKind {
        match self {
            Self::Worksheet => PartKind::Worksheet,
            Self::Chartsheet => PartKind::Chartsheet,
            Self::Dialogsheet => PartKind::Dialogsheet,
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The resolved graphs
// ---------------------------------------------------------------------------------------------

/// The workbook part's own part graph: every part the workbook relates to, resolved once when a
/// [`crate::Workbook`] is opened.
///
/// A singular relationship keeps at most one target, matching what §12.3.23 permits ("*a Workbook
/// part is permitted to have implicit relationships to …*", each listed once); the plural ones keep
/// every match in relationship order. `tests/fixtures/sample.xlsx` carries `theme`, `styles`,
/// `worksheet` and `sharedStrings` and nothing else, so every other field is `None`/empty there.
///
/// The sheets themselves are **not** read off this struct — a `.xlsx` orders its sheets by the
/// `x:sheets` list in the workbook's markup, not by relationship order, and that list is
/// [`crate::Workbook::sheets`]. [`worksheets`](Self::worksheets) and its two siblings are the raw
/// relationship view the validator compares that list against.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkbookParts {
    /// Every related worksheet part, in relationship order.
    pub worksheets: Vec<PartName>,
    /// Every related chartsheet part, in relationship order.
    pub chartsheets: Vec<PartName>,
    /// Every related dialogsheet part, in relationship order.
    pub dialogsheets: Vec<PartName>,
    /// `xl/sharedStrings.xml`, if related.
    pub shared_strings: Option<PartName>,
    /// `xl/styles.xml`, if related.
    pub styles: Option<PartName>,
    /// `xl/theme/themeN.xml`, if related. DrawingML rather than SpreadsheetML — resolved here
    /// because it is still part of the workbook part's own graph.
    pub theme: Option<PartName>,
    /// `xl/calcChain.xml`, if related.
    pub calculation_chain: Option<PartName>,
    /// `xl/connections.xml`, if related.
    pub connections: Option<PartName>,
    /// `xl/metadata.xml`, if related.
    pub metadata: Option<PartName>,
    /// `xl/volatileDependencies.xml`, if related.
    pub volatile_dependencies: Option<PartName>,
    /// Every related external workbook references part, in relationship order.
    pub external_links: Vec<PartName>,
    /// Every related pivot table cache definition, in relationship order.
    pub pivot_cache_definitions: Vec<PartName>,
}

impl WorkbookParts {
    /// Resolves every relationship of `workbook_part` this crate classifies, by type.
    ///
    /// A relationship type not asked for here is simply not visited: it stays untouched in the
    /// package, exactly as [`mjx_opc::Package::authored_xml_parts`] leaves whatever no edit dirties.
    ///
    /// # Errors
    /// Returns [`XlsxError::ExternalTarget`] if one of these relationships is `TargetMode="External"`
    /// (none of SpreadsheetML's own parts ever is), or [`XlsxError::TargetResolution`] if a target
    /// does not resolve to a valid part name.
    pub(crate) fn resolve(package: &Package, workbook_part: &PartName) -> Result<Self, XlsxError> {
        let Some(rels) = package.relationships_for(Some(workbook_part)) else {
            return Ok(Self::default());
        };
        Ok(Self {
            worksheets: many(workbook_part, rels, REL_WORKSHEET)?,
            chartsheets: many(workbook_part, rels, REL_CHARTSHEET)?,
            dialogsheets: many(workbook_part, rels, REL_DIALOGSHEET)?,
            shared_strings: single(workbook_part, rels, REL_SHARED_STRINGS)?,
            styles: single(workbook_part, rels, REL_STYLES)?,
            theme: single(workbook_part, rels, REL_THEME)?,
            calculation_chain: single(workbook_part, rels, REL_CALCULATION_CHAIN)?,
            connections: single(workbook_part, rels, REL_CONNECTIONS)?,
            metadata: single(workbook_part, rels, REL_METADATA)?,
            volatile_dependencies: single(workbook_part, rels, REL_VOLATILE_DEPENDENCIES)?,
            external_links: many(workbook_part, rels, REL_EXTERNAL_LINK)?,
            pivot_cache_definitions: many(workbook_part, rels, REL_PIVOT_CACHE_DEFINITION)?,
        })
    }

    /// Every sheet part the workbook relates to, paired with its kind, in relationship order within
    /// each kind — what [`crate::Workbook::sheets`]'s `x:sheets` list must agree with.
    #[must_use]
    pub fn sheet_parts(&self) -> Vec<(SheetKind, PartName)> {
        let mut parts = Vec::with_capacity(
            self.worksheets.len() + self.chartsheets.len() + self.dialogsheets.len(),
        );
        for (kind, group) in [
            (SheetKind::Worksheet, &self.worksheets),
            (SheetKind::Chartsheet, &self.chartsheets),
            (SheetKind::Dialogsheet, &self.dialogsheets),
        ] {
            parts.extend(group.iter().map(|part| (kind, part.clone())));
        }
        parts
    }
}

/// One sheet part's own part graph — everything that hangs off a worksheet, chartsheet or
/// dialogsheet rather than off the workbook.
///
/// `tests/fixtures/sample.xlsx`'s single worksheet relates to nothing at all, so every field is
/// empty there; a workbook with comments, a chart, an autofilter table or a saved printer
/// configuration fills them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorksheetParts {
    /// The DrawingML drawings part, if related — where a chart or a picture on this sheet lives.
    pub drawing: Option<PartName>,
    /// The legacy VML drawing part, if related — a comment's pop-up box, a form control's look.
    pub vml_drawing: Option<PartName>,
    /// The comments part, if related.
    pub comments: Option<PartName>,
    /// The printer settings part, if related. Opaque bytes, preserved verbatim.
    pub printer_settings: Option<PartName>,
    /// Every related table definition part, in relationship order.
    pub tables: Vec<PartName>,
    /// Every related query table part, in relationship order.
    pub query_tables: Vec<PartName>,
    /// Every related pivot table part, in relationship order.
    pub pivot_tables: Vec<PartName>,
}

impl WorksheetParts {
    /// Resolves every relationship of `sheet_part` this crate classifies, by type.
    ///
    /// # Errors
    /// As [`WorkbookParts::resolve`].
    pub(crate) fn resolve(package: &Package, sheet_part: &PartName) -> Result<Self, XlsxError> {
        let Some(rels) = package.relationships_for(Some(sheet_part)) else {
            return Ok(Self::default());
        };
        Ok(Self {
            drawing: single(sheet_part, rels, REL_DRAWING)?,
            vml_drawing: single(sheet_part, rels, REL_VML_DRAWING)?,
            comments: single(sheet_part, rels, REL_COMMENTS)?,
            printer_settings: single(sheet_part, rels, REL_PRINTER_SETTINGS)?,
            tables: many(sheet_part, rels, REL_TABLE)?,
            query_tables: many(sheet_part, rels, REL_QUERY_TABLE)?,
            pivot_tables: many(sheet_part, rels, REL_PIVOT_TABLE)?,
        })
    }
}

/// The first relationship of `rel_type` from `source`'s own `.rels`, resolved to a part name.
///
/// ECMA-376 permits at most one of each singular relationship type per part, so "first" and "only"
/// coincide for a conformant package; a non-conformant duplicate is not rejected here (see
/// [`crate::Workbook::validate`] for what is).
fn single(
    source: &PartName,
    rels: &Relationships,
    rel_type: &str,
) -> Result<Option<PartName>, XlsxError> {
    let Some(rel) = rels.by_type(rel_type).next() else {
        return Ok(None);
    };
    Ok(Some(resolve_one(source, rel)?))
}

/// Every relationship of `rel_type` from `source`'s own `.rels`, resolved, in relationship order.
fn many(
    source: &PartName,
    rels: &Relationships,
    rel_type: &str,
) -> Result<Vec<PartName>, XlsxError> {
    rels.by_type(rel_type)
        .map(|rel| resolve_one(source, rel))
        .collect()
}

/// Resolves one relationship's target to a part name, rejecting an external one.
fn resolve_one(source: &PartName, rel: &Relationship) -> Result<PartName, XlsxError> {
    if rel.mode == TargetMode::External {
        return Err(XlsxError::ExternalTarget {
            target: rel.target.clone(),
        });
    }
    crate::nav::resolve_target(source, &rel.target)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `PartKind::ALL` really is every variant.
    ///
    /// There is no discriminant count to compare against in stable Rust, so this gets the property a
    /// different way: the `match` in [`PartKind::relationship_type`] is exhaustive by the compiler's
    /// own rule, and the `from_content_type` round trip below only searches `ALL`. A variant added
    /// without an `ALL` row therefore fails *that* case rather than this one — so what this asserts
    /// is the cheap half: no duplicates, and the count matches the enum's documented size.
    #[test]
    fn every_part_kind_is_in_all_exactly_once() {
        let mut seen = std::collections::HashSet::new();
        for kind in PartKind::ALL {
            assert!(
                seen.insert(*kind),
                "{kind:?} is listed twice in PartKind::ALL"
            );
        }
        assert_eq!(
            PartKind::ALL.len(),
            21,
            "PartKind::ALL changed size — update the count and this crate's module documentation"
        );
    }

    /// Every kind pairs a relationship type with at least one content type, and every content type
    /// classifies back to the kind that declared it.
    ///
    /// This is the case a new variant with a missing `ALL` row fails: `from_content_type` searches
    /// `ALL` only, so a kind outside it cannot be found from its own content type.
    #[test]
    fn every_kind_round_trips_through_its_own_content_types() {
        for kind in PartKind::ALL {
            assert!(
                !kind.relationship_type().is_empty(),
                "{kind:?} has no relationship type"
            );
            assert!(
                !kind.content_types().is_empty(),
                "{kind:?} has no content type"
            );
            for content_type in kind.content_types() {
                assert_eq!(
                    PartKind::from_content_type(content_type),
                    Some(*kind),
                    "{content_type} must classify back to {kind:?}"
                );
            }
        }
    }

    /// No two kinds share a content type or a relationship type.
    ///
    /// A collision would silently make one kind unreachable through
    /// [`PartKind::from_content_type`] — and, worse, would make the sheet-kind lookup answer with
    /// whichever variant `ALL` happens to list first.
    #[test]
    fn no_two_part_kinds_share_a_content_type_or_a_relationship_type() {
        let mut content_types = std::collections::HashMap::new();
        let mut relationship_types = std::collections::HashMap::new();
        for kind in PartKind::ALL {
            for content_type in kind.content_types() {
                if let Some(other) = content_types.insert(*content_type, *kind) {
                    panic!("{content_type} is claimed by both {other:?} and {kind:?}");
                }
            }
            if let Some(other) = relationship_types.insert(kind.relationship_type(), *kind) {
                panic!(
                    "{} is claimed by both {other:?} and {kind:?}",
                    kind.relationship_type()
                );
            }
        }
    }

    /// The four constants `tests/fixtures/sample.xlsx` cannot open without are exactly the strings
    /// `mjx-chart`'s own embedded-workbook writer emits.
    ///
    /// `mjx_chart::workbook`'s copies are private, so this pins the literals rather than comparing
    /// symbols — which is the point: MJXOFF-112 removes that module and routes it through here, and
    /// a drift between the two before then would produce a package PowerPoint refuses to open with
    /// no test anywhere noticing.
    #[test]
    fn the_workbook_content_types_match_the_package_powerpoint_and_libreoffice_accept() {
        assert_eq!(
            CONTENT_TYPE_WORKBOOK,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"
        );
        assert_eq!(
            CONTENT_TYPE_WORKSHEET,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"
        );
        assert_eq!(
            CONTENT_TYPE_SHARED_STRINGS,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml"
        );
        assert_eq!(
            CONTENT_TYPE_STYLES,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"
        );
    }

    /// The three sheet kinds classify from their content types, and nothing else does.
    #[test]
    fn only_the_three_sheet_content_types_are_sheets() {
        assert_eq!(
            SheetKind::from_content_type(CONTENT_TYPE_WORKSHEET),
            Some(SheetKind::Worksheet)
        );
        assert_eq!(
            SheetKind::from_content_type(CONTENT_TYPE_CHARTSHEET),
            Some(SheetKind::Chartsheet)
        );
        assert_eq!(
            SheetKind::from_content_type(CONTENT_TYPE_DIALOGSHEET),
            Some(SheetKind::Dialogsheet)
        );
        for not_a_sheet in [
            CONTENT_TYPE_WORKBOOK,
            CONTENT_TYPE_STYLES,
            CONTENT_TYPE_SHARED_STRINGS,
            CONTENT_TYPE_THEME,
            "application/xml",
        ] {
            assert_eq!(
                SheetKind::from_content_type(not_a_sheet),
                None,
                "{not_a_sheet} is not a sheet"
            );
        }
        for kind in [
            SheetKind::Worksheet,
            SheetKind::Chartsheet,
            SheetKind::Dialogsheet,
        ] {
            assert_eq!(
                SheetKind::from_content_type(kind.part_kind().content_types()[0]),
                Some(kind)
            );
        }
    }

    /// A macro-enabled workbook's content type is deliberately unclassified — see this module's own
    /// doc comment for why guessing it would break this project's "never guess a wire token" rule.
    ///
    /// Written as a literal here **precisely because** the constant does not exist: if a later child
    /// finds the string in a normative source and adds it, this case fails and forces the note above
    /// to be updated with it, rather than leaving a stale claim behind.
    #[test]
    fn the_macro_enabled_workbook_content_type_is_not_declared() {
        assert_eq!(
            PartKind::from_content_type("application/vnd.ms-excel.sheet.macroEnabled.main+xml"),
            None,
            "`macroEnabled` appears nowhere in ECMA-376 Parts 1-4; this crate does not invent it"
        );
    }
}
