//! [`GridGeometry`] — where every row and every column is, without ever asking about a row or a
//! column the file does not mention.
//!
//! # This module is the whole of the sparsity argument
//!
//! A worksheet addresses **16,384 columns by 1,048,576 rows** — about seventeen billion cells — and a
//! renderer that thinks in terms of "all the rows" is dead before it draws anything. What makes a
//! grid tractable is that the geometry is almost entirely *default*: a sheet states a default row
//! height and a default column width once, and then overrides them for the handful of rows and runs
//! of columns that differ.
//!
//! So this holds **two sparse indices and no dense array**:
//!
//! * [`RowGeometry`] — one entry per row that states a height, a hidden flag, an outline level or a
//!   collapse flag, each carrying the cumulative offset of its own top. A sheet whose only populated
//!   cell is `XFD1048576` produces **one** entry, or none.
//! * [`ColumnGeometry`] — one entry per `col` **run**, which is what `CT_Col` already is:
//!   `<col min="1" max="16384" width="12"/>` is one entry for the whole sheet, not sixteen thousand.
//!
//! Both answer `where is row *n*` and `which row is at *y*` by binary search plus one multiplication
//! by the default, so the cost of a query is `O(log k)` in the number of *stated* overrides and is
//! completely independent of how far down the sheet the question is asked. That is the difference
//! between a grid that scrolls and one that does not, and it is measured rather than claimed:
//! `tests/sparsity_is_measured.rs` lays out a sheet whose one populated cell is in the far corner
//! under a counting allocator.
//!
//! # Units: three of them, converted here and nowhere else
//!
//! SpreadsheetML measures a row in **points**, a column in **characters of the maximum digit width**
//! of the workbook's Normal font, and this workspace positions fragments in **EMU**. All three meet
//! here:
//!
//! * A row's `@ht` is points, so [`Emu::from_points`] is exact and there is nothing to decide.
//! * A column's `@width` is not a length at all. ECMA-376 Part 1 §18.3.1.13 gives the round trip
//!   through pixels — `pixels = Truncate(((256 * width + Truncate(128 / MDW)) / 256) * MDW)` — where
//!   *MDW* is the width in whole pixels of the digit `0` in the Normal font at 96 dpi. That formula
//!   is implemented literally in [`column_characters_to_pixels`], truncations included, because the
//!   truncations are what make a column land on a whole pixel in Excel — **plus**
//!   [`COLUMN_PADDING_PIXELS`], which §18.3.1.13 omits and §18.3.1.81 names, and without which the
//!   default column comes out five pixels narrower than the one Excel draws.
//!
//! **MDW is measured, not assumed.** It is the one number in the grid that depends on a font, and
//! [`MaximumDigitWidth::measure`] shapes a single `0` through `mjx-text` to get it. Hard-coding the
//! familiar `7` would be hard-coding 11 pt Calibri into a renderer that must open a workbook whose
//! Normal style is 14 pt Times.
//!
//! # What is deliberately not here: recomputed row heights
//!
//! Excel auto-fits a row's height to its content and **writes the result into the file** — that is
//! what `ht` without `customHeight` means (see [`mjx_sml::RowHeight`]). So a row's height is read and
//! never recomputed here, and a row that states none is exactly `defaultRowHeight` tall.
//!
//! That is a design decision rather than an omission, and the reason is this module's own argument:
//! a height that depended on the row's text would make `row_top` depend on every row above it, which
//! is a prefix sum over the *addressable* range and not over the stated one. The sparse index and
//! recomputed row heights cannot both exist. Auto-fitting a **column**, which does not have that
//! property, is [`crate::autofit`].

use mjx_ooxml_core::measure::Emu;
use mjx_sml::{SheetFormatProperties, WorksheetPart};
use mjx_text::{FeatureSet, FontFace, FontSize, Shaper, ShapingRequest, TextScript};
use std::sync::Arc;

/// How many columns a worksheet can address — `XFD` is column 16,384.
pub const COLUMN_COUNT: u32 = 16_384;

/// How many rows a worksheet can address.
pub const ROW_COUNT: u32 = 1_048_576;

/// EMU in one pixel at 96 dpi, which is the resolution every SpreadsheetML pixel figure is quoted
/// at.
pub const EMU_PER_PIXEL: i64 = mjx_ooxml_core::measure::EMU_PER_INCH / 96;

