//! The error type for the SpreadsheetML layer.

use mjx_ooxml_core::FromXmlError;
use mjx_opc::OpcError;
use mjx_xml::XmlError;

/// Errors produced while reading, editing or writing SpreadsheetML markup.
///
/// # Deliberately exhaustive
///
/// This enum is **not** `#[non_exhaustive]`, for the reason
/// [`mjx_docx::DocxError`](https://docs.rs/mjx-docx) and
/// [`mjx_pptx::PptxError`](https://docs.rs/mjx-pptx) both state on themselves: the facade collapses
/// every variant into one of its stable error codes through a `match` with no wildcard arm, so
/// **adding a variant here fails to compile until it is classified there**. MJXOFF-137 (D20) writes
/// that mapping; until it does, the property is worth keeping rather than losing to a
/// `#[non_exhaustive]` that would have to be taken off again.
///
/// # Untrusted input
///
/// Every value of this type comes from a file somebody else wrote. No parse path in this crate may
/// `unwrap`, `expect` or `panic` — a malformed workbook is a returned error, never an abort.
///
/// # Status
///
/// MJXOFF-132 (D01) creates the crate and this enum with the three failures every SpreadsheetML
/// path can already produce: a package that will not open, a part that is not well-formed XML, and a
/// modelled element that does not match its schema type. MJXOFF-93 (D03) adds the fourth: an address
/// — a cell reference, a range, a `sqref` or a `spans` list — that does not parse. Each later Phase D
/// child adds the variants its own model needs — a shared-string index that names no entry
/// (MJXOFF-97), an `xf` index outside `cellXfs` (MJXOFF-108). MJXOFF-95 (D04) adds the fifth: a
/// worksheet whose bytes outgrow the cell store's `u32` address space.
#[derive(Debug, thiserror::Error)]
pub enum SmlError {
    /// The underlying OPC package could not be read, edited or written.
    #[error(transparent)]
    Opc(#[from] OpcError),

    /// A part was not well-formed XML.
    #[error(transparent)]
    Xml(#[from] XmlError),

    /// A modelled element did not match the shape its complex type declares.
    #[error(transparent)]
    Model(#[from] FromXmlError),

    /// A cell reference, range, `sqref` or `spans` value did not parse.
    #[error(transparent)]
    Address(#[from] crate::address::AddressError),

    /// A caller asked the cell store to write a number SpreadsheetML cannot express.
    ///
    /// `NaN` and the infinities have no representation in a `<v>`: Excel writes an **error cell**
    /// for them (`t="e"` with `#NUM!` or `#DIV/0!`), not a numeric one. Rust's own spellings —
    /// `NaN`, `inf`, `-inf` — are not `xsd:double` either (which wants `NaN`, `INF`, `-INF`), and
    /// `INF` does not parse back through `str::parse::<f64>`. So rather than write a number nothing
    /// can read, the store refuses and says what to write instead.
    #[error(
        "{value} cannot be written as a cell value; SpreadsheetML has no numeric spelling for it, \
         and Excel writes an error cell (`CellValue::Error(\"#NUM!\")`) in its place"
    )]
    UnrepresentableNumber {
        /// The value that was asked for.
        value: f64,
    },

    /// A worksheet's `sheetData`, or the edits made to it, outgrew the byte space the cell store
    /// addresses values in.
    ///
    /// The store keeps every preserved value as a `(start, length)` pair of `u32`s over one address
    /// space — see `crate::cells` for why — so the part's bytes plus whatever has been edited must
    /// stay under four gigabytes. No producer writes a worksheet part anywhere near that, and
    /// `mjx-xml`'s reader already stops recording byte ranges past the same limit; this is here
    /// because untrusted input does not get to decide whether an index is in range.
    #[error("the cell store's byte space cannot address {bytes} bytes (the limit is 4 GiB)")]
    SheetDataTooLarge {
        /// How many bytes were asked for.
        bytes: u64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds an [`SmlError`] the only way one will ever be built: through `?`, which is `From`.
    fn through_question_mark<E>(error: E) -> SmlError
    where
        SmlError: From<E>,
    {
        fn fail<E>(error: E) -> Result<(), SmlError>
        where
            SmlError: From<E>,
        {
            Err(error)?
        }
        fail(error).expect_err("the helper always fails")
    }

    /// Every variant is reachable through `?`, and every variant displays **exactly** what it wraps.
    ///
    /// The second half is the one worth asserting. `#[error(transparent)]` is what keeps a
    /// SpreadsheetML failure readable as the packaging or XML failure it really is; replacing it
    /// with, say, `#[error("opc error: {0}")]` would still compile, still `Display` plausibly, and
    /// silently prepend a layer of noise to every error the facade will one day map. The expected
    /// text is taken from the wrapped error itself rather than written out here, so the case cannot
    /// pass by agreeing with a copy of the message.
    #[test]
    fn every_variant_is_built_by_question_mark_and_displays_what_it_wraps() {
        let opc = OpcError::UnknownPart("/xl/workbook.xml".to_owned());
        let opc_text = opc.to_string();
        let xml = XmlError::Syntax("unclosed <sheetData>".to_owned());
        let xml_text = xml.to_string();
        let model = FromXmlError::InvalidUtf8;
        let model_text = model.to_string();
        let address = crate::address::AddressError::ColumnOutOfGrid;
        let address_text = address.to_string();

        let built = [
            (through_question_mark(opc), opc_text),
            (through_question_mark(xml), xml_text),
            (through_question_mark(model), model_text),
            (through_question_mark(address), address_text),
        ];

        assert!(matches!(built[0].0, SmlError::Opc(_)));
        assert!(matches!(built[1].0, SmlError::Xml(_)));
        assert!(matches!(built[2].0, SmlError::Model(_)));
        assert!(matches!(built[3].0, SmlError::Address(_)));

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
}
