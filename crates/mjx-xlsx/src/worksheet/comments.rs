//! Cell comments at the package tier: the two parts a comment is, the hop from a sheet's modern
//! markup to the legacy shape that draws it, and the surface that keeps the two halves consistent.
//!
//! # A comment is two parts, and a half is a defect
//!
//! | thing | where it lives |
//! |---|---|
//! | the author list and the text | `xl/commentsN.xml`, an `x:comments` — [`mjx_sml::Comments`] |
//! | its content type | `[Content_Types].xml`, [`CONTENT_TYPE_COMMENTS`] |
//! | the edge to it | `xl/worksheets/_rels/sheetN.xml.rels`, [`REL_COMMENTS`] |
//! | the **box** | `xl/drawings/vmlDrawingN.vml`, a `v:shape` — [`mjx_vml::Drawing`] |
//! | its content type | a `vml` Content-Types **Default**, [`CONTENT_TYPE_VML_DRAWING`] |
//! | the edge to it | the same `.rels`, [`REL_VML_DRAWING`] |
//! | the sheet's claim on it | `x:legacyDrawing@r:id` — [`mjx_sml::LegacyDrawing`], rank 30 |
//!
//! Seven things, and unlike a worksheet table or a drawing the two halves are in **different
//! vocabularies**: SpreadsheetML says what the comment says, and Transitional VML says where the
//! box is and whether it is showing. A comment part with no box and a box with no text are both
//! files Excel opens and repairs, so [`Workbook::add_comment`] writes all seven in one call and
//! [`Workbook::remove_comment`] takes all of them away — *both halves or neither*.
//!
//! That is a claim, and [`crate::validate`] is where it is checked rather than asserted:
//! [`SpreadsheetDefect::CommentWithoutABox`](crate::SpreadsheetDefect::CommentWithoutABox) and
//! [`SpreadsheetDefect::CommentBoxWithoutAComment`](crate::SpreadsheetDefect::CommentBoxWithoutAComment)
//! are reported by `Workbook::validate` over a **saved package**, which is the only place both
//! halves are bytes at the same time.
//!
//! # The anchor is read, never inferred
//!
//! A comment box has **two** anchors and they are not the same anchor:
//!
//! * `x:commentPr/anchor` in the comments part is a `CT_ObjectAnchor` — two `xdr:` cell markers.
//!   LibreOffice writes no `commentPr` at all, so for a large share of real workbooks it does not
//!   exist;
//! * `x:ClientData/x:Anchor` **inside the VML shape** is a comma-separated string of eight numbers,
//!   and `x:Row`/`x:Column` beside it name the cell. **This is the one Excel honours**, and this
//!   module reports it exactly as written — [`CommentBox::anchor_text`] is a `String`, not a decoded
//!   rectangle, because decoding it into EMU is MJXOFF-107's sheet geometry over a *different*
//!   anchor vocabulary and inventing a second decoder here would be the duplication the crate split
//!   exists to prevent.
//!
//! # The identifier hop, in Excel's spelling
//!
//! `mjx-pptx`'s gaps page states the PresentationML half: *"`p:oleObj@spid`, `p:control@spid` and
//! `o:OLEObject@ShapeID` all name a VML shape's `id`. `with_vml_shape_for_ole_object` walks it for
//! you."* SpreadsheetML makes the same hop with the same vocabulary and a different type:
//! `x:oleObject@shapeId`, `x:control@shapeId` and `x:comment@shapeId` are `xsd:unsignedInt`, and the
//! shape they name carries `_x0000_s` + that number. The number-to-string spelling and the three
//! attributes producers actually put it in are stated once, in
//! [`mjx_vml::Drawing::shape_by_numeric_identifier`] — one resolver both formats reach, not a second
//! mechanism.
//!
//! [`Workbook::with_vml_shape_for_ole_object`] is deliberately the name `mjx_pptx::Presentation`
//! uses, because it is the same hop. [`Workbook::with_vml_shape_for_form_control`] is *not*
//! PowerPoint's `with_vml_shape_for_activex_control`, and the difference is real: a `p:control` is
//! an **ActiveX** control with a persisted COM state, while an `x:control` is one of Excel's own
//! **form** controls, whose properties are an `x14:formControlPr` part. Naming them alike would
//! claim an equivalence the schemas do not have.
//!
//! # Adding a comment writes no cell, and editing a cell writes no comment
//!
//! The rule [`crate::worksheet::tables`] states, at another door. Adding a comment touches
//! `sheetData` not at all, and [`Workbook::set_cell_value`] never opens the comments part — so a
//! workbook whose comments this library did not touch re-emits both halves byte for byte, including
//! every `o:` attribute and every `v:shadow` no model here names.

use mjx_ooxml_core::{FromXml, Interner, RawDocument, ToXml};
use mjx_opc::{PartName, Relationship, TargetMode};
use mjx_sml::write::constants::XML_DECLARATION;
use mjx_sml::{CellReference, Comment, CommentAuthors, CommentList, Comments, LegacyDrawing};
use mjx_vml::{AttachedObjectKind, Drawing, DrawingContent, Shape, ShapeContent};

use crate::error::XlsxError;
use crate::parts::{
    SheetKind, CONTENT_TYPE_COMMENTS, CONTENT_TYPE_VML_DRAWING, REL_COMMENTS, REL_VML_DRAWING,
};
use crate::workbook::Workbook;

/// The `x:ClientData@ObjectType` a comment's box carries. Every other value on that attribute names
/// a form control, an auditing mark or a plain shape — see [`mjx_vml::AttachedObjectKind`].
const COMMENT_OBJECT_TYPE: AttachedObjectKind = AttachedObjectKind::Comment;

/// One cell comment, resolved across both of its parts.
///
/// A **report**, not a second model: every field is read out of [`mjx_sml::Comments`] and
/// [`mjx_vml::Drawing`], and nothing here can write one. The markup itself is reached through
/// [`Workbook::comments_markup`] and [`Workbook::vml_drawing_markup`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetComment {
    /// The cell the comment is attached to, from `x:comment@ref`.
    pub cell: CellReference,
    /// `x:comment@authorId` — an **index into the part's author list**, not a name.
    pub author_index: u32,
    /// The name at that index, or `None` when the list is shorter than the index claims. Absence is
    /// a fact about the file: a comment whose `@authorId` points past the list is one Excel shows
    /// with no author, and repairing it here would be inventing an author.
    pub author: Option<String>,
    /// The displayed text: the plain `t`, then each formatted run's `t`, concatenated.
    pub text: String,
    /// `x:comment@shapeId`, when the file states one. Excel writes it; LibreOffice does not.
    pub shape_id: Option<u32>,
    /// The box that draws it, or `None` when the sheet's legacy VML drawing holds no shape for this
    /// comment — the half-a-comment [`Workbook::validate`](crate::Workbook::validate) reports.
    pub comment_box: Option<CommentBox>,
}

