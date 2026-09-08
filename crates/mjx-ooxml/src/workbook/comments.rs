//! Cell comments on a sheet: what a file says, and the three edits that change it.
//!
//! [`mjx_xlsx::SheetComment`] is already owned, but it carries an [`mjx_sml::CellReference`];
//! [`SheetCommentInfo`] is the same value with the cell as A1 text, which is the only reason a
//! second type exists. The box comes with it as [`CommentBoxInfo`], flattened the same way.
//!
//! **The pair is one thing.** An Excel comment is an entry in `xl/commentsN.xml` *and* a `v:shape`
//! in the sheet's legacy VML drawing, and neither half is written or removed without the other —
//! that is `mjx-xlsx`'s tier, and this facade inherits it rather than restating it. A caller who
//! removes one half through the markup accessors and saves gets the refusal that tier reports.

use mjx_sml::CellReference;
use mjx_xlsx::{CommentBox, SheetComment};

use crate::error::Error;
use crate::index::index;

use super::Workbook;

/// One cell comment, resolved across both of the parts it lives in.
///
/// Every field is what the file *says*. [`author`](Self::author) is `None` both when the part lists
/// no author at that index and when the comment claims an index past the end of the list — a fact
/// about the file, not a repair this library performs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetCommentInfo {
    /// The cell the comment is attached to, as A1 text.
    pub cell: String,
    /// `@authorId` — an index into the part's author list, not a name.
    pub author_index: u32,
    /// The name at that index, or `None`.
    pub author: Option<String>,
    /// The displayed text: the plain `t`, then each formatted run's `t`, concatenated.
    pub text: String,
    /// `@shapeId`, when the file states one. Excel writes it; LibreOffice does not.
    pub shape_id: Option<u32>,
    /// The box that draws it, or `None` when the sheet's legacy VML drawing holds no shape for this
    /// comment — the half-a-comment [`Workbook::save`](super::Workbook::save) refuses.
    pub comment_box: Option<CommentBoxInfo>,
}

/// The `v:shape` that draws one comment's pop-up box.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentBoxInfo {
    /// The shape's own `@id`, as the file wrote it.
    pub identifier: Option<String>,
    /// `@o:spid`, the application's identifier for the shape — where LibreOffice puts the generated
    /// `_x0000_s…` while `@id` carries something else.
    pub application_identifier: Option<String>,
    /// Whether the box is showing without the pointer over the cell.
    pub is_visible: bool,
    /// `x:ClientData/x:Anchor` exactly as written — the comma-separated eight numbers Excel
    /// honours. **Not decoded**: it is a different anchor vocabulary from the sheet drawing's, and
    /// this library reports it rather than inventing a second decoder for it.
    pub anchor_text: Option<String>,
    /// `x:ClientData/x:Row` — the zero-based row the box states it is attached to.
    pub row: Option<u32>,
    /// `x:ClientData/x:Column` — the zero-based column.
    pub column: Option<u32>,
    /// The shape's CSS2 `@style`, verbatim. Nothing here parses it.
    pub style: Option<String>,
}

impl Workbook {
    /// Every comment on one sheet, in the order the comments part lists them.
    ///
    /// An empty list for a sheet with no comments part, which is most sheets.
    ///
    /// # Errors
    /// [`ErrorCode::IndexOutOfRange`](crate::ErrorCode::IndexOutOfRange) if `sheet` names no tab, or
    /// [`ErrorCode::MalformedDocument`](crate::ErrorCode::MalformedDocument) if the comments part or
    /// the VML drawing is not well-formed XML.
    pub fn sheet_comments(&self, sheet: u32) -> Result<Vec<SheetCommentInfo>, Error> {
        Ok(self
            .workbook
            .sheet_comments(index(sheet))?
            .into_iter()
            .map(info)
            .collect())
    }

    /// The comment attached to `reference`, or `None` when the cell has none.
    ///
    /// # Errors
    /// As [`sheet_comments`](Self::sheet_comments), plus
    /// [`ErrorCode::InvalidArgument`](crate::ErrorCode::InvalidArgument) if `reference` is not an A1
    /// cell.
    pub fn cell_comment(
        &self,
        sheet: u32,
        reference: &str,
    ) -> Result<Option<SheetCommentInfo>, Error> {
        let reference = CellReference::parse(reference)?;
        Ok(self.workbook.comment_at(index(sheet), reference)?.map(info))
    }

    /// Attaches a comment to `reference`, writing **both halves**: the entry in the comments part
    /// and the `v:shape` that draws its box.
    ///
    /// Creates the comments part, the legacy VML drawing part and everything that names them when
    /// the sheet has none. An existing comment on the same cell is replaced rather than joined.
    /// Answers the numeric shape identifier the new box carries.
    ///
    /// # Errors
    /// As [`sheet_comments`](Self::sheet_comments), plus
    /// [`ErrorCode::InvalidArgument`](crate::ErrorCode::InvalidArgument) if `reference` is not an A1
    /// cell.
    pub fn add_cell_comment(
        &mut self,
        sheet: u32,
        reference: &str,
        author: &str,
        text: &str,
    ) -> Result<u32, Error> {
        let reference = CellReference::parse(reference)?;
        Ok(self
            .workbook
            .add_comment(index(sheet), reference, author, text)?)
    }

