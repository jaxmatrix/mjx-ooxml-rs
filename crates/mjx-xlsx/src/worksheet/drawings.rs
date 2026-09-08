//! Worksheet drawings at the package tier: the part the anchors live in, the image parts they name,
//! and the surface that puts a picture on a sheet.
//!
//! # Four things that have to agree, again
//!
//! A drawing is the same shape of problem as a worksheet table, one part deeper:
//!
//! | thing | where it lives |
//! |---|---|
//! | the anchors | `xl/drawings/drawingN.xml`, an `xdr:wsDr` — [`mjx_dml::spreadsheet_drawing::WorksheetDrawing`] |
//! | its content type | `[Content_Types].xml`, [`CONTENT_TYPE_DRAWING`](crate::parts::CONTENT_TYPE_DRAWING) |
//! | the edge to it | `xl/worksheets/_rels/sheetN.xml.rels`, [`REL_DRAWING`](crate::parts::REL_DRAWING) |
//! | the sheet's claim on it | `x:drawing@r:id` — [`mjx_sml::SheetDrawing`], rank 29 of `CT_Worksheet` |
//!
//! …and a picture inside it adds a fifth and a sixth: the image part under `xl/media/`, and the
//! [`REL_IMAGE`](REL_IMAGE) relationship **from the drawing part**, because an
//! `a:blip@r:embed` is resolved against the part that contains it. Relating the image from the
//! *sheet* would produce a file Excel opens and repairs, which is the kind of near-miss this tier
//! exists to prevent.
//!
//! [`Workbook::add_two_cell_anchored_picture`] and its two siblings write all six in one call, and
//! create the drawing part on demand for a sheet that has none.
//!
//! # `mjx-sml` resolves nothing, and neither does `mjx-dml`
//!
//! The division is the crate split restated for a third feature. `mjx_dml::spreadsheet_drawing`
//! answers *what a two-cell anchor is*; `mjx_sml::SheetAnchors` answers *where it is on a sheet with
//! these column widths*; this file answers *which part in this package holds the image that anchor's
//! picture shows*. Neither of the two below has ever heard of a package.
//!
//! # Adding a picture writes no cell, and editing a cell moves no picture
//!
//! The rule [`crate::worksheet::tables`] states, at another door. Adding a picture touches
//! `sheetData` not at all, and [`Workbook::set_cell_value`] never opens the drawing part — so a
//! workbook whose drawing this library did not touch re-emits it byte for byte, anchor content this
//! crate does not model included.

use mjx_dml::spreadsheet_drawing::{
    new_absolute_anchor, new_anchored_picture, new_one_cell_anchor, new_two_cell_anchor, Anchor,
    AnchorShift, AnchoredObject, CellMarker, WorksheetDrawing,
};
use mjx_dml::{Position, Size};
use mjx_ooxml_core::{Interner, RawDocument, RawNode, ToXml};
use mjx_ooxml_types::spreadsheetdrawing::ResizingBehavior;
use mjx_opc::{ImageFormat, PartName, Relationship, TargetMode};
use mjx_sml::write::constants::XML_DECLARATION;
use mjx_sml::{ColumnMetrics, ResolvedAnchorBounds, SheetAnchors};

use crate::error::XlsxError;
use crate::parts::{PartKind, CONTENT_TYPE_DRAWING, REL_IMAGE};
use crate::workbook::Workbook;

/// One anchored object on a sheet, decoded.
///
/// A **report**, not a second model: every field is read out of
/// [`mjx_dml::spreadsheet_drawing`] and nothing here can write one. The markup itself is reached
/// through [`Workbook::drawing_markup`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetDrawingObject {
    /// The anchor's position in the drawing part, which is also its **paint order**: a later anchor
    /// is drawn over an earlier one.
    pub index: usize,
    /// Which of the three anchor elements pins it — `twoCellAnchor`, `oneCellAnchor` or
    /// `absoluteAnchor`.
    pub anchor: &'static str,
    /// Which of the six `EG_ObjectChoices` members it holds — `sp`, `pic`, `graphicFrame`,
    /// `grpSp`, `cxnSp` or `contentPart` — or `None` for an anchor holding none, which the schema
    /// forbids and this crate reports rather than repairs.
    pub object: Option<&'static str>,
    /// What the anchor promises to do when the cells under it move.
    pub resizing: ResizingBehavior,
    /// The object's `cNvPr@id`, or `None` for one with no non-visual block (a `contentPart`, or
    /// markup missing it).
    pub id: Option<u32>,
    /// The object's `cNvPr@name`.
    pub name: Option<String>,
    /// For an `xdr:pic`, the image part its `a:blip@r:embed` resolves to. `None` for every other
    /// object kind, for a picture that links an external image, and for a relationship identifier
    /// the drawing part does not declare.
    pub image: Option<PartName>,
    /// Whether the object prints with the sheet (`xdr:clientData@fPrintsWithSheet`, which
    /// **defaults to `true`**).
    pub prints_with_sheet: bool,
}

