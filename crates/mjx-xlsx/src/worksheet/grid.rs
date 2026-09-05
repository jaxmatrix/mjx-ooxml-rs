//! The sheet's geometry, at the package tier: opening a worksheet part, editing one cell, and
//! writing it back.
//!
//! `mjx-sml` answers *what a row is*; this file answers *what the worksheet part in this package
//! says about row 7*, and it is the only place a [`WorksheetPart`] meets a [`PartName`].
//!
//! # Why the part is read into an owned model rather than cached as a tree
//!
//! Every other part this crate reads goes through [`mjx_opc::Package::part_tree`], which parses once
//! and caches the tree on the package. A worksheet deliberately does not.
//!
//! `docs/BENCHMARKS.md` measures a 300,000-cell worksheet at **913 bytes of peak resident set per
//! cell** held as a `RawElement` tree, against 36.8 B/cell for MJXOFF-95's packed store. Caching the
//! tree would keep the 913 alive for as long as the workbook is open and hand the 25× straight back,
//! so [`Workbook::worksheet_markup`] reads the part's **bytes**, parses a document that lives for
//! the length of one call, and returns the [`WorksheetPart`] the document is consumed into. The part
//! stays [`Raw`](mjx_opc::PartProvenance) in the package, which is also what keeps a read from
//! dirtying it — the property `crates/mjx-xlsx/tests/roundtrip.rs` pins.
//!
//! # Reading does not dirty; writing is explicit
//!
//! [`worksheet_markup`](Workbook::worksheet_markup) takes `&self` and cannot change the package.
//! [`write_worksheet_markup`](Workbook::write_worksheet_markup) is the one call that does, and it
//! replaces the part's bytes wholesale with what the model emits — which, for a model nothing has
//! edited, is the bytes it was read from.

use mjx_opc::PartName;
use mjx_sml::{CellReference, CellValue, SharedStringTable, WorksheetPart};

use crate::error::XlsxError;
use crate::workbook::Workbook;

impl Workbook {
    /// Reads the worksheet part behind the tab at `index` into an owned model.
    ///
    /// `Ok(None)` when that tab reaches no part at all, or when the part it reaches is not an
    /// `x:worksheet` — a chartsheet or a dialogsheet, whose bodies are MJXOFF-129's (D17). Such a
    /// workbook **opens** and reports its sheet kind through [`Sheet::kind`](crate::Sheet::kind);
    /// it is only its cells that are not there to be read.
    ///
    /// This is not a mutation: the part keeps its container bytes, [`save`](Workbook::save) still
    /// re-emits them verbatim, and the package is never asked for a tree.
    ///
    /// # Errors
    /// [`XlsxError::NoSuchSheet`] if `index` names no tab; [`XlsxError`] if the part is not
    /// well-formed XML or its markup does not match `CT_Worksheet`.
    pub fn worksheet_markup(&self, index: usize) -> Result<Option<WorksheetPart>, XlsxError> {
        let sheet = self.sheets().get(index).ok_or(XlsxError::NoSuchSheet {
            index,
            sheets: self.sheets().len(),
        })?;
        let Some(part) = sheet.part.clone() else {
            return Ok(None);
        };
        self.worksheet_markup_of(&part)
    }

    /// [`worksheet_markup`](Self::worksheet_markup) for a caller that already holds the part name —
    /// from [`Worksheet::part`](crate::Worksheet::part), above all.
    ///
    /// # Errors
    /// As [`worksheet_markup`](Self::worksheet_markup), plus [`XlsxError::Opc`] if the package holds
    /// no such part.
    pub fn worksheet_markup_of(&self, part: &PartName) -> Result<Option<WorksheetPart>, XlsxError> {
        let Some(bytes) = self.package().part_bytes(part) else {
            return Err(XlsxError::MissingWorkbookPart(part.as_str().to_owned()));
        };
        Ok(WorksheetPart::read_part(bytes)?)
    }