/// The `v:shape` that draws one comment's pop-up box, read out of the sheet's legacy VML drawing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentBox {
    /// The shape's own `@id`, as the file wrote it — `_x0000_s1025` from Excel, `shape_0` from
    /// LibreOffice.
    pub identifier: Option<String>,
    /// `@o:spid`, the *application's* identifier for the shape. LibreOffice puts the generated
    /// `_x0000_s…` here and something else in `@id`, which is why the hop looks at both.
    pub application_identifier: Option<String>,
    /// Whether the box is showing without the pointer over the cell — `x:Visible` in the shape's
    /// `x:ClientData`. A value-less `<x:Visible/>` is *true* (`ST_TrueFalseBlank`); an absent one is
    /// false, which is Excel's default for a note.
    pub is_visible: bool,
    /// `x:ClientData/x:Anchor` **exactly as written** — the comma-separated eight numbers Excel
    /// honours.
    ///
    /// **Not decoded, deliberately.** It is a different anchor vocabulary from the sheet drawing's
    /// `xdr:` markers, and turning it into a rectangle would be a second decoder for a thing
    /// MJXOFF-107 already models once. `x:commentPr/anchor` is the *other* anchor, and LibreOffice
    /// writes none at all.
    pub anchor_text: Option<String>,
    /// `x:ClientData/x:Row` — the zero-based row the box is attached to, as the shape states it.
    pub row: Option<u32>,
    /// `x:ClientData/x:Column` — the zero-based column, as the shape states it.
    pub column: Option<u32>,
    /// The shape's CSS2 `@style`, verbatim. Position and size live in it, in points; nothing here
    /// parses it, because evaluating VML geometry is a documented non-goal.
    pub style: Option<String>,
}

impl Workbook {
    // -------------------------------------------------------------------------------------------
    // Reading
    // -------------------------------------------------------------------------------------------

    /// Every comment on the tab at `index`, in the order the comments part lists them, each with the
    /// box that draws it.
    ///
    /// An empty vector for a sheet with no comments part, which is most sheets.
    ///
    /// # Errors
    /// [`XlsxError::NoSuchSheet`] if `index` names no tab, or [`XlsxError`] if the comments part or
    /// the VML drawing is not well-formed XML.
    pub fn sheet_comments(&self, index: usize) -> Result<Vec<SheetComment>, XlsxError> {
        let Some((part, _)) = self.sheet_comments_part(index)? else {
            return Ok(Vec::new());
        };
        let Some((comments, interner)) = self.read_comments_part(&part)? else {
            return Ok(Vec::new());
        };
        let boxes = self.read_comment_boxes(index)?;
        let authors = comments.authors();
        let Some(list) = comments.list() else {
            return Ok(Vec::new());
        };
        let mut found = Vec::new();
        for comment in list.comments() {
            let Ok(cell) = comment.cell(&interner) else {
                // `@ref` is `use="required"` and `ST_Ref` admits a range. A comment this crate
                // cannot address is reported by being absent from this list rather than by refusing
                // to read the workbook; `comments_markup` still hands over every byte of it.
                continue;
            };
            let author_index = comment.author_index(&interner).unwrap_or(0);
            let shape_id = comment.shape_id(&interner).ok().flatten();
            found.push(SheetComment {
                cell,
                author_index,
                author: authors
                    .and_then(|authors| authors.author(author_index))
                    .map(|author| author.text().to_owned()),
                text: comment
                    .text()
                    .map(|text| text.text().to_owned())
                    .unwrap_or_default(),
                shape_id,
                comment_box: match_box(&boxes, cell, shape_id),
            });
        }
        Ok(found)
    }

    /// The comment attached to `cell` on the tab at `index`, or `None`.
    ///
    /// # Errors
    /// As [`sheet_comments`](Self::sheet_comments).
    pub fn comment_at(
        &self,
        index: usize,
        cell: CellReference,
    ) -> Result<Option<SheetComment>, XlsxError> {
        Ok(self
            .sheet_comments(index)?
            .into_iter()
            .find(|comment| comment.cell == cell))
    }

    /// Hands the comments part behind the tab at `index` to `read`, for a caller that needs the
    /// markup rather than the report.
    ///
    /// `Ok(None)` when the sheet has no comments part. The model comes with the interner its names
    /// live in, because a symbol read out of one part means nothing in another's — the trap
    /// MJXOFF-107 recorded: a wrong interner answers *whatever string sits at that index* rather
    /// than failing.
    ///
    /// # Errors
    /// As [`sheet_comments`](Self::sheet_comments).
    pub fn comments_markup<R>(
        &self,
        index: usize,
        read: impl FnOnce(&Comments, &Interner) -> R,
    ) -> Result<Option<R>, XlsxError> {
        let Some((part, _)) = self.sheet_comments_part(index)? else {
            return Ok(None);
        };
        let Some((comments, interner)) = self.read_comments_part(&part)? else {
            return Ok(None);
        };
        Ok(Some(read(&comments, &interner)))
    }

    /// Hands the comments part behind the tab at `index` to `edit`, then writes it back — dirtying
    /// **only** that part.
    ///
    /// `Ok(None)` when the sheet has no comments part; nothing is written in that case.
    ///
    /// # Errors
    /// As [`sheet_comments`](Self::sheet_comments), plus [`XlsxError::Opc`] if the package refuses
    /// the replacement.
    pub fn edit_comments_markup<R>(
        &mut self,
        index: usize,
        edit: impl FnOnce(&mut Comments, &mut Interner) -> R,
    ) -> Result<Option<R>, XlsxError> {
        let Some((part, _)) = self.sheet_comments_part(index)? else {
            return Ok(None);
        };
        let Some(bytes) = self.package().part_payload(&part) else {
            return Ok(None);
        };
        let mut document = mjx_xml::fidelity::parse(&bytes).map_err(mjx_sml::SmlError::from)?;
        let mut comments = Comments::from_xml(&document.root, &document.interner)
            .map_err(mjx_sml::SmlError::Model)?;
        let answer = {
            let RawDocument { interner, root, .. } = &mut document;
            let answer = edit(&mut comments, interner);
            comments.write_back(root, interner);
            answer
        };
        self.package_mut()
            .replace_part_bytes(&part, mjx_xml::fidelity::serialize_to_vec(&document))?;
        Ok(Some(answer))
    }