/// One sheet's drawing part, and what is anchored in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetDrawing {
    /// The part the anchors live in.
    pub part: PartName,
    /// The `x:drawing@r:id` the sheet reached it through.
    pub relationship_id: String,
    /// Every anchored object, in paint order.
    pub objects: Vec<SheetDrawingObject>,
}

impl Workbook {
    /// The drawing part behind the tab at `index`, and everything anchored in it.
    ///
    /// `Ok(None)` when the sheet writes no `x:drawing`, when the `r:id` it writes names no
    /// relationship, or when the part that relationship reaches is not an `xdr:wsDr`. All three are
    /// facts about the file, reported as absence rather than as an error.
    ///
    /// # Errors
    /// [`XlsxError::NoSuchSheet`] if `index` names no tab; [`XlsxError`] if the worksheet or the
    /// drawing part is not well-formed XML.
    pub fn sheet_drawing(&self, index: usize) -> Result<Option<SheetDrawing>, XlsxError> {
        let Some((part, relationship_id)) = self.sheet_drawing_part(index)? else {
            return Ok(None);
        };
        let Some((document, drawing)) = self.read_drawing_document(&part)? else {
            return Ok(None);
        };
        let objects = self.decode_objects(&part, &drawing, &document.interner);
        Ok(Some(SheetDrawing {
            part,
            relationship_id,
            objects,
        }))
    }

    /// Hands the drawing part behind the tab at `index` to `read`, for a caller that needs the
    /// markup rather than the report.
    ///
    /// `Ok(None)` when the sheet has no drawing part. The model is handed over with the interner its
    /// names live in, because an anchor read out of one part cannot be read through another's.
    ///
    /// # Errors
    /// As [`sheet_drawing`](Self::sheet_drawing).
    pub fn drawing_markup<R>(
        &self,
        index: usize,
        read: impl FnOnce(&WorksheetDrawing, &Interner) -> R,
    ) -> Result<Option<R>, XlsxError> {
        let Some((part, _)) = self.sheet_drawing_part(index)? else {
            return Ok(None);
        };
        let Some((document, drawing)) = self.read_drawing_document(&part)? else {
            return Ok(None);
        };
        Ok(Some(read(&drawing, &document.interner)))
    }

    /// Hands the drawing part behind the tab at `index` to `edit`, then writes it back.
    ///
    /// The part's bytes are replaced with what the model emits, so an edit that changes nothing
    /// still re-emits every anchor from the nodes it was parsed from — including an anchor kind this
    /// workspace does not model.
    ///
    /// `Ok(None)` when the sheet has no drawing part; nothing is written in that case.
    ///
    /// # Errors
    /// As [`sheet_drawing`](Self::sheet_drawing), plus [`XlsxError::Opc`] if the package refuses the
    /// replacement.
    pub fn edit_drawing_markup<R>(
        &mut self,
        index: usize,
        edit: impl FnOnce(&mut WorksheetDrawing, &mut Interner) -> R,
    ) -> Result<Option<R>, XlsxError> {
        let Some((part, _)) = self.sheet_drawing_part(index)? else {
            return Ok(None);
        };
        let Some((mut document, mut drawing)) = self.read_drawing_document(&part)? else {
            return Ok(None);
        };
        let answer = {
            let RawDocument { interner, root, .. } = &mut document;
            let answer = edit(&mut drawing, interner);
            drawing.write_back(root, interner);
            answer
        };
        self.package_mut()
            .replace_part_bytes(&part, mjx_xml::fidelity::serialize_to_vec(&document))?;
        Ok(Some(answer))
    }