/// `sheetFormatPr@defaultRowHeight`'s value when the sheet states none.
///
/// The attribute is `use="required"`, so a sheet that omits it is malformed — and a renderer must
/// still draw it. Fifteen points is Excel's own default for an 11 pt Calibri Normal style.
///
/// GUESS: that a malformed sheet should be drawn at Excel's default rather than at, say, the height
/// its tallest font would need. Excel's behaviour on a `sheetFormatPr` with no `defaultRowHeight`
/// has not been observed on Windows.
pub const ASSUMED_DEFAULT_ROW_HEIGHT_POINTS: f64 = 15.0;

/// `sheetFormatPr@baseColWidth`'s schema default — eight characters.
pub const DEFAULT_BASE_COLUMN_CHARACTERS: u32 = 8;

/// The padding, in pixels, a column carries beyond the characters it is sized for.
///
/// ECMA-376 Part 1 §18.3.1.81 writes the default column width as
/// `Truncate((baseColWidth * MDW + 5) / MDW * 256) / 256`, and the `5` is the gridline and the two
/// leading/trailing margins together.
pub const COLUMN_PADDING_PIXELS: i64 = 5;

/// The width, in whole pixels at 96 dpi, of the digit `0` in the workbook's Normal font.
///
/// Every column width in `sml.xsd` is quoted in multiples of this, so it is the conversion constant
/// the whole horizontal axis of a worksheet depends on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MaximumDigitWidth(i64);

impl MaximumDigitWidth {
    /// Excel's figure for an 11 pt Calibri Normal style, and the fallback when no face resolves.
    pub const ASSUMED: Self = Self(7);

    /// The width in pixels, never zero — a zero would make every column zero wide and would divide
    /// by zero in [`column_pixels_to_characters`].
    #[must_use]
    pub fn pixels(self) -> i64 {
        self.0.max(1)
    }

    /// The width the digit `0` actually occupies in `face` at `size`, rounded to a whole pixel at
    /// 96 dpi.
    ///
    /// Rounded rather than truncated: Excel's own figures — 7 px for 11 pt Calibri, 8 px for 12 pt —
    /// are whole pixels, and the specification's formulas take MDW as an integer throughout.
    ///
    /// GUESS: the rounding direction. §18.3.1.13 says *"maximum digit width"* and never says how a
    /// fractional advance becomes an integer, so a face whose `0` measures 7.5 px is a place where
    /// this may disagree with Excel by one pixel per column.
    ///
    /// # Errors
    /// [`mjx_text::FontError`] when the face will not shape, which it already parsed once.
    pub fn measure(
        shaper: &mut Shaper,
        face: &Arc<FontFace>,
        size: FontSize,
        features: &FeatureSet,
    ) -> Result<Self, mjx_text::FontError> {
        let request = ShapingRequest::new("0", TextScript::LATIN, size, features);
        let run = shaper.shape(face, &request)?;
        let pixels = points_to_pixels(run.advance_in_points()).round();
        #[allow(clippy::cast_possible_truncation)]
        let pixels = if pixels.is_finite() && (1.0..1_000.0).contains(&pixels) {
            pixels as i64
        } else {
            // A face whose digit measures nothing, or something absurd, is a face this cannot size a
            // column from. Excel's own default is a better answer than a zero-width grid.
            Self::ASSUMED.0
        };
        Ok(Self(pixels))
    }
}

/// `points * 96 / 72` — a typographic length as pixels at 96 dpi.
fn points_to_pixels(points: f64) -> f64 {
    points * 96.0 / 72.0
}

