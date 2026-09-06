//! [`Error`] — one error type, shaped so a foreign-function binding can act on it.
//!
//! `mjx-pptx` reports failures as [`PptxError`], sixty-six variants each carrying exactly the
//! context its own call site had; `mjx-docx` reports its own as [`DocxError`], thirty-five more; and
//! `mjx-xlsx` reports its own as [`XlsxError`], eleven that in turn open onto [`SmlError`]'s fifteen
//! and [`AddressError`]'s sixteen. That is the right shape for Rust and the wrong shape for a
//! binding: neither PyO3 nor wasm-bindgen can project a hundred-odd variants with as many payload
//! shapes into an exception hierarchy anyone would want to catch, and pinning a stable ABI to a
//! variant list that grows every release is a promise this library cannot keep.
//!
//! So the facade collapses them into **eleven stable [`ErrorCode`]s**, a human [`message`](Error::message),
//! and the machine-readable indices in [`ErrorDetail`] — the surface, shape, row, column and index a
//! caller needs to say *where*. Bindings switch on the code; Rust callers keep everything by
//! downcasting [`source`](std::error::Error::source) back to a [`PptxError`], a [`DocxError`] or an
//! [`XlsxError`], whichever the call that failed belongs to.
//!
//! # The mapping is exhaustive on purpose
//!
//! [`classify`] matches every `PptxError` and every [`OpcError`] variant, [`classify_docx`] matches
//! every [`DocxError`] variant, and [`classify_xlsx`] matches every [`XlsxError`] variant — and,
//! through [`sml_code`] and [`address_code`], every [`SmlError`] and [`AddressError`] variant too.
//! All five have **no wildcard arm**, which is why none of `PptxError`, `DocxError`, `XlsxError`,
//! `SmlError` and `AddressError` is `#[non_exhaustive]`. Adding a variant to any of them fails to
//! compile here until someone decides which code it belongs to. A catch-all arm would instead file
//! every future failure under whichever code happened to be the fallback — and no test would notice.
//! Every one of `DocxError`'s thirty-five variants, and every one of Excel's forty-two across the
//! three enumerations, fits an existing code from the PresentationML mapping; none needed a
//! twelfth.

use std::fmt;

