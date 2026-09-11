//! Worksheet drawings — everything on a sheet that is not a cell, and where it is.
//!
//! A **report plus three authoring doors**, not a second model. `mjx_xlsx::SheetDrawing` is already
//! owned but carries an [`mjx_opc::PartName`]; the types here restate it with the part as text, and
//! the three `add_*_anchored_picture` calls take their anchors as plain numbers rather than as an
//! [`mjx_dml::spreadsheet_drawing::CellMarker`] tree — the same rule every other authoring call on
//! this surface follows.
//!
//! # The anchors take the column first
//!
//! Every other `(row, column)` pair on this facade — [`CellBlock::value`](crate::CellBlock::value),
//! `Deck::cell_text`, `Document::set_cell_text` and forty-four siblings — takes the row first,
//! because each indexes a two-dimensional *body* of cells. These four do not
//! ([`Workbook::add_two_cell_anchored_picture`], [`Workbook::add_one_cell_anchored_picture`],
//! [`Workbook::add_chart`](crate::Workbook::add_chart) and
//! [`Workbook::add_range_chart`](crate::Workbook::add_range_chart)): a marker is
//! `<xdr:col><xdr:colOff><xdr:row><xdr:rowOff>` in the file, its two offsets interleave with the two
//! indices, and taking the row first would put each offset beside the wrong one. So
//! `(from_column, from_row, to_column, to_row)` reads straight down the element it writes — the same
//! reason [`mjx_sml::CellReference::relative`] takes `(column, row)`, written on that type.
//!
//! # Three calls, because there are three anchors
//!
//! Not one call with a mode argument. A `xdr:twoCellAnchor` has **no extent of its own** — its two
//! markers are its geometry — while a `xdr:oneCellAnchor` and a `xdr:absoluteAnchor` each carry one;
//! and an absolute anchor names no cell at all. A single call would have to take every argument all
//! three could want and ignore most of them, which is an API that cannot say what it did.
//!
//! # The honest answer, carried across the boundary
//!
//! [`sheet_anchor_bounds`](Workbook::sheet_anchor_bounds) takes the **maximum digit width** a column
//! width is measured in, because `col@width` is a character count and turning it into a length needs
//! a font measurement this library never makes. Every answer carries the metrics that produced it
//! and, per axis, whether the number came from a stated `ht`/`width`, from the sheet's own default,
//! or from `sheetFormatPr@baseColWidth`. `None` means the sheet does not state enough to place the
//! object — see [`mjx_sml::SheetAnchors`].

use mjx_ooxml_types::spreadsheetdrawing::ResizingBehavior;
use mjx_sml::{ColumnMetrics, GeometrySource};

use crate::error::Error;
use crate::index::{count, index};

use super::Workbook;

/// One anchored object on a sheet, decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetDrawingObjectInfo {
    /// The object's position in the drawing part, which is also its **paint order**: a later object
    /// is drawn over an earlier one.
    pub index: u32,
    /// Which of the three anchor elements pins it — `"twoCellAnchor"`, `"oneCellAnchor"` or
    /// `"absoluteAnchor"`.
    pub anchor: String,
    /// Which kind of object it holds — `"sp"`, `"pic"`, `"graphicFrame"`, `"grpSp"`, `"cxnSp"` or
    /// `"contentPart"` — or `None` for an anchor holding none, which the schema forbids and this
    /// library reports rather than repairs.
    pub object: Option<String>,
    /// What the anchor promises to do when the cells under it move.
    ///
    /// For a `twoCellAnchor` this is its `@editAs`, which **need not agree with the element name**:
    /// a producer writing `editAs="oneCell"` on a two-cell anchor is saying *keep this object's size
    /// and move only its top-left corner*, and that is what Apache POI writes for every
    /// move-but-do-not-resize anchor.
    pub resizing: ResizingBehavior,
    /// The object's `cNvPr@id`, or `None` for one with no non-visual block.
    pub id: Option<u32>,
    /// The object's `cNvPr@name`.
    pub name: Option<String>,
    /// For a picture, the image part it shows. `None` for every other object kind, and for a picture
    /// that links an external image.
    pub image: Option<String>,
    /// Whether the object prints with the sheet — `xdr:clientData@fPrintsWithSheet`, which
    /// **defaults to `true`**, unlike nearly every other flag in these formats.
    pub prints_with_sheet: bool,
}

