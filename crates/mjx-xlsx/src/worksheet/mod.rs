//! The sheet tier of the part graph: one worksheet, chartsheet or dialogsheet part and everything
//! that hangs off it.
//!
//! # Why a sheet has a part graph of its own
//!
//! ECMA-376 Part 1 §12.3 divides the workbook's parts in two. Some are the *workbook's* — the shared
//! string table, the styles, the calculation chain — and [`crate::WorkbookParts`] resolves those.
//! The rest belong to a single sheet: its drawings, its comments and the legacy VML that draws their
//! pop-up boxes, its tables, its query and pivot tables, the printer configuration somebody saved
//! for it. Those are reached from the *sheet* part's own `.rels`, and this module is where they are
//! resolved.
//!
//! # The module tree, and the child that fills each file
//!
//! | Module | Filled by |
//! |---|---|
//! | `mod.rs` (this file) | MJXOFF-91 (D02) — [`Worksheet`], the handle, and [`crate::WorksheetParts`] |
//! | [`grid`](self::grid) | MJXOFF-102 (D07) the 39-slot spine, MJXOFF-117 (D12) the sheet grid |
//! | [`features`](self::features) | MJXOFF-120/123/125/127/129 (D13-D17) — the optional worksheet features |
//!
//! # What this is not
//!
//! [`Worksheet`] holds no cells. `mjx_sml::worksheet` is where `CT_Worksheet` — the largest
//! `xsd:sequence` in this workspace, at 39 slots — is modelled, and MJXOFF-102 is what fills it.
//! Until then a [`Worksheet`] is a name, a kind and a resolved set of related parts, which is
//! precisely what a later child needs and no more than this one can honestly provide.

pub(crate) mod features;
pub(crate) mod grid;

use mjx_opc::{Package, PartName};

use crate::error::XlsxError;
use crate::parts::{SheetKind, WorksheetParts};
use crate::workbook::Sheet;

/// One sheet of a workbook, resolved to its part and its own part graph.
///
/// Borrows the [`crate::Workbook`] it came from — a sheet is a view onto a package, never a detached
/// copy of one, so there is no way to hold a `Worksheet` whose package has moved on beneath it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Worksheet<'a> {
    sheet: &'a Sheet,
    part: PartName,
    parts: WorksheetParts,
}

impl<'a> Worksheet<'a> {
    /// Resolves `sheet`'s own part graph, or `None` if the entry reaches no part at all.
    ///
    /// # Errors
    /// Returns [`XlsxError`] if one of the sheet's relationships has an unresolvable or external
    /// target.
    pub(crate) fn resolve(package: &Package, sheet: &'a Sheet) -> Result<Option<Self>, XlsxError> {
        let Some(part) = sheet.part.clone() else {
            return Ok(None);
        };
        let parts = WorksheetParts::resolve(package, &part)?;
        Ok(Some(Self { sheet, part, parts }))
    }

    /// The `x:sheets` entry this sheet was reached through — its tab name, `@sheetId` and
    /// visibility.
    #[must_use]
    pub fn entry(&self) -> &'a Sheet {
        self.sheet
    }

    /// The sheet part's own name.
    #[must_use]
    pub fn part(&self) -> &PartName {
        &self.part
    }

    /// Which of the three sheet kinds this is, or `None` if the part's content type is not one of
    /// the three (which is a defect [`crate::Workbook::validate`] reports, not a reason to refuse
    /// to read the file).
    #[must_use]
    pub fn kind(&self) -> Option<SheetKind> {
        self.sheet.kind
    }

    /// The parts this sheet relates to: its drawings, comments, tables, printer settings.
    #[must_use]
    pub fn parts(&self) -> &WorksheetParts {
        &self.parts
    }
}
