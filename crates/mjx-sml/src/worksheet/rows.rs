//! Row geometry as a mutation surface: height, hiding, outlining, and the two outline maxima.
//!
//! A row is not a slot of `CT_Worksheet`. It lives inside `sheetData`, which is MJXOFF-95's packed
//! store, so this file adds no model: [`Row`](crate::Row) already reads all eleven of `CT_Row`'s
//! non-key attributes and [`SheetData::set_row_attribute`](crate::SheetData::set_row_attribute)
//! already writes any one of them in place. What this file adds is the *shape* of the calls a caller
//! makes — and one type that exists to stop a caller making a call that does nothing.
//!
//! # `ht` and `customHeight` travel together, in the type
//!
//! ECMA-376 Part 1 §18.3.1.73 describes `customHeight` as *"1 if the row height has been manually
//! set"*, and §18.3.1.81 says the same of `sheetFormatPr@customHeight`. It is a claim about **where
//! the number came from**, and Excel acts on it: a height a consumer computed is a height that
//! consumer will compute again — autofit reflows it the next time the row's content changes — while
//! a height a person set is one Excel leaves alone.
//!
//! So `ht="30"` on its own is not "the row is 30 points tall". It is "*something* worked out 30, and
//! feel free to work it out again", and a caller who wanted a 30-point row and wrote only `ht` has
//! been failed by the API rather than by the file.
//!
//! [`RowHeight`] is what stops that. There is no call in this workspace that takes a bare height:
//! the number arrives inside a variant that says which claim is being made. The same shape, for the
//! same reason, is [`ColumnWidth`](super::ColumnWidth).
//!
//! **What the specification does *not* say.** It does not say `ht` without `customHeight` is ignored
//! outright — it makes both attributes descriptive. The behaviour above is Excel's, and the API
//! design here rests on the *specification's* reading, which is the stronger one: the two attributes
//! state one fact between them, so a caller has to state both halves of it.
//!
//! # The outline maxima are the sheet's, not the row's
//!
//! `sheetFormatPr@outlineLevelRow` and `@outlineLevelCol` record how deep the sheet's outlining
//! goes, and Part 1 says of both that *"these values shall be in synch with the actual sheet outline
//! levels"*. That `shall` cuts one way only here:
//!
//! * **Setting a level raises a maximum that is too shallow**, because writing a level deeper than
//!   the declared maximum would be *authoring* the disagreement this library reports in other
//!   people's files.
//! * **Nothing lowers a maximum, and nothing repairs one on read.** A sheet that declares
//!   `outlineLevelRow="3"` with no row deeper than 1 is a file Excel wrote and Excel opens;
//!   correcting it would rewrite a part nobody asked to edit.
//!   [`WorksheetPart::grid_anomalies`](super::GridAnomaly) reports it, and
//!   [`WorksheetPart::recompute_outline_levels`] rewrites it — for a caller who asks, and never
//!   otherwise. That is exactly the split
//!   [`recompute_dimension`](crate::WorksheetPart::recompute_dimension) already draws.
//! * **A maximum is never authored onto a sheet that has no `sheetFormatPr`.**
//!   `@defaultRowHeight` is `use="required"`, so creating that element to record a maximum would
//!   mean inventing a default row height the file never stated.

use crate::error::SmlError;

use super::frame::WorksheetPart;

/// A row height and the claim the file makes about where it came from.
///
/// See this module's own documentation for why the height and its `customHeight` flag are one value
/// rather than two arguments. The unit is **points**, as `CT_Row`'s `@ht` is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RowHeight {
    /// `ht="…" customHeight="1"` — a height a person set. Excel keeps it.
    Custom(f64),
    /// `ht="…"` with no `customHeight` — a height a consumer computed to fit the row's content,
    /// which it may compute again. This is what Excel writes for an auto-fitted row, so it has to be
    /// expressible; it is not what a caller who wants a particular height should choose.
    Fitted(f64),
}

impl RowHeight {
    /// The height itself, in points.
    #[must_use]
    pub fn points(self) -> f64 {
        match self {
            Self::Custom(height) | Self::Fitted(height) => height,
        }
    }

    /// Whether this height is written with `customHeight="1"`.
    #[must_use]
    pub fn is_custom(self) -> bool {
        matches!(self, Self::Custom(_))
    }
}

impl WorksheetPart {
    /// Sets the height of the row a file numbered `row`, or removes it with `None`.
    ///
    /// `row` is the `row@r` the file wrote — the same key
    /// [`SheetData::row`](crate::SheetData::row) is indexed by, one-based as `@r` is, and *not* a
    /// position among the populated rows.
    ///
    /// The row must already exist: a height is a property of a row, and creating an empty `<row>`
    /// to carry one would author markup for a row the sheet does not have. `Ok(())` either way —
    /// [`SheetData::row`](crate::SheetData::row) is how a caller asks whether it is there.
    ///
    /// # Errors
    /// [`SmlError::PackedStoreTooLarge`] if the store's byte space cannot hold the rewritten row.
    pub fn set_row_height(&mut self, row: u32, height: Option<RowHeight>) -> Result<(), SmlError> {
        let Some(store) = self.sheet_data_mut() else {
            return Ok(());
        };
        store.set_row_height(row, height)
    }