    /// The legacy VML drawing part behind the tab at `index`, and the `x:legacyDrawing@r:id` that
    /// reached it.
    ///
    /// `Ok(None)` when the sheet writes no `x:legacyDrawing`, or the `r:id` it writes names no
    /// relationship. Both are facts about the file, reported as absence rather than as an error.
    ///
    /// # Errors
    /// [`XlsxError::NoSuchSheet`] if `index` names no tab, or [`XlsxError`] if the worksheet is not
    /// well-formed XML.
    pub fn sheet_vml_drawing_part(
        &self,
        index: usize,
    ) -> Result<Option<(PartName, String)>, XlsxError> {
        let sheet_part = self.sheet_part_of(index)?;
        let Some(sheet_part) = sheet_part else {
            return Ok(None);
        };
        let Some(markup) = self.worksheet_markup_of(&sheet_part)? else {
            return Ok(None);
        };
        let Some(drawing) = markup.legacy_drawing() else {
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

    /// Hands the legacy VML drawing behind the tab at `index` to `read` as a typed
    /// [`mjx_vml::Drawing`].
    ///
    /// `Ok(None)` when the sheet has no VML drawing part. Reading dirties nothing.
    ///
    /// # Errors
    /// [`XlsxError::NoSuchSheet`] if `index` names no tab, [`XlsxError::PartIsNotVmlDrawing`] if the
    /// `legacyDrawing` relationship reaches a part that is not one, or [`XlsxError`] if the part is
    /// not well-formed XML.
    pub fn vml_drawing_markup<R>(
        &self,
        index: usize,
        read: impl FnOnce(&Drawing, &Interner) -> R,
    ) -> Result<Option<R>, XlsxError> {
        let Some((part, _)) = self.sheet_vml_drawing_part(index)? else {
            return Ok(None);
        };
        let Some((document, drawing)) = self.read_vml_document(&part)? else {
            return Ok(None);
        };
        Ok(Some(read(&drawing, &document.interner)))
    }

    /// Hands the legacy VML drawing behind the tab at `index` to `edit`, then writes it back —
    /// dirtying **only** that part.
    ///
    /// Everything the model does not name rides through unchanged, and since MJXOFF-143 an element
    /// the model rebuilt without changing it is copied out of the original bytes rather than
    /// reconstructed — so an edit to one shape leaves its siblings, their wrapped start tags and
    /// every `o:` attribute exactly as they were.
    ///
    /// `Ok(None)` when the sheet has no VML drawing part; nothing is written in that case.
    ///
    /// # Errors
    /// As [`vml_drawing_markup`](Self::vml_drawing_markup), plus [`XlsxError::Opc`] if the package
    /// refuses the replacement.
    pub fn edit_vml_drawing_markup<R>(
        &mut self,
        index: usize,
        edit: impl FnOnce(&mut Drawing, &mut Interner) -> R,
    ) -> Result<Option<R>, XlsxError> {
        let Some((part, _)) = self.sheet_vml_drawing_part(index)? else {
            return Ok(None);
        };
        let Some((mut document, mut drawing)) = self.read_vml_document(&part)? else {
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

    // -------------------------------------------------------------------------------------------
    // The identifier hop
    // -------------------------------------------------------------------------------------------

    /// Resolves the comment on `cell` to the VML shape that draws its box, and hands it to `read`.
    ///
    /// Two routes, and both are things the file says rather than things this library computes:
    ///
    /// 1. the comment's `@shapeId`, through [`mjx_vml::Drawing::shape_by_numeric_identifier`] — what
    ///    Excel writes;
    /// 2. failing that, the shape whose `x:ClientData` is a `Note` and whose `x:Row`/`x:Column` name
    ///    that cell — what LibreOffice leaves, since it writes no `@shapeId` at all.
    ///
    /// `Ok(None)` when the sheet has no comment there, has no VML drawing, or the drawing holds no
    /// shape either route reaches.
    ///
    /// # Errors
    /// As [`sheet_comments`](Self::sheet_comments).
    pub fn with_vml_shape_for_comment<R>(
        &self,
        index: usize,
        cell: CellReference,
        read: impl FnOnce(&Shape, &Interner) -> R,
    ) -> Result<Option<R>, XlsxError> {
        let Some(comment) = self.comment_at(index, cell)? else {
            return Ok(None);
        };
        let Some((part, _)) = self.sheet_vml_drawing_part(index)? else {
            return Ok(None);
        };
        let Some((document, drawing)) = self.read_vml_document(&part)? else {
            return Ok(None);
        };
        let interner = &document.interner;
        let shape = comment_box_shape(&drawing, interner, cell, comment.shape_id);
        Ok(shape.map(|shape| read(shape, interner)))
    }

    /// Resolves the embedded OLE object at `object_index` on the tab at `index` to the VML shape
    /// that draws it, and hands it to `read`.
    ///
    /// The hop `x:oleObjects/oleObject@shapeId` makes — Excel's spelling of `p:oleObj@spid`, and
    /// deliberately the same method name `mjx_pptx::Presentation` gives it. `@shapeId` is
    /// `use="required"`, so an object that reaches no shape means the drawing does not hold one,
    /// never that the object failed to name it.
    ///
    /// `Ok(None)` when the sheet lists no such object, has no VML drawing, or the drawing holds no
    /// shape with that identifier.
    ///
    /// # Errors
    /// [`XlsxError::NoSuchSheet`] if `index` names no tab, or [`XlsxError`] if the worksheet or the
    /// VML part is not well-formed XML.
    pub fn with_vml_shape_for_ole_object<R>(
        &self,
        index: usize,
        object_index: usize,
        read: impl FnOnce(&Shape, &Interner) -> R,
    ) -> Result<Option<R>, XlsxError> {
        let Some(shape_id) = self.legacy_shape_id(index, LegacyHost::OleObject, object_index)?
        else {
            return Ok(None);
        };
        self.with_vml_shape(index, shape_id, read)
    }

    /// Resolves the form control at `control_index` on the tab at `index` to the VML shape that
    /// draws it, and hands it to `read`.
    ///
    /// As [`with_vml_shape_for_ole_object`](Self::with_vml_shape_for_ole_object), from
    /// `x:controls/control@shapeId`. **Not** PowerPoint's `with_vml_shape_for_activex_control`: an
    /// `x:control` is one of Excel's own form controls, whose properties are an `x14:formControlPr`
    /// part, where a `p:control` is an ActiveX control with a persisted COM state.
    ///
    /// # Errors
    /// As [`with_vml_shape_for_ole_object`](Self::with_vml_shape_for_ole_object).
    pub fn with_vml_shape_for_form_control<R>(
        &self,
        index: usize,
        control_index: usize,
        read: impl FnOnce(&Shape, &Interner) -> R,
    ) -> Result<Option<R>, XlsxError> {
        let Some(shape_id) = self.legacy_shape_id(index, LegacyHost::Control, control_index)?
        else {
            return Ok(None);
        };
        self.with_vml_shape(index, shape_id, read)
    }

    /// The `@shapeId` of one `x:oleObject` or `x:control` on the tab at `index`.
    ///
    /// Both lists sit in `CT_Worksheet` beside the `drawing` slot rather than inside it (see
    /// [`mjx_sml::features::embedded`]), and both may be wrapped in `mc:AlternateContent` —
    /// LibreOffice wraps `x:controls` in one because Excel 2010 form controls need the `x14`
    /// namespace, exactly as PowerPoint wraps a `p:oleObj`. `WorksheetPart` types the *unwrapped*
    /// slot, so a wrapped one arrives as raw markup and this reaches it through the resolved view.
    fn legacy_shape_id(
        &self,
        index: usize,
        host: LegacyHost,
        entry_index: usize,
    ) -> Result<Option<u32>, XlsxError> {
        let Some(markup) = self.worksheet_markup(index)? else {
            return Ok(None);
        };
        let interner = markup.interner();
        let typed = match host {
            LegacyHost::OleObject => markup.embedded_objects().and_then(|objects| {
                objects
                    .entries()
                    .nth(entry_index)
                    .map(|object| object.shape_id(interner))
            }),
            LegacyHost::Control => markup.form_controls().and_then(|controls| {
                controls
                    .entries()
                    .nth(entry_index)
                    .map(|control| control.shape_id(interner))
            }),
        };
        if let Some(shape_id) = typed {
            return Ok(shape_id.ok());
        }
        let unmodelled: Vec<mjx_ooxml_core::RawNode> = markup
            .children()
            .filter_map(|child| match child {
                mjx_sml::WorksheetContent::Raw(node) => Some(node.clone()),
                _ => None,
            })
            .collect();
        Ok(crate::nav::mce_shape_id(
            &unmodelled,
            interner,
            host.wire_list(),
            host.wire_entry(),
            entry_index,
        ))
    }

    /// Hands the shape `shape_id` names in the tab's VML drawing to `read`.
    fn with_vml_shape<R>(
        &self,
        index: usize,
        shape_id: u32,
        read: impl FnOnce(&Shape, &Interner) -> R,
    ) -> Result<Option<R>, XlsxError> {
        let Some((part, _)) = self.sheet_vml_drawing_part(index)? else {
            return Ok(None);
        };
        let Some((document, drawing)) = self.read_vml_document(&part)? else {
            return Ok(None);
        };
        let interner = &document.interner;
        Ok(drawing
            .shape_by_numeric_identifier(interner, shape_id)
            .map(|shape| read(shape, interner)))
    }

    // -------------------------------------------------------------------------------------------
    // Authoring, editing and deleting — both halves or neither
    // -------------------------------------------------------------------------------------------

    /// Attaches a comment to `cell` on the tab at `index`, writing **both halves**: the entry in the
    /// comments part and the `v:shape` that draws its box.
    ///
    /// Creates the comments part, the VML drawing part, their content types, their relationships and
    /// the sheet's `x:legacyDrawing` entry when the sheet has none. `author` joins the part's author
    /// list, or is reused when the list already carries it exactly.
    ///
    /// The box is authored hidden, sized like Excel's default note, and anchored to `cell` through
    /// its `x:ClientData` `x:Row`/`x:Column` — the pair Excel honours. A caller who wants a different
    /// box reaches [`edit_vml_drawing_markup`](Self::edit_vml_drawing_markup) afterwards.
    ///
    /// Replaces an existing comment on the same cell rather than adding a second: `x:comment@ref` is
    /// how a comment is addressed, and two on one cell is a file no producer writes.
    ///
    /// Only a **worksheet** can carry one. A comment's box is a `v:shape` in a legacy VML drawing
    /// that the sheet's own markup points at through `x:legacyDrawing`, and its anchor is a cell —
    /// neither of which a chartsheet or a dialogsheet has. Such a tab is refused, and refused
    /// *first*: see below.
    ///
    /// # Errors
    /// [`XlsxError::NoSuchSheet`] if `index` names no tab, [`XlsxError::MissingWorkbookPart`] if it
    /// reaches no part, [`XlsxError::SheetIsNotAWorksheet`] if it is a chartsheet or a dialogsheet
    /// — the tab's part is there and it simply cannot carry a comment (MJXOFF-241) — or
    /// [`XlsxError`] if a part is malformed or the package refuses an edit.
    pub fn add_comment(
        &mut self,
        index: usize,
        cell: CellReference,
        author: &str,
        text: &str,
    ) -> Result<u32, XlsxError> {
        // An edit is all-or-nothing. MJXOFF-208 established that for a chart data edit, MJXOFF-210's
        // preservation gate now asserts it for every method on the facade against every fixture, and
        // this is the ordering that satisfies it here.
        //
        // A tab that cannot hold a comment is refused **before the first `insert_part`**, and from
        // the sheet's own kind rather than from its markup — so the answer costs no parse and does
        // not depend on whether the tab happens already to have a VML part. Written the other way
        // round, `comments_part_or_create` had already inserted `xl/commentsN.xml`, amended
        // `[Content_Types].xml` and grown the sheet's `.rels` by the time the dialogsheet in
        // `tests/fixtures/print_and_sheet_kinds.xlsx` was turned away, so a caller who handled the
        // error and saved anyway shipped a comments part for a comment that does not exist
        // (MJXOFF-213).
        if let Some(kind @ (SheetKind::Chartsheet | SheetKind::Dialogsheet)) =
            self.sheets().get(index).and_then(|sheet| sheet.kind)
        {
            return Err(XlsxError::SheetIsNotAWorksheet {
                index,
                kind: Some(kind),
            });
        }
        // The drawing before the comments part, because it is the half that reads the sheet's
        // markup — the last thing here that can refuse a tab whose part is not an `x:worksheet` at
        // all, and it refuses before writing anything of its own.
        let vml_part = self.vml_drawing_part_or_create(index)?;
        let comments_part = self.comments_part_or_create(index)?;
        let shape_id = self.next_legacy_shape_id(index)?;

        self.edit_comments_part(&comments_part, |comments, interner| {
            let prefix = comments
                .element_name()
                .prefix
                .map(|symbol| interner.resolve(symbol).to_owned());
            if comments.authors().is_none() {
                let authors = CommentAuthors::new(interner, prefix.as_deref());
                comments.set_authors(authors);
            }
            if comments.list().is_none() {
                let list = CommentList::new(interner, prefix.as_deref());
                comments.set_list(list);
            }
            let author_index = comments
                .authors_mut()
                .map(|authors| authors.index_for(interner, author))
                .unwrap_or(0);
            let mut comment = Comment::new(interner, prefix.as_deref(), cell, author_index, text);
            comment.set_shape_id(interner, Some(shape_id));
            if let Some(list) = comments.list_mut() {
                let existing = list
                    .comments()
                    .position(|held| held.cell(interner).ok() == Some(cell));
                if let Some(existing) = existing {
                    list.remove(existing);
                }
                list.push(comment);
            }
        })?;

        self.edit_vml_part(&vml_part, |drawing, interner| {
            remove_comment_shape(drawing, interner, cell);
            let shape = new_comment_shape(interner, shape_id, cell);
            drawing.push(DrawingContent::Shape(shape));
        })?;
        Ok(shape_id)
    }

    /// Replaces the text of the comment on `cell`, leaving its box exactly as it was.
    ///
    /// `Ok(false)` when the sheet has no comment there — the text half is what changes, so the box
    /// is not touched and neither half can be left inconsistent.
    ///
    /// # Errors
    /// As [`sheet_comments`](Self::sheet_comments), plus [`XlsxError::Opc`] if the package refuses
    /// the replacement.
    pub fn set_comment_text(
        &mut self,
        index: usize,
        cell: CellReference,
        text: &str,
    ) -> Result<bool, XlsxError> {
        Ok(self
            .edit_comments_markup(index, |comments, interner| {
                let Some(list) = comments.list_mut() else {
                    return false;
                };
                for comment in list.comments_mut() {
                    if comment.cell(interner).ok() == Some(cell) {
                        comment.set_text(interner, text);
                        return true;
                    }
                }
                false
            })?
            .unwrap_or(false))
    }

    /// Removes the comment on `cell` — **both halves**: its entry in the comments part and the
    /// `v:shape` that drew it.
    ///
    /// `Ok(false)` when the sheet has no comment there, in which case nothing is written at all.
    ///
    /// The comments part is **removed from the package** when its last comment goes, along with its
    /// relationship and its content-type override, because an `x:commentList` with no `comment` is
    /// exactly the half-a-comment [`validate`](Self::validate) reports. The VML part is *not*
    /// removed with it: it also draws form controls and OLE fallbacks, so it goes only when the
    /// last shape in it goes.
    ///
    /// # Errors
    /// As [`sheet_comments`](Self::sheet_comments), plus [`XlsxError::Opc`] if the package refuses
    /// an edit.
    pub fn remove_comment(&mut self, index: usize, cell: CellReference) -> Result<bool, XlsxError> {
        let Some((comments_part, _)) = self.sheet_comments_part(index)? else {
            return Ok(false);
        };
        let removed = self
            .edit_comments_markup(index, |comments, interner| {
                let list = comments.list_mut()?;
                let at = list
                    .comments()
                    .position(|held| held.cell(interner).ok() == Some(cell))?;
                let removed = list.remove(at);
                removed.map(|_| list.is_empty())
            })?
            .flatten();
        let Some(list_is_now_empty) = removed else {
            return Ok(false);
        };

        if let Some((vml_part, _)) = self.sheet_vml_drawing_part(index)? {
            self.edit_vml_part(&vml_part, |drawing, interner| {
                remove_comment_shape(drawing, interner, cell);
            })?;
        }
        if list_is_now_empty {
            self.remove_sheet_part(index, &comments_part, REL_COMMENTS)?;
        }
        Ok(true)
    }

    // -------------------------------------------------------------------------------------------
    // The part graph
    // -------------------------------------------------------------------------------------------

    /// The comments part behind the tab at `index` and the relationship that reached it.
    ///
    /// Unlike a drawing, a comments part is reached from the sheet's `.rels` **alone**: `CT_Worksheet`
    /// has no `comments` slot, so there is no `r:id` in the markup to cross-check. That is why the
    /// two halves need a validator rather than falling out of the part graph.
    pub(crate) fn sheet_comments_part(
        &self,
        index: usize,
    ) -> Result<Option<(PartName, String)>, XlsxError> {
        let Some(sheet_part) = self.sheet_part_of(index)? else {
            return Ok(None);
        };
        let Some(relationships) = self.package().relationships_for(Some(&sheet_part)) else {
            return Ok(None);
        };
        let Some(relationship) = relationships
            .iter()
            .find(|relationship| relationship.rel_type == REL_COMMENTS)
        else {
            return Ok(None);
        };
        let id = relationship.id.clone();
        let Some(part) = self.resolve_sheet_relationship(&sheet_part, &id)? else {
            return Ok(None);
        };
        Ok(Some((part, id)))
    }

    /// The comments part behind the tab at `index`, creating it — and its relationship and its
    /// content-type override — when the sheet has none.
    fn comments_part_or_create(&mut self, index: usize) -> Result<PartName, XlsxError> {
        if let Some((part, _)) = self.sheet_comments_part(index)? {
            return Ok(part);
        }
        let sheet_part = self
            .sheet_part_of(index)?
            .ok_or_else(|| XlsxError::MissingWorkbookPart(format!("sheet {index}")))?;
        let part = PartName::new(&self.free_numbered_part_name("/xl/comments", ".xml"))?;
        self.package_mut()
            .insert_part(&part, CONTENT_TYPE_COMMENTS, new_comments_part_bytes())?;
        let relationship_id = self.next_sheet_relationship_id(&sheet_part);
        let target = crate::worksheet::tables::relative_target(&sheet_part, &part);
        self.package_mut().add_relationship(
            Some(&sheet_part),
            Relationship {
                id: relationship_id,
                rel_type: REL_COMMENTS.to_owned(),
                target,
                mode: TargetMode::Internal,
            },
        )?;
        Ok(part)
    }

    /// The legacy VML drawing part behind the tab at `index`, creating it — and the
    /// `x:legacyDrawing` entry, the relationship and the `vml` content-type Default — when the sheet
    /// has none.
    fn vml_drawing_part_or_create(&mut self, index: usize) -> Result<PartName, XlsxError> {
        if let Some((part, _)) = self.sheet_vml_drawing_part(index)? {
            return Ok(part);
        }
        let sheet_part = self
            .sheet_part_of(index)?
            .ok_or_else(|| XlsxError::MissingWorkbookPart(format!("sheet {index}")))?;
        let mut markup = self.require_worksheet_markup(index)?;
        // An `x:legacyDrawing` is nothing but an `r:id`, so the part has to be able to spell one.
        let prefix = markup.bind_relationship_prefix();

        let part = PartName::new(&self.free_numbered_part_name("/xl/drawings/vmlDrawing", ".vml"))?;
        // Registering the `Default` first means `insert_part` adds no per-part `Override`, which is
        // how Office writes it — a `.vml` extension is shared with no other content type.
        self.package_mut()
            .set_content_type_default(mjx_vml::VML_DEFAULT_EXTENSION, CONTENT_TYPE_VML_DRAWING)?;
        self.package_mut()
            .insert_part(&part, CONTENT_TYPE_VML_DRAWING, new_vml_part_bytes())?;
        let relationship_id = self.next_sheet_relationship_id(&sheet_part);
        let target = crate::worksheet::tables::relative_target(&sheet_part, &part);
        self.package_mut().add_relationship(
            Some(&sheet_part),
            Relationship {
                id: relationship_id.clone(),
                rel_type: REL_VML_DRAWING.to_owned(),
                target,
                mode: TargetMode::Internal,
            },
        )?;
        {
            let entry_prefix = markup.element_prefix().map(str::to_owned);
            let interner = markup.interner_mut();
            let mut entry = LegacyDrawing::new(interner, entry_prefix.as_deref());
            entry.set_relationship_id(interner, &prefix, &relationship_id);
            markup.set_legacy_drawing(Some(entry));
        }
        self.write_worksheet_markup(index, &markup)?;
        Ok(part)
    }

    /// One past the largest numeric shape identifier any legacy shape on the tab already uses.
    ///
    /// One past the **maximum** rather than the count, for the reason
    /// [`Workbook::next_table_id`](crate::Workbook) follows the same rule: an identifier a caller
    /// removed may still be named by markup this library did not write, and reusing one would
    /// silently repoint it. Excel starts at 1025, and so does a drawing this library authors.
    fn next_legacy_shape_id(&self, index: usize) -> Result<u32, XlsxError> {
        const EXCEL_FIRST_SHAPE_ID: u32 = 1025;
        let Some((part, _)) = self.sheet_vml_drawing_part(index)? else {
            return Ok(EXCEL_FIRST_SHAPE_ID);
        };
        let Some((document, drawing)) = self.read_vml_document(&part)? else {
            return Ok(EXCEL_FIRST_SHAPE_ID);
        };
        let interner = &document.interner;
        let mut highest = EXCEL_FIRST_SHAPE_ID - 1;
        for shape in drawing.all_shapes() {
            for identifier in [
                shape.identifier(interner),
                shape.application_shape_identifier(interner),
            ]
            .into_iter()
            .flatten()
            {
                if let Some(number) = numeric_shape_identifier(&identifier) {
                    highest = highest.max(number);
                }
            }
        }
        Ok(highest.saturating_add(1))
    }

    /// Parses one comments part into its model and the interner its names live in.
    fn read_comments_part(
        &self,
        part: &PartName,
    ) -> Result<Option<(Comments, Interner)>, XlsxError> {
        let Some(bytes) = self.package().part_payload(part) else {
            return Ok(None);
        };
        Ok(Some(mjx_sml::parse_comments(&bytes)?))
    }

    /// Parses one VML part into **the document it came from** and a model over its root.
    ///
    /// The whole document, not just the model, for the reason
    /// [`read_drawing_document`](Self::read_drawing_document) states: building a fresh document
    /// around a rebuilt root would rewrite the part's own XML declaration. It matters more here than
    /// anywhere else — XlsxWriter writes a `.vml` with **no declaration at all**, and LibreOffice
    /// writes one, so "edit one shape" would otherwise add or drop the first line of somebody else's
    /// file.
    fn read_vml_document(
        &self,
        part: &PartName,
    ) -> Result<Option<(RawDocument, Drawing)>, XlsxError> {
        if !self
            .package()
            .content_type_of(part)
            .is_some_and(mjx_vml::is_vml_content_type)
        {
            return Err(XlsxError::PartIsNotVmlDrawing(part.as_str().to_owned()));
        }
        let Some(bytes) = self.package().part_payload(part) else {
            return Ok(None);
        };
        let document = mjx_xml::fidelity::parse(&bytes).map_err(mjx_sml::SmlError::from)?;
        let drawing = Drawing::from_xml(&document.root, &document.interner)
            .map_err(mjx_sml::SmlError::Model)?;
        Ok(Some((document, drawing)))
    }

    /// Reads, edits and writes back one comments part named by [`PartName`].
    fn edit_comments_part(
        &mut self,
        part: &PartName,
        edit: impl FnOnce(&mut Comments, &mut Interner),
    ) -> Result<(), XlsxError> {
        let Some(bytes) = self.package().part_payload(part) else {
            return Err(XlsxError::MissingWorkbookPart(part.as_str().to_owned()));
        };
        let mut document = mjx_xml::fidelity::parse(&bytes).map_err(mjx_sml::SmlError::from)?;
        let mut comments = Comments::from_xml(&document.root, &document.interner)
            .map_err(mjx_sml::SmlError::Model)?;
        {
            let RawDocument { interner, root, .. } = &mut document;
            edit(&mut comments, interner);
            comments.write_back(root, interner);
        }
        self.package_mut()
            .replace_part_bytes(part, mjx_xml::fidelity::serialize_to_vec(&document))?;
        Ok(())
    }

    /// Reads, edits and writes back one VML part named by [`PartName`].
    fn edit_vml_part(
        &mut self,
        part: &PartName,
        edit: impl FnOnce(&mut Drawing, &mut Interner),
    ) -> Result<(), XlsxError> {
        let Some((mut document, mut drawing)) = self.read_vml_document(part)? else {
            return Err(XlsxError::MissingWorkbookPart(part.as_str().to_owned()));
        };
        {
            let RawDocument { interner, root, .. } = &mut document;
            edit(&mut drawing, interner);
            drawing.write_back(root, interner);
        }
        self.package_mut()
            .replace_part_bytes(part, mjx_xml::fidelity::serialize_to_vec(&document))?;
        Ok(())
    }

    /// The worksheet part behind the tab at `index`, or `None` when the entry reaches none.
    ///
    /// # Errors
    /// [`XlsxError::NoSuchSheet`] if `index` names no tab.
    pub(crate) fn sheet_part_of(&self, index: usize) -> Result<Option<PartName>, XlsxError> {
        let sheets = self.sheets().len();
        Ok(self
            .sheets()
            .get(index)
            .ok_or(XlsxError::NoSuchSheet { index, sheets })?
            .part
            .clone())
    }

    /// Removes `part` from the package and the sheet's relationship of type `rel_type` that reached
    /// it.
    ///
    /// The relationship first, so that a package refusing the part removal is not left with a
    /// dangling edge; `remove_part` takes the content-type override with it.
    fn remove_sheet_part(
        &mut self,
        index: usize,
        part: &PartName,
        rel_type: &str,
    ) -> Result<(), XlsxError> {
        let Some(sheet_part) = self.sheet_part_of(index)? else {
            return Ok(());
        };
        let ids: Vec<String> = self
            .package()
            .relationships_for(Some(&sheet_part))
            .map(|relationships| {
                relationships
                    .iter()
                    .filter(|relationship| relationship.rel_type == rel_type)
                    .map(|relationship| relationship.id.clone())
                    .collect()
            })
            .unwrap_or_default();
        for id in ids {
            self.package_mut()
                .remove_relationship(Some(&sheet_part), &id)?;
        }
        self.package_mut().remove_part(part)?;
        Ok(())
    }

    /// Every comment box on the tab at `index`, read out of its VML drawing.
    fn read_comment_boxes(&self, index: usize) -> Result<Vec<CommentBox>, XlsxError> {
        // The error is **propagated**, not swallowed into an empty list. A sheet with no VML part
        // answers `None` here and that is absence; a VML part that will not parse is a malformed
        // file, and `sheet_comments` says so rather than reporting every comment as boxless — which
        // would look exactly like the half-a-comment `validate` refuses.
        Ok(self
            .vml_drawing_markup(index, |drawing, interner| {
                drawing
                    .all_shapes()
                    .into_iter()
                    .filter(|shape| is_comment_shape(shape, interner))
                    .map(|shape| read_comment_box(shape, interner))
                    .collect()
            })?
            .unwrap_or_default())
    }
}

/// Which of `CT_Worksheet`'s two legacy-object lists a `@shapeId` is being read from.
#[derive(Debug, Clone, Copy)]
enum LegacyHost {
    /// `x:oleObjects/oleObject` — rank 34.
    OleObject,
    /// `x:controls/control` — rank 35.
    Control,
}

impl LegacyHost {
    /// The wire local name of the list element.
    fn wire_list(self) -> &'static str {
        match self {
            Self::OleObject => "oleObjects",
            Self::Control => "controls",
        }
    }

    /// The wire local name of one entry.
    fn wire_entry(self) -> &'static str {
        match self {
            Self::OleObject => "oleObject",
            Self::Control => "control",
        }
    }
}

/// The box among `boxes` that draws the comment on `cell`.
///
/// `shape_id` first, then the `x:Row`/`x:Column` the shape itself states — the two routes
/// [`Workbook::with_vml_shape_for_comment`] documents, applied to the boxes already read.
fn match_box(
    boxes: &[CommentBox],
    cell: CellReference,
    shape_id: Option<u32>,
) -> Option<CommentBox> {
    if let Some(number) = shape_id {
        let generated = mjx_vml::shape_identifier_for_number(number);
        if let Some(found) = boxes.iter().find(|drawn| {
            drawn.identifier.as_deref() == Some(generated.as_str())
                || drawn.application_identifier.as_deref() == Some(generated.as_str())
        }) {
            return Some(found.clone());
        }
    }
    boxes
        .iter()
        .find(|drawn| {
            drawn.row == Some(cell.row()) && drawn.column == Some(u32::from(cell.column()))
        })
        .cloned()
}

/// The `v:shape` in `drawing` that draws the comment on `cell`, by the two routes
/// [`Workbook::with_vml_shape_for_comment`] documents — the comment's own `@shapeId` first, then the
/// `x:Row`/`x:Column` the shape itself states.
///
/// **The one implementation of the pairing rule.** `Workbook::sheet_comments`,
/// `Workbook::with_vml_shape_for_comment` and [`crate::validate`]'s two-halves check all resolve a
/// comment to its box through this, so a mutation to it reddens every one of them rather than
/// leaving a second copy quietly agreeing with itself.
pub(crate) fn comment_box_shape<'a>(
    drawing: &'a Drawing,
    interner: &Interner,
    cell: CellReference,
    shape_id: Option<u32>,
) -> Option<&'a Shape> {
    shape_id
        .and_then(|number| drawing.shape_by_numeric_identifier(interner, number))
        .filter(|shape| is_comment_shape(shape, interner))
        .or_else(|| comment_shape_for_cell(drawing, interner, cell))
}