/// ECMA-376 Part 1 §18.3.1.13's characters-to-pixels formula, truncations included, **plus the
/// column's own padding**.
///
/// `pixels = Truncate(((256 * width + Truncate(128 / MDW)) / 256) * MDW) + 5`
///
/// GUESS: the `+ 5`. §18.3.1.13's formula alone gives 59 pixels for the familiar default width of
/// 8.43 characters at MDW = 7, and Excel's own column-width dialogue reports that column as **64
/// pixels** — the difference being exactly [`COLUMN_PADDING_PIXELS`], which §18.3.1.81 names as part
/// of a column's width in its own formula and §18.3.1.13 silently omits from this one. Adding it is
/// also what makes this and [`column_pixels_to_characters`] genuine inverses: 8.43 characters is 64
/// pixels and 64 pixels is 8.43 characters. Which of the two the specification meant is not
/// decidable from the text, and the Windows sitting is where the observable number settles it.
#[must_use]
pub fn column_characters_to_pixels(width: f64, digit: MaximumDigitWidth) -> i64 {
    if !width.is_finite() || width <= 0.0 {
        return 0;
    }
    let digit_pixels = digit.pixels();
    #[allow(clippy::cast_precision_loss)]
    let mdw = digit_pixels as f64;
    let rounding = (128.0 / mdw).trunc();
    let pixels = (((256.0 * width + rounding) / 256.0) * mdw).trunc();
    #[allow(clippy::cast_possible_truncation)]
    if pixels.is_finite() && (0.0..1.0e9).contains(&pixels) {
        pixels as i64 + COLUMN_PADDING_PIXELS
    } else {
        0
    }
}

/// The inverse: how many characters a column of `pixels` is worth.
///
/// ECMA-376 Part 1 §18.3.1.13: `width = Truncate((pixels - 5) / MDW * 100 + 0.5) / 100`, which is
/// what an auto-fitted width has to be reported in, because `@width` is written in characters.
#[must_use]
pub fn column_pixels_to_characters(pixels: i64, digit: MaximumDigitWidth) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    let usable = (pixels - COLUMN_PADDING_PIXELS).max(0) as f64;
    #[allow(clippy::cast_precision_loss)]
    let mdw = digit.pixels() as f64;
    (usable / mdw * 100.0 + 0.5).trunc() / 100.0
}

/// One row that says something about itself.
///
/// Present in [`RowGeometry`] only for a row the file gave a height, a hidden flag, an outline level
/// or a collapse flag. Every other row is the sheet's default, costs nothing, and is answered by
/// arithmetic.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RowSpan {
    /// The zero-based row number.
    pub row: u32,
    /// How tall it is. Zero for a hidden row.
    pub height: Emu,
    /// Whether it is hidden — `row@hidden`, or `sheetFormatPr@zeroHeight` for a row that states
    /// nothing.
    pub hidden: bool,
    /// `row@outlineLevel` — how deep in the outline it sits, 0 for the top level.
    pub outline_level: u8,
    /// `row@collapsed` — whether the group *below* this summary row is collapsed.
    pub collapsed: bool,
    /// Where its top edge is, measured from the top of the sheet.
    pub top: Emu,
}

impl RowSpan {
    /// Where its bottom edge is.
    #[must_use]
    pub fn bottom(&self) -> Emu {
        self.top + self.height
    }
}

/// One run of columns that says something about itself — a `col` element, as the file wrote it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ColumnSpan {
    /// The first zero-based column of the run.
    pub first: u16,
    /// The last zero-based column, inclusive.
    pub last: u16,
    /// How wide each of them is. Zero for a hidden run.
    pub width: Emu,
    /// Whether the run is hidden.
    pub hidden: bool,
    /// `col@outlineLevel`.
    pub outline_level: u8,
    /// `col@collapsed`.
    pub collapsed: bool,
    /// `col@bestFit` — whether a consumer is asked to size these columns to their content.
    pub best_fit: bool,
    /// Whether `@width` was written with `customWidth="1"`, which is the difference between a width
    /// Excel keeps and one it may recompute.
    pub custom_width: bool,
    /// Where the run's left edge is, measured from the left of the sheet.
    pub left: Emu,
}

impl ColumnSpan {
    /// How many columns the run covers.
    #[must_use]
    pub fn count(&self) -> u32 {
        u32::from(self.last.saturating_sub(self.first)) + 1
    }

    /// Where the run's right edge is.
    #[must_use]
    pub fn right(&self) -> Emu {
        self.left + self.width.times(i64::from(self.count()))
    }
}

/// The vertical axis: one entry per row that states something, and a default for all the rest.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RowGeometry {
    spans: Vec<RowSpan>,
    default_height: Emu,
    default_hidden: bool,
}