/// One sheet's drawing part, and what is anchored in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetDrawingInfo {
    /// The part the anchors live in — `"/xl/drawings/drawing1.xml"` in everything a real producer
    /// writes, though nothing requires that spelling.
    pub part: String,
    /// The `x:drawing@r:id` the sheet reached it through.
    pub relationship_id: String,
    /// Every anchored object, in paint order.
    pub objects: Vec<SheetDrawingObjectInfo>,
}

/// Where an anchor puts its object, in EMU, and what the answer rests on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnchorBoundsInfo {
    /// The object's left edge, in EMU from the sheet origin.
    pub x_emu: i64,
    /// The object's top edge, in EMU from the sheet origin.
    pub y_emu: i64,
    /// The object's width, in EMU.
    pub width_emu: i64,
    /// The object's height, in EMU.
    pub height_emu: i64,
    /// Where the **vertical** half of this answer came from. A row height is stated in points, so a
    /// `Stated` one is exact.
    pub row_source: GeometrySource,
    /// Where the **horizontal** half came from. Never exact whatever this says: see
    /// [`maximum_digit_width_pixels`](Self::maximum_digit_width_pixels).
    pub column_source: GeometrySource,
    /// The maximum digit width, in pixels, the horizontal half was computed through — the number
    /// the caller passed. A column width is a character count, so it is not a length until one of
    /// these is chosen.
    pub maximum_digit_width_pixels: f64,
    /// The pixels per inch that width was stated at.
    pub pixels_per_inch: f64,
}

/// What one anchor did when rows or columns moved under it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnchorShiftInfo {
    /// The anchor's position in the drawing part.
    pub index: u32,
    /// What the anchor promises to do when the cells under it move.
    pub promise: ResizingBehavior,
    /// Whether the object's top-left corner moved.
    pub moved: bool,
    /// Whether the object's extent changed — true only when the edit fell **between** a two-cell
    /// anchor's two markers.
    pub resized: bool,
    /// Whether the markers alone could keep the anchor's own promise.
    ///
    /// `false` in exactly one case: a two-cell anchor resized while its `@editAs` says its size must
    /// not change. A two-cell anchor has no extent of its own, so restoring the size means
    /// recomputing its `to` marker from the sheet's own geometry — which is a decision, not a
    /// repair, and is left to the caller who now knows it is needed.
    pub promise_kept: bool,
}

impl Workbook {
    /// The drawing part behind one sheet, and everything anchored in it.
    ///
    /// `None` when the sheet writes no `x:drawing`, when the `r:id` it writes names no relationship,
    /// or when the part that relationship reaches is not a drawing. All three are facts about the
    /// file, reported as absence.
    ///
    /// # Errors
    /// [`ErrorCode::IndexOutOfRange`](crate::ErrorCode::IndexOutOfRange) if `sheet` names no tab, or
    /// [`ErrorCode::MalformedDocument`](crate::ErrorCode::MalformedDocument) if the worksheet or the
    /// drawing part is not well-formed.
    pub fn sheet_drawing(&self, sheet: u32) -> Result<Option<SheetDrawingInfo>, Error> {
        Ok(self.workbook.sheet_drawing(index(sheet))?.map(info))
    }

    /// Where the anchor at `anchor` on one sheet puts its object, in EMU.
    ///
    /// `maximum_digit_width_pixels` and `pixels_per_inch` are the font measurement a column width is
    /// converted through — **7.0 and 96.0** are ECMA-376's own worked example, for 11-point Calibri,
    /// and are the right values for a workbook whose Normal style nobody has changed. See this
    /// module's own documentation for why they are arguments.
    ///
    /// `None` when there is no such anchor, when the sheet has no drawing, or when the sheet does
    /// not state enough to place the object.
    ///
    /// # Errors
    /// As [`sheet_drawing`](Self::sheet_drawing).
    pub fn sheet_anchor_bounds(
        &self,
        sheet: u32,
        anchor: u32,
        maximum_digit_width_pixels: f64,
        pixels_per_inch: f64,
    ) -> Result<Option<AnchorBoundsInfo>, Error> {
        let metrics = ColumnMetrics {
            maximum_digit_width_pixels,
            pixels_per_inch,
        };
        Ok(self
            .workbook
            .sheet_anchor_bounds(index(sheet), index(anchor), metrics)?
            .map(|bounds| AnchorBoundsInfo {
                x_emu: bounds.position.x.emu(),
                y_emu: bounds.position.y.emu(),
                width_emu: bounds.size.width.emu(),
                height_emu: bounds.size.height.emu(),
                row_source: bounds.row_source,
                column_source: bounds.column_source,
                maximum_digit_width_pixels: bounds.column_metrics.maximum_digit_width_pixels,
                pixels_per_inch: bounds.column_metrics.pixels_per_inch,
            }))
    }