/// The cell a comment box states it is attached to — its `x:ClientData` `x:Column` and `x:Row`.
pub(crate) fn comment_box_cell(shape: &Shape, interner: &Interner) -> Option<CellReference> {
    let data = shape.attached_object_data()?;
    let column = client_data_number(data, interner, "Column")?;
    let row = client_data_number(data, interner, "Row")?;
    CellReference::relative(u16::try_from(column).ok()?, row).ok()
}

/// Whether `shape` is a comment's box — its `x:ClientData` says `ObjectType="Note"`.
pub(crate) fn is_comment_shape(shape: &Shape, interner: &Interner) -> bool {
    shape
        .attached_object_data()
        .and_then(|data| data.kind(interner))
        == Some(COMMENT_OBJECT_TYPE)
}

/// The comment box in `drawing` whose `x:ClientData` names `cell`.
fn comment_shape_for_cell<'a>(
    drawing: &'a Drawing,
    interner: &Interner,
    cell: CellReference,
) -> Option<&'a Shape> {
    drawing.all_shapes().into_iter().find(|shape| {
        is_comment_shape(shape, interner)
            && shape.attached_object_data().is_some_and(|data| {
                client_data_number(data, interner, "Row") == Some(cell.row())
                    && client_data_number(data, interner, "Column")
                        == Some(u32::from(cell.column()))
            })
    })
}