    /// Where the anchor at `anchor_index` on the tab at `index` actually is, in EMU, resolved
    /// against that sheet's own column widths and row heights.
    ///
    /// **Read [`ResolvedAnchorBounds`] before treating the answer as a measurement.** A row height
    /// is exact; a column width is a character count that only becomes a length through `metrics`,
    /// and the answer says per axis whether it came from a stated `ht`/`width`, from the sheet's own
    /// default, or from `sheetFormatPr@baseColWidth`. `Ok(None)` when the sheet does not state
    /// enough to place the object — see [`mjx_sml::SheetAnchors::resolve`].
    ///
    /// # Errors
    /// As [`sheet_drawing`](Self::sheet_drawing).
    pub fn sheet_anchor_bounds(
        &self,
        index: usize,
        anchor_index: usize,
        metrics: ColumnMetrics,
    ) -> Result<Option<ResolvedAnchorBounds>, XlsxError> {
        let Some(sheet) = self.worksheet_markup(index)? else {
            return Ok(None);
        };
        let Some((part, _)) = self.sheet_drawing_part(index)? else {
            return Ok(None);
        };
        let Some((document, drawing)) = self.read_drawing_document(&part)? else {
            return Ok(None);
        };
        let Some(anchor) = drawing.anchor(&document.interner, anchor_index) else {
            return Ok(None);
        };
        Ok(SheetAnchors::new(&sheet, metrics).resolve(&anchor, &document.interner))
    }

    /// Moves every anchor on the tab at `index` for `count` rows inserted at the zero-based `at`.
    ///
    /// Each anchor mode does what it promises — a two-cell anchor moves and sizes, a one-cell anchor
    /// moves and keeps its size, an absolute anchor does neither — and the returned report says per
    /// anchor what happened, including the one promise the markers alone cannot keep. See
    /// [`WorksheetDrawing::insert_rows`].
    ///
    /// **This moves the drawing, not the cells.** Nothing in this library inserts a row into
    /// `sheetData`; a caller doing that itself calls this so the objects over those rows travel with
    /// them.
    ///
    /// # Errors
    /// As [`edit_drawing_markup`](Self::edit_drawing_markup).
    pub fn insert_rows_into_drawing(
        &mut self,
        index: usize,
        at: i32,
        count: u32,
    ) -> Result<Vec<AnchorShift>, XlsxError> {
        Ok(self
            .edit_drawing_markup(index, |drawing, interner| {
                drawing.insert_rows(interner, at, count)
            })?
            .unwrap_or_default())
    }

    /// [`insert_rows_into_drawing`](Self::insert_rows_into_drawing) for rows removed.
    ///
    /// # Errors
    /// As [`edit_drawing_markup`](Self::edit_drawing_markup).
    pub fn remove_rows_from_drawing(
        &mut self,
        index: usize,
        at: i32,
        count: u32,
    ) -> Result<Vec<AnchorShift>, XlsxError> {
        Ok(self
            .edit_drawing_markup(index, |drawing, interner| {
                drawing.remove_rows(interner, at, count)
            })?
            .unwrap_or_default())
    }

    /// [`insert_rows_into_drawing`](Self::insert_rows_into_drawing) on the column axis.
    ///
    /// # Errors
    /// As [`edit_drawing_markup`](Self::edit_drawing_markup).
    pub fn insert_columns_into_drawing(
        &mut self,
        index: usize,
        at: i32,
        count: u32,
    ) -> Result<Vec<AnchorShift>, XlsxError> {
        Ok(self
            .edit_drawing_markup(index, |drawing, interner| {
                drawing.insert_columns(interner, at, count)
            })?
            .unwrap_or_default())
    }

    /// [`remove_rows_from_drawing`](Self::remove_rows_from_drawing) on the column axis.
    ///
    /// # Errors
    /// As [`edit_drawing_markup`](Self::edit_drawing_markup).
    pub fn remove_columns_from_drawing(
        &mut self,
        index: usize,
        at: i32,
        count: u32,
    ) -> Result<Vec<AnchorShift>, XlsxError> {
        Ok(self
            .edit_drawing_markup(index, |drawing, interner| {
                drawing.remove_columns(interner, at, count)
            })?
            .unwrap_or_default())
    }

