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

// The four relationship types and four content types a package *this workspace authors* carries are
// declared once, in `mjx_sml::write::constants`, and re-exported here. They were declared twice —
// there and in `mjx-chart` — and would have been three times over the moment MJXOFF-112 wrote a
// third producer; `mjx-sml` is below this crate in the layering, so the one set can live there and
// be reached from both sides. Every other constant below is this crate's own: reading a part graph
// needs twenty-nine relationship types and an author needs four.
pub use mjx_sml::write::constants::{REL_OFFICE_DOCUMENT, REL_WORKSHEET};

/// The relationship type from the workbook part to a chartsheet part (§12.3.2).
pub const REL_CHARTSHEET: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/chartsheet";

/// The relationship type from the workbook part to a dialogsheet part (§12.3.7).
pub const REL_DIALOGSHEET: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/dialogsheet";

pub use mjx_sml::write::constants::{REL_SHARED_STRINGS, REL_STYLES};

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

/// The relationship type binding a **drawing** part to the chart part it frames (MJXOFF-111, E4).
///
/// The edge is from the *drawing* part, not from the sheet: an `a:graphicData`'s `c:chart@r:id` is
/// resolved against the part that contains it, exactly as an `a:blip@r:embed` is. Relating a chart
/// from the worksheet would produce a file Excel opens and repairs.
pub const REL_CHART: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart";

/// The relationship type binding a chart part to an embedded OPC **package** — the workbook Office's
/// *Edit Data* opens (MJXOFF-111, E4).
///
/// A chart on a worksheet normally has none: its `c:f` names a live range in the sheets it lives
/// among, and the cells are the source. One authored from a [`ChartData`](mjx_chart::ChartData)
/// description does, because that description carries values and no cells to point at.
pub const REL_PACKAGE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/package";

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

/// The relationship type of a **hyperlink**, ECMA-376 Part 1 §15.2.9.
///
/// The one relationship in this file that reaches **no part**: its `Target` is an external URI and
/// its `TargetMode` is `External`, so nothing in [`PartKind`] corresponds to it and
/// [`WorksheetParts`] does not resolve one. It is here because
/// [`Workbook::set_cell_hyperlink`](crate::Workbook::set_cell_hyperlink) writes it and
/// [`Workbook::validate`](crate::Workbook::validate) reads it, and those two must agree on the
/// string.
///
/// The same URI `mjx-pptx` declares for the same purpose. It is spelled out in both crates rather
/// than shared, because they are siblings in the format tier and neither may depend on the other.
pub const REL_HYPERLINK: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink";

/// The relationship type from a sheet part to its printer settings part (Part 1 §15.2.13, the
/// *shared* part summary — one Printer Settings part per chartsheet, dialogsheet or worksheet).
pub const REL_PRINTER_SETTINGS: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/printerSettings";

/// The relationship type from a sheet part to an **image** part (ECMA-376 Part 1 §15.2.14).
///
/// In a workbook this is what a sheet's background picture (`x:picture`,
/// [`mjx_sml::SheetBackgroundPicture`]) reaches — `xl/media/imageN.png` and its siblings. The image
/// itself is bytes `mjx-opc` carries verbatim; nothing in this crate decodes one.
///
/// The same URI `mjx-pptx` and `mjx-docx` each declare for the same OPC concept, declared again here
/// for the reason [`REL_THEME`] is: reaching across for it would be a sideways crate edge.
pub const REL_IMAGE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image";

/// The relationship type from the workbook part to a theme part (DrawingML, Part 1 §14.2.7).
///
/// Not SpreadsheetML — the same URI and the same OPC concept `mjx-pptx` and `mjx-docx` each declare
/// for themselves, declared again here for the same reason they do: reaching across for it would be
/// a sideways crate edge.
pub const REL_THEME: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme";

/// The relationship type from a **worksheet** part to a Custom Property part (§12.3.5).
///
/// §12.3's summary table says the relationship source is the Workbook part; §12.3.5's own body and
/// its example both say the Worksheet part, and every file this project has read agrees with the
/// body. The two statements are the specification's, quoted rather than reconciled, and
/// [`WorksheetParts::custom_properties`] follows the body.
pub const REL_CUSTOM_PROPERTY: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/customProperty";

/// The relationship type from the workbook part to the Custom XML Mappings part (§12.3.6).
pub const REL_CUSTOM_XML_MAPPINGS: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/xmlMaps";