    /// Writes `markup` back over the worksheet part behind the tab at `index`.
    ///
    /// The part's bytes are replaced with what the model emits, which for a model nothing edited is
    /// the buffer it was read from — so writing back an untouched worksheet is a no-op the
    /// byte-identity suites cannot tell from not writing at all.
    ///
    /// # Errors
    /// [`XlsxError::NoSuchSheet`] if `index` names no tab, [`XlsxError::MissingWorkbookPart`] if it
    /// reaches no part, or [`XlsxError::Opc`] if the package refuses the replacement.
    pub fn write_worksheet_markup(
        &mut self,
        index: usize,
        markup: &WorksheetPart,
    ) -> Result<(), XlsxError> {
        let sheets = self.sheets().len();
        let part = self
            .sheets()
            .get(index)
            .ok_or(XlsxError::NoSuchSheet { index, sheets })?
            .part
            .clone()
            .ok_or_else(|| XlsxError::MissingWorkbookPart(format!("sheet {index}")))?;
        self.package_mut()
            .replace_part_bytes(&part, markup.to_markup())?;
        Ok(())
    }

    /// Sets one cell's value on the tab at `index`, leaving every other row and every other
    /// worksheet child byte-identical.
    ///
    /// Read, edit, write — the three calls above in one, because setting a cell is the operation
    /// this surface exists for. The isolation is not this method's doing: it is
    /// [`WorksheetPart`]'s slot-level copy-on-write and the cell store's row-level copy-on-write,
    /// and `crates/mjx-xlsx/tests/worksheet_part.rs` asserts it against the file's own bytes rather
    /// than against a second run of the writer.
    ///
    /// # Errors
    /// [`XlsxError::NoSuchSheet`] if `index` names no tab, [`XlsxError::MissingWorkbookPart`] if the
    /// tab reaches no worksheet part, or [`XlsxError::Sml`] if the store refuses the value.
    pub fn set_cell_value(
        &mut self,
        index: usize,
        reference: CellReference,
        value: CellValue<'_>,
    ) -> Result<(), XlsxError> {
        let mut markup = self
            .worksheet_markup(index)?
            .ok_or_else(|| XlsxError::MissingWorkbookPart(format!("sheet {index}")))?;
        markup.set_cell_value(reference, value)?;
        self.write_worksheet_markup(index, &markup)
    }

    /// The text of one cell on the tab at `index`, with a `t="s"` index resolved through
    /// `xl/sharedStrings.xml`.
    ///
    /// **The one thing this tier adds to the markup model.** A shared string is an index into
    /// another part, so `mjx-sml` — which has never heard of a package — can report the index and
    /// nothing more. Every other cell type answers from the cell itself: an `inlineStr` from its own
    /// `<is>`, a number, a boolean or an error from its `<v>`.
    ///
    /// `Ok(None)` for a cell that is not populated, for a tab whose part is not a worksheet, and for
    /// a `t="s"` whose index names no entry in the string table — which is a defect in the file,
    /// reported as absence rather than repaired.
    ///
    /// # Errors
    /// As [`worksheet_markup`](Self::worksheet_markup), plus [`XlsxError::Sml`] if the shared-string
    /// part is unreadable or a value will not decode.
    pub fn cell_text(
        &self,
        index: usize,
        reference: CellReference,
    ) -> Result<Option<String>, XlsxError> {
        let Some(markup) = self.worksheet_markup(index)? else {
            return Ok(None);
        };
        let Some(cell) = markup.cell(reference) else {
            return Ok(None);
        };
        if let Some(shared) = cell.shared_string_index() {
            let Some(table) = self.shared_strings()? else {
                return Ok(None);
            };
            return Ok(match table.item(shared) {
                Some(item) => Some(item.text().map_err(mjx_sml::SmlError::from)?.into_owned()),
                None => None,
            });
        }
        if let Some(inline) = cell.inline_string_markup() {
            let string = mjx_sml::InlineString::parse(inline)?;
            let text = string.item().text().map_err(mjx_sml::SmlError::from)?;
            return Ok(Some(text.into_owned()));
        }
        Ok(cell
            .value()
            .map_err(mjx_sml::SmlError::from)?
            .map(std::borrow::Cow::into_owned))
    }

    /// The workbook's shared-string table, read fresh, or `None` when the workbook has no
    /// `xl/sharedStrings.xml`.
    ///
    /// # Errors
    /// [`XlsxError`] if the part is not well-formed or its markup does not match `CT_Sst`.
    pub fn shared_strings(&self) -> Result<Option<SharedStringTable>, XlsxError> {
        let Some(part) = self.parts().shared_strings.clone() else {
            return Ok(None);
        };
        let Some(bytes) = self.package().part_bytes(&part) else {
            return Ok(None);
        };
        let document = mjx_xml::fidelity::parse(bytes).map_err(mjx_sml::SmlError::from)?;
        Ok(SharedStringTable::read_part(&document)?)
    }
}