/// Reads one `v:shape` into the report.
fn read_comment_box(shape: &Shape, interner: &Interner) -> CommentBox {
    let data = shape.attached_object_data();
    CommentBox {
        identifier: shape.identifier(interner),
        application_identifier: shape.application_shape_identifier(interner),
        is_visible: data.is_some_and(|data| data.flag(interner, "Visible").unwrap_or(false)),
        anchor_text: data.and_then(|data| data.anchor(interner)),
        row: data.and_then(|data| client_data_number(data, interner, "Row")),
        column: data.and_then(|data| client_data_number(data, interner, "Column")),
        style: shape.style(interner),
    }
}

/// One `x:ClientData` setting read as an unsigned number, or `None` when it is absent or is not one.
fn client_data_number(
    data: &mjx_vml::AttachedObjectData,
    interner: &Interner,
    local: &str,
) -> Option<u32> {
    data.setting(interner, local)?.trim().parse().ok()
}

/// The number in a generated VML shape identifier — `_x0000_s1025` is 1025 — or `None` for one that
/// is not in that form.
fn numeric_shape_identifier(identifier: &str) -> Option<u32> {
    identifier.strip_prefix("_x0000_s")?.parse().ok()
}

/// Removes the comment box for `cell` from `drawing`, if it holds one.
///
/// **By position, never by identifier.** LibreOffice gives *every* comment shape in a part the same
/// `@id` — `shape_0`, twice over in `tests/fixtures/cell_comments.xlsx` — so removing "the shape
/// whose id is this" takes every comment box on the sheet with it. That is a defect the two-halves
/// invariant then reports on the *other* comments, which is how it was found; a producer's file
/// disagreeing with what this library would have written is the whole reason that fixture exists.
///
/// Only a top-level shape is removed. A comment box inside a `v:group` is not something any producer
/// writes, and reaching into a group to delete one member would change the group's own geometry.
fn remove_comment_shape(drawing: &mut Drawing, interner: &Interner, cell: CellReference) {
    let at = drawing.content().iter().position(|child| match child {
        DrawingContent::Shape(shape) => {
            is_comment_shape(shape, interner) && comment_box_cell(shape, interner) == Some(cell)
        }
        _ => false,
    });
    if let Some(at) = at {
        drawing.content_mut().remove(at);
    }
}

