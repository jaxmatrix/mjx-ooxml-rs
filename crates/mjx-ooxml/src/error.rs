//! [`Error`] — one error type, shaped so a foreign-function binding can act on it.
//!
//! `mjx-pptx` reports failures as [`PptxError`], sixty-five variants each carrying exactly the
//! context its own call site had. That is the right shape for Rust and the wrong shape for a
//! binding: neither PyO3 nor wasm-bindgen can project sixty-five variants with sixty-five payload
//! shapes into an exception hierarchy anyone would want to catch, and pinning a stable ABI to a
//! variant list that grows every release is a promise this library cannot keep.
//!
//! So the facade collapses them into **eleven stable [`ErrorCode`]s**, a human [`message`](Error::message),
//! and the machine-readable indices in [`ErrorDetail`] — the surface, shape, row, column and index a
//! caller needs to say *where*. Bindings switch on the code; Rust callers keep everything by
//! downcasting [`source`](std::error::Error::source) back to a [`PptxError`].
//!
//! # The mapping is exhaustive on purpose
//!
//! [`classify`] matches every `PptxError` and every [`OpcError`] variant with **no wildcard arm**,
//! which is why [`PptxError`] is deliberately not `#[non_exhaustive]`. Adding a variant down there
//! fails to compile up here until someone decides which code it belongs to. A catch-all arm would
//! instead file every future failure under whichever code happened to be the fallback — and no test
//! would notice.

use std::fmt;

use mjx_pptx::{OpcError, PptxError};

use crate::address::{ShapePath, Surface};

use crate::index::count;

/// The stable classification a binding switches on.
///
/// Eleven codes, chosen so that each one implies a different thing for the caller to *do*: fix an
/// argument, look somewhere else, give up on this document, or report a bug. They are the contract;
/// [`Error::message`] and [`Error::detail`] are the explanation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ErrorCode {
    /// The container bytes could not be read or written — a truncated or corrupt ZIP, or a writer
    /// that failed. Nothing about the document was learned.
    Io,
    /// The bytes are a package, but its markup is not what the schema requires: a part that is not
    /// well-formed XML, a missing `officeDocument` relationship, a `p:sldId` naming a relationship
    /// that is not there, a geometry formula that does not evaluate.
    MalformedDocument,
    /// The document in memory breaks an invariant, so writing it was refused. This is what
    /// [`Deck::save`](crate::Deck::save) reports rather than emitting a file PowerPoint would offer
    /// to repair; [`Deck::save_unchecked`](crate::Deck::save_unchecked) is the deliberate override.
    InvalidDocument,
    /// An index or range argument is outside what the document holds — a slide, layout, master,
    /// shape, paragraph, run, field, table cell, chart series, axis, trendline or control.
    /// [`ErrorDetail`] says which.
    IndexOutOfRange,
    /// The thing at that address is of a kind that cannot answer the call: a shape that is not a
    /// group being descended into, a graphic frame that holds a chart being read as a table, a part
    /// that is not a VML drawing.
    WrongKind,
    /// A name or identifier resolved to nothing — a table style GUID no `tableStyles.xml` defines, a
    /// relationship id that names no media reference, a part the package does not hold.
    NotFound,
    /// The target exists and is of the right kind, but states nothing for this call: a shape with no
    /// text body, a picture that embeds rather than links its image, a chart with no external
    /// workbook, a slide with no notes.
    NothingToRead,
    /// An argument is refused before anything is written: a slide size outside what `p:sldSz` can
    /// express, a table with no rows, a chart with no data, bytes that are not an image or not
    /// InkML.
    InvalidArgument,
    /// The edit conflicts with the structure the document already has: shapes that are not siblings
    /// asked to be grouped, a shape moved inside itself, a merge that would cut an existing merged
    /// region in half.
    StructureConflict,
    /// The document uses a construct this build does not model, or asks for one it cannot write — an
    /// unrecognized preset shape, an image fill on a chart series, an OPC control part.
    UnsupportedContent,
    /// The file is a valid Office document of a format this build cannot open yet. Word and Excel
    /// documents are detected — see [`detect_format`](crate::detect_format) — before they can be
    /// edited, so a caller can say so precisely instead of reporting a parse failure.
    UnsupportedFormat,
}

