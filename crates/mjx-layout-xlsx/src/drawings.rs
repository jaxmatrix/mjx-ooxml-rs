//! Cell drawings: the three anchor modes, resolved against **this** crate's grid.
//!
//! # The one test that tells the three modes apart
//!
//! `xdr:twoCellAnchor`, `xdr:oneCellAnchor` and `xdr:absoluteAnchor` differ in exactly one way, and
//! it is not their markup — it is which half of the answer the grid supplies:
//!
//! | Mode | Position | Size |
//! |---|---|---|
//! | two-cell | from the grid | from the grid |
//! | one-cell | from the grid | stated in EMU |
//! | absolute | stated in EMU | stated in EMU |
//!
//! So **change one row's height and ask which rectangles moved**. A two-cell anchor below it moves
//! and one crossing it grows; a one-cell anchor below it moves and keeps its size; an absolute
//! anchor does neither. Three separate fixtures each assert that a mode produced *a* rectangle;
//! one fixture that perturbs the geometry asserts that it produced *the right* rectangle, and
//! `tests/three_anchor_modes.rs` is that fixture.
//!
//! # Why this is not `Workbook::sheet_anchor_bounds`
//!
//! `mjx-xlsx` already resolves an anchor, through `mjx_sml::SheetAnchors`, and it is right to: a
//! caller holding only a package needs an answer. But it has to take a
//! [`ColumnMetrics`](mjx_sml::ColumnMetrics) from its caller, because a column's width is *"the
//! number of characters of the maximum digit width of the numbers 0…9 as rendered in the normal
//! style's font"* and that crate opens no fonts.
//!
//! This crate **has measured that glyph** — [`MaximumDigitWidth`](crate::MaximumDigitWidth) shapes a
//! `0` through `mjx-text` rather than hard-coding 11-point Calibri's seven pixels — and it holds the
//! `col` runs, the stated row heights and the hidden flags in the same two sparse indices every cell
//! is placed through. Resolving the anchor anywhere else would put a drawing on a different grid
//! from the cells underneath it, which is visible at a glance the moment one row is hidden.
//!
//! # ⚠ What is placed here is a rectangle, and not a picture
//!
//! A drawing's *content* is DrawingML: an `a:prstGeom`, an `a:solidFill`, a `p:txBody`. Laying that
//! out is `mjx-layout-pptx`'s whole subject — and `mjx-layout-pptx` is at rank **3.6**, the same
//! rank as this crate, so an edge to it is *sideways* and `xtask/tests/layering.rs` refuses it by
//! name. The ticket's instruction to *"consume MJXOFF-170's DrawingML layout … not a second shape
//! renderer"* therefore cannot be followed as written: there is no legal edge along which to consume
//! it, and writing one here would be the second shape renderer the same sentence forbids.
//!
//! What this does instead is the half that genuinely belongs to a grid: **where the object is**, in
//! the sheet's own coordinates, with the anchor mode named and the image part it reaches reported.
//! A shape's interior is a hole, it is stated as a hole, and closing it needs a crate below both box
//! models that neither of them owns yet. That is a finding, not a shortcut.

use mjx_layout::LayoutRect;
use mjx_ooxml_core::measure::Emu;
use mjx_xlsx::{AnchorCell, AnchorPlacement, SheetDrawing, SheetDrawingObject};

use crate::geometry::{GridGeometry, COLUMN_COUNT, ROW_COUNT};

/// Which of the three anchor elements pinned an object.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AnchorMode {
    /// `xdr:twoCellAnchor` — both corners follow the grid.
    TwoCell,
    /// `xdr:oneCellAnchor` — the top-left follows the grid; the size is fixed.
    OneCell,
    /// `xdr:absoluteAnchor` — neither follows the grid.
    Absolute,
}

impl AnchorMode {
    /// Whether the object's **position** moves when the rows or columns above and left of it change
    /// size.
    #[must_use]
    pub fn position_follows_the_grid(self) -> bool {
        matches!(self, Self::TwoCell | Self::OneCell)
    }

    /// Whether its **size** does.
    #[must_use]
    pub fn size_follows_the_grid(self) -> bool {
        matches!(self, Self::TwoCell)
    }
}