impl RowGeometry {
    /// The rows a worksheet states, in ascending order, with their cumulative offsets.
    ///
    /// Reads the packed cell store rather than a coordinate range: a row exists here only because
    /// the file wrote a `<row>` element for it, so a sheet with one populated row builds a one-entry
    /// index whatever its row number is.
    #[must_use]
    pub fn read(worksheet: &WorksheetPart, format: Option<&SheetFormatProperties>) -> Self {
        let interner = worksheet.interner();
        let default_height = format
            .and_then(|format| format.default_row_height(interner).ok())
            .filter(|height| height.is_finite() && *height >= 0.0)
            .map_or_else(
                || Emu::from_points(ASSUMED_DEFAULT_ROW_HEIGHT_POINTS),
                Emu::from_points,
            );
        let default_hidden = format
            .and_then(|format| format.rows_hidden_by_default(interner).ok())
            .unwrap_or(false);

        let mut spans: Vec<RowSpan> = Vec::new();
        if let Some(cells) = worksheet.sheet_data() {
            for row in cells.rows() {
                // A row that wrote no `@r` is held with no number by the store, deliberately, so
                // there is no position to put it at. It is reported by `SheetData::anomalies` and it
                // is skipped here rather than being given a number nobody wrote.
                let Some(number) = row.number() else { continue };
                let Some(zero_based) = number.checked_sub(1) else {
                    continue;
                };
                let stated_height = row
                    .height()
                    .filter(|height| height.is_finite() && *height >= 0.0);
                let hidden = row.is_hidden() || (default_hidden && stated_height.is_none());
                let outline_level = row.outline_level();
                let collapsed = row.is_collapsed();
                if stated_height.is_none() && !hidden && outline_level == 0 && !collapsed {
                    // Says nothing the default does not already say. Keeping it would make the index
                    // as large as the populated row count for no answer it could give.
                    continue;
                }
                let height = if hidden {
                    Emu::ZERO
                } else {
                    stated_height.map_or(default_height, Emu::from_points)
                };
                spans.push(RowSpan {
                    row: zero_based,
                    height,
                    hidden,
                    outline_level,
                    collapsed,
                    top: Emu::ZERO,
                });
            }
        }
        // A file may write its rows in any order and this library never sorts a file; the *index*
        // is sorted because a binary search is what it is for, and the store keeps the document
        // order it was read in.
        spans.sort_unstable_by_key(|span| span.row);
        spans.dedup_by_key(|span| span.row);

        let mut geometry = Self {
            spans,
            default_height,
            default_hidden,
        };
        geometry.accumulate();
        geometry
    }

    /// Fills in each span's `top` by walking the list once.
    fn accumulate(&mut self) {
        let default = self.default_row_height();
        let mut cursor = Emu::ZERO;
        let mut previous: Option<u32> = None;
        for index in 0..self.spans.len() {
            let (row, height) = {
                let span = &self.spans[index];
                (span.row, span.height)
            };
            let gap = match previous {
                None => u64::from(row),
                Some(before) => u64::from(row.saturating_sub(before).saturating_sub(1)),
            };
            cursor += default.times(i64::try_from(gap).unwrap_or(i64::MAX));
            self.spans[index].top = cursor;
            cursor += height;
            previous = Some(row);
        }
    }

    /// How tall a row that states nothing is. Zero when `sheetFormatPr@zeroHeight` is set, because
    /// that is precisely what that attribute says.
    #[must_use]
    pub fn default_row_height(&self) -> Emu {
        if self.default_hidden {
            Emu::ZERO
        } else {
            self.default_height
        }
    }

    /// Whether a row that states nothing is hidden.
    #[must_use]
    pub fn rows_hidden_by_default(&self) -> bool {
        self.default_hidden
    }

    /// Every row that states something, ascending.
    #[must_use]
    pub fn spans(&self) -> &[RowSpan] {
        &self.spans
    }

    /// The stated entry for `row`, or `None` when it takes the sheet's default.
    #[must_use]
    pub fn span(&self, row: u32) -> Option<&RowSpan> {
        self.spans
            .binary_search_by_key(&row, |span| span.row)
            .ok()
            .and_then(|index| self.spans.get(index))
    }

    /// How tall `row` is. Zero for a hidden row, which is what makes a hidden row occupy nothing
    /// rather than being skipped by every caller separately.
    #[must_use]
    pub fn height(&self, row: u32) -> Emu {
        self.span(row)
            .map_or_else(|| self.default_row_height(), |span| span.height)
    }