    /// Removes the anchor at `anchor_index` from the tab at `index`, reporting whether there was
    /// one.
    ///
    /// **The image part it named is left in the package.** Another anchor, or another sheet's
    /// drawing, may show the same image; sweeping it is
    /// [`Package::remove_unreferenced_parts`](mjx_opc::Package::remove_unreferenced_parts)'s, for a
    /// caller who has decided it wants that.
    ///
    /// # Errors
    /// As [`edit_drawing_markup`](Self::edit_drawing_markup).
    pub fn remove_sheet_drawing_object(
        &mut self,
        index: usize,
        anchor_index: usize,
    ) -> Result<bool, XlsxError> {
        Ok(self
            .edit_drawing_markup(index, |drawing, interner| {
                drawing.remove_anchor(interner, anchor_index)
            })?
            .unwrap_or(false))
    }

    /// Anchors a picture of `bytes` between two cells on the tab at `index`, and answers its
    /// position in the drawing's paint order.
    ///
    /// The object has **no extent of its own**: the two markers are its geometry, so widening a
    /// column between them makes it wider. `resizing` is written as `@editAs` unless it is the
    /// schema's own default.
    ///
    /// Six things are written, and they are written together: the drawing part (created if the sheet
    /// has none), its content-type override, the [`REL_DRAWING`](crate::parts::REL_DRAWING)
    /// relationship from the sheet, the `x:drawing` entry at rank 29 of `CT_Worksheet`, the image
    /// part under `xl/media/` with its extension registered as a content-type `Default`, and the
    /// [`REL_IMAGE`](REL_IMAGE) relationship **from the drawing part**.
    ///
    /// **Identical images are stored once.** If a media part already holds exactly these bytes it is
    /// reused, and if the drawing part already relates to it the existing relationship identifier is
    /// used and no relationship is added.
    ///
    /// # Errors
    /// [`XlsxError::NoSuchSheet`] if `index` names no tab, [`XlsxError::MissingWorkbookPart`] if it
    /// reaches no worksheet part, [`XlsxError::UnrecognizedImageFormat`] if the bytes match no image
    /// format this build knows, or [`XlsxError`] if the package refuses an edit.
    #[allow(clippy::too_many_arguments)]
    pub fn add_two_cell_anchored_picture(
        &mut self,
        index: usize,
        bytes: &[u8],
        name: &str,
        from: CellMarker,
        to: CellMarker,
        resizing: ResizingBehavior,
    ) -> Result<usize, XlsxError> {
        self.add_anchored_picture(index, bytes, name, |interner, object| {
            Anchor::TwoCell(new_two_cell_anchor(interner, from, to, object, resizing))
        })
    }

    /// Anchors a picture of `bytes` to one cell on the tab at `index`, at its own `size`.
    ///
    /// It moves with the cell `from` names and keeps its size, whatever happens to the columns and
    /// rows between it and the object's other corner — which is what a `xdr:oneCellAnchor` *is*.
    ///
    /// # Errors
    /// As [`add_two_cell_anchored_picture`](Self::add_two_cell_anchored_picture).
    pub fn add_one_cell_anchored_picture(
        &mut self,
        index: usize,
        bytes: &[u8],
        name: &str,
        from: CellMarker,
        size: Size,
    ) -> Result<usize, XlsxError> {
        self.add_anchored_picture(index, bytes, name, |interner, object| {
            Anchor::OneCell(new_one_cell_anchor(interner, from, size, object))
        })
    }

    /// Anchors a picture of `bytes` to the **sheet** on the tab at `index`, at `position` and
    /// `size`.
    ///
    /// It names no cell, so nothing that happens to the rows and columns moves or resizes it.
    ///
    /// # Errors
    /// As [`add_two_cell_anchored_picture`](Self::add_two_cell_anchored_picture).
    pub fn add_absolute_anchored_picture(
        &mut self,
        index: usize,
        bytes: &[u8],
        name: &str,
        position: Position,
        size: Size,
    ) -> Result<usize, XlsxError> {
        self.add_anchored_picture(index, bytes, name, |interner, object| {
            Anchor::Absolute(new_absolute_anchor(interner, position, size, object))
        })
    }