/// The relationship type from the workbook part to the Shared Workbook Revision Headers part
/// (§12.3.16).
pub const REL_REVISION_HEADERS: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/revisionHeaders";

/// The relationship type from the revision headers part to one Shared Workbook Revision Log part
/// (§12.3.17) — an **explicit** relationship, named by a `x:header@r:id`.
pub const REL_REVISION_LOG: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/revisionLog";

/// The relationship type from the workbook part to the Shared Workbook User Data part (§12.3.18).
///
/// The URI's last segment is `usernames`, all lower case, where the part's content type spells the
/// same word `userNames`. Both spellings are §12.3.18's own, quoted exactly.
pub const REL_SHARED_WORKBOOK_USER_DATA: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/usernames";

/// The relationship type from a worksheet part to its Single Cell Table Definitions part
/// (§12.3.19).
pub const REL_SINGLE_CELL_TABLE_DEFINITIONS: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/tableSingleCells";

// ---------------------------------------------------------------------------------------------
// Content types (ECMA-376 Part 1 §12.3)
// ---------------------------------------------------------------------------------------------

pub use mjx_sml::write::constants::CONTENT_TYPE_WORKBOOK;

/// The content type of the workbook part of a spreadsheet *template* (§12.3.23, second of the two).
pub const CONTENT_TYPE_WORKBOOK_TEMPLATE: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.template.main+xml";

pub use mjx_sml::write::constants::CONTENT_TYPE_WORKSHEET;

/// The content type of a chartsheet part (§12.3.2).
pub const CONTENT_TYPE_CHARTSHEET: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.chartsheet+xml";

/// The content type of a dialogsheet part (§12.3.7).
pub const CONTENT_TYPE_DIALOGSHEET: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.dialogsheet+xml";

pub use mjx_sml::write::constants::{CONTENT_TYPE_SHARED_STRINGS, CONTENT_TYPE_STYLES};

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

/// The content type of a DrawingML **chart** part (ECMA-376 Part 1 §14.2.1) — MJXOFF-111 (E4).
///
/// DrawingML, not SpreadsheetML: `xl/charts/chartN.xml` is a `c:chartSpace`, the same part a
/// `.pptx` and a `.docx` carry, which is why `mjx-chart` models it once for all three formats.
pub const CONTENT_TYPE_CHART: &str =
    "application/vnd.openxmlformats-officedocument.drawingml.chart+xml";

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

/// A content type a Custom Property part (§12.3.5) may be registered under.
///
/// §12.3.5 states the content type as *"Any content, support for which is application-defined"* and
/// then offers two examples in a Note: this string, and `application/xml`. So this is **an**
/// example the specification gives rather than **the** content type of the part, which is why
/// [`PartKind::CustomProperty`] is identified by its relationship type and not by this string. It
/// is declared because `[Content_Types].xml` has to register the part under *something* and this is
/// what real producers write; `tests/fixtures/hyperlinks.xlsx` carries one.
pub const CONTENT_TYPE_CUSTOM_PROPERTY: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.customProperty";

/// The content type of the Custom XML Mappings part (§12.3.6) — plain `application/xml`, the one
/// content type in this file that names no format at all.
///
/// See [`AMBIGUOUS_CONTENT_TYPES`] for why a part is never classified as XML maps *by* this string.
pub const CONTENT_TYPE_CUSTOM_XML_MAPPINGS: &str = "application/xml";

/// The content type of the Shared Workbook Revision Headers part (§12.3.16).
pub const CONTENT_TYPE_REVISION_HEADERS: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.revisionHeaders+xml";

/// The content type of a Shared Workbook Revision Log part (§12.3.17).
pub const CONTENT_TYPE_REVISION_LOG: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.revisionLog+xml";

/// The content type of the Shared Workbook User Data part (§12.3.18).
pub const CONTENT_TYPE_SHARED_WORKBOOK_USER_DATA: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.userNames+xml";

/// The content type of a Single Cell Table Definitions part (§12.3.19).
pub const CONTENT_TYPE_SINGLE_CELL_TABLE_DEFINITIONS: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.tableSingleCells+xml";

