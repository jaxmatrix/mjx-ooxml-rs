//! The error type for the PresentationML layer.

use mjx_ooxml_core::FromXmlError;
use mjx_opc::OpcError;
use mjx_xml::XmlError;

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

    /// A shape index was out of range on the given slide.
    #[error("shape index {index} out of range on slide {slide} (0..{count})")]
    ShapeIndexOutOfRange {
        /// The slide index.
        slide: usize,
        /// The requested shape index.
        index: usize,
        /// The number of shapes on that slide.
        count: usize,
    },

    /// A run index was out of range within the shape's text body.
    #[error("run index {index} out of range in shape (0..{count})")]
    RunIndexOutOfRange {
        /// The requested run index.
        index: usize,
        /// The number of typed runs in the shape.
        count: usize,
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

    /// The shape has no `p:spPr` shape-properties element to edit.
    #[error("shape has no properties element")]
    ShapeHasNoProperties,
}