    /// The one implementation the three anchor modes share.
    fn add_anchored_picture(
        &mut self,
        index: usize,
        bytes: &[u8],
        name: &str,
        build: impl FnOnce(&mut Interner, &AnchoredObject) -> Anchor,
    ) -> Result<usize, XlsxError> {
        // Everything fallible that does not touch the package happens first, so the common refusals
        // leave the workbook exactly as it was.
        let format = ImageFormat::sniff(bytes).ok_or(XlsxError::UnrecognizedImageFormat)?;
        let drawing_part = self.drawing_part_or_create(index)?;

        let media_part = match self.media_part_with_bytes(bytes) {
            Some(existing) => existing,
            None => {
                let part = PartName::new(&self.free_media_part_name(format.file_extension()))?;
                // Registering the `Default` first means `insert_part` adds no per-part `Override`,
                // which is what every producer writes for an image.
                self.package_mut()
                    .set_content_type_default(format.file_extension(), format.content_type())?;
                self.package_mut()
                    .insert_part(&part, format.content_type(), bytes.to_vec())?;
                part
            }
        };
        let relationship_id = match self.image_relationship_id(&drawing_part, &media_part)? {
            Some(existing) => existing,
            None => {
                let id = self.next_sheet_relationship_id(&drawing_part);
                let target = crate::worksheet::tables::relative_target(&drawing_part, &media_part);
                self.package_mut().add_relationship(
                    Some(&drawing_part),
                    Relationship {
                        id: id.clone(),
                        rel_type: REL_IMAGE.to_owned(),
                        target,
                        mode: TargetMode::Internal,
                    },
                )?;
                id
            }
        };

        let Some((mut document, mut drawing)) = self.read_drawing_document(&drawing_part)? else {
            return Err(XlsxError::MissingWorkbookPart(
                drawing_part.as_str().to_owned(),
            ));
        };
        let at = {
            let RawDocument { interner, root, .. } = &mut document;
            let id = next_drawing_id(&drawing, interner);
            let picture = new_anchored_picture(interner, id, name, &relationship_id);
            let anchor = build(interner, &AnchoredObject::Picture(picture));
            drawing.push_anchor(interner, &anchor);
            let at = drawing.anchor_count(interner).saturating_sub(1);
            drawing.write_back(root, interner);
            at
        };
        self.package_mut().replace_part_bytes(
            &drawing_part,
            mjx_xml::fidelity::serialize_to_vec(&document),
        )?;
        Ok(at)
    }

    // -------------------------------------------------------------------------------------------
    // The part graph
    // -------------------------------------------------------------------------------------------

    /// The drawing part behind the tab at `index` and the `x:drawing@r:id` that reached it.
    pub(crate) fn sheet_drawing_part(
        &self,
        index: usize,
    ) -> Result<Option<(PartName, String)>, XlsxError> {
        let sheets = self.sheets().len();
        let sheet_part = self
            .sheets()
            .get(index)
            .ok_or(XlsxError::NoSuchSheet { index, sheets })?
            .part
            .clone();
        let Some(sheet_part) = sheet_part else {
            return Ok(None);
        };
        let Some(markup) = self.worksheet_markup_of(&sheet_part)? else {
            return Ok(None);
        };
        let Some(drawing) = markup.drawing() else {
            return Ok(None);
        };
        let prefix = markup.relationship_prefix();
        let Ok(Some(relationship_id)) = drawing.relationship_id(markup.interner(), prefix) else {
            return Ok(None);
        };
        let Some(part) = self.resolve_sheet_relationship(&sheet_part, &relationship_id)? else {
            return Ok(None);
        };
        Ok(Some((part, relationship_id)))
    }