    /// Whether `row` is hidden.
    #[must_use]
    pub fn is_hidden(&self, row: u32) -> bool {
        self.span(row)
            .map_or(self.default_hidden, |span| span.hidden)
    }

    /// `row@outlineLevel`, zero for a row that states none.
    #[must_use]
    pub fn outline_level(&self, row: u32) -> u8 {
        self.span(row).map_or(0, |span| span.outline_level)
    }

    /// Where `row`'s top edge is, measured from the top of the sheet.
    ///
    /// `O(log k)` in the number of *stated* rows, and independent of `row`: asking about row
    /// 1,048,575 of a sheet that states nothing costs one multiplication.
    #[must_use]
    pub fn top(&self, row: u32) -> Emu {
        let default = self.default_row_height();
        match self.spans.binary_search_by_key(&row, |span| span.row) {
            Ok(index) => self.spans[index].top,
            Err(0) => default.times(i64::from(row)),
            Err(index) => {
                let previous = &self.spans[index - 1];
                let gap = u64::from(row.saturating_sub(previous.row).saturating_sub(1));
                previous.bottom() + default.times(i64::try_from(gap).unwrap_or(i64::MAX))
            }
        }
    }

    /// The first **visible** row whose bottom edge is past `offset`, or `None` when nothing is.
    ///
    /// This is what turns a scroll position into a row number, and it is the reason `top` alone is
    /// not enough: a run of hidden rows all share one offset, and a reader scrolled to it must land
    /// on the first row that actually occupies space.
    #[must_use]
    pub fn row_at(&self, offset: Emu) -> Option<u32> {
        let offset = offset.maximum(Emu::ZERO);
        let default = self.default_row_height();
        // The last stated row whose top is at or before the offset bounds the search; everything
        // before it is settled, and everything after it is default-height arithmetic.
        let at = self.spans.partition_point(|span| span.top <= offset);
        let mut row = match at.checked_sub(1) {
            None => {
                if default <= Emu::ZERO {
                    // Every unstated row is zero-high, so no arithmetic can reach the offset; the
                    // answer is the first stated row, if there is one.
                    return self.spans.first().map(|span| span.row);
                }
                let steps = offset.emu() / default.emu().max(1);
                u32::try_from(steps)
                    .unwrap_or(ROW_COUNT - 1)
                    .min(ROW_COUNT - 1)
            }
            Some(index) => {
                let span = &self.spans[index];
                if offset < span.bottom() && !span.hidden {
                    return Some(span.row);
                }
                if default <= Emu::ZERO {
                    return self
                        .spans
                        .get(index + 1)
                        .map(|next| next.row)
                        .or(Some(span.row));
                }
                let past = (offset - span.bottom()).emu().max(0);
                let steps = past / default.emu().max(1);
                span.row
                    .saturating_add(1)
                    .saturating_add(u32::try_from(steps).unwrap_or(0))
                    .min(ROW_COUNT - 1)
            }
        };
        // The arithmetic above lands on the right row unless that row is hidden or stated, in which
        // case the next visible one is the answer. `next_visible_row` walks stated entries, never
        // addressable ones.
        if self.is_hidden(row) {
            row = self.next_visible_row(row)?;
        }
        Some(row)
    }

    /// The first visible row at or after `row`, or `None` when every row from there down is hidden.
    ///
    /// Walks the **stated** entries, so a block of ten thousand hidden rows costs ten thousand
    /// steps — which is the number of `<row>` elements the file wrote for them — and never costs
    /// anything for the rows nobody wrote.
    #[must_use]
    pub fn next_visible_row(&self, row: u32) -> Option<u32> {
        if row >= ROW_COUNT {
            return None;
        }
        if !self.is_hidden(row) {
            return Some(row);
        }
        let mut at = self.spans.partition_point(|span| span.row < row);
        let mut candidate = row;
        while let Some(span) = self.spans.get(at) {
            if span.row > candidate {
                // The gap between `candidate` and this stated row is all default rows.
                return (!self.default_hidden).then_some(candidate);
            }
            if !span.hidden {
                return Some(span.row);
            }
            candidate = span.row.saturating_add(1);
            at += 1;
        }
        (!self.default_hidden && candidate < ROW_COUNT).then_some(candidate)
    }
}

/// The horizontal axis: one entry per `col` run, and a default for every column no run covers.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ColumnGeometry {
    spans: Vec<ColumnSpan>,
    default_width: Emu,
}

