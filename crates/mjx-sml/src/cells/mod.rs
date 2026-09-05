//! The cell store: rows, cells, cell values and the memory model behind them.
//!
//! **Filled by MJXOFF-95 (D04), with cached formula values arriving in MJXOFF-115 (D11).** Nothing
//! here yet — this child (MJXOFF-132) creates the crate and the tree, and models nothing.
//!
//! What belongs here: `CT_SheetData`, `CT_Row` and `CT_Cell` (the generated
//! [`WORKSHEET_ROW`](mjx_ooxml_types::child_order::WORKSHEET_ROW) and
//! [`WORKSHEET_CELL`](mjx_ooxml_types::child_order::WORKSHEET_CELL) orders), the `ST_CellType`
//! discriminant, and the storage decision itself. This is a **directory** rather than a file because
//! MJXOFF-95 is where `CLAUDE.md`'s "arena for bulk data, owned trees for small structures" stops
//! being a sentence: a worksheet is the one part of OOXML where the cell count, not the element
//! count, decides whether the library is usable, and MJXOFF-147's benchmarks and MJXOFF-151's
//! measured `~32x` materialisation cost are the numbers that choice is made against.
