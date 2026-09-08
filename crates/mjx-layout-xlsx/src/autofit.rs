//! Auto-fit — **the one place a grid asks the text engine to decide geometry rather than to fill
//! it.**
//!
//! # Columns only, and that is a decision
//!
//! Excel auto-fits both axes, and this crate auto-fits one. The reason is
//! [`crate::geometry`]'s own argument: a recomputed row height would make the top of row *n* depend
//! on the text of every row above it, which is a prefix sum over the addressable range rather than
//! over the stated one — the sparse index and recomputed row heights cannot both exist.
//!
//! The good news is that they do not have to. Excel **writes the fitted height into the file**:
//! `ht` without `customHeight` is precisely *"a consumer computed this and may compute it again"*
//! (see [`mjx_sml::RowHeight`]), so a row that has ever been auto-fitted by Excel already carries
//! the number, and honouring it reproduces what the author saw. A column is different only because
//! `col@bestFit` asks the consumer to size it *now*.
//!
//! # What it costs, and why that is bounded
//!
//! Fitting a column measures **every populated cell in it**, and populated is the operative word:
//! the walk is over the rows the file wrote, never over the 1,048,576 the grid addresses. A sheet
//! whose only cell is `XFD1048576` measures one cell. A sheet of three hundred thousand cells in ten
//! thousand rows measures ten thousand, once, and the answer is memoised — which is what makes this
//! safe to reach from a layout pass at all.
//!
//! **It runs only when asked, and its answer is reported rather than applied.** A column with a
//! stated `@width` is that wide, full stop; a column with none is
//! [`ColumnGeometry::default_column_width`](crate::geometry::ColumnGeometry). Only
//! `col@bestFit="1"` and an explicit [`SheetBoxModel::auto_fit_width`](crate::SheetBoxModel) call
//! reach here, so the common sheet never pays — and a `bestFit` column keeps the width its file
//! states, because that width is the one Excel computed when it last laid the column out, exactly as
//! `ht` without `customHeight` is the height it computed for a row. The fit is put in
//! [`PageCatalogue::auto_fits`](crate::PageCatalogue::auto_fits), which is what a *Format → AutoFit
//! Column Width* command reads; nothing here rewrites the geometry with it.
//!
//! # ⚠ The number this produces is a reading
//!
//! GUESS: a fitted width is the widest cell's text plus the cell's own padding, expressed in whole
//! pixels and then converted back to characters through §18.3.1.13's inverse. Excel adds a further
//! margin whose size is not written down anywhere, and caps a fitted column at 255 characters.
//! Both are reproduced here as the closest available answers, and the sitting is where the exact
//! figures come from.

use std::collections::HashMap;

use mjx_ooxml_core::measure::Emu;

use crate::cell::{CellStyle, CELL_PADDING_PIXELS};
use crate::geometry::{column_pixels_to_characters, MaximumDigitWidth, EMU_PER_PIXEL};
use crate::sheet::SheetGrid;
use crate::text::{self, TextEngine};

/// The widest a fitted column may be, in characters — Excel's own cap.
pub const MAXIMUM_FITTED_CHARACTERS: f64 = 255.0;

/// How many populated cells one column's fit will measure before it stops and answers with what it
/// has.
///
/// GUESS: a guard rather than a rule. A column of a million populated cells would otherwise shape a
/// million runs on the frame that first asked for it, and the widest of the first fifty thousand is
/// almost always the widest of all of them. It is recorded in
/// [`AutoFit::sampled_every_cell`] so a caller can tell a measured answer from a sampled one rather
/// than having to trust it.
pub const MAXIMUM_CELLS_SAMPLED: usize = 50_000;

/// What fitting a column found.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct AutoFit {
    /// The width, in EMU.
    pub width: Emu,
    /// The same width in the characters `col@width` is written in, so a caller that wants to *store*
    /// the fit has the number the file takes.
    pub characters: f64,
    /// How many populated cells were measured.
    pub cells_measured: usize,
    /// Whether that was all of them. `false` means [`MAXIMUM_CELLS_SAMPLED`] stopped the walk and the
    /// answer is a sample rather than a measurement.
    pub sampled_every_cell: bool,
}

/// Auto-fitted column widths, memoised per column.
///
/// Per box model rather than per page: a column's fit does not depend on which band of rows is on
/// screen, and computing it per band would give the same column two widths at two scroll positions —
/// which would break the equivalence
/// [`BoxModel::layout_page`](mjx_layout::BoxModel::layout_page) promises.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct AutoFitCache {
    sheet: Option<usize>,
    widths: HashMap<u16, AutoFit>,
}