impl ColumnGeometry {
    /// The runs a worksheet states, clipped so that no two overlap, in ascending order and with
    /// their cumulative offsets.
    ///
    /// Where two `col` runs overlap — which a file may write — the **first in document order** wins,
    /// which is the same rule
    /// [`WorksheetPart::column_run_covering`](mjx_sml::WorksheetPart::column_run_covering) applies.
    /// Nothing is merged: two adjacent runs with identical attributes stay two, because the number
    /// of elements is part of the file.
    ///
    /// # Errors
    /// [`mjx_sml::SmlError`] when a `col` is missing `@min` or `@max`, both of which are
    /// `use="required"`: a run with no bounds is not a run this can place.
    pub fn read(
        worksheet: &WorksheetPart,
        format: Option<&SheetFormatProperties>,
        digit: MaximumDigitWidth,
    ) -> Result<Self, mjx_sml::SmlError> {
        let interner = worksheet.interner();
        let default_width = default_column_width(format, interner, digit);

        let mut claimed: Vec<ColumnSpan> = Vec::new();
        for block in worksheet.column_blocks() {
            for run in block.runs() {
                let first = run
                    .first_column(interner)
                    .map_err(mjx_ooxml_core::FromXmlError::from)?;
                let last = run
                    .last_column(interner)
                    .map_err(mjx_ooxml_core::FromXmlError::from)?;
                let (low, high) = (first.min(last), first.max(last));
                let Some(low) = low.checked_sub(1) else {
                    // `@min="0"` names no column: the wire is one-based and `A` is 1.
                    continue;
                };
                let high = high.saturating_sub(1).min(COLUMN_COUNT - 1);
                if low > high {
                    continue;
                }
                let low = u16::try_from(low).unwrap_or(u16::MAX);
                let high = u16::try_from(high).unwrap_or(u16::MAX);

                let hidden = run.hidden(interner).unwrap_or(false);
                let stated = run
                    .width(interner)
                    .ok()
                    .flatten()
                    .filter(|width| width.is_finite() && *width >= 0.0);
                let width = if hidden {
                    Emu::ZERO
                } else {
                    stated.map_or(default_width, |width| {
                        Emu::from_emu(column_characters_to_pixels(width, digit) * EMU_PER_PIXEL)
                    })
                };
                let span = ColumnSpan {
                    first: low,
                    last: high,
                    width,
                    hidden,
                    outline_level: run.outline_level(interner).unwrap_or(0),
                    collapsed: run.collapsed(interner).unwrap_or(false),
                    best_fit: run.best_fit(interner).unwrap_or(false),
                    custom_width: run.custom_width(interner).unwrap_or(false),
                    left: Emu::ZERO,
                };
                push_clipped(&mut claimed, span);
            }
        }
        claimed.sort_unstable_by_key(|span| span.first);

        let mut geometry = Self {
            spans: claimed,
            default_width,
        };
        geometry.accumulate();
        Ok(geometry)
    }

    /// Fills in each run's `left` by walking the list once.
    fn accumulate(&mut self) {
        let default = self.default_width;
        let mut cursor = Emu::ZERO;
        let mut previous: Option<u16> = None;
        for index in 0..self.spans.len() {
            let (first, width, count) = {
                let span = &self.spans[index];
                (span.first, span.width, span.count())
            };
            let gap = match previous {
                None => u32::from(first),
                Some(before) => u32::from(first.saturating_sub(before).saturating_sub(1)),
            };
            cursor += default.times(i64::from(gap));
            self.spans[index].left = cursor;
            cursor += width.times(i64::from(count));
            previous = Some(self.spans[index].last);
        }
    }

    /// How wide a column no run covers is.
    #[must_use]
    pub fn default_column_width(&self) -> Emu {
        self.default_width
    }

    /// Every run, ascending and non-overlapping.
    #[must_use]
    pub fn spans(&self) -> &[ColumnSpan] {
        &self.spans
    }

    /// The run covering `column`, or `None` when it takes the sheet's default.
    #[must_use]
    pub fn span(&self, column: u16) -> Option<&ColumnSpan> {
        let at = self.spans.partition_point(|span| span.first <= column);
        let span = self.spans.get(at.checked_sub(1)?)?;
        (span.last >= column).then_some(span)
    }