    /// Anchors a picture between two cells, and answers its position in the drawing's paint order.
    ///
    /// The object has **no extent of its own**: the two markers are its geometry, so widening a
    /// column between them makes it wider. Every offset is EMU **into** the cell it accompanies,
    /// measured from that cell's top-left corner — not from the sheet origin.
    ///
    /// The drawing part is created if the sheet has none, and identical image bytes are stored once.
    ///
    /// # Errors
    /// [`ErrorCode::IndexOutOfRange`](crate::ErrorCode::IndexOutOfRange) if `sheet` names no tab,
    /// [`ErrorCode::InvalidArgument`](crate::ErrorCode::InvalidArgument) if the bytes match no image
    /// format this build knows, or
    /// [`ErrorCode::MalformedDocument`](crate::ErrorCode::MalformedDocument) if the worksheet is not
    /// well-formed.
    #[allow(clippy::too_many_arguments)]
    pub fn add_two_cell_anchored_picture(
        &mut self,
        sheet: u32,
        image_bytes: &[u8],
        name: &str,
        from_column: u32,
        from_column_offset_emu: i64,
        from_row: u32,
        from_row_offset_emu: i64,
        to_column: u32,
        to_column_offset_emu: i64,
        to_row: u32,
        to_row_offset_emu: i64,
        resizing: ResizingBehavior,
    ) -> Result<u32, Error> {
        let from = marker(
            from_column,
            from_column_offset_emu,
            from_row,
            from_row_offset_emu,
        );
        let to = marker(to_column, to_column_offset_emu, to_row, to_row_offset_emu);
        Ok(count(self.workbook.add_two_cell_anchored_picture(
            index(sheet),
            image_bytes,
            name,
            from,
            to,
            resizing,
        )?))
    }

    /// Anchors a picture to one cell, at its own size.
    ///
    /// It moves with the cell it names and keeps that size, whatever happens to the rows and columns
    /// under it — which is what a one-cell anchor *is*, and why it takes an extent where the two-cell
    /// call takes a second marker.
    ///
    /// # Errors
    /// As [`add_two_cell_anchored_picture`](Self::add_two_cell_anchored_picture).
    #[allow(clippy::too_many_arguments)]
    pub fn add_one_cell_anchored_picture(
        &mut self,
        sheet: u32,
        image_bytes: &[u8],
        name: &str,
        from_column: u32,
        from_column_offset_emu: i64,
        from_row: u32,
        from_row_offset_emu: i64,
        width_emu: i64,
        height_emu: i64,
    ) -> Result<u32, Error> {
        let from = marker(
            from_column,
            from_column_offset_emu,
            from_row,
            from_row_offset_emu,
        );
        Ok(count(self.workbook.add_one_cell_anchored_picture(
            index(sheet),
            image_bytes,
            name,
            from,
            mjx_dml::Size::from_emu(width_emu, height_emu),
        )?))
    }

    /// Anchors a picture to the **sheet**, at an absolute position and size in EMU.
    ///
    /// It names no cell, so nothing that happens to the rows and columns moves or resizes it.
    ///
    /// # Errors
    /// As [`add_two_cell_anchored_picture`](Self::add_two_cell_anchored_picture).
    #[allow(clippy::too_many_arguments)]
    pub fn add_absolute_anchored_picture(
        &mut self,
        sheet: u32,
        image_bytes: &[u8],
        name: &str,
        x_emu: i64,
        y_emu: i64,
        width_emu: i64,
        height_emu: i64,
    ) -> Result<u32, Error> {
        Ok(count(self.workbook.add_absolute_anchored_picture(
            index(sheet),
            image_bytes,
            name,
            mjx_dml::Position::from_emu(x_emu, y_emu),
            mjx_dml::Size::from_emu(width_emu, height_emu),
        )?))
    }

    /// Removes one anchored object from a sheet, reporting whether there was one.
    ///
    /// **The image part it named is left in the package.** Another object may show the same image.
    ///
    /// # Errors
    /// As [`sheet_drawing`](Self::sheet_drawing).
    pub fn remove_sheet_drawing_object(&mut self, sheet: u32, anchor: u32) -> Result<bool, Error> {
        Ok(self
            .workbook
            .remove_sheet_drawing_object(index(sheet), index(anchor))?)
    }

