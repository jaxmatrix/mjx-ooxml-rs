//! The grid itself: the cached bounding box, the calculation flag, and the seam to the cell store.
//!
//! | Type | `sml.xsd` | Element |
//! |---|---|---|
//! | `CT_SheetDimension` | 2323 | `x:dimension` |
//! | `CT_SheetCalcPr` | 2219 | `x:sheetCalcPr` |
//! | `CT_SheetData` | 2214 | `x:sheetData` — [`SheetData`](crate::SheetData), MJXOFF-95's |
//!
//! # `dimension` is a cached value, and this library treats it as one
//!
//! `<dimension ref="A1:C3"/>` is not a constraint on where cells may be. It is a **cached bounding
//! box**, in exactly the sense a formula's `<v>` is a cached result: the producer computed it when
//! it saved, and a consumer that disagrees recomputes silently. Excel does; it does not report a
//! repair, and it does not refuse the file.
//!
//! So the rule here has two halves, and both are deliberate:
//!
//! * **Reading never recomputes.** [`WorksheetPart::dimension`](crate::WorksheetPart::dimension)
//!   reports the range the file wrote, even where the cells disagree with it — the same
//!   preserve-and-report rule [`SheetData::anomalies`](crate::SheetData) applies to a row out of
//!   order. A "helpful" recompute on read would rewrite a part nobody asked to edit, which costs
//!   fidelity and hides nothing, because Excel would have repaired it anyway.
//! * **Writing a cell outside the box widens it.** When
//!   [`WorksheetPart::set_cell_value`](crate::WorksheetPart::set_cell_value) creates a cell the
//!   recorded range does not contain, the `dimension` element is widened to contain it. That is not
//!   the same act: the lie would then be *this library's*, written into a file it was editing, and
//!   preserving somebody else's stale cache is a different thing from authoring one.
//! * **[`WorksheetPart::recompute_dimension`](crate::WorksheetPart::recompute_dimension) is the
//!   caller's ask.** It replaces the cached box with the one the populated cells actually occupy.
//!   Nothing calls it implicitly.
//!
//! # The `sheetData` slot holds a packed store, not a subtree
//!
//! Rank 5 of `CT_Worksheet` is the only slot of the thirty-nine whose value is not a
//! [`RawElement`](mjx_ooxml_core::RawElement) or a [`RawNode`](mjx_ooxml_core::RawNode). It is
//! [`SheetData`](crate::SheetData) — MJXOFF-95's packed store — and it is why
//! [`WorksheetPart`](crate::WorksheetPart) writes **bytes** rather than returning a tree.
//!
//! `docs/BENCHMARKS.md` measures a 300,000-cell worksheet at **913 bytes of peak resident set per
//! cell** held as a `RawElement` tree. The store holds the same sheet in 36.8 B/cell, and
//! `crates/mjx-sml/tests/cell_store_allocation.rs` bounds it at 48. A frame that held its
//! `sheetData` slot as a subtree — even briefly, to serialize it — would give that back, so the
//! frame never materialises one: it writes its own start tag, then each slot's bytes, then its end
//! tag, and the store contributes `SheetData::write_into`'s three-level copy-on-write.

use mjx_ooxml_core::Enumeration;
use mjx_ooxml_types::support::OnOff;

use crate::address::CellRange;
use crate::leaf::attribute_bag;

attribute_bag! {
    /// `x:dimension` (`CT_SheetDimension`, `sml.xsd:2323`) — the cached bounding box of the
    /// populated cells.
    ///
    /// One attribute, `@ref`, and the schema declares it `use="required"`. It is an `ST_Ref`, which
    /// is MJXOFF-93's [`CellRange`] — the same parser `sqref`, `spans` and `oleSize` go through, so
    /// a `$`-anchored or single-cell form (`<dimension ref="A1"/>`, which Excel writes for a sheet
    /// with one cell) is understood here exactly as it is there.
    ///
    /// See this module's own documentation for why nothing recomputes this on a read.
    #[xml(attribute(local = "ref", codec = Enumeration<CellRange>, accessor = range, required))]
    SheetDimension, "dimension"
}

attribute_bag! {
    /// `x:sheetCalcPr` (`CT_SheetCalcPr`, `sml.xsd:2219`) — whether a consumer should recalculate
    /// this sheet's formulas the next time it opens the workbook.
    ///
    /// **Reported, never acted on.** This crate does not evaluate formulas and is not scheduled to
    /// (see [`crate::formula`]), so `fullCalcOnLoad` is a flag carried from the producer to the next
    /// consumer. Acting on it would mean recomputing cached values, which is the one thing a
    /// fidelity library must not do to a part it was not asked to edit.
    #[xml(attribute(local = "fullCalcOnLoad", codec = OnOff, accessor = full_calculation_on_load, default = false))]
    SheetCalculationProperties, "sheetCalcPr"
}