    /// The drawing part behind the tab at `index`, creating it — and the `x:drawing` entry, the
    /// relationship and the content-type override — when the sheet has none.
    pub(crate) fn drawing_part_or_create(&mut self, index: usize) -> Result<PartName, XlsxError> {
        if let Some((part, _)) = self.sheet_drawing_part(index)? {
            return Ok(part);
        }
        let sheets = self.sheets().len();
        let sheet_part = self
            .sheets()
            .get(index)
            .ok_or(XlsxError::NoSuchSheet { index, sheets })?
            .part
            .clone()
            .ok_or_else(|| XlsxError::MissingWorkbookPart(format!("sheet {index}")))?;

        let mut markup = self
            .worksheet_markup(index)?
            .ok_or_else(|| XlsxError::MissingWorkbookPart(format!("sheet {index}")))?;
        // An `x:drawing` is nothing but an `r:id`, so the part has to be able to spell one — and a
        // worksheet this library authored declares only the SpreadsheetML namespace.
        let prefix = markup.bind_relationship_prefix();

        let part = PartName::new(&self.free_drawing_part_name())?;
        let bytes = new_drawing_part_bytes();

        let relationship_id = self.next_sheet_relationship_id(&sheet_part);
        let target = crate::worksheet::tables::relative_target(&sheet_part, &part);
        self.package_mut()
            .insert_part(&part, CONTENT_TYPE_DRAWING, bytes)?;
        self.package_mut().add_relationship(
            Some(&sheet_part),
            Relationship {
                id: relationship_id.clone(),
                rel_type: PartKind::Drawing.relationship_type().to_owned(),
                target,
                mode: TargetMode::Internal,
            },
        )?;

        {
            let entry_prefix = markup.element_prefix().map(str::to_owned);
            let interner = markup.interner_mut();
            let mut entry = mjx_sml::SheetDrawing::new(interner, entry_prefix.as_deref());
            entry.set_relationship_id(interner, &prefix, &relationship_id);
            markup.set_drawing(Some(entry));
        }
        self.write_worksheet_markup(index, &markup)?;
        Ok(part)
    }

    /// Parses one drawing part into **the document it came from** and a model over its root.
    ///
    /// The whole document, not just the model, because that is what an edit has to go back through:
    /// [`ToXml::write_back`] restores the source range of every node a rebuild reproduced unchanged,
    /// and serializing the original document keeps the part's own XML declaration, its byte-order
    /// mark and anything beside its root. Building a fresh document around a rebuilt root would
    /// re-flow the part and rewrite its prologue — Apache POI writes
    /// `<?xml version="1.0" encoding="UTF-8"?>` where this project writes `standalone="yes"`, so
    /// *edit one anchor* would otherwise change the first line of somebody else's file.
    pub(crate) fn read_drawing_document(
        &self,
        part: &PartName,
    ) -> Result<Option<(RawDocument, WorksheetDrawing)>, XlsxError> {
        let Some(bytes) = self.package().part_payload(part) else {
            return Ok(None);
        };
        let document = mjx_xml::fidelity::parse(&bytes).map_err(mjx_sml::SmlError::from)?;
        let Some(drawing) = WorksheetDrawing::read_root(&document.root, &document.interner)
            .map_err(mjx_sml::SmlError::Model)?
        else {
            return Ok(None);
        };
        Ok(Some((document, drawing)))
    }

    /// Every anchored object of `drawing`, with its image relationship resolved against `part`.
    fn decode_objects(
        &self,
        part: &PartName,
        drawing: &WorksheetDrawing,
        interner: &Interner,
    ) -> Vec<SheetDrawingObject> {
        drawing
            .anchors(interner)
            .enumerate()
            .map(|(index, anchor)| {
                let object = anchor.object(interner);
                let identity = anchor
                    .object(interner)
                    .and_then(|object| object.identity(interner));
                let image = match &object {
                    Some(AnchoredObject::Picture(picture)) => picture
                        .image_relationship_id(interner)
                        .and_then(|id| self.resolve_sheet_relationship(part, &id).ok().flatten()),
                    _ => None,
                };
                SheetDrawingObject {
                    index,
                    anchor: anchor.local(),
                    object: object.as_ref().map(AnchoredObject::local),
                    resizing: anchor.resizing_behavior(interner),
                    id: identity.as_ref().and_then(|props| props.id(interner).ok()),
                    name: identity
                        .as_ref()
                        .and_then(|props| props.drawing_name(interner)),
                    image,
                    prints_with_sheet: anchor
                        .client_data(interner)
                        .is_none_or(|data| data.prints_with_sheet(interner)),
                }
            })
            .collect()
    }

    /// The media part whose stored bytes equal `bytes`, if the package already holds one.
    ///
    /// Comparing slices short-circuits on length, so this is a cheap scan even for large images —
    /// the same deduplication `mjx_pptx::Presentation::add_image` does, one surface earlier.
    fn media_part_with_bytes(&self, bytes: &[u8]) -> Option<PartName> {
        self.package()
            .part_names()
            .filter(|part| part.as_str().starts_with("/xl/media/"))
            .find(|part| self.package().part_payload(part).as_deref() == Some(bytes))
    }