    /// Moves every anchor on a sheet for `count` rows inserted at the zero-based `at`, and reports
    /// per anchor what happened.
    ///
    /// A two-cell anchor moves and sizes, a one-cell anchor moves and keeps its size, an absolute
    /// anchor does neither. **This moves the drawing, not the cells**: nothing in this library
    /// inserts a row into a sheet, so a caller doing that itself calls this so the objects over
    /// those rows travel with them.
    ///
    /// # Errors
    /// As [`sheet_drawing`](Self::sheet_drawing).
    pub fn insert_rows_into_drawing(
        &mut self,
        sheet: u32,
        at: u32,
        rows: u32,
    ) -> Result<Vec<AnchorShiftInfo>, Error> {
        Ok(shifts(self.workbook.insert_rows_into_drawing(
            index(sheet),
            axis(at),
            rows,
        )?))
    }

    /// [`insert_rows_into_drawing`](Self::insert_rows_into_drawing) for rows removed.
    ///
    /// An anchor naming a row inside the removed run is clamped to `at` with no offset: the cell it
    /// named is gone, and **an anchor naming a row beyond the sheet is untrusted input, not a bug**.
    ///
    /// # Errors
    /// As [`sheet_drawing`](Self::sheet_drawing).
    pub fn remove_rows_from_drawing(
        &mut self,
        sheet: u32,
        at: u32,
        rows: u32,
    ) -> Result<Vec<AnchorShiftInfo>, Error> {
        Ok(shifts(self.workbook.remove_rows_from_drawing(
            index(sheet),
            axis(at),
            rows,
        )?))
    }

    /// [`insert_rows_into_drawing`](Self::insert_rows_into_drawing) on the column axis.
    ///
    /// # Errors
    /// As [`sheet_drawing`](Self::sheet_drawing).
    pub fn insert_columns_into_drawing(
        &mut self,
        sheet: u32,
        at: u32,
        columns: u32,
    ) -> Result<Vec<AnchorShiftInfo>, Error> {
        Ok(shifts(self.workbook.insert_columns_into_drawing(
            index(sheet),
            axis(at),
            columns,
        )?))
    }

    /// [`remove_rows_from_drawing`](Self::remove_rows_from_drawing) on the column axis.
    ///
    /// # Errors
    /// As [`sheet_drawing`](Self::sheet_drawing).
    pub fn remove_columns_from_drawing(
        &mut self,
        sheet: u32,
        at: u32,
        columns: u32,
    ) -> Result<Vec<AnchorShiftInfo>, Error> {
        Ok(shifts(self.workbook.remove_columns_from_drawing(
            index(sheet),
            axis(at),
            columns,
        )?))
    }
}

/// A marker from four plain numbers.
///
/// The column and row are `u32` here and `xsd:int` on the wire, which the schema restricts to
/// non-negative — so a value past `i32::MAX` names a cell the grid does not have and is clamped
/// rather than wrapping.
fn marker(
    column: u32,
    column_offset_emu: i64,
    row: u32,
    row_offset_emu: i64,
) -> mjx_dml::CellMarker {
    mjx_dml::CellMarker::new(axis(column), column_offset_emu, axis(row), row_offset_emu)
}

/// A row or column index as the wire states it, clamped to the non-negative range `xdr:ST_ColID`
/// and `ST_RowID` allow.
fn axis(index: u32) -> i32 {
    i32::try_from(index).unwrap_or(i32::MAX)
}

/// One model drawing as the facade states it.
fn info(drawing: mjx_xlsx::SheetDrawing) -> SheetDrawingInfo {
    SheetDrawingInfo {
        part: drawing.part.as_str().to_owned(),
        relationship_id: drawing.relationship_id,
        objects: drawing
            .objects
            .into_iter()
            .map(|object| SheetDrawingObjectInfo {
                index: count(object.index),
                anchor: object.anchor.to_owned(),
                object: object.object.map(str::to_owned),
                resizing: object.resizing,
                id: object.id,
                name: object.name,
                image: object.image.map(|part| part.as_str().to_owned()),
                prints_with_sheet: object.prints_with_sheet,
            })
            .collect(),
    }
}

/// A model shift report as the facade states it.
fn shifts(report: Vec<mjx_dml::AnchorShift>) -> Vec<AnchorShiftInfo> {
    report
        .into_iter()
        .map(|shift| AnchorShiftInfo {
            index: count(shift.index),
            promise: shift.promise,
            moved: shift.moved,
            resized: shift.resized,
            promise_kept: shift.promise_kept,
        })
        .collect()
}
