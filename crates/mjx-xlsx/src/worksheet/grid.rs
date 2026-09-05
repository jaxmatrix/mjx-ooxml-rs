//! The sheet's geometry: rows, columns, merges, panes, breaks — everything about *where* a cell is
//! rather than what is in it.
//!
//! **Filled by MJXOFF-102 (D07) — the worksheet spine — and MJXOFF-117 (D12) — the grid itself.**
//! Nothing here yet: MJXOFF-91 (D02) builds the package and the part graph and models nothing.
//!
//! What belongs here at the package tier: the [`crate::Worksheet`] handle's geometry accessors,
//! sitting on top of `mjx_sml::worksheet`'s models of `CT_SheetDimension`, `CT_Cols`/`CT_Col`,
//! `CT_Row`, `CT_MergeCells`, `CT_SheetView`'s panes and `CT_PageBreak`.
//!
//! The division of labour is the crate split: `mjx-sml` answers *what a row is*, this file answers
//! *what the worksheet part in this package says about row 7*. A model that needed no package would
//! not belong here at all.
