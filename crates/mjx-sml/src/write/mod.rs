//! Writing SpreadsheetML back out — the parts, and the whole package.
//!
//! # What this is
//!
//! The package writer that replaces
//! [`mjx_chart::EmbeddedWorkbook`](https://docs.rs/mjx-chart) — the workspace's one deliberate
//! duplicate, which exists only because this crate did not. It authors `[Content_Types].xml`,
//! `_rels/.rels`, `xl/workbook.xml`, `xl/_rels/workbook.xml.rels`, `xl/worksheets/sheetN.xml`,
//! `xl/sharedStrings.xml` and `xl/styles.xml`, and optionally `docProps/core.xml` and
//! `docProps/app.xml`.
//!
//! Start at [`WorkbookPackage`]. Everything else here is one of its parts.
//!
//! ```
//! use mjx_sml::write::{AuthoredCellValue, WorkbookPackage};
//!
//! # fn main() -> Result<(), mjx_sml::SmlError> {
//! let mut workbook = WorkbookPackage::new()?;
//! workbook.push_row(0, &[
//!     AuthoredCellValue::Blank,
//!     AuthoredCellValue::SharedText("Revenue".to_owned()),
//! ])?;
//! workbook.push_row(0, &[
//!     AuthoredCellValue::SharedText("Q1".to_owned()),
//!     AuthoredCellValue::Number(19.2),
//! ])?;
//! workbook.recompute_dimensions();
//!
//! let bytes = workbook.to_package_bytes()?;
//! assert_eq!(&bytes[..2], b"PK", "a workbook is a ZIP package");
//! # Ok(())
//! # }
//! ```
//!
//! # Why it is here and not in `mjx-xlsx`
//!
//! `mjx-chart` (rank 2.2) has to reach it, and `mjx-xlsx` is rank 3.0 — an upward edge, which
//! `CLAUDE.md` forbids. `mjx-chart → mjx-sml` (2.2 → 2.1) points down, and that is the whole reason
//! MJXOFF-132 created this crate. `xtask/tests/layering.rs` checks the direction mechanically;
//! `crates/mjx-sml/tests/package_writer.rs` proves the writer needs nothing above it by exercising
//! it from a crate whose dependency graph holds no `mjx-xlsx` at all.
//!
//! The division with `mjx-xlsx` is unchanged by any of this: **`mjx-sml` owns `sml.xsd` content,
//! `mjx-xlsx` owns the `Workbook` a caller holds.** [`WorkbookPackage`] is a *writer* — a thing you
//! fill and then serialize once — and [`mjx_xlsx::Workbook`](https://docs.rs/mjx-xlsx) is a package
//! you open, edit and save. `Workbook::blank` is the seam: it calls this writer and hands the
//! resulting [`Package`](mjx_opc::Package) to `Workbook::from_package`, so a workbook built from
//! nothing is resolved by the same code that resolves one read off disk.
//!
//! # The module tree
//!
//! | Module | Subject |
//! |---|---|
//! | [`constants`] | part names, content types, relationship types — the workspace's one set |
//! | [`workbook`] | `xl/workbook.xml`: the sheet list, and why there is no empty workbook |
//! | [`sheet`] | one authored worksheet: its name, its cells, its cached bounding box |
//! | [`stylesheet`] | `xl/styles.xml`: the skeleton, and the four appends |
//! | [`style_specs`] | the plain-data descriptions those appends take |
//! | [`package`] | [`WorkbookPackage`] itself: the parts, the content types, the relationships |
//!
//! # Every child is placed by the generated rank table
//!
//! Nothing here writes an element at a hand-chosen position. `dimension` lands at rank 1 of
//! `CT_Worksheet` and `sheetData` at rank 5 because
//! [`mjx_ooxml_types::child_order::WORKSHEET`] says so; the six style tables land in
//! `CT_Stylesheet`'s order because [`STYLESHEET`](mjx_ooxml_types::child_order::STYLESHEET) does.
//! MJXOFF-89 deleted fourteen hand-rolled ordering tables and this directory does not add a
//! fifteenth.
//!
//! # Authored parts are *read* before they are written
//!
//! Three of the parts here are seeded as minimal **bytes** carrying their namespace declaration,
//! parsed, and only then modelled. That is the rule `crates/mjx-xlsx/src/blank.rs` states in full,
//! and it exists because a freshly constructed root has no ancestor to inherit an `xmlns` from:
//! `mjx-docx`'s `create_footnotes_part` wrote one over a parsed root, the declaration vanished, and
//! every footnote with it — with a green gate throughout, because the gate asserted on the model
//! rather than on the file that came back.

pub mod constants;
pub mod package;
pub mod sheet;
pub mod style_specs;
pub mod stylesheet;
pub mod workbook;

pub use constants::{
    worksheet_part_name, worksheet_relationship_target, CONTENT_TYPE_SHARED_STRINGS,
    CONTENT_TYPE_STYLES, CONTENT_TYPE_WORKBOOK, CONTENT_TYPE_WORKBOOK_PACKAGE,
    CONTENT_TYPE_WORKSHEET, DEFAULT_SHEET_NAME, REL_OFFICE_DOCUMENT, REL_SHARED_STRINGS,
    REL_STYLES, REL_WORKSHEET, SHARED_STRINGS_PART, STYLES_PART, WORKBOOK_PART, XML_DECLARATION,
};
pub use package::{AuthoredCellValue, WorkbookPackage};
pub use sheet::AuthoredWorksheet;
pub use style_specs::{BorderEdgeSpec, BorderSpec, CellFormatSpec, PatternFillSpec};
pub use stylesheet::{AuthoredStylesheet, CellFormatTarget};
pub use workbook::AuthoredWorkbook;