    /// How wide `column` is. Zero for a hidden one.
    #[must_use]
    pub fn width(&self, column: u16) -> Emu {
        self.span(column)
            .map_or(self.default_width, |span| span.width)
    }

    /// Whether `column` is hidden.
    #[must_use]
    pub fn is_hidden(&self, column: u16) -> bool {
        self.span(column).is_some_and(|span| span.hidden)
    }

    /// `col@outlineLevel`, zero for a column no run covers.
    #[must_use]
    pub fn outline_level(&self, column: u16) -> u8 {
        self.span(column).map_or(0, |span| span.outline_level)
    }

    /// Whether `column` asks a consumer to size it to its content — `col@bestFit`, or a run that
    /// states a width without `customWidth="1"`.
    #[must_use]
    pub fn wants_auto_fit(&self, column: u16) -> bool {
        self.span(column).is_some_and(|span| span.best_fit)
    }

    /// Where `column`'s left edge is, measured from the left of the sheet.
    #[must_use]
    pub fn left(&self, column: u16) -> Emu {
        let at = self.spans.partition_point(|span| span.first <= column);
        match at.checked_sub(1) {
            None => self.default_width.times(i64::from(column)),
            Some(index) => {
                let span = &self.spans[index];
                if span.last >= column {
                    let inside = u32::from(column.saturating_sub(span.first));
                    span.left + span.width.times(i64::from(inside))
                } else {
                    let gap = u32::from(column.saturating_sub(span.last).saturating_sub(1));
                    span.right() + self.default_width.times(i64::from(gap))
                }
            }
        }
    }

    /// The first **visible** column whose right edge is past `offset`, or `None`.
    #[must_use]
    pub fn column_at(&self, offset: Emu) -> Option<u16> {
        let offset = offset.maximum(Emu::ZERO);
        let default = self.default_width;
        let at = self.spans.partition_point(|span| span.left <= offset);
        let mut column = match at.checked_sub(1) {
            None => {
                if default <= Emu::ZERO {
                    return self.spans.first().map(|span| span.first);
                }
                let steps = offset.emu() / default.emu().max(1);
                u16::try_from(steps.min(i64::from(COLUMN_COUNT - 1))).unwrap_or(u16::MAX)
            }
            Some(index) => {
                let span = &self.spans[index];
                if offset < span.right() && !span.hidden {
                    let inside = (offset - span.left).emu() / span.width.emu().max(1);
                    let inside = u32::try_from(inside).unwrap_or(0);
                    return Some(
                        span.first
                            .saturating_add(u16::try_from(inside).unwrap_or(0))
                            .min(span.last),
                    );
                }
                if default <= Emu::ZERO {
                    return self
                        .spans
                        .get(index + 1)
                        .map(|next| next.first)
                        .or(Some(span.first));
                }
                let past = (offset - span.right()).emu().max(0);
                let steps = past / default.emu().max(1);
                span.last
                    .saturating_add(1)
                    .saturating_add(u16::try_from(steps).unwrap_or(0))
            }
        };
        if column >= COLUMN_COUNT_U16 {
            return None;
        }
        if self.is_hidden(column) {
            column = self.next_visible_column(column)?;
        }
        Some(column)
    }

    /// The first visible column at or after `column`, or `None` when every column from there right
    /// is hidden.
    ///
    /// Walks the **runs**, so a hidden block of five thousand columns is one step and not five
    /// thousand — which is the point of `col` being a run rather than a column.
    #[must_use]
    pub fn next_visible_column(&self, column: u16) -> Option<u16> {
        if column >= COLUMN_COUNT_U16 {
            return None;
        }
        if !self.is_hidden(column) {
            return Some(column);
        }
        let mut at = self.spans.partition_point(|span| span.last < column);
        let mut candidate = column;
        while let Some(span) = self.spans.get(at) {
            if span.first > candidate {
                return Some(candidate);
            }
            if !span.hidden {
                return Some(span.first.max(candidate));
            }
            candidate = span.last.checked_add(1)?;
            at += 1;
        }
        (candidate < COLUMN_COUNT_U16).then_some(candidate)
    }
}

/// `COLUMN_COUNT` as the `u16` a column index is, so a comparison does not have to widen.
const COLUMN_COUNT_U16: u16 = 16_384;