/// A fresh comment box for `cell`, sized and anchored the way Excel authors a new note.
///
/// The `@style` is Excel's own default note geometry, stated once here rather than at every call
/// site; the `x:ClientData` carries the `x:Row`/`x:Column` pair that anchors it and the `x:Anchor`
/// Excel honours, both derived from `cell` and neither guessed at afterwards.
fn new_comment_shape(interner: &mut Interner, shape_id: u32, cell: CellReference) -> Shape {
    const DEFAULT_NOTE_STYLE: &str =
        "position:absolute;margin-left:59.25pt;margin-top:1.5pt;width:96pt;height:55.5pt;\
         z-index:1;visibility:hidden";
    let mut shape = Shape::new(
        interner,
        &mjx_vml::shape_identifier_for_number(shape_id),
        DEFAULT_NOTE_STYLE,
    );
    shape.set_template_identifier(interner, "_x0000_t202");
    shape.set_fill_color(interner, "#ffffe1");
    let mut data = mjx_vml::AttachedObjectData::new(interner, COMMENT_OBJECT_TYPE);
    data.set_setting(interner, "MoveWithCells", "");
    data.set_setting(interner, "SizeWithCells", "");
    // The eight-number anchor Excel honours: the box hangs one column to the right of the cell and
    // spans two columns by four rows, which is the shape of a default note.
    data.set_setting(
        interner,
        "Anchor",
        &format!(
            "{}, 15, {}, 10, {}, 15, {}, 4",
            u32::from(cell.column()) + 1,
            cell.row(),
            u32::from(cell.column()) + 3,
            cell.row() + 4
        ),
    );
    data.set_setting(interner, "AutoFill", "False");
    data.set_setting(interner, "Row", &cell.row().to_string());
    data.set_setting(interner, "Column", &cell.column().to_string());
    shape.push(ShapeContent::AttachedObjectData(data));
    shape
}

/// An empty comments part, prologue and all.
///
/// Built as bytes rather than through the model because a fresh `x:comments` has to declare its own
/// namespace, and the two required children are what [`Workbook::add_comment`] fills in. The
/// prologue is [`XML_DECLARATION`] exactly — the same first line `mjx-sml`'s package writer emits —
/// because a part this library authors should be indistinguishable from every other part this
/// library authors.
fn new_comments_part_bytes() -> Vec<u8> {
    let mut bytes = XML_DECLARATION.as_bytes().to_vec();
    bytes.extend_from_slice(
        br#"<comments xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><authors/><commentList/></comments>"#,
    );
    bytes
}

/// An empty legacy VML drawing part, prologue and all.
///
/// The root is built through [`mjx_vml::Drawing::new`], so the namespace declarations a VML part
/// needs are stated once, in the crate that owns the vocabulary.
fn new_vml_part_bytes() -> Vec<u8> {
    let mut interner = Interner::new();
    let drawing = Drawing::new(&mut interner);
    let root = drawing.to_xml(&mut interner);
    let mut bytes = XML_DECLARATION.as_bytes().to_vec();
    mjx_xml::fidelity::serialize_element(&root, &interner, None, &mut bytes);
    bytes
}
