//! The three sheet kinds that are not worksheets: chartsheet, dialogsheet and macrosheet.
//!
//! **Filled by MJXOFF-129 (D17).**
//!
//! # A workbook's tabs are not all grids
//!
//! `x:sheets` in `xl/workbook.xml` is *one* list over *four* part shapes. ECMA-376 Part 1 §12.3.23
//! names three explicit relationships a workbook part may have to a sheet — Worksheet (§12.3.24),
//! Chartsheet (§12.3.2) and Dialogsheet (§12.3.7) — and a fourth shape, the XLM macro sheet, exists
//! in `sml.xsd` as `CT_Macrosheet` with no global element, no content type and no relationship type
//! of its own. [`crate::worksheet`] models the first; this module models the other three.
//!
//! | Module | Type | `sml.xsd` | Slots | Modelled |
//! |---|---|---|---|---|
//! | [`chartsheet`] | `CT_Chartsheet` | 2955 | 14 | 10 |
//! | [`dialogsheet`] | `CT_Dialogsheet` | 2150 | 16 | 10 |
//! | [`macrosheet`] | `CT_Macrosheet` | 2118 | 27 | 20 |
//!
//! Every slot not modelled is held verbatim in its schema position, exactly as
//! [`crate::worksheet`]'s fourteen are, and each is somebody's: the drawing family is MJXOFF-107's
//! (E3) and MJXOFF-114's (E5), `extLst` is the unknown bucket by design, and a macrosheet's
//! `sheetData` is deliberately preserved rather than stored (see [`macrosheet`]).
//!
//! # No sheet kind here has a cell accessor, and that is the point
//!
//! The ticket's constraint is that *"a chartsheet's absence of cells is part of its type, not an
//! empty-collection special case"*, and it is met by omission rather than by a special case:
//! [`ChartSheetPart`], [`DialogSheetPart`] and [`MacroSheetPart`] simply have no method that answers
//! with a cell. Asking one for its cells does not compile. A caller that has an
//! `mjx_xlsx::SheetMarkup` in hand must match out the `Worksheet` variant to reach
//! [`WorksheetPart`](crate::WorksheetPart), and that match is the single place the four kinds are
//! distinguished.
//!
//! # One frame, three kinds
//!
//! The part mechanics — prologue and epilogue, the root element's own attributes, the slot-level
//! copy-on-write, generated placement, the byte writer — are one private type,
//! `crate::sheets::frame::SheetFrame`, shared by all three. [`WorksheetPart`](crate::WorksheetPart)
//! deliberately keeps its own; the reason is written down in that module and is a property of
//! `CT_Worksheet`'s packed `sheetData` store rather than a preference.

pub mod chartsheet;
pub mod dialogsheet;
pub(crate) mod frame;
pub mod macrosheet;

pub use chartsheet::{
    ChartSheetContent, ChartSheetPart, ChartSheetProperties, ChartSheetPropertiesContent,
    ChartSheetProtection, ChartSheetView, ChartSheetViews, ChartSheetViewsContent,
    CustomChartSheetView, CustomChartSheetViewContent, CustomChartSheetViews,
    CustomChartSheetViewsContent, SheetDrawing,
};
pub use dialogsheet::{DialogSheetContent, DialogSheetPart};
pub use macrosheet::{MacroSheetContent, MacroSheetPart};