impl AutoFitCache {
    /// An empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Forgets everything, because the content changed underneath it.
    pub fn clear(&mut self) {
        self.sheet = None;
        self.widths.clear();
    }

    /// How many columns have been fitted.
    #[must_use]
    pub fn len(&self) -> usize {
        self.widths.len()
    }

    /// Whether nothing has been fitted.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.widths.is_empty()
    }

    /// The fitted width of `column`, measuring it if this is the first ask.
    ///
    /// The cache is keyed by sheet as well as by column, so handing the same box model a second
    /// worksheet answers about that worksheet rather than about the first.
    ///
    /// # Errors
    /// [`mjx_text::FontError`] when a face will not shape;
    /// [`mjx_xlsx::XlsxError`] when the style index in force names no record in `cellXfs`.
    pub fn width(
        &mut self,
        engine: &mut TextEngine<'_>,
        grid: &SheetGrid,
        digit: MaximumDigitWidth,
        column: u16,
    ) -> Result<AutoFit, crate::SheetLayoutError> {
        if self.sheet != Some(grid.index()) {
            self.sheet = Some(grid.index());
            self.widths.clear();
        }
        if let Some(fit) = self.widths.get(&column) {
            return Ok(*fit);
        }
        let fit = measure(engine, grid, digit, column)?;
        self.widths.insert(column, fit);
        Ok(fit)
    }
}

/// Measures `column` by shaping every populated cell in it.
///
/// # Errors
/// As [`AutoFitCache::width`].
pub fn measure(
    engine: &mut TextEngine<'_>,
    grid: &SheetGrid,
    digit: MaximumDigitWidth,
    column: u16,
) -> Result<AutoFit, crate::SheetLayoutError> {
    let resolver = grid.formatting().resolver()?;
    let mut widest = Emu::ZERO;
    let mut measured = 0_usize;
    let mut complete = true;

    let Some(cells) = grid.worksheet().sheet_data() else {
        return Ok(finish(widest, digit, measured, complete));
    };
    for row in cells.rows() {
        // Iterating the **populated rows** and asking each for one column, never iterating the
        // column's coordinate range: `Row::cell` is a binary search in the row's own cell slice.
        let Some(cell) = row.cell(column) else {
            continue;
        };
        let Some(text) = grid.cell_text(&cell) else {
            continue;
        };
        if text.is_empty() {
            continue;
        }
        if measured >= MAXIMUM_CELLS_SAMPLED {
            complete = false;
            break;
        }
        let format = resolver.formats().effective_cell_format(
            Some(&cell),
            Some(&row),
            resolver
                .columns()
                .style_index(u32::from(column).saturating_add(1)),
        )?;
        let interner = resolver.formats().interner();
        let font = resolver
            .formats()
            .font(&format)
            .map(|font| font.properties(interner));
        let alignment = resolver.formats().alignment(&format);
        let style = CellStyle::resolve(alignment, interner, font.as_ref(), cell.cell_type());
        if style.wrap {
            // A wrapped cell fits any width by definition, so it says nothing about how wide the
            // column should be. Excel agrees: auto-fitting a column of wrapped cells does not widen
            // it to their unwrapped length.
            continue;
        }
        let bidi = mjx_text::BidiAnalysis::resolve(&text, text::reading_order(style.reading_order));
        let items = text::itemise_cell(engine, &text, &style.run, &bidi)?;
        let runs = text::composer_runs(engine.rasteriser, engine.features, &items, style.run.size);
        let composed = text::compose_cell(
            engine.shaper,
            &text,
            &runs,
            &bidi,
            mjx_text::LineBreakOptions::default(),
            1.0e9,
        )?;
        for line in &composed {
            widest = widest.maximum(Emu::from_points(line.width_in_points));
        }
        measured += 1;
    }
    Ok(finish(widest, digit, measured, complete))
}

/// Turns the widest measured line into a column width, padding and cap included.
fn finish(
    widest: Emu,
    digit: MaximumDigitWidth,
    cells_measured: usize,
    sampled_every_cell: bool,
) -> AutoFit {
    let padded = widest + Emu::from_emu(CELL_PADDING_PIXELS * 2 * EMU_PER_PIXEL);
    let pixels = padded.emu().div_euclid(EMU_PER_PIXEL);
    let characters =
        column_pixels_to_characters(pixels + crate::geometry::COLUMN_PADDING_PIXELS, digit)
            .clamp(0.0, MAXIMUM_FITTED_CHARACTERS);
    AutoFit {
        width: Emu::from_emu(
            crate::geometry::column_characters_to_pixels(characters, digit) * EMU_PER_PIXEL,
        ),
        characters,
        cells_measured,
        sampled_every_cell,
    }
}