    /// The id of `source`'s existing image relationship pointing at `target`, or `None`.
    fn image_relationship_id(
        &self,
        source: &PartName,
        target: &PartName,
    ) -> Result<Option<String>, XlsxError> {
        let Some(rels) = self.package().relationships_for(Some(source)) else {
            return Ok(None);
        };
        for rel in rels.by_type(REL_IMAGE) {
            if rel.mode == TargetMode::External {
                continue; // a linked image never names a part in this package
            }
            if &crate::nav::resolve_target(source, &rel.target)? == target {
                return Ok(Some(rel.id.clone()));
            }
        }
        Ok(None)
    }

    /// `/xl/drawings/drawingN.xml` for the smallest `N` the package does not already hold.
    ///
    /// Not the drawing count, for the reason [`free_table_part_name`] is not the table count: a
    /// workbook whose second sheet's drawing was deleted holds `drawing1.xml` and `drawing3.xml`.
    ///
    /// [`free_table_part_name`]: crate::worksheet::tables
    fn free_drawing_part_name(&self) -> String {
        let taken: Vec<String> = self
            .package()
            .part_names()
            .map(|part| part.as_str().to_ascii_lowercase())
            .collect();
        for number in 1..=u32::MAX {
            let candidate = format!("/xl/drawings/drawing{number}.xml");
            if !taken.iter().any(|name| name == &candidate) {
                return candidate;
            }
        }
        // Unreachable: the loop runs to four billion and a package cannot hold that many parts.
        "/xl/drawings/drawing1.xml".to_owned()
    }

    /// `/xl/media/imageN.{extension}` for the smallest `N` no `xl/media/` part already uses,
    /// **whatever its extension** — a package holding `image1.png` gets `image2.jpeg`, not a second
    /// `image1`.
    fn free_media_part_name(&self, extension: &str) -> String {
        let mut highest = 0u32;
        for part in self.package().part_names() {
            let Some(stem) = part.as_str().strip_prefix("/xl/media/image") else {
                continue;
            };
            let digits: String = stem.chars().take_while(char::is_ascii_digit).collect();
            if let Ok(number) = digits.parse::<u32>() {
                highest = highest.max(number);
            }
        }
        format!("/xl/media/image{}.{extension}", highest.saturating_add(1))
    }
}

/// The next `cNvPr@id` free in `drawing` — one past the highest any object in it uses.
///
/// One past the **maximum** rather than the count: an id a caller removed may still be named by
/// markup this library did not write, and reusing one would silently repoint it. The same rule
/// `Workbook::next_table_id` follows for a table's `@id`.
fn next_drawing_id(drawing: &WorksheetDrawing, interner: &Interner) -> u32 {
    let mut highest = 0u32;
    for anchor in drawing.anchors(interner) {
        if let Some(id) = anchor
            .object(interner)
            .and_then(|object| object.identity(interner))
            .and_then(|props| props.id(interner).ok())
        {
            highest = highest.max(id);
        }
    }
    highest.saturating_add(1)
}

/// An empty drawing part, prologue and all.
///
/// Built through [`WorksheetDrawing::new`] rather than from a string template, so the three
/// namespace declarations a drawing part needs are stated once, in the crate that owns the schema.
/// The prologue is [`XML_DECLARATION`] exactly — the same first line `mjx-sml`'s package writer and
/// `mjx-pptx`'s and `mjx-docx`'s `blank` constructors emit — because a part this library authors
/// should be indistinguishable from every other part this library authors.
fn new_drawing_part_bytes() -> Vec<u8> {
    let mut interner = Interner::default();
    let root = WorksheetDrawing::new(&mut interner).to_xml(&mut interner);
    let declaration = XML_DECLARATION.trim_end_matches('\n');
    let prologue = vec![
        RawNode::Declaration(Box::from(
            declaration
                .trim_start_matches("<?")
                .trim_end_matches("?>")
                .as_bytes(),
        )),
        RawNode::Text(Box::from(&b"\n"[..])),
    ];
    let document = RawDocument::new(interner, false, prologue, root, Vec::new());
    mjx_xml::fidelity::serialize_to_vec(&document)
}