use mjx_docx::DocxError;
use mjx_pptx::{OpcError, PptxError};
use mjx_sml::{AddressError, SmlError};
use mjx_xlsx::XlsxError;

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
    /// The file is a valid Office document, but not one the call it was handed to opens — a `.docx`
    /// given to [`Deck::open`](crate::Deck::open), or a `.xlsb`, whose main part is the MS-XLSB
    /// binary record stream rather than SpreadsheetML and which **no** call here opens.
    ///
    /// Every format is detected — see [`detect_format`](crate::detect_format) — before any of it is
    /// parsed, so a caller is told which format it actually handed over instead of being shown a
    /// parse failure on markup from another language.
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
    /// `None` for every Word failure: WordprocessingML has no equivalent of a shape-bearing surface,
    /// so a `Document` error never populates this field.
    pub surface: Option<Surface>,
    /// The shape addressed, as the path a caller passed: `[2]` top-level, `[2, 1]` inside a group.
    /// `None` for every Word failure, for the same reason as [`surface`](Self::surface).
    pub shape: Option<ShapePath>,
    /// The table row addressed — a `mjx_pptx` table cell, or a `mjx_docx` one (`w:tbl`'s own
    /// `(row, column)` addressing).
    pub row: Option<u32>,
    /// The table column addressed.
    pub column: Option<u32>,
    /// Whatever else was indexed — a slide, layout, master, paragraph, run, field, chart series,
    /// axis, plot, trendline, ActiveX control, the start of a text range, or (for Word) a section.
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

    /// Builds an error the facade itself raises about one `(row, column)` — a block offset outside
    /// the block it addresses.
    pub(crate) fn with_cell(
        code: ErrorCode,
        message: impl Into<String>,
        row: u32,
        column: u32,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            detail: ErrorDetail {
                row: Some(row),
                column: Some(column),
                ..ErrorDetail::default()
            },
            source: None,
        }
    }

    /// Builds an error the facade itself raises about one index — a sheet, above all.
    pub(crate) fn with_index(code: ErrorCode, message: impl Into<String>, index: u32) -> Self {
        Self {
            code,
            message: message.into(),
            detail: ErrorDetail {
                index: Some(index),
                ..ErrorDetail::default()
            },
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

impl From<DocxError> for Error {
    fn from(error: DocxError) -> Self {
        let (code, detail) = classify_docx(&error);
        Self {
            code,
            message: error.to_string(),
            detail,
            source: Some(Box::new(error)),
        }
    }
}

impl From<XlsxError> for Error {
    fn from(error: XlsxError) -> Self {
        let (code, detail) = classify_xlsx(&error);
        Self {
            code,
            message: error.to_string(),
            detail,
            source: Some(Box::new(error)),
        }
    }
}

impl From<SmlError> for Error {
    /// A markup-layer failure that reached this crate without an [`XlsxError`] around it — which the
    /// facade's own cell readers produce when they decode a value themselves.
    fn from(error: SmlError) -> Self {
        Self::from(XlsxError::from(error))
    }
}

impl From<AddressError> for Error {
    /// A cell reference or range the caller spelled wrong. It never reached a file, so it carries no
    /// coordinates — the message names the text that would not parse.
    fn from(error: AddressError) -> Self {
        Self::from(SmlError::from(error))
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
        // A chart's embedded workbook is written by `mjx-sml`, so a failure there is an `SmlError`
        // reaching this crate through PresentationML rather than through `mjx-xlsx`. It is classified
        // by what it says, not by which format carried it — `sml_code` is the same function
        // `classify_xlsx` delegates to.
        PptxError::Sml(sml) => sml_code(sml),
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

/// Classifies a [`ChartAccessError`](mjx_chart::ChartAccessError) — the failures that are about the
/// chart itself rather than about reaching it.
///
/// **Exhaustive, with no wildcard**, for A9's reason: `ChartAccessError` is deliberately not
/// `#[non_exhaustive]` precisely so that a variant added to it stops this file compiling until
/// someone decides which stable code it answers.
///
/// `mjx-pptx` reaches these same verdicts through its own pre-existing `PptxError` variants, and
/// this function gives each the *same* code `classify` gives that variant — so a Word chart and a
/// PowerPoint chart refusing the same index answer the same [`ErrorCode`], which is what makes the
/// two surfaces interchangeable to a binding caller.
fn chart_access_code(error: &mjx_chart::ChartAccessError) -> (ErrorCode, ErrorDetail) {
    use mjx_chart::ChartAccessError as Chart;
    use ErrorCode as C;

    let none = ErrorDetail::default;
    /// The coordinates of a failure that names one index (a series, an axis, a plot, a trendline).
    fn nth(index: usize) -> ErrorDetail {
        ErrorDetail {
            index: Some(count(index)),
            ..ErrorDetail::default()
        }
    }
    match error {
        Chart::SeriesOutOfRange { index, .. }
        | Chart::TrendlineOutOfRange { index, .. }
        | Chart::PlotOutOfRange { index, .. }
        | Chart::AxisOutOfRange { index, .. } => (C::IndexOutOfRange, nth(*index)),
        Chart::SeriesNotEditable { index, .. } => (C::WrongKind, nth(*index)),
        Chart::NoChartElement => (C::MalformedDocument, none()),
        Chart::FillNotSupported => (C::UnsupportedContent, none()),
        Chart::Data(_) => (C::InvalidArgument, none()),
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
        // A `DocumentTimestamp` field out of range (MJXOFF-149) is refused before anything is
        // written, the same shape as an out-of-range slide size.
        OpcError::InvalidDocumentTimestamp { .. } => ErrorCode::InvalidArgument,
    }
}

/// Classifies a [`DocxError`] into its stable code and the coordinates it carries.
///
/// **This match names every variant and has no wildcard arm** — the same discipline [`classify`]
/// keeps for [`PptxError`], and for the same reason: adding a `DocxError` variant must fail to
/// compile here until someone decides what code it belongs to. Every one of the current thirty-five
/// variants fits an existing [`ErrorCode`]; none needed a twelfth.
fn classify_docx(error: &DocxError) -> (ErrorCode, ErrorDetail) {
    use ErrorCode as C;

    let none = ErrorDetail::default;
    /// The coordinates of a failure that names one index (a section, for Word).
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

    match error {
        // --- the layer below, classified by what it means here -----------------------------
        DocxError::Opc(opc) => (opc_code(opc), none()),
        DocxError::Xml(_) | DocxError::Model(_) | DocxError::Mce(_) | DocxError::Vml(_) => {
            (C::MalformedDocument, none())
        }

        // --- the package or a part is not the markup the schema requires -------------------
        DocxError::MissingOfficeDocument
        | DocxError::MissingDocumentPart(_)
        | DocxError::MalformedDocument(_)
        | DocxError::TargetResolution { .. }
        | DocxError::BasedOnChainTooDeep { .. }
        | DocxError::MissingAbstractNumberingReference(_)
        | DocxError::NumberingStyleLinkTooDeep { .. }
        | DocxError::UnbalancedField(_) => (C::MalformedDocument, none()),

        // --- the caller asked to follow a reference that leaves the package ------------------
        DocxError::ExternalTarget { .. } => (C::UnsupportedContent, none()),

        // --- the document is there, but states nothing for this call -----------------------
        DocxError::NoBody
        | DocxError::FieldHasNoCachedResult
        | DocxError::ChartHasNoExternalData
        | DocxError::NumberingStyleLinkHasNoNumbering { .. } => (C::NothingToRead, none()),

        // --- an address or an index argument is outside the document -----------------------
        DocxError::AddressNotFound(_) => (C::IndexOutOfRange, none()),
        DocxError::SectionOutOfRange { index, .. } => (C::IndexOutOfRange, nth(*index)),
        DocxError::TableCellOutOfRange { row, column, .. } => {
            (C::IndexOutOfRange, cell(*row, *column))
        }

        // --- the addressed thing is of a kind that cannot answer ---------------------------
        DocxError::NumberingStyleLinkWrongKind { .. } | DocxError::DrawingIsNotAChart { .. } => {
            (C::WrongKind, none())
        }

        // --- a name resolved to nothing ------------------------------------------------------
        DocxError::UnknownStyleId(_)
        | DocxError::UnknownNumberingId(_)
        | DocxError::UnknownAbstractNumberingId(_)
        | DocxError::NumberingStyleLinkTargetMissing { .. }
        | DocxError::FieldNotFound(_)
        | DocxError::DataBindingPartNotFound { .. }
        | DocxError::DataBindingXPathNotFound { .. }
        | DocxError::AltChunkRelationshipNotFound { .. } => (C::NotFound, none()),

        // --- refused before anything was written --------------------------------------------
        DocxError::InvalidPageSize { .. }
        | DocxError::InvalidTableSize { .. }
        | DocxError::ValueTooLong { .. }
        | DocxError::InvalidChartData
        | DocxError::ChartData(_)
        | DocxError::MalformedDateTime(_) => (C::InvalidArgument, none()),

        // --- a chart-level refusal, classified by `chart_access_code` -----------------------
        //
        // `mjx-chart`'s `ChartAccessError` is a whole enum of its own, and it is reached from both
        // host surfaces (MJXOFF-103). Collapsing it to one code here would be a wildcard arm wearing
        // a variant name — an axis index past the end and an image fill on a series would answer the
        // same thing — so it is classified exhaustively in its own function, the way `sml_code`
        // already is for `XlsxError::Sml`.
        DocxError::ChartAccess(problem) => chart_access_code(problem),
        DocxError::Sml(problem) => sml_code(problem),

        // --- the edit conflicts with the structure already there ----------------------------
        DocxError::FieldHasNestedContent { .. } | DocxError::BookmarkNameInUse(_) => {
            (C::StructureConflict, none())
        }
    }
}

/// Classifies an [`XlsxError`] into its stable code and the coordinates it carries.
///
/// **This match names every variant and has no wildcard arm**, the same discipline [`classify`] and
/// [`classify_docx`] keep — and it reaches one layer further than either of them has to, because
/// `mjx-xlsx` delegates most of what can go wrong to the markup tier: [`sml_code`] and
/// [`address_code`] below are exhaustive over [`SmlError`] and [`AddressError`] for the same reason
/// this one is over [`XlsxError`]. Collapsing `XlsxError::Sml(_)` to one code would have been a
/// wildcard arm wearing a variant name: a number a caller mistyped and a worksheet whose bytes
/// outgrow a `u32` are not the same failure, and only the first is the caller's to fix.
fn classify_xlsx(error: &XlsxError) -> (ErrorCode, ErrorDetail) {
    use ErrorCode as C;

    let none = ErrorDetail::default;
    /// The coordinates of a failure that names one index — a sheet, for Excel.
    fn nth(index: usize) -> ErrorDetail {
        ErrorDetail {
            index: Some(count(index)),
            ..ErrorDetail::default()
        }
    }

    match error {
        // --- the layers below, classified by what they mean here ---------------------------
        XlsxError::Opc(opc) => (opc_code(opc), none()),
        XlsxError::Xml(_) | XlsxError::Model(_) => (C::MalformedDocument, none()),
        XlsxError::Sml(sml) => sml_code(sml),
        XlsxError::InvalidWorkbook(_) => (C::InvalidDocument, none()),

        // --- the package or a part is not the markup the schema requires -------------------
        XlsxError::MissingOfficeDocument
        | XlsxError::MissingWorkbookPart(_)
        | XlsxError::MalformedWorkbook(_)
        | XlsxError::TargetResolution { .. } => (C::MalformedDocument, none()),

        // --- an index argument is outside the workbook -------------------------------------
        XlsxError::NoSuchSheet { index, .. } => (C::IndexOutOfRange, nth(*index)),

        // --- the caller asked to follow a reference that leaves the package ------------------
        //
        // The same reading `PptxError::ExternalTarget` and `DocxError::ExternalTarget` get: an
        // external relationship is legitimate markup, and this library does no external I/O.
        XlsxError::ExternalTarget { .. } => (C::UnsupportedContent, none()),

        // --- refused before anything was written --------------------------------------------
        //
        // The same reading `PptxError::UnrecognizedImageFormat` gets, for the same call: the bytes
        // the caller handed over are the argument, and they are not an image this build knows.
        XlsxError::UnrecognizedImageFormat => (C::InvalidArgument, none()),
    }
}

/// Classifies an [`SmlError`]. Exhaustive for the same reason [`classify_xlsx`] is.
fn sml_code(error: &SmlError) -> (ErrorCode, ErrorDetail) {
    use ErrorCode as C;

    let none = ErrorDetail::default;
    /// The coordinates of a failure that names one index.
    fn nth(index: usize) -> ErrorDetail {
        ErrorDetail {
            index: Some(count(index)),
            ..ErrorDetail::default()
        }
    }

    match error {
        // --- the layers below ---------------------------------------------------------------
        SmlError::Opc(opc) => (opc_code(opc), none()),
        SmlError::Xml(_) | SmlError::Model(_) => (C::MalformedDocument, none()),
        SmlError::Address(address) => (address_code(address), none()),

        // --- refused before anything was written ---------------------------------------------
        //
        // `UnrepresentableNumber` is the caller handing over a `NaN` or an infinity, which
        // SpreadsheetML has no spelling for. `TableHasNoColumns` and `TableGeometryDoesNotFit` are
        // the table authoring surface refusing a spec, exactly as `InvalidTableSize` is for the
        // other two formats.
        SmlError::UnrepresentableNumber { .. }
        | SmlError::TableGeometryDoesNotFit { .. }
        | SmlError::TableHasNoColumns { .. }
        | SmlError::DegenerateMerge { .. } => (C::InvalidArgument, none()),

        // --- an index argument is outside what the file holds ---------------------------------
        SmlError::SheetIndexOutOfRange { index, .. } => (C::IndexOutOfRange, nth(*index)),
        SmlError::CellFormatIndexOutOfRange { index, .. } => {
            (C::IndexOutOfRange, nth(usize_from(*index)))
        }

        // --- the edit conflicts with the structure already there ------------------------------
        SmlError::MergeOverlapsExistingMerge { .. } => (C::StructureConflict, none()),

        // --- the file states nothing this call can work from -----------------------------------
        //
        // A conditional-formatting block with no `@sqref`, or a rule with no `@priority`, is markup
        // that exists and answers nothing — the shape `NothingToRead` names for the other two
        // formats. Neither is repaired; both are still written back verbatim.
        SmlError::ConditionalFormattingBlockHasNoRange { .. }
        | SmlError::ConditionalFormattingRuleHasNoPriority { .. } => (C::NothingToRead, none()),

        // --- this build cannot represent it -----------------------------------------------------
        //
        // `PackedStoreTooLarge` is a real ceiling, not a malformed file: a worksheet larger than the
        // cell store's `u32` byte space is valid SpreadsheetML this build declines to hold.
        // `AuthoredPartSeedRejected` is this library failing to read back what it just wrote, which
        // is a bug here rather than anything the caller did — and `UnsupportedContent` is the code
        // whose documentation already says "report a bug".
        SmlError::PackedStoreTooLarge { .. } | SmlError::AuthoredPartSeedRejected { .. } => {
            (C::UnsupportedContent, none())
        }
    }
}

/// Classifies an [`AddressError`]. Exhaustive for the same reason [`sml_code`] is.
///
/// Every variant is one code: a cell reference or range that does not parse is an **argument**
/// fault, whether it came from a caller's `"A0"` or from a file's `sqref`. The two are told apart by
/// the message, not by the code — and there is no coordinate to carry, because nothing was located.
fn address_code(error: &AddressError) -> ErrorCode {
    match error {
        AddressError::Empty
        | AddressError::MissingColumnLetters
        | AddressError::MissingRowNumber
        | AddressError::UnexpectedCharacter(_)
        | AddressError::ColumnOutOfGrid
        | AddressError::RowOutOfGrid { .. }
        | AddressError::TooManyRangeEnds
        | AddressError::MismatchedRangeEnds
        | AddressError::UnterminatedSheetName
        | AddressError::EmptySheetName
        | AddressError::MissingSheetSeparator
        | AddressError::InvalidExternalBookIndex
        | AddressError::MissingRowColumnMarker
        | AddressError::UnterminatedOffset
        | AddressError::InvalidOffset
        | AddressError::MissingSpanSeparator => ErrorCode::InvalidArgument,
    }
}

/// A `u32` index as the `usize` [`ErrorDetail`]'s helpers take, saturating rather than wrapping.
fn usize_from(index: u32) -> usize {
    usize::try_from(index).unwrap_or(usize::MAX)
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

    /// A `DocxError` converts to the code its own doc comment on [`classify_docx`] claims — picked
    /// asymmetrically (a table address and a name lookup, never the same code twice) so a shuffled
    /// match arm is caught here rather than only by the exhaustiveness check.
    #[test]
    fn docx_errors_classify_into_the_code_their_shape_implies() {
        let out_of_range = DocxError::TableCellOutOfRange {
            row: 4,
            column: 1,
            rows: 2,
            columns: 2,
        };
        let error = Error::from(out_of_range);
        assert_eq!(error.code(), ErrorCode::IndexOutOfRange);
        assert_eq!(error.detail().row, Some(4));
        assert_eq!(error.detail().column, Some(1));

        let not_found = DocxError::UnknownStyleId("Heading9".to_owned());
        assert_eq!(Error::from(not_found).code(), ErrorCode::NotFound);

        let conflict = DocxError::BookmarkNameInUse("Intro".to_owned());
        assert_eq!(Error::from(conflict).code(), ErrorCode::StructureConflict);

        let no_body = DocxError::NoBody;
        assert_eq!(Error::from(no_body).code(), ErrorCode::NothingToRead);

        let section = DocxError::SectionOutOfRange { index: 3, count: 1 };
        let error = Error::from(section);
        assert_eq!(error.code(), ErrorCode::IndexOutOfRange);
        assert_eq!(error.detail().index, Some(3));
    }
}
