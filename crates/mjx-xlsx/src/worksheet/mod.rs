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
//! | [`grid`](self::grid) | **MJXOFF-102 (D07) — done**: opening a worksheet part, one cell in or out, writing it back |
//! | [`geometry`](self::geometry) | **MJXOFF-117 (D12) — done**: merging, row and column geometry, the merged-range format, the grid anomaly report |
//! | [`formatting`](self::formatting) | **MJXOFF-108 (D09) — done**: `xl/styles.xml` plus one worksheet, and every cell's [`mjx_sml::EffectiveCellFormat`]. The resolution order itself is `mjx-sml`'s and is not repeated here |
//! | [`features`](self::features) | **MJXOFF-120 (D13) — done**: conditional formatting, which spans the worksheet and `xl/styles.xml`; MJXOFF-123/125/127/129 (D14-D17) fill the rest |
//!
//! # What this is not
//!
//! [`Worksheet`] itself still holds no cells: it is a name, a kind and a resolved set of related
//! parts. The **markup** is [`mjx_sml::WorksheetPart`] — `CT_Worksheet`, the largest `xsd:sequence`
//! in this workspace at 39 slots — and this tier's job is to hand one over and take it back, which
//! [`crate::Workbook::worksheet_markup`] and [`crate::Workbook::write_worksheet_markup`] do.
//!
//! The division is the crate split restated: `mjx-sml` answers *what a row is*, and this module
//! answers *which part in this package holds row 7*.

pub(crate) mod features;
pub(crate) mod formatting;
pub(crate) mod geometry;
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
