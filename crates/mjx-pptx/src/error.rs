//! The error type for the PresentationML layer.

use mjx_ooxml_core::FromXmlError;
use mjx_opc::OpcError;
use mjx_xml::XmlError;

use crate::slide::ShapeKind;
use crate::surface::Surface;

/// Errors produced while opening, reading, editing, or saving a presentation.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PptxError {
    /// The underlying OPC package could not be read, edited, or written.
    #[error(transparent)]
    Opc(#[from] OpcError),

    /// A part was not well-formed XML.
    #[error(transparent)]
    Xml(#[from] XmlError),

    /// A modeled element (e.g. a text body) was malformed.
    #[error(transparent)]
    Model(#[from] FromXmlError),

    /// The package root has no `officeDocument` relationship (not an Office document).
    #[error("package has no officeDocument relationship")]
    MissingOfficeDocument,

    /// The presentation part named by the officeDocument relationship is absent.
    #[error("presentation part {0} is missing from the package")]
    MissingPresentationPart(String),

    /// `presentation.xml` did not have the expected structure.
    #[error("presentation.xml is malformed: {0}")]
    MalformedPresentation(&'static str),

    /// A slide part did not have the expected structure.
    #[error("slide is malformed: {0}")]
    MalformedSlide(&'static str),

    /// A `p:sldId` referenced a relationship id that is not in `presentation.xml.rels`.
    #[error("slide relationship {id} not found")]
    SlideRelNotFound {
        /// The missing relationship id.
        id: String,
    },

    /// A relationship target could not be resolved to a part name.
    #[error("relationship target {target} could not be resolved")]
    TargetResolution {
        /// The unresolvable target.
        target: String,
    },

    /// A relationship target points outside the package (not supported here).
    #[error("external relationship target {target} is not supported")]
    ExternalTarget {
        /// The external target.
        target: String,
    },

    /// A slide index was out of range.
    #[error("slide index {index} out of range (0..{count})")]
    SlideIndexOutOfRange {
        /// The requested index.
        index: usize,
        /// The number of slides.
        count: usize,
    },

    /// A slide-master index was out of range.
    #[error("master index {index} out of range (0..{count})")]
    MasterIndexOutOfRange {
        /// The requested index.
        index: usize,
        /// The number of masters.
        count: usize,
    },

    /// A slide-layout index was out of range.
    #[error("layout index {index} out of range (0..{count})")]
    LayoutIndexOutOfRange {
        /// The requested index.
        index: usize,
        /// The number of layouts.
        count: usize,
    },

    /// A shape index was out of range on the given surface.
    #[error("shape index {index} out of range on {surface} (0..{count})")]
    ShapeIndexOutOfRange {
        /// The surface addressed (slide, layout, or master).
        surface: Surface,
        /// The requested shape index.
        index: usize,
        /// The number of shapes on that surface.
        count: usize,
    },

    /// A paragraph index was out of range within the shape's text body.
    #[error("paragraph index {index} out of range in shape (0..{count})")]
    ParagraphIndexOutOfRange {
        /// The requested paragraph index.
        index: usize,
        /// The number of paragraphs in the shape's text body.
        count: usize,
    },

    /// A run index was out of range within the addressed scope — the whole shape for the flat
    /// [`set_shape_text`](crate::Presentation::set_shape_text), or one paragraph for the
    /// paragraph-addressed calls.
    #[error("run index {index} out of range (0..{count})")]
    RunIndexOutOfRange {
        /// The requested run index.
        index: usize,
        /// The number of typed runs in the addressed scope.
        count: usize,
    },

    /// A text range ran past the end of the paragraph's text, or ended before it started.
    #[error("text range {start}..{end} out of bounds (paragraph has {length} characters)")]
    TextRangeOutOfBounds {
        /// The requested start offset.
        start: usize,
        /// The requested end offset.
        end: usize,
        /// The length of the paragraph's text in the offset unit that was used.
        length: usize,
    },

    /// The shape has no `p:txBody`.
    #[error("shape has no text body")]
    ShapeHasNoTextBody,

    /// The selected run has no `a:t` text element to set.
    #[error("run has no text element")]
    RunHasNoText,

    /// The shape has no `a:prstGeom` preset geometry (it may use `a:custGeom` or inherit geometry
    /// from a placeholder).
    #[error("shape has no preset geometry")]
    ShapeHasNoGeometry,

    /// The shape's `a:prstGeom@prst` names a shape type this build does not recognize.
    #[error("shape has an unrecognized preset geometry type")]
    UnknownShapeType,

    /// A slide cannot be added because there is no existing slide to inherit a layout from.
    #[error("cannot add a slide: no existing slide to borrow a layout from")]
    NoSlideLayout,

    /// The supplied image bytes match no image format this build recognizes (see
    /// [`mjx_opc::ImageFormat`]).
    #[error("image bytes match no recognized image format")]
    UnrecognizedImageFormat,

    /// The shape has no `p:spPr` shape-properties element to edit.
    #[error("shape has no properties element")]
    ShapeHasNoProperties,

    /// The addressed shape is not a picture (`p:pic`), so it has no image to read or replace.
    #[error("shape is not a picture")]
    ShapeIsNotAPicture,

    /// The picture is missing its `p:blipFill` (or its `a:blip`), which the schema requires.
    #[error("picture has no blip fill")]
    PictureHasNoBlipFill,

    /// The addressed shape does not frame a table — it is not a `p:graphicFrame` at all, or the
    /// graphic it frames is a chart or a diagram rather than an `a:tbl`.
    #[error("shape is not a table")]
    ShapeIsNotATable,

    /// The addressed cell is outside the table, which is `rows` by `columns`.
    ///
    /// Merged cells do not create holes — every position within the table is addressable — so this
    /// means the address is genuinely past an edge.
    #[error("cell ({row}, {column}) is outside a {rows}x{columns} table")]
    TableCellOutOfRange {
        /// The row asked for.
        row: usize,
        /// The column asked for.
        column: usize,
        /// The table's row count.
        rows: usize,
        /// The table's column count.
        columns: usize,
    },

    /// The addressed shape's kind has no transform in its schema, so it cannot be positioned or
    /// sized. Only a `p:contentPart` (`CT_Rel`, a reference to an external part) is such a kind.
    #[error("a {kind:?} has no transform to set")]
    ShapeCannotBePositioned {
        /// The kind of shape addressed.
        kind: ShapeKind,
    },
}
