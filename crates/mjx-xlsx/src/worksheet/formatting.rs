//! The effective-format surface at the package tier: `xl/styles.xml` plus one worksheet, read once,
//! and every cell's format answered from them.
//!
//! # This file reimplements nothing
//!
//! The resolution order — cell → row → column → the default record, and then, per aspect, the direct
//! `cellXfs` record or the `cellStyleXfs` record beneath it according to `applyX` — lives in
//! [`mjx_sml::styles::effective`] and lives there only. `mjx-sml` owns `sml.xsd` content; this crate
//! owns OPC structure, and what it adds here is exactly the OPC part: *which part in this package is
//! the styles part*, and *which part holds row 7*.
//!
//! MJXOFF-108 is where that division would have been easiest to break. A second walk written here
//! would be a second answer to the same question, free to drift, and invisible to
//! `crates/mjx-sml/tests/effective_cell_format.rs` — which is the suite that actually pins the
//! behaviour.
//!
//! # Read once, resolve many
//!
//! [`SheetFormatting`] owns both parsed parts, and [`SheetFormatting::resolver`] decodes every `xf`
//! and every `col` run **once**. A caller resolving a whole sheet builds one resolver and calls
//! [`SheetFormatResolver::effective_cell_format`] per cell; a caller wanting a single answer has
//! [`crate::Workbook::effective_cell_format`], which builds and discards one.
//!
//! Reading does not dirty the package. [`crate::Workbook::sheet_formatting`] takes `&self`, asks the
//! package for part *bytes* rather than for a tree — the same choice
//! [`grid`](super::grid) makes, and for the same 913-versus-36.8 bytes-per-cell reason — and neither
//! part is ever written back.

use mjx_ooxml_core::RawDocument;
use mjx_opc::PartName;
use mjx_sml::styles::effective::{CellFormatResolver, ColumnStyles};
use mjx_sml::{CellReference, EffectiveCellFormat, SheetData, StylesheetPart, WorksheetPart};

use crate::error::XlsxError;
use crate::workbook::Workbook;

/// One worksheet and the styles part it resolves against, both parsed, ready to answer.
///
/// Owns the styles part's [`RawDocument`] because the model's names are symbols in that document's
/// interner, and the worksheet's [`WorksheetPart`] because that carries its own. Nothing here
/// borrows the package, so a caller may keep one while the workbook is edited elsewhere — and
/// nothing here can write, so keeping one cannot make the package stale in the other direction.
#[derive(Debug)]
pub struct SheetFormatting {
    styles_document: RawDocument,
    stylesheet: StylesheetPart,
    worksheet: WorksheetPart,
}

impl SheetFormatting {
    /// The styles part, modelled.
    #[must_use]
    pub fn stylesheet(&self) -> &StylesheetPart {
        &self.stylesheet
    }

    /// The worksheet part, modelled — its `cols` blocks and its [`SheetData`] among them.
    #[must_use]
    pub fn worksheet(&self) -> &WorksheetPart {
        &self.worksheet
    }

    /// Decodes both `xf` tables and every `col` run, once.
    ///
    /// # Errors
    /// [`XlsxError::Sml`] if an `xf` or a `col` carries a value its declared type rejects.
    pub fn resolver(&self) -> Result<SheetFormatResolver<'_>, XlsxError> {
        let formats = CellFormatResolver::new(&self.stylesheet, &self.styles_document.interner)?;
        let columns =
            ColumnStyles::read(self.worksheet.column_blocks(), self.worksheet.interner())?;
        Ok(SheetFormatResolver {
            formats,
            columns,
            cells: self.worksheet.sheet_data(),
        })
    }
}

/// [`SheetFormatting`] with both tables decoded — the per-cell surface.
#[derive(Debug)]
pub struct SheetFormatResolver<'a> {
    formats: CellFormatResolver<'a>,
    columns: ColumnStyles,
    cells: Option<&'a SheetData>,
}

