//! The error type for the WordprocessingML layer.

use mjx_ooxml_core::FromXmlError;
use mjx_opc::OpcError;
use mjx_xml::XmlError;

/// Errors produced while opening, reading, editing, or saving a Word document.
///
/// # Deliberately exhaustive
///
/// This enum is **not** `#[non_exhaustive]`, so a `match` over it must name every variant — the same
/// contract `mjx_pptx::PptxError` documents on itself: [`mjx_ooxml::Error`] collapses every variant
/// here into one of its stable [`ErrorCode`]s through a `match` with no wildcard arm, so **adding a
/// variant to this enum fails to compile until it is classified there**. A wildcard arm would
/// silently file the new failure under whatever code the catch-all named, which is exactly the bug
/// the exhaustive match exists to prevent.
///
/// [`mjx_ooxml::Error`]: https://docs.rs/mjx-ooxml
/// [`ErrorCode`]: https://docs.rs/mjx-ooxml
#[derive(Debug, thiserror::Error)]
pub enum DocxError {
    /// The underlying OPC package could not be read, edited, or written.
    #[error(transparent)]
    Opc(#[from] OpcError),

    /// A part was not well-formed XML.
    #[error(transparent)]
    Xml(#[from] XmlError),

    /// A modeled element (e.g. the document root's `@conformance`) was malformed.
    #[error(transparent)]
    Model(#[from] FromXmlError),

    /// The package root has no `officeDocument` relationship (not an Office document).
    #[error("package has no officeDocument relationship")]
    MissingOfficeDocument,

    /// The main document part named by the `officeDocument` relationship is absent.
    #[error("document part {0} is missing from the package")]
    MissingDocumentPart(String),

    /// `word/document.xml` (or another part this crate resolves) did not have the expected
    /// structure.
    #[error("document is malformed: {0}")]
    MalformedDocument(&'static str),

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

    /// The document declares no `w:body` (legal per the schema — `CT_Document`'s `body` is
    /// `minOccurs="0"` — but there is then nothing for a paragraph/run address to resolve against).
    #[error("document has no w:body")]
    NoBody,

    /// A [`crate::BlockPath`] or [`crate::RunPath`] did not resolve — out of range, or (for a run
    /// path) landing on a container rather than, in the end, a run.
    #[error("{0}")]
    AddressNotFound(String),
}
