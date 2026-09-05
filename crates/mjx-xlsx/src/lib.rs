//! `mjx-xlsx` — the SpreadsheetML **package**: the container, the part graph, and the [`Workbook`] a
//! caller holds.
//!
//! The entry point is [`Workbook`]: open a `.xlsx`'s container bytes with [`Workbook::open`], read
//! its tabs with [`Workbook::sheets`] and its part graph with [`Workbook::parts`], and save with
//! [`Workbook::save`]. A worksheet's cells are read with [`Workbook::worksheet_markup`] and one is
//! written with [`Workbook::set_cell_value`]. Everything this crate does not model — which is still
//! most of a workbook, thirty-two of `CT_Worksheet`'s thirty-nine slots included — is preserved
//! verbatim by the OPC copy-on-write layer.
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
//! touching a byte. MJXOFF-100 (D06) adds the first model: `xl/workbook.xml`, through
//! [`mjx_sml::WorkbookPart`], plus the navigation surface over it — [`Workbook::sheets`],
//! [`Workbook::sheet_by_name`], [`Workbook::defined_names`], [`Workbook::date_system`] and
//! [`Workbook::rename_sheet`]. See `crates/mjx-xlsx/src/workbook/mod.rs`'s and
//! `crates/mjx-xlsx/src/worksheet/mod.rs`'s own module documentation for the file-by-file map of
//! which later Phase D child fills what, and [`crate::preserve`] for the fidelity contract
//! everything here rests on.

mod blank;
mod error;
pub mod guide;
mod nav;
pub mod parts;
pub mod preserve;
mod validate;
mod workbook;
mod worksheet;

pub use error::XlsxError;
pub use parts::{PartKind, SheetKind, WorkbookParts, WorksheetParts};
pub use preserve::{PartClassification, PartInventoryEntry};
pub use validate::SpreadsheetDefect;
pub use workbook::{
    CalculationSettings, DateSystem, DefinedNameEntry, DefinedNameScope, Sheet, Workbook,
    WorkbookWindow,
};
pub use worksheet::formatting::{SheetFormatResolver, SheetFormatting};
pub use worksheet::Worksheet;

/// Re-exported so that a caller who holds a [`Workbook`] can name what it is built on without
/// declaring `mjx-opc` themselves — the same courtesy `mjx-pptx` extends for the same types.
pub use mjx_opc::{OpcError, Package, PartName, TargetMode};