    /// Hides or shows the row a file numbered `row`.
    ///
    /// `false` **removes** `@hidden` rather than writing `hidden="0"`: the schema's default is
    /// `false`, so the two say the same thing and the shorter one is what a producer writes.
    ///
    /// # Errors
    /// As [`set_row_height`](Self::set_row_height).
    pub fn set_row_hidden(&mut self, row: u32, hidden: bool) -> Result<(), SmlError> {
        let Some(store) = self.sheet_data_mut() else {
            return Ok(());
        };
        store.set_row_hidden(row, hidden.then_some(true))
    }

    /// Sets `row@collapsed` on the row a file numbered `row`.
    ///
    /// `collapsed` says the outline group *below* this row is collapsed — it is the summary row's
    /// flag, not the hidden rows'. Hiding the rows themselves is `@hidden` on each of them, which is
    /// why Excel writes both and why this does not write one from the other.
    ///
    /// # Errors
    /// As [`set_row_height`](Self::set_row_height).
    pub fn set_row_collapsed(&mut self, row: u32, collapsed: bool) -> Result<(), SmlError> {
        let Some(store) = self.sheet_data_mut() else {
            return Ok(());
        };
        store.set_row_collapsed(row, collapsed.then_some(true))
    }

    /// Sets the outline level of the row a file numbered `row`, and raises
    /// `sheetFormatPr@outlineLevelRow` when the new level is deeper than the one the sheet declares.
    ///
    /// Level `0` removes the attribute, which is the schema's default.
    ///
    /// See this module's own documentation for why the maximum is raised and never lowered.
    ///
    /// # Errors
    /// As [`set_row_height`](Self::set_row_height).
    pub fn set_row_outline_level(&mut self, row: u32, level: u8) -> Result<(), SmlError> {
        let Some(store) = self.sheet_data_mut() else {
            return Ok(());
        };
        store.set_row_outline_level(row, (level != 0).then_some(level))?;
        self.raise_row_outline_maximum(level);
        Ok(())
    }

    /// The deepest outline level any populated row states, and the deepest any `col` run states.
    ///
    /// The two numbers `sheetFormatPr@outlineLevelRow` and `@outlineLevelCol` are *supposed* to
    /// carry. Reading them off the rows and runs rather than off `sheetFormatPr` is what makes
    /// [`grid_anomalies`](super::GridAnomaly) able to notice they disagree.
    ///
    /// # Errors
    /// [`SmlError::Model`] if a `col` is missing `@min` or `@max` — the same bounds
    /// [`column_run_covering`](Self::column_run_covering) needs, and the same reason.
    pub fn outline_levels_in_use(&self) -> Result<(u8, u8), SmlError> {
        let rows = self
            .rows()
            .map(|row| row.outline_level())
            .max()
            .unwrap_or(0);
        let mut columns = 0;
        for block in self.column_blocks() {
            for run in block.runs() {
                columns = columns.max(
                    run.outline_level(self.interner())
                        .map_err(mjx_ooxml_core::FromXmlError::from)?,
                );
            }
        }
        Ok((rows, columns))
    }

    /// Rewrites `sheetFormatPr@outlineLevelRow` and `@outlineLevelCol` from the levels the rows and
    /// column runs actually state.
    ///
    /// **The caller's ask, never implicit** — the same contract as
    /// [`recompute_dimension`](Self::recompute_dimension). Returns the pair written, or `None` when
    /// the sheet has no `sheetFormatPr` to write into (it is never authored; see this module's own
    /// documentation).
    ///
    /// Unlike the raise that happens on a set, this can *lower* a maximum: a caller asking for the
    /// two numbers to be recomputed is asking for exactly that.
    ///
    /// # Errors
    /// As [`outline_levels_in_use`](Self::outline_levels_in_use).
    pub fn recompute_outline_levels(&mut self) -> Result<Option<(u8, u8)>, SmlError> {
        let (rows, columns) = self.outline_levels_in_use()?;
        if self.format_properties().is_none() {
            return Ok(None);
        }
        self.with_interner(|part, interner| {
            if let Some(format) = part.format_properties_mut() {
                format.set_deepest_row_outline_level(interner, Some(rows));
                format.set_deepest_column_outline_level(interner, Some(columns));
            }
        });
        Ok(Some((rows, columns)))
    }

    /// Raises `sheetFormatPr@outlineLevelRow` to `level` if the sheet declares a shallower one.
    ///
    /// Never lowers it, and never authors a `sheetFormatPr` that is not there.
    fn raise_row_outline_maximum(&mut self, level: u8) {
        self.with_interner(|part, interner| {
            let Some(format) = part.format_properties() else {
                return;
            };
            if format.deepest_row_outline_level(interner).unwrap_or(0) >= level {
                return;
            }
            if let Some(format) = part.format_properties_mut() {
                format.set_deepest_row_outline_level(interner, Some(level));
            }
        });
    }
}