/// Content types that identify **no single part kind**, each with the reason, so that
/// [`PartKind::from_content_type`] can refuse to guess from one.
///
/// The list exists because two of §12.3's part types are not identified by their content type at
/// all, and answering from one anyway would be a confident wrong answer rather than a missing one.
/// A part registered under one of these is classified through
/// [`PartKind::from_relationship_type`] instead — the edge the graph actually reaches it through,
/// which is how §12.3 identifies every one of its part types in the first place.
pub const AMBIGUOUS_CONTENT_TYPES: &[(&str, &str)] = &[(
    CONTENT_TYPE_CUSTOM_XML_MAPPINGS,
    "§12.3.6 gives the Custom XML Mappings part this content type and §12.3.5's Note offers it as \
     one a Custom Property part may carry. It is also the string `[Content_Types].xml` most often \
     registers as the `Default` for the `xml` extension, so in a package written by a real \
     producer it is the content type of every part carrying no `Override` at all — a workbook's \
     `docProps/custom.xml`, an embedded custom XML item, a producer's own scratch part. Answering \
     `XmlMaps` from it would misclassify all of them.",
)];

// ---------------------------------------------------------------------------------------------
// Part kinds
// ---------------------------------------------------------------------------------------------

/// One kind of part a workbook package holds, in the sense that it names both a relationship type
/// (how the graph reaches it) and one or more content types (how `[Content_Types].xml` registers
/// it).
///
/// # Every §12.3 part type is here
///
/// The set is **all twenty-four part types ECMA-376 Part 1 §12.3 defines** (§12.3.1 through
/// §12.3.24, [`Drawing`](Self::Drawing) at §12.3.8 among them), plus the three a workbook relates
/// to that §12.3 does not define: [`Theme`](Self::Theme) (§14.2.7),
/// [`PrinterSettings`](Self::PrinterSettings) (§15.2.13) and [`VmlDrawing`](Self::VmlDrawing)
/// (Part 4 §8.2). Twenty-seven in all, counting [`Workbook`](Self::Workbook) once for the two
/// content types §12.3.23 gives it.
///
/// MJXOFF-91 (D02) left six of the twenty-four out: Custom Property, Custom XML Mappings, the
/// three Shared Workbook parts and Single Cell Table Definitions. MJXOFF-133 (D18) puts them in,
/// because *"we do not model it"* and *"we do not recognise it"* are different statements and only
/// the first is a guarantee. **Recognising a part is not modelling it:** every one of these is
/// still carried through a save as the bytes it arrived as — see [`crate::preserve`] and
/// `crates/mjx-xlsx/docs/guide/fidelity_and_the_part_graph.md`'s unmodelled-cluster table.
///
/// # Two of them are not identified by their content type
///
/// [`CustomProperty`](Self::CustomProperty)'s content type is, in §12.3.5's own words, *"any
/// content, support for which is application-defined"* — so the string
/// [`CONTENT_TYPE_CUSTOM_PROPERTY`] is one example of what such a part may carry rather than a
/// guarantee, and a producer registering the same part as `text/plain` has broken no rule.
/// [`CustomXmlMappings`](Self::CustomXmlMappings)'s is plain `application/xml`, which in a real
/// package is also the `Default` for every unregistered `.xml` part; classifying from it would
/// misidentify unrelated parts, so it is on [`AMBIGUOUS_CONTENT_TYPES`] and
/// [`from_content_type`](Self::from_content_type) refuses to answer from it.
///
/// Both are therefore identified by [`from_relationship_type`](Self::from_relationship_type) — the
/// edge §12.3 identifies every one of its part types by — and
/// [`crate::preserve::classify`] tries it whenever a content type answers nothing.
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
    /// `c:chartSpace` — a DrawingML chart part (ECMA-376 Part 1 §14.2.1), reached from a *drawing*
    /// part rather than from a sheet (MJXOFF-111, E4).
    ///
    /// DrawingML, not SpreadsheetML, and identified by its content type: it is the same part a
    /// `.pptx` and a `.docx` carry, modelled once by [`mjx_chart`] for all three formats.
    Chart,
    /// A legacy VML drawing part (Part 4 §8.2) — a comment's pop-up box, a form control's look.
    VmlDrawing,
    /// A printer settings part (§15.2.13). Opaque bytes, never XML.
    PrinterSettings,
    /// `a:theme` — a theme part (§14.2.7). DrawingML, not SpreadsheetML.
    Theme,
    /// A Custom Property part (§12.3.5) — user-defined data hung off a worksheet, of **any**
    /// content the producing application cares to write. Never opened here.
    ///
    /// Identified by [`REL_CUSTOM_PROPERTY`] and not by a content type; see the enum's own
    /// documentation.
    CustomProperty,
    /// `x:MapInfo` — the Custom XML Mappings part (§12.3.6), which says how a custom XML schema is
    /// mapped into cells.
    ///
    /// SpreadsheetML markup registered under `application/xml`, so identified by
    /// [`REL_CUSTOM_XML_MAPPINGS`] and not by a content type; see the enum's own documentation.
    CustomXmlMappings,
    /// `x:headers` — the Shared Workbook Revision Headers part (§12.3.16): one entry per editing
    /// session, each naming a revision log.
    RevisionHeaders,
    /// `x:revisions` — one Shared Workbook Revision Log part (§12.3.17), the cell edits of one
    /// session.
    RevisionLog,
    /// `x:users` — the Shared Workbook User Data part (§12.3.18), the list of users sharing the
    /// workbook.
    SharedWorkbookUserData,
    /// `x:singleXmlCells` — a Single Cell Table Definitions part (§12.3.19): how non-repeating
    /// custom XML elements map into individual cells of one worksheet.
    SingleCellTableDefinitions,
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
            Self::Chart => REL_CHART,
            Self::VmlDrawing => REL_VML_DRAWING,
            Self::PrinterSettings => REL_PRINTER_SETTINGS,
            Self::Theme => REL_THEME,
            Self::CustomProperty => REL_CUSTOM_PROPERTY,
            Self::CustomXmlMappings => REL_CUSTOM_XML_MAPPINGS,
            Self::RevisionHeaders => REL_REVISION_HEADERS,
            Self::RevisionLog => REL_REVISION_LOG,
            Self::SharedWorkbookUserData => REL_SHARED_WORKBOOK_USER_DATA,
            Self::SingleCellTableDefinitions => REL_SINGLE_CELL_TABLE_DEFINITIONS,
        }
    }

    /// The kind a part reached through a relationship of this type is, or `None` for a relationship
    /// type this crate does not classify.
    ///
    /// The exact inverse of [`relationship_type`](Self::relationship_type), and the *only* way to
    /// identify the two kinds whose content type says nothing —
    /// [`CustomProperty`](Self::CustomProperty) and
    /// [`CustomXmlMappings`](Self::CustomXmlMappings). It is well defined because no two kinds
    /// share a relationship type, which
    /// `no_two_part_kinds_share_a_content_type_or_a_relationship_type` asserts rather than assumes.
    ///
    /// `None` for [`REL_HYPERLINK`], which reaches no part at all.
    #[must_use]
    pub fn from_relationship_type(relationship_type: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|kind| kind.relationship_type() == relationship_type)
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
            Self::Chart => &[CONTENT_TYPE_CHART],
            Self::VmlDrawing => &[CONTENT_TYPE_VML_DRAWING],
            Self::PrinterSettings => &[CONTENT_TYPE_PRINTER_SETTINGS],
            Self::Theme => &[CONTENT_TYPE_THEME],
            Self::CustomProperty => &[CONTENT_TYPE_CUSTOM_PROPERTY],
            Self::CustomXmlMappings => &[CONTENT_TYPE_CUSTOM_XML_MAPPINGS],
            Self::RevisionHeaders => &[CONTENT_TYPE_REVISION_HEADERS],
            Self::RevisionLog => &[CONTENT_TYPE_REVISION_LOG],
            Self::SharedWorkbookUserData => &[CONTENT_TYPE_SHARED_WORKBOOK_USER_DATA],
            Self::SingleCellTableDefinitions => &[CONTENT_TYPE_SINGLE_CELL_TABLE_DEFINITIONS],
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
        Self::Chart,
        Self::VmlDrawing,
        Self::PrinterSettings,
        Self::Theme,
        Self::CustomProperty,
        Self::CustomXmlMappings,
        Self::RevisionHeaders,
        Self::RevisionLog,
        Self::SharedWorkbookUserData,
        Self::SingleCellTableDefinitions,
    ];

    /// The kind a part with this content type is, or `None` for a content type this crate does not
    /// classify — **or one that classifies nothing**, see [`AMBIGUOUS_CONTENT_TYPES`].
    ///
    /// `None` is never a rejection — see [`crate::PartClassification`]. It is what a `.xlsm`'s
    /// macro-enabled workbook and an image both report, and what
    /// [`crate::preserve::classify`] then tries [`from_relationship_type`](Self::from_relationship_type)
    /// on before giving up.
    #[must_use]
    pub fn from_content_type(content_type: &str) -> Option<Self> {
        if AMBIGUOUS_CONTENT_TYPES
            .iter()
            .any(|(ambiguous, _)| *ambiguous == content_type)
        {
            return None;
        }
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
    /// `xl/xmlMaps.xml`, if related — the Custom XML Mappings part (§12.3.6), of which §12.3.6
    /// permits at most one per package.
    pub custom_xml_mappings: Option<PartName>,
    /// `xl/revisions/revisionHeaders.xml`, if related — §12.3.16 permits at most one, and its
    /// presence is what says this workbook is in shared mode.
    pub revision_headers: Option<PartName>,
    /// `xl/revisions/userNames.xml`, if related — §12.3.18 permits at most one.
    pub shared_workbook_user_data: Option<PartName>,
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
            custom_xml_mappings: single(workbook_part, rels, REL_CUSTOM_XML_MAPPINGS)?,
            revision_headers: single(workbook_part, rels, REL_REVISION_HEADERS)?,
            shared_workbook_user_data: single(workbook_part, rels, REL_SHARED_WORKBOOK_USER_DATA)?,
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
/// configuration fills them. `tests/fixtures/print_and_sheet_kinds.xlsx` fills
/// [`printer_settings`](Self::printer_settings) and [`background_image`](Self::background_image).
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
    /// The image part this sheet's background picture (`x:picture`) draws, if related. Opaque
    /// bytes, preserved verbatim — nothing here decodes an image.
    ///
    /// This is the *part graph* view: it says the sheet relates to an image, not that the sheet's
    /// `x:picture` names that relationship. [`crate::Workbook::sheet_background_image`] answers the
    /// second question, by reading the `r:id` off the markup.
    pub background_image: Option<PartName>,
    /// Every related table definition part, in relationship order.
    pub tables: Vec<PartName>,
    /// Every related query table part, in relationship order.
    pub query_tables: Vec<PartName>,
    /// Every related pivot table part, in relationship order.
    pub pivot_tables: Vec<PartName>,
    /// Every related Custom Property part (§12.3.5), in relationship order. Opaque bytes of a
    /// content type the specification leaves entirely to the producer; nothing here opens one.
    ///
    /// The sheet's own `x:customProperties` list ([`mjx_sml::CustomProperties`]) is the other half:
    /// it holds the `customPr@r:id` and the name the sheet gives each one. This is the part-graph
    /// view.
    pub custom_properties: Vec<PartName>,
    /// The Single Cell Table Definitions part (§12.3.19), if related — at most one per worksheet.
    pub single_cell_table_definitions: Option<PartName>,
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
            background_image: single(sheet_part, rels, REL_IMAGE)?,
            tables: many(sheet_part, rels, REL_TABLE)?,
            query_tables: many(sheet_part, rels, REL_QUERY_TABLE)?,
            pivot_tables: many(sheet_part, rels, REL_PIVOT_TABLE)?,
            custom_properties: many(sheet_part, rels, REL_CUSTOM_PROPERTY)?,
            single_cell_table_definitions: single(
                sheet_part,
                rels,
                REL_SINGLE_CELL_TABLE_DEFINITIONS,
            )?,
        })
    }
}

