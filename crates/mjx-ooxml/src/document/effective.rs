//! Effective properties — the character, paragraph and table-cell formatting a run/paragraph/cell
//! *renders* with, resolved across the full ladder (`w:docDefaults` → numbering → style chain →
//! direct), mirroring [`crate::deck::effective`]'s own role for PresentationML.
//!
//! Every reader here returns one of `mjx_docx`'s own `Effective*` structs unchanged — they are
//! already flat, owned, `Copy`-free-but-`Clone` value types with no `Interner` dependency (every
//! colour is already baked to a concrete `RRGGBB`, every theme font already resolved), exactly the
//! shape [`crate::deck::effective`]'s `Resolved*` returns have. Only the *address* crosses the u32/
//! `impl Into` translation every other facade method does.

use crate::error::Error;

use super::{BlockPath, RunPath};

impl super::Document {
    /// The **effective** character formatting of the run at `run` within the paragraph at
    /// `paragraph` — every `EG_RPrBase` member resolved across the full ladder. See
    /// [the guide](mjx_docx::effective_properties) for the ladder order.
    ///
    /// # Errors
    /// [`ErrorCode::NothingToRead`](crate::ErrorCode::NothingToRead) if the document declares no
    /// body, [`ErrorCode::IndexOutOfRange`](crate::ErrorCode::IndexOutOfRange) if either address does
    /// not resolve, or [`ErrorCode::MalformedDocument`](crate::ErrorCode::MalformedDocument) if a
    /// related part cannot be read or a style chain does not terminate.
    pub fn effective_run_properties(
        &mut self,
        paragraph: impl Into<BlockPath>,
        run: impl Into<RunPath>,
    ) -> Result<mjx_docx::EffectiveCharacterProperties, Error> {
        Ok(self
            .document
            .effective_run_properties(paragraph.into().to_model(), run.into().to_model())?)
    }

    /// The **effective** paragraph formatting of the paragraph at `paragraph` — every `CT_PPrBase`
    /// member resolved across the ladder (no character-style tier: `w:rStyle` never affects a
    /// paragraph's own layout).
    ///
    /// # Errors
    /// As [`effective_run_properties`](Self::effective_run_properties).
    pub fn effective_paragraph_properties(
        &mut self,
        paragraph: impl Into<BlockPath>,
    ) -> Result<mjx_docx::EffectiveParagraphProperties, Error> {
        Ok(self
            .document
            .effective_paragraph_properties(paragraph.into().to_model())?)
    }

    /// The **effective** fill of the cell at `(row, column)` of table `table` — the table style's
    /// conditional-formatting region for that cell, resolved against `w:tblLook`, folded under the
    /// cell's/row's own direct `w:tcPr`/`w:trPr`.
    ///
    /// # Errors
    /// [`ErrorCode::IndexOutOfRange`](crate::ErrorCode::IndexOutOfRange) if `table` or `(row,
    /// column)` is out of range, or [`ErrorCode::MalformedDocument`](crate::ErrorCode::MalformedDocument)
    /// if a related part cannot be read.
    pub fn effective_cell_fill(
        &mut self,
        table: u32,
        row: u32,
        column: u32,
    ) -> Result<Option<mjx_docx::EffectiveShading>, Error> {
        Ok(self.document.effective_cell_fill(
            crate::index::index(table),
            crate::index::index(row),
            crate::index::index(column),
        )?)
    }

    /// The **effective** border on `edge` of the cell at `(row, column)` of table `table` — as
    /// [`effective_cell_fill`](Self::effective_cell_fill), for `w:tcBorders`/`w:tblBorders`.
    ///
    /// # Errors
    /// As [`effective_cell_fill`](Self::effective_cell_fill).
    pub fn effective_cell_border(
        &mut self,
        table: u32,
        row: u32,
        column: u32,
        edge: mjx_docx::CellBorderEdge,
    ) -> Result<Option<mjx_docx::EffectiveBorder>, Error> {
        Ok(self.document.effective_cell_border(
            crate::index::index(table),
            crate::index::index(row),
            crate::index::index(column),
            edge,
        )?)
    }

    /// The **effective** character formatting of the run at `run` within the paragraph at
    /// `paragraph`, both addressed inside the cell at `(row, column)` of table `table` — folded
    /// under [`effective_run_properties`](Self::effective_run_properties)'s own ladder plus the
    /// table style's own tier.
    ///
    /// # Errors
    /// As [`effective_cell_fill`](Self::effective_cell_fill), plus
    /// [`ErrorCode::IndexOutOfRange`](crate::ErrorCode::IndexOutOfRange) if `(paragraph, run)` does
    /// not resolve within the addressed cell.
    pub fn effective_cell_run_properties(
        &mut self,
        table: u32,
        row: u32,
        column: u32,
        paragraph: u32,
        run: u32,
    ) -> Result<mjx_docx::EffectiveCharacterProperties, Error> {
        Ok(self.document.effective_cell_run_properties(
            crate::index::index(table),
            crate::index::index(row),
            crate::index::index(column),
            crate::index::index(paragraph),
            crate::index::index(run),
        )?)
    }
}