    /// Replaces the text of the comment on `reference`, leaving its box exactly as it was.
    ///
    /// `false` when the cell has no comment, in which case nothing is written.
    ///
    /// # Errors
    /// As [`add_cell_comment`](Self::add_cell_comment).
    pub fn set_cell_comment_text(
        &mut self,
        sheet: u32,
        reference: &str,
        text: &str,
    ) -> Result<bool, Error> {
        let reference = CellReference::parse(reference)?;
        Ok(self
            .workbook
            .set_comment_text(index(sheet), reference, text)?)
    }

    /// Removes the comment on `reference` — **both halves**.
    ///
    /// `false` when the cell has no comment, in which case nothing is written. The comments part is
    /// removed with its last comment; the VML drawing is not, because it also draws form controls
    /// and OLE fallbacks.
    ///
    /// # Errors
    /// As [`add_cell_comment`](Self::add_cell_comment).
    pub fn remove_cell_comment(&mut self, sheet: u32, reference: &str) -> Result<bool, Error> {
        let reference = CellReference::parse(reference)?;
        Ok(self.workbook.remove_comment(index(sheet), reference)?)
    }

    /// The `@id` of the legacy VML shape the OLE object at `object` on `sheet` is drawn as, or
    /// `None`.
    ///
    /// The hop `x:oleObjects/oleObject@shapeId` makes — Excel's spelling of `p:oleObj@spid`, which
    /// [`Deck::vml_shape_id_for_ole_object`](crate::Deck) answers for a slide. `None` when the sheet
    /// lists no such object, has no VML drawing, or the drawing holds no shape with that identifier.
    ///
    /// # Errors
    /// As [`sheet_comments`](Self::sheet_comments).
    pub fn vml_shape_id_for_ole_object(
        &self,
        sheet: u32,
        object: u32,
    ) -> Result<Option<String>, Error> {
        Ok(self
            .workbook
            .with_vml_shape_for_ole_object(index(sheet), index(object), |shape, interner| {
                shape.identifier(interner)
            })?
            .flatten())
    }

    /// The `@id` of the legacy VML shape the form control at `control` on `sheet` is drawn as, or
    /// `None`.
    ///
    /// As [`vml_shape_id_for_ole_object`](Self::vml_shape_id_for_ole_object), from
    /// `x:controls/control@shapeId`. An `x:control` is one of Excel's own **form** controls — a
    /// different thing from a slide's ActiveX `p:control`, which is why the two are not one method.
    ///
    /// # Errors
    /// As [`sheet_comments`](Self::sheet_comments).
    pub fn vml_shape_id_for_form_control(
        &self,
        sheet: u32,
        control: u32,
    ) -> Result<Option<String>, Error> {
        Ok(self
            .workbook
            .with_vml_shape_for_form_control(index(sheet), index(control), |shape, interner| {
                shape.identifier(interner)
            })?
            .flatten())
    }

    /// The bytes of the legacy VML drawing part behind `sheet`, or `None` when it has none.
    ///
    /// Verbatim for a part nothing has edited, which is every part of a file this facade only read;
    /// what the part now contains for one it has (MJXOFF-222).
    ///
    /// Preserve-first, the same shape [`Deck::vml_part_bytes`](crate::Deck) has: the bytes are the
    /// legacy VML this library stores and re-emits, and handing them over is the whole of what a
    /// caller outside Rust can do with a `v:shape` this facade does not model.
    ///
    /// # Errors
    /// As [`sheet_comments`](Self::sheet_comments).
    pub fn sheet_vml_part_bytes(&self, sheet: u32) -> Result<Option<Vec<u8>>, Error> {
        let Some((part, _)) = self.workbook.sheet_vml_drawing_part(index(sheet))? else {
            return Ok(None);
        };
        Ok(self
            .workbook
            .package()
            .part_payload(&part)
            .map(std::borrow::Cow::into_owned))
    }
}

/// Flattens one resolved comment into the facade's own shape.
fn info(comment: SheetComment) -> SheetCommentInfo {
    SheetCommentInfo {
        cell: comment.cell.to_string(),
        author_index: comment.author_index,
        author: comment.author,
        text: comment.text,
        shape_id: comment.shape_id,
        comment_box: comment.comment_box.map(box_info),
    }
}

/// Flattens one comment box.
fn box_info(drawn: CommentBox) -> CommentBoxInfo {
    CommentBoxInfo {
        identifier: drawn.identifier,
        application_identifier: drawn.application_identifier,
        is_visible: drawn.is_visible,
        anchor_text: drawn.anchor_text,
        row: drawn.row,
        column: drawn.column,
        style: drawn.style,
    }
}
