//! The worksheet part: the 39-slot spine and the sheet's own geometry.
//!
//! `CT_Worksheet` (`sml.xsd:2170`) is the **widest content model in this workspace** — a
//! thirty-nine member `xsd:sequence`, ten times `CT_Slide`'s and twice `CT_Workbook`'s.
//! [`WorksheetPart`] is the frame that holds all thirty-nine: seven modelled here, thirty-two kept
//! as the markup the file wrote, every one of them in its schema position.
//!
//! # The module tree, and the child that fills each file
//!
//! It is a **directory** for the same reason `CT_Worksheet` has thirty-nine children: no writer
//! should be holding that order in its head, and no single file should be holding all of those
//! subjects. `mjx-pptx`'s `presentation.rs` reached 12,771 lines before MJXOFF-60 (A8) spent a whole
//! child splitting it, and this is the type most likely to repeat that.
//!
//! | Module | Subject | Filled by |
//! |---|---|---|
//! | `frame.rs` | `CT_Worksheet` itself: the thirty-nine slots, placement, the byte writer | MJXOFF-102 (D07) |
//! | `views.rs` | `sheetPr`, `sheetViews`, panes, selections | MJXOFF-102 (D07) |
//! | `columns.rs` | `sheetFormatPr`, `cols`/`col` | MJXOFF-102 (D07) |
//! | `grid.rs` | `dimension`, `sheetCalcPr`, the `sheetData` seam | MJXOFF-102 (D07) |
//!
//! MJXOFF-117 (D12) adds merged cells, row and column geometry, outline levels, page breaks and
//! sheet protection; MJXOFF-120 through MJXOFF-133 (D13–D18) fill the optional features, which live
//! in [`crate::features`] rather than here. Each of those lands in a slot this frame already holds,
//! so none of them has to touch `frame.rs` to be reachable.
//!
//! # What is *held* and what is *modelled* are different claims
//!
//! Thirty-two of the thirty-nine slots — `sheetProtection`, `mergeCells`, `conditionalFormatting`,
//! `dataValidations`, `hyperlinks`, `pageSetup`, `headerFooter`, `drawing`, `tableParts`, `extLst`
//! and the rest — are held as [`WorksheetContent::Raw`], the markup the producer wrote, in the
//! position it wrote it. **A worksheet whose `pageSetup` survives is proof the frame works, not
//! proof `pageSetup` was modelled.**
//!
//! That distinction is what makes the later children cheap: each replaces one `Raw` slot with a
//! typed one and changes nothing else, and until it does, a caller's file is not damaged by the
//! absence.
//!
//! # Layering: this model has never heard of a package
//!
//! `drawing`, `legacyDrawing`, `legacyDrawingHF`, `drawingHF`, `picture`, `oleObjects`, `controls`,
//! `hyperlinks` and `tableParts` all reach *other parts*, through an `r:id`. None of them is
//! resolved here: this crate holds the identifier as the string the file wrote and
//! [`WorksheetPart::relationship_prefix`] says which prefix that file bound the relationship
//! namespace to. Resolving one to a [`PartName`](https://docs.rs/mjx-opc) is `mjx-xlsx`'s, in
//! `crates/mjx-xlsx/src/worksheet/`. `xtask/tests/layering.rs` checks the dependency graph;
//! `crates/mjx-sml/tests/worksheet_spine.rs` checks it the other way round, reading and re-emitting
//! whole worksheet parts without naming `mjx_opc` once.

mod columns;
mod frame;
mod grid;
mod views;

use mjx_ooxml_core::{RawAttribute, RawElement, RawName, RawNode};

pub use columns::{ColumnBlock, ColumnBlockContent, ColumnRun, SheetFormatProperties};
pub use frame::{WorksheetContent, WorksheetPart};
pub use grid::{SheetCalculationProperties, SheetDimension};
pub use views::{
    OutlineProperties, PageSetupProperties, PivotSelection, Selection, SheetPane, SheetProperties,
    SheetPropertiesContent, SheetView, SheetViewContent, SheetViews, SheetViewsContent,
};

/// Rebuilds one of this module's container elements as a [`RawElement`], **without an interner**.
///
/// [`ToXml::to_xml`](mjx_ooxml_core::ToXml::to_xml) takes `&mut Interner` because a model that
/// authors a name has to intern it. Nothing here ever does: every element keeps the [`RawName`] it
/// was read with and every attribute the file wrote. The types in this module therefore expose an
/// `as_raw_element(&self)` and implement `to_xml` as that method with the parameter ignored — which
/// is what lets [`WorksheetPart::write_into`] be a `&self` byte writer, and in turn what lets the
/// `sheetData` slot be MJXOFF-95's packed store rather than a subtree.
///
/// The rebuilt element carries **no** verbatim source range, which is correct: a rebuild is only
/// ever reached for a slot whose claim on its original bytes has already been given up.
#[must_use]
pub(crate) fn rebuild_element(
    name: RawName,
    attributes: &[RawAttribute],
    children: Vec<RawNode>,
    empty: bool,
) -> RawElement {
    let empty = empty && children.is_empty();
    RawElement::rebuilt(name, attributes.to_vec(), children, empty)
}