/// A pivot table part's own part graph: the cache definition it draws its data from (§12.3.11 —
/// *"A Pivot Table part shall have an implicit relationship to a Pivot Table Cache Definition
/// part"*).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PivotTableParts {
    /// The pivot table cache definition this table reads, if related.
    pub cache_definition: Option<PartName>,
}

impl PivotTableParts {
    /// Resolves the relationships of the pivot table part at `pivot_table_part`.
    ///
    /// # Errors
    /// As [`WorkbookParts::resolve`].
    pub(crate) fn resolve(
        package: &Package,
        pivot_table_part: &PartName,
    ) -> Result<Self, XlsxError> {
        let Some(rels) = package.relationships_for(Some(pivot_table_part)) else {
            return Ok(Self::default());
        };
        Ok(Self {
            cache_definition: single(pivot_table_part, rels, REL_PIVOT_CACHE_DEFINITION)?,
        })
    }
}

/// A pivot table cache definition's own part graph: the records part holding the cached rows
/// (§12.3.13).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PivotCacheParts {
    /// The cache records part, if related. Absent for a cache saved with `saveData="0"`, which is a
    /// real shape and not a defect.
    pub records: Option<PartName>,
}

impl PivotCacheParts {
    /// Resolves the relationships of the cache definition part at `cache_definition_part`.
    ///
    /// # Errors
    /// As [`WorkbookParts::resolve`].
    pub(crate) fn resolve(
        package: &Package,
        cache_definition_part: &PartName,
    ) -> Result<Self, XlsxError> {
        let Some(rels) = package.relationships_for(Some(cache_definition_part)) else {
            return Ok(Self::default());
        };
        Ok(Self {
            records: single(cache_definition_part, rels, REL_PIVOT_CACHE_RECORDS)?,
        })
    }
}