/// One anchored object, placed in the sheet's own coordinates.
#[derive(Clone, PartialEq, Debug)]
pub struct PlacedDrawing {
    /// The anchor's position in the drawing part, which is also its paint order.
    pub index: usize,
    /// Which of the three modes pinned it.
    pub mode: AnchorMode,
    /// `cNvPr@name`.
    pub name: Option<String>,
    /// Which of the six `EG_ObjectChoices` members it holds — `sp`, `pic`, `graphicFrame`, `grpSp`,
    /// `cxnSp` or `contentPart`.
    pub object: Option<&'static str>,
    /// The image part an `xdr:pic` reaches, as a part name. `None` for every other object kind and
    /// for a picture that links an external image.
    pub image: Option<String>,
    /// `xdr:clientData@fPrintsWithSheet`, which defaults to `true` — what
    /// [`crate::print`] reads to decide whether the object appears on a page.
    pub prints_with_sheet: bool,
    /// Where it is, in **sheet** coordinates: the same space [`GridGeometry::cell_rect`] answers in,
    /// before a pane region's origin has been applied.
    pub rect: LayoutRect,
}

/// Places every object of `drawing` against `geometry`.
///
/// An object whose anchor states less than its own kind requires is **dropped**: `CT_Marker`'s four
/// children are all `minOccurs="1"`, so a marker missing one names no cell, and a rectangle invented
/// for it would be a position presented as a measurement. `mjx_dml::CellMarker::read` reaches the
/// same answer one crate down for the same reason.
#[must_use]
pub fn place(drawing: &SheetDrawing, geometry: &GridGeometry) -> Vec<PlacedDrawing> {
    drawing
        .objects
        .iter()
        .filter_map(|object| place_one(object, geometry))
        .collect()
}

/// Places one object, or `None` when its anchor places nothing.
#[must_use]
pub fn place_one(object: &SheetDrawingObject, geometry: &GridGeometry) -> Option<PlacedDrawing> {
    let (mode, rect) = match object.placement {
        AnchorPlacement::TwoCell { from, to } => {
            let start = corner(from, geometry);
            let end = corner(to, geometry);
            (
                AnchorMode::TwoCell,
                LayoutRect::from_edges(
                    start.0.min(end.0),
                    start.1.min(end.1),
                    start.0.max(end.0),
                    start.1.max(end.1),
                ),
            )
        }
        AnchorPlacement::OneCell { from, extent } => {
            let (left, top) = corner(from, geometry);
            (
                AnchorMode::OneCell,
                LayoutRect::from_edges(
                    left,
                    top,
                    left + Emu::from_emu(extent.0.max(0)),
                    top + Emu::from_emu(extent.1.max(0)),
                ),
            )
        }
        AnchorPlacement::Absolute { position, extent } => {
            let (left, top) = (Emu::from_emu(position.0), Emu::from_emu(position.1));
            (
                AnchorMode::Absolute,
                LayoutRect::from_edges(
                    left,
                    top,
                    left + Emu::from_emu(extent.0.max(0)),
                    top + Emu::from_emu(extent.1.max(0)),
                ),
            )
        }
        AnchorPlacement::Unplaced => return None,
    };
    Some(PlacedDrawing {
        index: object.index,
        mode,
        name: object.name.clone(),
        object: object.object,
        image: object.image.as_ref().map(|part| part.as_str().to_owned()),
        prints_with_sheet: object.prints_with_sheet,
        rect,
    })
}

/// Where one marker lands, in sheet coordinates.
///
/// A negative column or row — which `xdr:col` permits, being an `xsd:int` — is clamped to the
/// origin, and one past the grid is clamped to its far edge. Both are files that are wrong about
/// their own sheet, and clamping is what keeps a wrong drawing from moving every right one.
fn corner(marker: AnchorCell, geometry: &GridGeometry) -> (Emu, Emu) {
    let column = u16::try_from(marker.column.clamp(0, i64_column())).unwrap_or(0);
    let row = u32::try_from(marker.row.max(0))
        .unwrap_or(0)
        .min(row_limit());
    (
        geometry.columns().left(column) + Emu::from_emu(marker.column_offset),
        geometry.rows().top(row) + Emu::from_emu(marker.row_offset),
    )
}

/// The last addressable column, as an `i32` for the clamp above.
fn i64_column() -> i32 {
    i32::try_from(COLUMN_COUNT.saturating_sub(1)).unwrap_or(i32::MAX)
}

/// The last addressable row.
fn row_limit() -> u32 {
    ROW_COUNT.saturating_sub(1)
}