/// `sheetFormatPr`'s default column width, in EMU.
///
/// Two spellings, and the file may write either. `@defaultColWidth` is the width in characters and
/// goes through §18.3.1.13's round trip; `@baseColWidth` is a plain character *count* and
/// §18.3.1.81's `baseColWidth * MDW + 5` is the pixel width it means. The second is used only when
/// the first is absent, because deriving one from the other and back would lose a fraction the file
/// stated.
fn default_column_width(
    format: Option<&SheetFormatProperties>,
    interner: &mjx_ooxml_core::Interner,
    digit: MaximumDigitWidth,
) -> Emu {
    if let Some(width) = format
        .and_then(|format| format.default_column_width(interner).ok().flatten())
        .filter(|width| width.is_finite() && *width > 0.0)
    {
        return Emu::from_emu(column_characters_to_pixels(width, digit) * EMU_PER_PIXEL);
    }
    let characters = format
        .and_then(|format| format.base_column_character_width(interner).ok())
        .unwrap_or(DEFAULT_BASE_COLUMN_CHARACTERS);
    let pixels = i64::from(characters) * digit.pixels() + COLUMN_PADDING_PIXELS;
    Emu::from_emu(pixels * EMU_PER_PIXEL)
}

/// Adds `span` to `claimed`, dropping any part of it an earlier run already covers.
///
/// Document order decides: an earlier `col` wins the overlap, which is what
/// `WorksheetPart::column_run_covering` answers with and therefore what every other reader in this
/// workspace already believes.
fn push_clipped(claimed: &mut Vec<ColumnSpan>, span: ColumnSpan) {
    let mut pending = vec![span];
    for existing in claimed.iter() {
        let mut next = Vec::with_capacity(pending.len() + 1);
        for piece in pending {
            if piece.last < existing.first || piece.first > existing.last {
                next.push(piece);
                continue;
            }
            if piece.first < existing.first {
                next.push(ColumnSpan {
                    last: existing.first.saturating_sub(1),
                    ..piece
                });
            }
            if piece.last > existing.last {
                next.push(ColumnSpan {
                    first: existing.last.saturating_add(1),
                    ..piece
                });
            }
        }
        pending = next;
        if pending.is_empty() {
            return;
        }
    }
    claimed.extend(pending);
}

/// Both axes, together — everything a cell's rectangle is computed from.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GridGeometry {
    rows: RowGeometry,
    columns: ColumnGeometry,
    digit: MaximumDigitWidth,
}

impl GridGeometry {
    /// Both indices, built from one worksheet.
    #[must_use]
    pub fn new(rows: RowGeometry, columns: ColumnGeometry, digit: MaximumDigitWidth) -> Self {
        Self {
            rows,
            columns,
            digit,
        }
    }

    /// The vertical axis.
    #[must_use]
    pub fn rows(&self) -> &RowGeometry {
        &self.rows
    }

    /// The horizontal axis.
    #[must_use]
    pub fn columns(&self) -> &ColumnGeometry {
        &self.columns
    }

    /// The width of the digit `0` every column width on this sheet is quoted in.
    #[must_use]
    pub fn maximum_digit_width(&self) -> MaximumDigitWidth {
        self.digit
    }

    /// The rectangle a single cell occupies, in **sheet** coordinates — the origin is the top-left
    /// of `A1`, whatever is scrolled into view.
    #[must_use]
    pub fn cell_rect(&self, row: u32, column: u16) -> mjx_layout::LayoutRect {
        let left = self.columns.left(column);
        let top = self.rows.top(row);
        mjx_layout::LayoutRect::from_edges(
            left,
            top,
            left + self.columns.width(column),
            top + self.rows.height(row),
        )
    }

    /// The rectangle a rectangular block of cells occupies, in sheet coordinates — what a merged
    /// region is drawn at.
    #[must_use]
    pub fn block_rect(
        &self,
        first_row: u32,
        first_column: u16,
        last_row: u32,
        last_column: u16,
    ) -> mjx_layout::LayoutRect {
        let left = self.columns.left(first_column);
        let top = self.rows.top(first_row);
        let right = self.columns.left(last_column) + self.columns.width(last_column);
        let bottom = self.rows.top(last_row) + self.rows.height(last_row);
        mjx_layout::LayoutRect::from_edges(left, top, right.maximum(left), bottom.maximum(top))
    }
}