impl ErrorCode {
    /// The code's name, exactly as a binding should expose it — `"IndexOutOfRange"`,
    /// `"MalformedDocument"`, and so on. Stable; a change here is a breaking change.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Io => "Io",
            Self::MalformedDocument => "MalformedDocument",
            Self::InvalidDocument => "InvalidDocument",
            Self::IndexOutOfRange => "IndexOutOfRange",
            Self::WrongKind => "WrongKind",
            Self::NotFound => "NotFound",
            Self::NothingToRead => "NothingToRead",
            Self::InvalidArgument => "InvalidArgument",
            Self::StructureConflict => "StructureConflict",
            Self::UnsupportedContent => "UnsupportedContent",
            Self::UnsupportedFormat => "UnsupportedFormat",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Where the failure happened, in the same addressing a caller used to get there.
///
/// Every field is `None` when the failure carried no such coordinate — an unreadable ZIP names no
/// shape. A binding turns these into attributes on its exception; Rust code that wants the rest
/// downcasts [`Error::source`](std::error::Error::source) to a
/// [`PptxError`](mjx_pptx::PptxError).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ErrorDetail {
    /// The surface addressed — a slide, layout, master, notes slide, or the notes master.
    pub surface: Option<Surface>,
    /// The shape addressed, as the path a caller passed: `[2]` top-level, `[2, 1]` inside a group.
    pub shape: Option<ShapePath>,
    /// The table row addressed.
    pub row: Option<u32>,
    /// The table column addressed.
    pub column: Option<u32>,
    /// Whatever else was indexed — a slide, layout, master, paragraph, run, field, chart series,
    /// axis, plot, trendline, ActiveX control, or the start of a text range.
    pub index: Option<u32>,
}

