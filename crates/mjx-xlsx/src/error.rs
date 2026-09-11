//! The error type for the SpreadsheetML **package** layer.

use mjx_ooxml_core::FromXmlError;
use mjx_opc::OpcError;
use mjx_sml::SmlError;
use mjx_xml::XmlError;

/// Errors produced while opening, reading, or saving a workbook package.
///
/// # Deliberately exhaustive
///
/// This enum is **not** `#[non_exhaustive]`, so a `match` over it must name every variant — the same
/// contract [`mjx_pptx::PptxError`](https://docs.rs/mjx-pptx) and
/// [`mjx_docx::DocxError`](https://docs.rs/mjx-docx) document on themselves: the facade collapses
/// every variant here into one of its stable error codes through a `match` with no wildcard arm, so
/// **adding a variant fails to compile until it is classified there**. MJXOFF-137 (D20) wrote that
/// mapping — `mjx_ooxml::error::classify_xlsx` — and proved the property by adding a twelfth variant
/// here and watching the facade fail to compile. It reaches further than this enum: `classify_xlsx`
/// descends into [`SmlError`] and, through it, `AddressError`, and — since MJXOFF-111 (E4) — into
/// `mjx_chart::ChartAccessError`, all four with no wildcard arm.
///
/// # Untrusted input
///
/// Every value of this type comes from a file somebody else wrote. No path in this crate may
/// `unwrap`, `expect` or `panic` on one — a malformed workbook is a returned error, never an abort.
///
/// # Why both [`Opc`](Self::Opc) and [`Sml`](Self::Sml)
///
/// [`SmlError`] wraps an [`OpcError`], an [`XmlError`] and a [`FromXmlError`] of its own, so the two
/// families overlap in what they can *contain*. They do not overlap in what they *mean*, and that is
/// the distinction worth keeping: [`Opc`](Self::Opc) is this crate failing at the package — a
/// container that will not open, a part that is not there — while [`Sml`](Self::Sml) is the markup
/// layer failing inside a part whose bytes were handed to it. Flattening them would throw away which
/// layer a caller has to look at. This mirrors how `mjx-docx` keeps `Vml` and `Mce` separate from its
/// own `Xml`.
#[derive(Debug, thiserror::Error)]
pub enum XlsxError {
    /// The underlying OPC package could not be read or written.
    #[error(transparent)]
    Opc(#[from] OpcError),

    /// A part was not well-formed XML.
    #[error(transparent)]
    Xml(#[from] XmlError),

    /// A modelled element did not match the shape its complex type declares.
    #[error(transparent)]
    Model(#[from] FromXmlError),

    /// The SpreadsheetML markup layer ([`mjx_sml`]) failed on a part this crate handed it.
    #[error(transparent)]
    Sml(#[from] SmlError),

    /// A SpreadsheetML package invariant was broken, so [`Workbook::save`](crate::Workbook::save)
    /// refused to write it. See [`SpreadsheetDefect`](crate::SpreadsheetDefect).
    ///
    /// Boxed for the reason `mjx_pptx::PptxError::InvalidPresentation` states: a defect carries the
    /// part, the sheet and the identifiers at fault — enough context to fix the fault without
    /// re-deriving it — and every fallible call in this crate would otherwise pay for that on its
    /// `Result`.
    #[error(transparent)]
    InvalidWorkbook(Box<crate::validate::SpreadsheetDefect>),

    /// The package root has no `officeDocument` relationship (not an Office document).
    #[error("package has no officeDocument relationship")]
    MissingOfficeDocument,

    /// The workbook part named by the `officeDocument` relationship is absent from the container.
    #[error("workbook part {0} is missing from the package")]
    MissingWorkbookPart(String),

    /// `xl/workbook.xml` (or another part this crate resolves) did not have the expected structure.
    #[error("workbook is malformed: {0}")]
    MalformedWorkbook(&'static str),

    /// A caller named a tab by an index the sheet list does not have.
    ///
    /// A distinct variant rather than an `Option` return, because the two failures a caller wants
    /// told apart — "there is no such tab" and "the tab is there but its relationship reaches no
    /// part" — are different questions, and the second is already reported as `None` by
    /// [`Workbook::worksheet`](crate::Workbook::worksheet).
    #[error("sheet index {index} is out of range: the workbook lists {sheets} sheet(s)")]
    NoSuchSheet {
        /// The index that was asked for.
        index: usize,
        /// How many tabs the sheet list actually names.
        sheets: usize,
    },

    /// A relationship target could not be resolved to a part name.
    #[error("relationship target {target} could not be resolved")]
    TargetResolution {
        /// The unresolvable target.
        target: String,
    },

    /// A relationship target points outside the package, where SpreadsheetML's own parts never live.
    ///
    /// An *external* target is legal OPC and this crate never rejects one it does not have to reach
    /// — only the relationships that must name a part inside the container (the `officeDocument`
    /// relationship, a `x:sheet`'s target, the workbook's styles or shared strings) are refused when
    /// they point outward. `externalLinkPath`, whose whole purpose is to name another workbook, is
    /// deliberately not among them.
    #[error("external relationship target {target} is not supported here")]
    ExternalTarget {
        /// The external target.
        target: String,
    },

    /// A read or an edit of a chart that had already been found and parsed failed — an index past
    /// the end, a series with nothing editable, a part declaring no `c:chart` (MJXOFF-111).
    ///
    /// # Why this wraps rather than restates
    ///
    /// `mjx-chart`'s [`ChartAccessError`](mjx_chart::ChartAccessError) is the single source of these
    /// verdicts for all three host surfaces (MJXOFF-103), and it is deliberately **not**
    /// `#[non_exhaustive]` so that every host must account for a new variant. The two hosts before
    /// this one answer that obligation differently, and the difference is history rather than
    /// design: `mjx-pptx` maps each variant onto a `PptxError` variant it had already written before
    /// the shared body existed, while `mjx-docx` wraps the enum whole.
    ///
    /// This crate takes the **`mjx-docx` shape**, and the reason is on the type above rather than in
    /// a preference. `XlsxError` is exhaustively classified by `mjx_ooxml::error::classify_xlsx`,
    /// which has no wildcard arm and already **descends** into [`SmlError`] — and through it into
    /// `AddressError` — through functions of their own. A chart-level refusal takes the same road:
    /// the facade classifies `ChartAccessError` in `chart_access_code`, which is itself exhaustive
    /// with no wildcard and is the *same* function `DocxError::ChartAccess` goes through. So a
    /// variant added to `ChartAccessError` still fails to compile until somebody decides what it
    /// means, which is the whole property `mjx-pptx` buys by restating eight variants — bought here
    /// without inventing eight variants this crate never wrote first, and with the guarantee that a
    /// chart refusing the same index answers the same `mjx_ooxml::ErrorCode` from a workbook, a
    /// document and a presentation alike.
    #[error(transparent)]
    ChartAccess(#[from] mjx_chart::ChartAccessError),

    /// A [`ChartData`](mjx_chart::ChartData) description cannot be written as a schema-valid chart
    /// part — a stock chart given the wrong number of series, for instance. Refused before anything
    /// is written, so the workbook is untouched (MJXOFF-111).
    ///
    /// The "nothing to draw" case is [`InvalidChartData`](Self::InvalidChartData) instead, the split
    /// `mjx_pptx::PptxError` and `mjx_docx::DocxError` both already make.
    #[error(transparent)]
    ChartData(#[from] mjx_chart::ChartDataError),

    /// A chart description with nothing to draw — no series, or every series empty (MJXOFF-111).
    #[error("a chart description has nothing to draw")]
    InvalidChartData,

    /// A caller addressed a chart by an anchor that frames something else, or by an anchor index
    /// the sheet's drawing does not have (MJXOFF-111).
    ///
    /// The address is `(sheet index, anchor index)` — the anchor's position in the drawing part,
    /// which is also its paint order and is what
    /// [`SheetDrawingObject::index`](crate::SheetDrawingObject) reports and what
    /// [`Workbook::remove_sheet_drawing_object`](crate::Workbook::remove_sheet_drawing_object)
    /// takes. A chart is not given a second addressing scheme of its own.
    #[error("anchor {anchor_index} on sheet {sheet_index} does not frame a chart")]
    AnchorIsNotAChart {
        /// The tab that was asked for.
        sheet_index: usize,
        /// The anchor that was asked for.
        anchor_index: usize,
    },

    /// The chart references no backing workbook (`c:externalData`), so there is nothing to detach
    /// (MJXOFF-111).
    ///
    /// This is the **ordinary** state of a chart on a worksheet, not a defect: such a chart names a
    /// live range in the sheets it lives among and has no embedded copy of its data. Only
    /// [`detach_chart_workbook`](crate::Workbook::detach_chart_workbook) raises it, because
    /// detaching nothing is a caller error;
    /// [`refresh_chart_workbook`](crate::Workbook::refresh_chart_workbook) answers `false` instead.
    #[error("chart has no external data reference")]
    ChartHasNoExternalData,

    /// The bytes handed to an image-adding call match no format this build recognises
    /// (MJXOFF-107).
    ///
    /// [`ImageFormat::sniff`](mjx_opc::ImageFormat::sniff) reads a magic-byte signature and nothing
    /// else, so this says *these leading bytes are not a PNG, JPEG, GIF, BMP, TIFF, EMF, WMF or
    /// SVG* — not that the payload is a corrupt image. Refusing here is what keeps a caller from
    /// registering a content type for a file Excel will not draw; nothing in this library ever
    /// decodes a pixel.
    #[error("the bytes match no image format this build recognises")]
    UnrecognizedImageFormat,

    /// A caller aimed an edit only a worksheet can carry at a tab that is not one (MJXOFF-241).
    ///
    /// **The tab's part is present.** This is not [`MissingWorkbookPart`](Self::MissingWorkbookPart),
    /// which it was reported as until MJXOFF-241 — a message that sent the reader looking for a
    /// broken package when the package is fine. A chartsheet (ECMA-376 Part 1 §12.3.2) is one chart
    /// occupying a whole tab and a dialogsheet (§12.3.7) is a legacy Excel 5.0 dialog; neither has a
    /// cell to address, so a cell value, a merge, a hyperlink, a table, a drawing or a comment has
    /// nowhere to go on one.
    ///
    /// `kind` is what the tab is instead, read from the content type of the part its `r:id` reaches.
    /// [`Some(SheetKind::Worksheet)`](crate::SheetKind::Worksheet) is not a contradiction: it says
    /// the content type claims a worksheet while the part's root element is not `x:worksheet`, which
    /// is a file this library will not guess at either.
    #[error("sheet {index} {}, and only a worksheet can carry this edit", not_a_worksheet_phrase(*.kind))]
    SheetIsNotAWorksheet {
        /// The tab that was asked for.
        index: usize,
        /// What the tab is instead, or `None` when its part is of no sheet content type at all.
        kind: Option<crate::SheetKind>,
    },

    /// A part reached as a legacy VML drawing whose content type says it is not one (MJXOFF-114).
    ///
    /// Refused rather than parsed: a `v:shape` model over a worksheet would answer plausible
    /// nonsense, because [`mjx_vml::Drawing`] does not check the root element's own name — it is the
    /// same type that reads a `w:pict` inside a Word body.
    #[error("{0} is not a legacy VML drawing part")]
    PartIsNotVmlDrawing(String),
}

/// How [`XlsxError::SheetIsNotAWorksheet`] says what a tab is instead.
///
/// A predicate rather than a noun, so that the three shapes read as one English sentence: the two
/// sheet kinds that simply are not worksheets, and the two ways a tab can fail to reach worksheet
/// markup at all. Every phrase is true of the tab it describes — none of them says a part is
/// missing, which is the whole point of MJXOFF-241.
fn not_a_worksheet_phrase(kind: Option<crate::SheetKind>) -> &'static str {
    match kind {
        Some(crate::SheetKind::Chartsheet) => "is a chartsheet",
        Some(crate::SheetKind::Dialogsheet) => "is a dialogsheet",
        Some(crate::SheetKind::Worksheet) => {
            "is typed as a worksheet, but its part's root element is not x:worksheet"
        }
        None => "reaches a part of no sheet content type",
    }
}

impl From<mjx_ooxml_core::AttributeError> for XlsxError {
    /// A malformed attribute value is a modelled element not matching its complex type, which is
    /// what [`Model`](XlsxError::Model) already means — [`FromXmlError`] carries `AttributeError`
    /// for exactly this reason. Declared so that `?` works on the typed accessors `mjx-sml`'s models
    /// expose, rather than every call site writing the same two-step conversion out.
    fn from(error: mjx_ooxml_core::AttributeError) -> Self {
        Self::Model(FromXmlError::from(error))
    }
}

impl From<crate::validate::SpreadsheetDefect> for XlsxError {
    fn from(defect: crate::validate::SpreadsheetDefect) -> Self {
        Self::InvalidWorkbook(Box::new(defect))
    }
}

impl From<mjx_chart::ChartWorkbookError> for XlsxError {
    /// Lifts a failure from writing a chart's data into the workbook it embeds (MJXOFF-208).
    ///
    /// The two halves land where they already belonged: a verdict about the chart is a
    /// [`ChartAccess`](XlsxError::ChartAccess), and the embedded package refusing to be read or
    /// written is an [`Sml`](XlsxError::Sml) like any other malformed part.
    fn from(error: mjx_chart::ChartWorkbookError) -> Self {
        match error {
            mjx_chart::ChartWorkbookError::Access(problem) => Self::ChartAccess(problem),
            mjx_chart::ChartWorkbookError::Sml(problem) => Self::Sml(problem),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    /// Builds an [`XlsxError`] the way one is really built: through `?`, which is `From`.
    fn through_question_mark<E>(error: E) -> XlsxError
    where
        XlsxError: From<E>,
    {
        fn fail<E>(error: E) -> Result<(), XlsxError>
        where
            XlsxError: From<E>,
        {
            Err(error)?
        }
        fail(error).expect_err("the helper always fails")
    }

    /// Every wrapping variant is reachable through `?`, and every `transparent` one displays
    /// **exactly** what it wraps.
    ///
    /// The second half is the assertion that earns its keep. `#[error(transparent)]` is what keeps a
    /// packaging failure readable as the packaging failure it is; replacing it with, say,
    /// `#[error("opc error: {0}")]` would still compile, still `Display` plausibly, and silently
    /// prepend a layer of noise to every error `mjx_ooxml::Error` maps. The expected text comes
    /// from the wrapped error itself, so this cannot pass by agreeing with a copy of the message.
    #[test]
    fn every_wrapping_variant_is_built_by_question_mark_and_displays_what_it_wraps() {
        let opc = OpcError::UnknownPart("/xl/workbook.xml".to_owned());
        let opc_text = opc.to_string();
        let xml = XmlError::Syntax("unclosed <sheetData>".to_owned());
        let xml_text = xml.to_string();
        let model = FromXmlError::InvalidUtf8;
        let model_text = model.to_string();
        let sml = SmlError::Xml(XmlError::Syntax("unclosed <row>".to_owned()));
        let sml_text = sml.to_string();
        let defect = crate::validate::SpreadsheetDefect::WorkbookIsNotTheOfficeDocument {
            workbook_part: "/xl/workbook.xml".to_owned(),
            office_document_target: "xl/other.xml".to_owned(),
        };
        let defect_text = defect.to_string();

        let built = [
            (through_question_mark(opc), opc_text),
            (through_question_mark(xml), xml_text),
            (through_question_mark(model), model_text),
            (through_question_mark(sml), sml_text),
            (through_question_mark(defect), defect_text),
        ];

        assert!(matches!(built[0].0, XlsxError::Opc(_)));
        assert!(matches!(built[1].0, XlsxError::Xml(_)));
        assert!(matches!(built[2].0, XlsxError::Model(_)));
        assert!(matches!(built[3].0, XlsxError::Sml(_)));
        assert!(matches!(built[4].0, XlsxError::InvalidWorkbook(_)));

        for (error, wrapped) in &built {
            assert!(
                !wrapped.is_empty(),
                "{error:?} wraps an error with no message"
            );
            assert_eq!(
                &error.to_string(),
                wrapped,
                "{error:?} is declared `#[error(transparent)]`, so it must display exactly what it \
                 wraps"
            );
        }
    }

    /// The variants this crate raises itself say which part or target is at fault.
    ///
    /// A message that names no part sends the reader back to the file with nothing to grep for, and
    /// these are exactly the failures a caller meets when handing this crate a container it did not
    /// write.
    #[test]
    fn the_locally_raised_variants_name_the_part_or_target_at_fault() {
        let cases = [
            (
                XlsxError::MissingWorkbookPart("/xl/workbook.xml".to_owned()),
                "/xl/workbook.xml",
            ),
            (
                XlsxError::MalformedWorkbook("root element is not x:workbook"),
                "x:workbook",
            ),
            (
                XlsxError::TargetResolution {
                    target: "../../outside.xml".to_owned(),
                },
                "../../outside.xml",
            ),
            (
                XlsxError::ExternalTarget {
                    target: "https://example.invalid/book.xlsx".to_owned(),
                },
                "https://example.invalid/book.xlsx",
            ),
        ];
        for (error, expected) in cases {
            let text = error.to_string();
            assert!(
                text.contains(expected),
                "{error:?} must name {expected} in its message; it said: {text}"
            );
        }
        assert_eq!(
            XlsxError::MissingOfficeDocument.to_string(),
            "package has no officeDocument relationship"
        );
    }
}