/// The revision headers part's own part graph: one revision log per editing session (§12.3.17).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RevisionHeadersParts {
    /// Every related revision log part, in relationship order.
    ///
    /// Relationship order, not `x:header` order: the headers part's own list is what orders the
    /// sessions, and [`crate::RevisionState`] reads it.
    pub logs: Vec<PartName>,
}

impl RevisionHeadersParts {
    /// Resolves the relationships of the revision headers part at `revision_headers_part`.
    ///
    /// # Errors
    /// As [`WorkbookParts::resolve`].
    pub(crate) fn resolve(
        package: &Package,
        revision_headers_part: &PartName,
    ) -> Result<Self, XlsxError> {
        let Some(rels) = package.relationships_for(Some(revision_headers_part)) else {
            return Ok(Self::default());
        };
        Ok(Self {
            logs: many(revision_headers_part, rels, REL_REVISION_LOG)?,
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
            28,
            "PartKind::ALL changed size — update the count and this crate's module documentation"
        );
    }

    /// Every kind is reachable from its own relationship type, and the two whose content type says
    /// nothing are reachable **only** that way.
    ///
    /// This is the case MJXOFF-133 rests on: a Custom Property part carries "any content" and a
    /// Custom XML Mappings part carries `application/xml`, so if `from_relationship_type` did not
    /// exist neither could be identified at all, and both would report
    /// [`crate::PartClassification::Unclassified`] in a workbook that plainly relates to them.
    #[test]
    fn every_kind_round_trips_through_its_own_relationship_type() {
        for kind in PartKind::ALL {
            assert_eq!(
                PartKind::from_relationship_type(kind.relationship_type()),
                Some(*kind),
                "{kind:?} is not reachable from its own relationship type"
            );
        }
        assert_eq!(
            PartKind::from_content_type(CONTENT_TYPE_CUSTOM_XML_MAPPINGS),
            None,
            "CustomXmlMappings must not be identified by application/xml: see \
             AMBIGUOUS_CONTENT_TYPES and §12.3.6"
        );
        // A hyperlink relationship reaches no part, so it names no kind.
        assert_eq!(PartKind::from_relationship_type(REL_HYPERLINK), None);
    }

    /// Every [`AMBIGUOUS_CONTENT_TYPES`] row is a content type some kind really declares, carries a
    /// reason, and is refused by [`PartKind::from_content_type`].
    ///
    /// The allowlist rule MJXOFF-110 established, applied to this list: an entry that named a
    /// content type no kind uses would be a rule guarding nothing, and one with an empty reason
    /// would be the unexplained skip the schema gate exists to reject.
    #[test]
    fn every_ambiguous_content_type_is_declared_by_a_kind_and_refused() {
        assert!(!AMBIGUOUS_CONTENT_TYPES.is_empty());
        for (content_type, reason) in AMBIGUOUS_CONTENT_TYPES {
            assert!(
                PartKind::ALL
                    .iter()
                    .any(|kind| kind.content_types().contains(content_type)),
                "{content_type} is on the ambiguous list but no PartKind declares it — the entry \
                 guards nothing"
            );
            assert!(
                reason.len() > 40,
                "{content_type} is refused without a written reason"
            );
            assert_eq!(
                PartKind::from_content_type(content_type),
                None,
                "{content_type} is on the ambiguous list and must classify to nothing"
            );
        }
    }

    /// The one content type on the ambiguous list still classifies through its relationship.
    ///
    /// Without this the previous case could be satisfied by a kind nothing can ever identify.
    #[test]
    fn a_part_registered_as_plain_xml_is_still_identified_by_its_relationship() {
        assert_eq!(
            PartKind::from_content_type(CONTENT_TYPE_CUSTOM_XML_MAPPINGS),
            None
        );
        assert_eq!(
            PartKind::from_relationship_type(REL_CUSTOM_XML_MAPPINGS),
            Some(PartKind::CustomXmlMappings)
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
                if AMBIGUOUS_CONTENT_TYPES
                    .iter()
                    .any(|(ambiguous, _)| ambiguous == content_type)
                {
                    // Declared by this kind, but identifying nothing — the round trip runs through
                    // `from_relationship_type` instead, which
                    // `every_kind_round_trips_through_its_own_relationship_type` asserts.
                    continue;
                }
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