impl<'a> SheetFormatResolver<'a> {
    /// The markup-tier resolver, for the accessors that turn an [`EffectiveCellFormat`]'s indices
    /// into fonts, fills, borders and format codes.
    #[must_use]
    pub fn formats(&self) -> &CellFormatResolver<'a> {
        &self.formats
    }

    /// The `col@style` runs this sheet wrote.
    #[must_use]
    pub fn columns(&self) -> &ColumnStyles {
        &self.columns
    }

    /// The effective format of the cell at `reference`, walking cell → row → column → the default
    /// record.
    ///
    /// A position the sheet writes no `<c>` for still has a format, so this answers for **every**
    /// reference in the grid and not only for the ones the file mentions.
    ///
    /// # Errors
    /// [`XlsxError::Sml`] if the style index in force names no record in `cellXfs`, which is a
    /// corrupt workbook rather than an unformatted cell.
    pub fn effective_cell_format(
        &self,
        reference: CellReference,
    ) -> Result<EffectiveCellFormat, XlsxError> {
        let cell = self.cells.and_then(|cells| cells.cell(reference));
        let row = self
            .cells
            .and_then(|cells| cells.row(reference.row().saturating_add(1)));
        let column = u32::from(reference.column()).saturating_add(1);
        Ok(self.formats.effective_cell_format(
            cell.as_ref(),
            row.as_ref(),
            self.columns.style_index(column),
        )?)
    }
}

impl Workbook {
    /// Reads `xl/styles.xml` into an owned model, or `None` when the workbook relates to no styles
    /// part at all.
    ///
    /// Not a mutation: the part keeps its container bytes and [`save`](Workbook::save) still
    /// re-emits them verbatim.
    ///
    /// # Errors
    /// [`XlsxError::MissingWorkbookPart`] if the relationship names a part the package does not
    /// hold; [`XlsxError`] if the part is not well-formed XML or does not match `CT_Stylesheet`.
    pub fn styles_markup(&self) -> Result<Option<StylesheetPart>, XlsxError> {
        let Some(part) = self.parts().styles.clone() else {
            return Ok(None);
        };
        Ok(self.styles_document(&part)?.1)
    }

    /// Reads the styles part and the worksheet behind the tab at `index`, ready to resolve.
    ///
    /// `Ok(None)` when the tab reaches no part, when that part is not an `x:worksheet`, or when the
    /// workbook relates to no styles part — in each case there is nothing to resolve *against*,
    /// which is a question rather than an error.
    ///
    /// # Errors
    /// [`XlsxError::NoSuchSheet`] if `index` names no tab; [`XlsxError`] if either part is not
    /// well-formed XML or does not match its complex type.
    pub fn sheet_formatting(&self, index: usize) -> Result<Option<SheetFormatting>, XlsxError> {
        let Some(worksheet) = self.worksheet_markup(index)? else {
            return Ok(None);
        };
        let Some(part) = self.parts().styles.clone() else {
            return Ok(None);
        };
        let (styles_document, stylesheet) = self.styles_document(&part)?;
        let Some(stylesheet) = stylesheet else {
            return Ok(None);
        };
        Ok(Some(SheetFormatting {
            styles_document,
            stylesheet,
            worksheet,
        }))
    }

    /// The effective format of one cell of the tab at `index`.
    ///
    /// **Reads and decodes both parts on every call.** For more than a handful of cells, hold a
    /// [`SheetFormatting`] from [`sheet_formatting`](Self::sheet_formatting) and one
    /// [`SheetFormatResolver`] from it instead: that is what makes resolution per cell free of
    /// parsing.
    ///
    /// `Ok(None)` when [`sheet_formatting`](Self::sheet_formatting) answers `None`.
    ///
    /// # Errors
    /// As [`sheet_formatting`](Self::sheet_formatting), plus [`XlsxError::Sml`] if the style index
    /// in force names no record in `cellXfs`.
    pub fn effective_cell_format(
        &self,
        index: usize,
        reference: CellReference,
    ) -> Result<Option<EffectiveCellFormat>, XlsxError> {
        let Some(formatting) = self.sheet_formatting(index)? else {
            return Ok(None);
        };
        let resolver = formatting.resolver()?;
        resolver.effective_cell_format(reference).map(Some)
    }

    /// Parses the styles part at `part`, returning the document beside the model so that a caller
    /// can keep the interner the model's names live in.
    fn styles_document(
        &self,
        part: &PartName,
    ) -> Result<(RawDocument, Option<StylesheetPart>), XlsxError> {
        let Some(bytes) = self.package().part_bytes(part) else {
            return Err(XlsxError::MissingWorkbookPart(part.as_str().to_owned()));
        };
        let document = mjx_xml::fidelity::parse(bytes)?;
        let stylesheet = StylesheetPart::read_part(&document)?;
        Ok((document, stylesheet))
    }
}
