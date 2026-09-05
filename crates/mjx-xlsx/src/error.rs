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
/// **adding a variant fails to compile until it is classified there**. MJXOFF-137 (D20) writes that
/// mapping for Excel; until it does, the property is worth keeping rather than losing to a
/// `#[non_exhaustive]` that would have to come off again.
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
}

impl From<crate::validate::SpreadsheetDefect> for XlsxError {
    fn from(defect: crate::validate::SpreadsheetDefect) -> Self {
        Self::InvalidWorkbook(Box::new(defect))
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
    /// prepend a layer of noise to every error MJXOFF-137 will one day map. The expected text comes
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