impl ErrorDetail {
    /// Whether the failure carried no coordinates at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// A failure from any facade call: a stable [`code`](Error::code), a human
/// [`message`](Error::message), the [`detail`](Error::detail) coordinates, and the underlying error
/// as [`source`](std::error::Error::source).
#[derive(Debug)]
pub struct Error {
    code: ErrorCode,
    message: String,
    detail: ErrorDetail,
    source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
}

impl Error {
    /// Builds an error the facade itself raises — one with no lower-layer cause, such as a format
    /// this build cannot edit.
    pub(crate) fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            detail: ErrorDetail::default(),
            source: None,
        }
    }

    /// The stable classification. This is what a binding switches on.
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        self.code
    }

    /// The human-readable explanation, as the layer that raised it phrased it. Written for a person
    /// reading a log or a stack trace; do not parse it.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Where it happened, in the caller's own addressing.
    #[must_use]
    pub fn detail(&self) -> &ErrorDetail {
        &self.detail
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {
    /// The underlying failure, still fully typed. Rust callers recover everything the collapse into
    /// a code left behind:
    ///
    /// ```no_run
    /// # use std::error::Error as _;
    /// # fn f(err: mjx_ooxml::Error) {
    /// if let Some(mjx_ooxml::PptxError::ShapeIndexOutOfRange { count, .. }) =
    ///     err.source().and_then(|s| s.downcast_ref())
    /// {
    ///     eprintln!("the surface holds {count} shapes");
    /// }
    /// # }
    /// ```
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_ref()
            .map(|boxed| boxed.as_ref() as &(dyn std::error::Error + 'static))
    }
}

impl From<PptxError> for Error {
    fn from(error: PptxError) -> Self {
        let (code, detail) = classify(&error);
        Self {
            code,
            message: error.to_string(),
            detail,
            source: Some(Box::new(error)),
        }
    }
}

impl From<OpcError> for Error {
    fn from(error: OpcError) -> Self {
        Self::from(PptxError::from(error))
    }
}

/// Classifies a [`PptxError`] into its stable code and the coordinates it carries.
///
/// **This match names every variant and has no wildcard arm.** Adding a variant to [`PptxError`]
/// stops the workspace compiling until it is classified here, which is the whole point: a new
/// failure mode is a decision about what callers should do about it, not something to inherit from a
/// catch-all.
fn classify(error: &PptxError) -> (ErrorCode, ErrorDetail) {
    use ErrorCode as C;

    /// The coordinates of a shape-addressed failure.
    fn at(surface: mjx_pptx::Surface, path: &mjx_pptx::ShapePath) -> ErrorDetail {
        ErrorDetail {
            surface: Some(Surface::from(surface)),
            shape: Some(ShapePath::from(path.clone())),
            ..ErrorDetail::default()
        }
    }
    /// The coordinates of a failure that names one index.
    fn nth(index: usize) -> ErrorDetail {
        ErrorDetail {
            index: Some(count(index)),
            ..ErrorDetail::default()
        }
    }
    /// The coordinates of a failure that names one table cell.
    fn cell(row: usize, column: usize) -> ErrorDetail {
        ErrorDetail {
            row: Some(count(row)),
            column: Some(count(column)),
            ..ErrorDetail::default()
        }
    }
    let none = ErrorDetail::default;

    match error {
        // --- the layers below, classified by what they mean here ---------------------------
        PptxError::Opc(opc) => (opc_code(opc), none()),
        PptxError::Xml(_) | PptxError::Model(_) | PptxError::GuideFormula(_) => {
            (C::MalformedDocument, none())
        }
        PptxError::InvalidPresentation(_) => (C::InvalidDocument, none()),

        // --- the package or a part is not the markup the schema requires -------------------
        PptxError::MissingOfficeDocument
        | PptxError::MissingPresentationPart(_)
        | PptxError::MalformedPresentation(_)
        | PptxError::MalformedSlide(_)
        | PptxError::SlideRelNotFound { .. }
        | PptxError::TargetResolution { .. }
        | PptxError::ChartHasNoChartElement
        | PptxError::DiagramPartMissing { .. } => (C::MalformedDocument, none()),

        // --- an index argument is outside the document ------------------------------------
        PptxError::SlideIndexOutOfRange { index, .. }
        | PptxError::MasterIndexOutOfRange { index, .. }
        | PptxError::LayoutIndexOutOfRange { index, .. }
        | PptxError::ParagraphIndexOutOfRange { index, .. }
        | PptxError::RunIndexOutOfRange { index, .. }
        | PptxError::FieldIndexOutOfRange { index, .. }
        | PptxError::ChartSeriesOutOfRange { index, .. }
        | PptxError::ChartTrendlineOutOfRange { index, .. }
        | PptxError::ChartPlotOutOfRange { index, .. }
        | PptxError::ChartAxisOutOfRange { index, .. }
        | PptxError::ActiveXControlOutOfRange { index, .. } => (C::IndexOutOfRange, nth(*index)),
        PptxError::TextRangeOutOfBounds { start, .. } => (C::IndexOutOfRange, nth(*start)),
        PptxError::ShapeIndexOutOfRange { surface, path, .. } => {
            (C::IndexOutOfRange, at(*surface, path))
        }
        PptxError::TableCellOutOfRange { row, column, .. } => {
            (C::IndexOutOfRange, cell(*row, *column))
        }

        // --- the addressed thing is of a kind that cannot answer ---------------------------
        PptxError::ShapeIsNotAGroup { surface, path } => (C::WrongKind, at(*surface, path)),
        PptxError::ShapeCannotBePositioned { .. }
        | PptxError::ShapeIsNotAPicture
        | PptxError::ShapeIsNotATable
        | PptxError::ShapeIsNotAChart
        | PptxError::ShapeIsNotAnOleObject
        | PptxError::ShapeIsNotADiagram
        | PptxError::ShapeIsNotAContentPart
        | PptxError::PartIsNotVmlDrawing { .. } => (C::WrongKind, none()),

        // --- a name resolved to nothing ---------------------------------------------------
        PptxError::NotAMediaReference { .. } | PptxError::TableStyleNotFound { .. } => {
            (C::NotFound, none())
        }

        // --- it is there, and states nothing for this call ---------------------------------
        PptxError::SurfaceHasNoNotes { slide } => (
            C::NothingToRead,
            ErrorDetail {
                surface: Some(Surface::Slide(count(*slide))),
                ..ErrorDetail::default()
            },
        ),
        PptxError::ShapeHasNoBounds { surface, path } => (C::NothingToRead, at(*surface, path)),
        PptxError::ChartSeriesNotEditable { index, .. } => (C::NothingToRead, nth(*index)),
        PptxError::SurfaceHasNoNotesMaster
        | PptxError::ShapeHasNoTextBody
        | PptxError::RunHasNoText
        | PptxError::ShapeHasNoGeometry
        | PptxError::ShapeHasNoProperties
        | PptxError::PictureHasNoImage
        | PptxError::PictureImageNotLinked
        | PptxError::ChartHasNoExternalData
        | PptxError::NoSlideLayout => (C::NothingToRead, none()),

        // --- refused before anything was written ------------------------------------------
        PptxError::GroupNeedsTwoShapes { surface, .. } => (
            C::InvalidArgument,
            ErrorDetail {
                surface: Some(Surface::from(*surface)),
                ..ErrorDetail::default()
            },
        ),
        PptxError::InvalidSlideSize { .. }
        | PptxError::UnrecognizedImageFormat
        | PptxError::InvalidTableSize { .. }
        | PptxError::InvalidChartData
        | PptxError::ChartData(_)
        | PptxError::InvalidInkContent => (C::InvalidArgument, none()),

        // --- the edit conflicts with the structure already there ---------------------------
        PptxError::ShapeCannotBePlaced { surface, path }
        | PptxError::ShapesAreNotSiblings { surface, path }
        | PptxError::ShapeCannotContainItself { surface, path }
        | PptxError::ShapeHasNoParent { surface, path } => {
            (C::StructureConflict, at(*surface, path))
        }
        PptxError::TableMergeCrossesSelection { row, column } => {
            (C::StructureConflict, cell(*row, *column))
        }

        // --- this build does not model it, or cannot follow it ------------------------------
        //
        // `ExternalTarget` is not a malformed document: a linked image, a linked OLE object or a
        // chart's external workbook are all legitimate markup. It means the caller asked to read
        // *through* a reference that leaves the package, and this library does no external I/O by
        // design. `Deck::external_links` is the surface that reports such references.
        PptxError::UnknownShapeType
        | PptxError::ChartFillNotSupported
        | PptxError::ExternalTarget { .. } => (C::UnsupportedContent, none()),
    }
}

/// Classifies an [`OpcError`]. Exhaustive for the same reason [`classify`] is.
fn opc_code(error: &OpcError) -> ErrorCode {
    match error {
        OpcError::Zip(_) | OpcError::Io(_) => ErrorCode::Io,
        OpcError::Invalid(_) => ErrorCode::InvalidDocument,
        OpcError::Xml(_) | OpcError::Malformed(_) | OpcError::TargetResolution(_) => {
            ErrorCode::MalformedDocument
        }
        OpcError::UnknownPart(_) => ErrorCode::NotFound,
        OpcError::ExternalTarget(_) | OpcError::ControlPart(_) => ErrorCode::UnsupportedContent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A binding hands the error to another thread or another task; the type must permit that.
    #[test]
    fn errors_are_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Error>();
        assert_send_sync::<ErrorCode>();
        assert_send_sync::<ErrorDetail>();
    }

    #[test]
    fn a_code_names_itself_stably() {
        assert_eq!(ErrorCode::IndexOutOfRange.as_str(), "IndexOutOfRange");
        assert_eq!(
            ErrorCode::UnsupportedFormat.to_string(),
            "UnsupportedFormat"
        );
    }
}
