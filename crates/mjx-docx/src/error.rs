//! The error type for the WordprocessingML layer.

use mjx_ooxml_core::FromXmlError;
use mjx_ooxml_types::wordprocessingml::StyleType;
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

    /// [`crate::Document::blank`] (or [`crate::Document::blank_with_properties`]) was asked for a
    /// page size this crate's fixed margins cannot fit inside — see [`crate::PageSize`]'s own doc
    /// comment for why this, not a `ST_TwipsMeasure` numeric range, is the real constraint.
    #[error("page size {width_twips}x{height_twips} twips is invalid: {reason}")]
    InvalidPageSize {
        /// The page width asked for, in twips.
        width_twips: u32,
        /// The page height asked for, in twips.
        height_twips: u32,
        /// Why the size was refused.
        reason: &'static str,
    },

    /// A style sheet has no style with this `styleId` — returned by [`crate::StyleIndex`] when the
    /// *starting* point of a lookup or a `w:basedOn` walk does not resolve (an *ancestor* the walk
    /// cannot resolve is not an error — see [`crate::StyleIndex::based_on_chain`]'s own doc
    /// comment).
    #[error("no style with styleId {0:?}")]
    UnknownStyleId(String),

    /// A `w:basedOn` chain starting at `style_id` did not terminate within `limit` steps — almost
    /// certainly a cycle (`w:basedOn` pointing back at an ancestor, directly or indirectly) rather
    /// than a legitimately deep hierarchy. See [`crate::MAX_BASED_ON_CHAIN_DEPTH`]'s own doc comment
    /// for why a bounded depth, not a visited-set, is how this is detected.
    #[error(
        "w:basedOn chain starting at styleId {style_id:?} did not terminate within {limit} steps \
         (likely a cycle)"
    )]
    BasedOnChainTooDeep {
        /// The style the chain walk started from.
        style_id: String,
        /// The bound that was exceeded.
        limit: usize,
    },

    /// [`crate::NumberingIndex::resolve`] (or [`crate::Document::resolve_numbering`]): a `numId`
    /// naming no `w:num` in the numbering definitions part — or, when the document relates to no
    /// `word/numbering.xml` at all, any non-zero `numId` a caller asks to resolve. `0` is never this
    /// — it means "no numbering"; see [`crate::NumberingLookup::None`].
    #[error("no w:num with numId {0}")]
    UnknownNumberingId(i64),

    /// A `w:num`'s own `w:abstractNumId` names no `w:abstractNum` in the same numbering definitions
    /// part.
    #[error("no w:abstractNum with abstractNumId {0}")]
    UnknownAbstractNumberingId(i64),

    /// The `w:num` named by `.0` (its own `numId`) carries no `w:abstractNumId` at all — legal only
    /// for a non-conformant file (`CT_Num`'s `abstractNumId` is `minOccurs="1"`); never rejected on
    /// read (fidelity-first: preserve what is there), only here, on resolution.
    #[error("w:num with numId {0} carries no w:abstractNumId")]
    MissingAbstractNumberingReference(i64),

    /// [`crate::Document::resolve_numbering`]: a `w:abstractNum/w:numStyleLink` names a `styleId`
    /// that does not resolve — either the document relates to no `word/styles.xml` at all, or that
    /// part does not define a style with this id.
    #[error("w:numStyleLink names styleId {style_id:?}, which does not resolve")]
    NumberingStyleLinkTargetMissing {
        /// The unresolved `styleId`.
        style_id: String,
    },

    /// A `w:numStyleLink` target resolves, but is not itself a numbering-type style
    /// (`w:style/@type="numbering"`).
    #[error(
        "w:numStyleLink names styleId {style_id:?}, which is not a numbering-type style (found \
         {found:?})"
    )]
    NumberingStyleLinkWrongKind {
        /// The linked style's own id.
        style_id: String,
        /// The linked style's actual kind, if it states one.
        found: Option<StyleType>,
    },

    /// A `w:numStyleLink` target resolves to a numbering-type style, but that style carries no
    /// `w:pPr/w:numPr/w:numId` of its own to redirect through.
    #[error("numbering style {style_id:?} carries no w:pPr/w:numPr/w:numId")]
    NumberingStyleLinkHasNoNumbering {
        /// The linked style's own id.
        style_id: String,
    },

    /// A `w:numStyleLink` redirect chain starting at `numbering_id` did not terminate within
    /// `limit` hops — almost certainly a cycle. See [`crate::MAX_NUM_STYLE_LINK_DEPTH`]'s own doc
    /// comment for why a bounded depth, not a visited-set, is how this is detected.
    #[error(
        "w:numStyleLink chain starting at numId {numbering_id} did not terminate within {limit} \
         hops (likely a cycle)"
    )]
    NumberingStyleLinkTooDeep {
        /// The `numId` the redirect chain started from.
        numbering_id: i64,
        /// The bound that was exceeded.
        limit: usize,
    },
}
