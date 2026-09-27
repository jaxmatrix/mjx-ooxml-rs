//! `mjx-xlsx` — the SpreadsheetML **package**: the container, the part graph, and the [`Workbook`] a
//! caller holds.
//!
//! **Start at [`guide`]** — the Excel guide set, one page per feature area, including the two a
//! caller wants before meeting a surprise: what a big sheet costs, and what this library
//! deliberately refuses to do to a workbook.
//!
//! The entry point is [`Workbook`]: open a `.xlsx`'s container bytes with [`Workbook::open`], read
//! its tabs with [`Workbook::sheets`] and its part graph with [`Workbook::parts`], and save with
//! [`Workbook::save`]. A worksheet's cells are read with [`Workbook::worksheet_markup`] and one is
//! written with [`Workbook::set_cell_value`]. Everything this crate does not model — eight of
//! `CT_Worksheet`'s thirty-nine slots, and the nine `sml.xsd` clusters [`Workbook::preserved_parts`]
//! names — is preserved verbatim by the OPC copy-on-write layer.
//!
//! ```no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let bytes = std::fs::read("book.xlsx")?;
//! let workbook = mjx_xlsx::Workbook::open(&bytes)?;
//! for sheet in workbook.sheets() {
//!     println!("{} -> {:?}", sheet.name, sheet.part);
//! }
//! let saved = workbook.save()?;
//! # let _ = saved;
//! # Ok(())
//! # }
//! ```
//!
//! # Excel is two crates
//!
//! **`mjx-sml` owns `sml.xsd` content; `mjx-xlsx` owns OPC structure.** A cell, a row, a shared
//! string, an `xf` — what they *are* — belong to [`mjx_sml`], in the shared-markup tier, because an
//! embedded workbook inside a `.pptx` or a `.docx` is SpreadsheetML too and `mjx-chart` must be able
//! to reach it without an upward or sideways crate edge. Parts, content types, relationships, the
//! ZIP and the [`Workbook`] surface are this crate's, in the format tier beside `mjx-pptx` and
//! `mjx-docx`. `xtask/tests/layering.rs` checks that this crate depends on `mjx-sml` and that
//! nothing at or below the shared-markup tier depends on this one.
//!
//! # Status — the package spine, and nothing else
//!
//! MJXOFF-91 (D02) builds the package, the part graph, and a workbook that opens and saves without
//! touching a byte. MJXOFF-112 (D10) adds [`Workbook::blank`] and the authoring surface beside it —
//! [`Workbook::add_sheet`], [`Workbook::set_cell_style`], [`Workbook::intern_shared_string`] and the
//! four `append_*` methods — every part of which is markup [`mjx_sml::write`] writes.
//! MJXOFF-100 (D06) adds the first model: `xl/workbook.xml`, through
//! [`mjx_sml::WorkbookPart`], plus the navigation surface over it — [`Workbook::sheets`],
//! [`Workbook::sheet_by_name`], [`Workbook::defined_names`], [`Workbook::date_system`] and
//! [`Workbook::rename_sheet`]. See `crates/mjx-xlsx/src/workbook/mod.rs`'s and
//! `crates/mjx-xlsx/src/worksheet/mod.rs`'s own module documentation for the file-by-file map of
//! which later Phase D child fills what, and [`crate::preserve`] for the fidelity contract
//! everything here rests on.
//!
//! MJXOFF-127 (D16) adds hyperlinks — [`Workbook::sheet_hyperlinks`], [`Workbook::cell_hyperlink`],
//! [`Workbook::set_cell_hyperlink`] and [`Workbook::remove_cell_hyperlink`] — where the whole point
//! of the tier is that **a hyperlink and its relationship are one thing**: the entry is in the
//! worksheet and an external target is in the sheet's `.rels`, and neither half is written or
//! removed without the other.
//!
//! MJXOFF-133 (D18) adds the identification surface over the **half of `sml.xsd` this project
//! deliberately does not model** — pivot tables and their caches, external links, connections,
//! query tables, XML maps, cell metadata, volatile dependencies, single-cell tables and the
//! shared-workbook revision parts, 184 of `sml.xsd`'s 367 complex types between them.
//! [`Workbook::preserved_parts`] inventories them, [`Workbook::pivot_tables`],
//! [`Workbook::external_links`], [`Workbook::connections`], [`Workbook::query_tables`],
//! [`Workbook::xml_maps`] and [`Workbook::revision_state`] report what each says, and **not one of
//! them is modelled**: see `crates/mjx-xlsx/docs/guide/fidelity_and_the_part_graph.md` for the
//! cluster-by-cluster table and [`mjx_sml::preserved`] for why the pivot cluster is a phase of its
//! own rather than a gap.

mod authoring;
mod blank;
pub mod effective_properties;
mod error;
pub mod guide;
mod nav;
pub mod parts;
pub mod preserve;
mod validate;
mod workbook;
mod worksheet;

pub use error::XlsxError;
pub use parts::{
    PartKind, PivotCacheParts, PivotTableParts, RevisionHeadersParts, SheetKind, WorkbookParts,
    WorksheetParts,
};
pub use preserve::{PartClassification, PartInventoryEntry};
pub use validate::SpreadsheetDefect;
pub use workbook::{
    CalculationSettings, DateSystem, DefinedNameEntry, DefinedNameScope, PreservedParts,
    RevisionState, Sheet, SheetPivotTable, SheetQueryTable, Workbook, WorkbookConnection,
    WorkbookExternalLink, WorkbookWindow, WorkbookXmlMaps,
};
pub use worksheet::chart_ranges::{
    RangeCellValue, RangeProblem, ResolvedArea, ResolvedRange, ResolvedRangeCell,
};
pub use worksheet::charts::{
    ChartSeriesFreshness, SheetChartSeries, SheetChartSource, SheetChartWorkbook,
};
pub use worksheet::comments::{CommentBox, SheetComment};
pub use worksheet::drawings::{AnchorCell, AnchorPlacement, SheetDrawing, SheetDrawingObject};

/// The three DrawingML types this crate's own drawing surface takes as arguments.
///
/// [`Workbook::add_two_cell_anchored_picture`], [`Workbook::add_one_cell_anchored_picture`] and
/// [`Workbook::add_absolute_anchored_picture`] each take one or two of them, so a caller that
/// cannot name them cannot call those methods — which forced a `mjx-dml` dependency on every
/// consumer of this crate's drawing surface, including one that has stated in its own manifest that
/// it declares no such edge. Re-exported rather than mirrored: a second `CellMarker` would be a
/// second answer to *where is this object anchored*.
pub mod drawing_geometry {
    pub use mjx_dml::spreadsheet_drawing::CellMarker;
    pub use mjx_dml::{Position, Size};
    pub use mjx_ooxml_types::spreadsheetdrawing::ResizingBehavior;
}
pub use worksheet::formatting::{SheetFormatResolver, SheetFormatting};
pub use worksheet::hyperlinks::{HyperlinkKind, HyperlinkTarget, SheetHyperlink};
pub use worksheet::print::SheetMarkup;
pub use worksheet::tables::{SheetTable, SheetTableColumn};
pub use worksheet::Worksheet;

/// Re-exported so that a caller who holds a [`Workbook`] can name what it is built on without
/// declaring `mjx-opc` themselves — the same courtesy `mjx-pptx` extends for the same types.
pub use mjx_opc::{OpcError, Package, PartName, TargetMode};
