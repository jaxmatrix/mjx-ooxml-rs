//! The worksheet part: the 39-slot spine and the sheet's own geometry.
//!
//! **Filled by MJXOFF-102 (D07) — the spine — and MJXOFF-117 (D12) — the grid.** Nothing here yet —
//! this child (MJXOFF-132) creates the crate and the tree, and models nothing.
//!
//! What belongs here: `CT_Worksheet` (the generated
//! [`WORKSHEET`](mjx_ooxml_types::child_order::WORKSHEET) order — **39 slots, the largest
//! `xsd:sequence` in this workspace**), `sheetPr`, `dimension`, `sheetViews` and panes, `cols`,
//! `sheetData`'s frame, then MJXOFF-117's merged cells, row and column geometry, outline levels,
//! page breaks and sheet protection.
//!
//! It is a **directory** for the same reason `CT_Worksheet` has 39 children: no writer should be
//! holding that order in its head, and no single file should be holding all of those subjects.
