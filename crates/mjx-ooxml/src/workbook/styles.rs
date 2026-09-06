//! Cell formatting: appending to `xl/styles.xml`'s four tables, pointing a cell at one of the
//! records, and reading back what a cell's format resolves to.
//!
//! # Why the four `append_*` calls are here and the other four vocabularies are not
//!
//! [`mjx_sml::FontProperties`], [`mjx_sml::PatternFillSpec`], [`mjx_sml::BorderSpec`] and
//! [`mjx_sml::CellFormatSpec`] are four **flat structs** of `Option` fields, no deeper than a
//! [`mjx_sml::Color`] or a [`mjx_sml::BorderEdgeSpec`]. Re-exporting them is the same courtesy the
//! DrawingML authoring vocabulary already gets from [`crate::Deck`], and it is the difference
//! between a caller who can make a style index and one who can only apply an index it has no way to
//! obtain — which would make [`Workbook::set_cell_style`] useless in a binding.
//!
//! The conditional-formatting, autofilter, data-validation and worksheet-table vocabularies are
//! spec *trees* rather than flat structs, and [the module documentation above](super) says why they
//! are left to [`Workbook::workbook_mut`] with their results readable here.

use mjx_sml::{
    BorderSpec, CellFormatSpec, CellFormatTarget, CellReference, EffectiveCellFormat,
    FontProperties, PatternFillSpec,
};

use crate::error::Error;
use crate::index::index;

use super::Workbook;

impl Workbook {
    /// Appends a font to `xl/styles.xml`'s `fonts` table and answers its index.
    ///
    /// Appends rather than deduplicates: an index this call hands back stays valid for the life of
    /// the workbook, which a table that merged equal entries could not promise.
    ///
    /// # Errors
    /// [`ErrorCode::MalformedDocument`](crate::ErrorCode::MalformedDocument) if the styles part
    /// cannot be read.
    pub fn append_font(&mut self, properties: &FontProperties) -> Result<u32, Error> {
        Ok(self.workbook.append_font(properties)?)
    }

    /// Appends a pattern fill to the `fills` table and answers its index.
    ///
    /// # Errors
    /// As [`append_font`](Self::append_font).
    pub fn append_pattern_fill(&mut self, spec: &PatternFillSpec) -> Result<u32, Error> {
        Ok(self.workbook.append_pattern_fill(spec)?)
    }

    /// Appends a border to the `borders` table and answers its index.
    ///
    /// # Errors
    /// As [`append_font`](Self::append_font).
    pub fn append_border(&mut self, spec: &BorderSpec) -> Result<u32, Error> {
        Ok(self.workbook.append_border(spec)?)
    }

    /// Appends an `x:xf` to `cellXfs` or `cellStyleXfs` and answers its index.
    ///
    /// [`CellFormatTarget::CellFormats`] is the table `c@s`, `row@s` and `col@style` index — the one
    /// [`set_cell_style`](Self::set_cell_style) takes an index into.
    ///
    /// # Errors
    /// As [`append_font`](Self::append_font).
    pub fn append_cell_format(
        &mut self,
        target: CellFormatTarget,
        spec: &CellFormatSpec,
    ) -> Result<u32, Error> {
        Ok(self.workbook.append_cell_format(target, spec)?)
    }

    /// Points one cell at `cellXfs[style]`, or removes its `@s` with `None`.
    ///
    /// The cell must already exist — a style is a property of a cell, and creating one to carry a
    /// style would author a cell the sheet does not have. Write the value first with
    /// [`write_cells`](Self::write_cells).
    ///
    /// # Errors
    /// [`ErrorCode::IndexOutOfRange`](crate::ErrorCode::IndexOutOfRange) if `sheet` names no tab or
    /// `style` names no `x:xf`, [`ErrorCode::NothingToRead`](crate::ErrorCode::NothingToRead) if the
    /// tab holds no worksheet, or
    /// [`ErrorCode::InvalidArgument`](crate::ErrorCode::InvalidArgument) if `reference` is not an A1
    /// cell.
    pub fn set_cell_style(
        &mut self,
        sheet: u32,
        reference: &str,
        style: Option<u32>,
    ) -> Result<(), Error> {
        let reference = CellReference::parse(reference)?;
        Ok(self
            .workbook
            .set_cell_style(index(sheet), reference, style)?)
    }

    /// Interns `text` into `xl/sharedStrings.xml` and answers its index, creating the part if the
    /// workbook has none.
    ///
    /// [`CellInput::SharedText`](super::CellInput::SharedText) does this for a caller, so reach for
    /// this only when the same index is wanted in several places without the text being re-hashed.
    ///
    /// # Errors
    /// [`ErrorCode::MalformedDocument`](crate::ErrorCode::MalformedDocument) if the shared-string
    /// part cannot be read.
    pub fn intern_shared_string(&mut self, text: &str) -> Result<u32, Error> {
        Ok(self.workbook.intern_shared_string(text)?)
    }

    /// What one cell's format resolves to, after the `cellXfs` → `cellStyleXfs` ladder and the
    /// column and row defaults above it.
    ///
    /// `Ok(None)` when the tab holds no worksheet, or the workbook has no styles part.
    ///
    /// **What the file states, not what a renderer shows.** No number format is applied and no
    /// conditional-formatting rule is evaluated — [`conditional_formatting_ranges`
    /// ](Self::conditional_formatting_ranges) reports where the rules are, and
    /// [*Deliberate limitations*](mjx_xlsx::guide::deliberate_limitations) says why neither engine
    /// exists.
    ///
    /// # Errors
    /// [`ErrorCode::IndexOutOfRange`](crate::ErrorCode::IndexOutOfRange) if `sheet` names no tab or
    /// a `c@s` names no `x:xf`, or
    /// [`ErrorCode::InvalidArgument`](crate::ErrorCode::InvalidArgument) if `reference` is not an A1
    /// cell.
    pub fn effective_cell_format(
        &self,
        sheet: u32,
        reference: &str,
    ) -> Result<Option<EffectiveCellFormat>, Error> {
        let reference = CellReference::parse(reference)?;
        Ok(self
            .workbook
            .effective_cell_format(index(sheet), reference)?)
    }

    /// The same ladder, answered for the **anchor** of the merged region `reference` falls in.
    ///
    /// A merged region renders with the top-left cell's format, so a covered cell's own format is
    /// not what a reader sees. `Ok(None)` when the tab holds no worksheet or the workbook has no
    /// styles part; a cell that is not merged answers its own format.
    ///
    /// # Errors
    /// As [`effective_cell_format`](Self::effective_cell_format).
    pub fn effective_merged_cell_format(
        &self,
        sheet: u32,
        reference: &str,
    ) -> Result<Option<EffectiveCellFormat>, Error> {
        let reference = CellReference::parse(reference)?;
        Ok(self
            .workbook
            .effective_merged_cell_format(index(sheet), reference)?)
    }
}
