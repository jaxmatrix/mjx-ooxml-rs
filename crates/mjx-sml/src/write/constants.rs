//! The part names, content types and relationship types a SpreadsheetML package is assembled from.
//!
//! # Why these live here and not beside the writer that uses them
//!
//! They are the workspace's **one** set for authoring a workbook package. `mjx-chart`'s
//! `crates/mjx-chart/src/workbook.rs` declares its own copies (`workbook.rs:55–80`) because it was
//! written before this crate existed; MJXOFF-99 removes that module and with it those copies.
//! `crates/mjx-xlsx/src/parts.rs` re-exports the eight that overlap — four content types and four
//! relationship types — rather than declaring them a third time, so a producer, a reader and a chart
//! all name the same `&'static str`.
//!
//! The full relationship-type and content-type vocabulary — pivot caches, query tables, printer
//! settings and the twenty other part kinds a workbook can relate to — stays in
//! [`mjx_xlsx::parts`](https://docs.rs/mjx-xlsx), because reading a part graph is that crate's job
//! and *authoring* one is this module's. What is here is exactly what a package this crate writes
//! puts in `[Content_Types].xml` and in a `.rels`.
//!
//! # Sources
//!
//! Every content type and every relationship type below is from ECMA-376 Part 1 §12.3's clause for
//! the part it names, and each carries its clause number. The workbook package's own content type
//! ([`CONTENT_TYPE_WORKBOOK_PACKAGE`]) is not in that table at all — it is the `.xlsx` *file*'s
//! type, which matters here because an embedded workbook inside a `.pptx` is registered by it in
//! the **host** package's `[Content_Types].xml`.

/// The XML declaration every part this module's writers emit begins with — the same one Office
/// writes, and the same one `mjx-pptx`'s and `mjx-docx`'s `blank` constructors use.
pub const XML_DECLARATION: &str = concat!(
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
    "\n"
);

/// The default sheet name, and the one a chart's synthesized `c:f` formulas (`Sheet1!$A$2:$A$4`)
/// qualify their ranges with.
///
/// Identical to `mjx_chart::DEFAULT_SHEET_NAME`, which MJXOFF-99 removes in favour of this.
pub const DEFAULT_SHEET_NAME: &str = "Sheet1";

/// `/xl/workbook.xml` — the part the package root's `officeDocument` relationship names.
pub const WORKBOOK_PART: &str = "/xl/workbook.xml";

/// `/xl/sharedStrings.xml`.
pub const SHARED_STRINGS_PART: &str = "/xl/sharedStrings.xml";

/// `/xl/styles.xml`.
pub const STYLES_PART: &str = "/xl/styles.xml";

/// The name of the `index`-th worksheet part, one-based on the wire: `/xl/worksheets/sheet1.xml`.
///
/// Nothing in OPC requires this spelling — a sheet is found through its relationship, never through
/// its name (see `crates/mjx-sml/src/workbook/sheets.rs`) — but it is what every producer writes,
/// and a package this crate authors is easier to read in a ZIP listing for matching it.
#[must_use]
pub fn worksheet_part_name(index: usize) -> String {
    format!("/xl/worksheets/sheet{}.xml", index.saturating_add(1))
}

/// The target a worksheet part is related from `/xl/workbook.xml` by — relative to `/xl/`, which is
/// the directory the source part sits in.
#[must_use]
pub fn worksheet_relationship_target(index: usize) -> String {
    format!("worksheets/sheet{}.xml", index.saturating_add(1))
}

/// The content type of the workbook **package** — the `.xlsx` file itself, as the *host* package
/// must register it for `/ppt/embeddings/*.xlsx`.
///
/// Not a part's content type: nothing inside a workbook carries this. It is here because the one
/// caller that embeds a workbook in another document needs it, and because keeping it beside the
/// part types is what stops a second copy being written next to that caller.
pub const CONTENT_TYPE_WORKBOOK_PACKAGE: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";

/// The content type of the workbook part of a spreadsheet document (ECMA-376 Part 1 §12.3.23,
/// first of the two the clause lists; the second is the *template*'s, which this crate never
/// authors).
pub const CONTENT_TYPE_WORKBOOK: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml";

/// The content type of a worksheet part (§12.3.24).
pub const CONTENT_TYPE_WORKSHEET: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml";

/// The content type of the shared string table (§12.3.15).
pub const CONTENT_TYPE_SHARED_STRINGS: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml";

/// The content type of the styles part (§12.3.20).
pub const CONTENT_TYPE_STYLES: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml";

/// The relationship type from the package root to the workbook part (§12.3.23).
pub const REL_OFFICE_DOCUMENT: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";

/// The relationship type from the workbook part to a worksheet part (§12.3.24).
pub const REL_WORKSHEET: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet";

/// The relationship type from the workbook part to the shared string table (§12.3.15).
pub const REL_SHARED_STRINGS: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings";

/// The relationship type from the workbook part to the styles part (§12.3.20).
pub const REL_STYLES: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles";
