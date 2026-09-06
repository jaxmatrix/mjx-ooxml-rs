//! Anchor ↔ absolute geometry: where on a sheet a `xdr:twoCellAnchor` actually is, in EMU, and
//! which half of that answer the sheet stated rather than defaulted.
//!
//! # Two axes, and only one of them can be exact
//!
//! This is Excel's version of the effective-properties problem, and it has the same obligation to be
//! honest — but the two axes are not equally answerable, and the difference is not a matter of
//! effort:
//!
//! * **A row height is exact.** `row@ht` and `sheetFormatPr@defaultRowHeight` are in **points**, and
//!   a point is 12,700 EMU by definition. Nothing has to be measured.
//! * **A column width is not, and cannot be.** `col@width` and `sheetFormatPr@defaultColWidth` are
//!   *"the number of characters of the maximum digit width of the numbers 0, 1, 2, …, 9 as rendered
//!   in the normal style's font"* (ECMA-376 Part 1 §18.3.1.13). Turning that into a length needs the
//!   **maximum digit width in pixels of a font this library never opens**. It is a rendering fact,
//!   and this project does no rendering.
//!
//! So the column half is computed through a [`ColumnMetrics`] the **caller supplies**, and every
//! answer carries the metrics it was computed with. [`ColumnMetrics::CALIBRI_11_AT_96_DPI`] is the
//! one the specification's own worked example uses (7 pixels at 96 dpi) and is what a caller with
//! nothing better to go on should pass — but it is passed, not assumed, because a workbook whose
//! Normal style is 14-point Consolas is laid out differently and nothing in the file says by how
//! much.
//!
//! # What is defaulted is said, not smoothed over
//!
//! [`ResolvedAnchorBounds`] reports, per axis, the coarsest source that contributed to it:
//!
//! | [`GeometrySource`] | Means |
//! |---|---|
//! | [`Stated`](GeometrySource::Stated) | every row/column crossed states its own `ht`/`width` |
//! | [`SheetDefault`](GeometrySource::SheetDefault) | at least one fell back to `sheetFormatPr@defaultRowHeight`/`@defaultColWidth` |
//! | [`BaseColumnWidth`](GeometrySource::BaseColumnWidth) | the sheet states no `@defaultColWidth` either, so the width is derived from `@baseColWidth` (whose own schema default is 8) through §18.3.1.81's formula |
//!
//! And where the sheet states **nothing** that could place the object, the answer is `None`.
//! `@defaultRowHeight` is `use="required"`, so a worksheet with no `x:sheetFormatPr` at all states no
//! default row height; a row that does not write `@ht` on such a sheet has no height this library
//! can report, and inventing one would be presenting a guess as a measurement. That is the same
//! answer `mjx_pptx`'s `effective_shape_bounds` gives for a transform naming a rotation but not both
//! `a:off` and `a:ext` — *a partial transform places nothing, and `None` is the honest report of
//! that*.
//!
//! # A hidden row or column has no height or width
//!
//! `row@hidden`, `col@hidden` and `sheetFormatPr@zeroHeight` are layout, not decoration: a hidden
//! row occupies no space and every object below it moves up. They are honoured here, and a caller
//! that wants the geometry as if nothing were hidden is asking a different question this does not
//! answer.
//!
//! # Cost
//!
//! [`SheetAnchors::resolve`] is linear in the **stated** rows and column runs, never in the row or
//! column index: the offset of row 1,000,000 is one multiplication plus a walk over the rows that
//! write their own `@ht`, which is bounded by the populated rows rather than by the grid. A caller
//! resolving several anchors builds one [`SheetAnchors`] and reuses it.

use mjx_dml::spreadsheet_drawing::{Anchor, CellMarker};
use mjx_dml::{Position, Size};

use super::frame::WorksheetPart;

/// The EMU in one point — `914400 / 72`, exactly, which is why a row height is never approximate.
const EMU_PER_POINT: f64 = 12_700.0;

/// The EMU in one inch, the unit every DrawingML length is stated in.
const EMU_PER_INCH: f64 = 914_400.0;

/// The pixels of padding ECMA-376 Part 1 §18.3.1.13 says a column width includes: *"There are 4
/// pixels of margin padding (two on each side), plus 1 pixel padding for the gridlines."*
const COLUMN_PADDING_PIXELS: f64 = 5.0;

/// The font measurement a column width has to be converted through, and the resolution it is stated
/// at.
///
/// A column width is a **character count**, not a length. Nothing in a `.xlsx` says how wide a digit
/// of its Normal style's font is, and this library never opens a font — so the number has to come
/// from the caller, and every answer computed from it says so.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColumnMetrics {
    /// The width, in pixels, of the widest of the digits `0`–`9` in the workbook's Normal style
    /// font — *"maximum digit width"* in §18.3.1.13's terms.
    pub maximum_digit_width_pixels: f64,
    /// The pixels per inch the width above is stated at.
    pub pixels_per_inch: f64,
}

impl ColumnMetrics {
    /// The metrics ECMA-376 Part 1 §18.3.1.13's own worked example uses: *"Using the Calibri font as
    /// an example, the maximum digit width of 11 point font size is 7 pixels (at 96 dpi)."*
    ///
    /// The right default for a workbook whose Normal style nobody has changed, and **a stated
    /// assumption rather than a measurement** for every other workbook. It is Excel's own default
    /// font, so it is the assumption most files will be laid out under; it is still an assumption,
    /// and [`ResolvedAnchorBounds::column_metrics`] carries it into every answer so a caller can see
    /// which one produced the number.
    pub const CALIBRI_11_AT_96_DPI: Self = Self {
        maximum_digit_width_pixels: 7.0,
        pixels_per_inch: 96.0,
    };

    /// One column width in characters, converted to EMU.
    ///
    /// §18.3.1.13 gives the pixel form exactly:
    /// `Truncate(((256 * width + Truncate(128 / MDW)) / 256) * MDW)`, which for its own example
    /// (`width = 8.7109375`, `MDW = 7`) is 61 pixels. The pixels are then EMU at
    /// [`pixels_per_inch`](Self::pixels_per_inch).
    #[must_use]
    pub fn characters_to_emu(self, characters: f64) -> f64 {
        let digit = self.maximum_digit_width_pixels;
        if digit <= 0.0 || self.pixels_per_inch <= 0.0 {
            return 0.0;
        }
        let pixels = (((256.0 * characters + (128.0 / digit).trunc()) / 256.0) * digit).trunc();
        pixels * EMU_PER_INCH / self.pixels_per_inch
    }

    /// The width in characters that `sheetFormatPr@baseColWidth` implies for a column the sheet says
    /// nothing else about.
    ///
    /// §18.3.1.81: *"defaultColWidth = baseColumnWidth + {margin padding (2 pixels on each side,
    /// totalling 4 pixels)} + {gridline (1 pixel)}"* — the same five pixels §18.3.1.13's own
    /// `width` formula adds, applied to `baseColWidth` characters:
    /// `Truncate([n * MDW + 5] / MDW * 256) / 256`. For `n = 8` and `MDW = 7` this is `8.7109375`,
    /// which is the number §18.3.1.13's example writes out in full.
    #[must_use]
    pub fn base_column_width_characters(self, base_column_width: u32) -> f64 {
        let digit = self.maximum_digit_width_pixels;
        if digit <= 0.0 {
            return f64::from(base_column_width);
        }
        let characters = f64::from(base_column_width);
        ((characters * digit + COLUMN_PADDING_PIXELS) / digit * 256.0).trunc() / 256.0
    }
}

/// Where a resolved length's number came from — the honesty half of an answer.
///
/// Ordered from most to least stated, and [`ResolvedAnchorBounds`] reports the **coarsest** source
/// any row or column it crossed needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GeometrySource {
    /// Every row or column the answer crossed states its own `row@ht` / `col@width`.
    Stated,
    /// At least one fell back to `sheetFormatPr@defaultRowHeight` / `@defaultColWidth`, which the
    /// sheet does state.
    SheetDefault,
    /// The sheet states no `@defaultColWidth` either, so the width comes from
    /// `sheetFormatPr@baseColWidth` — or, when the sheet writes no `x:sheetFormatPr` at all, from
    /// that attribute's own schema default of `8`. **Columns only**: there is no equivalent fallback
    /// for a row height, and a row with nothing to fall back to answers `None` instead.
    BaseColumnWidth,
}

/// An anchor resolved to a rectangle on the sheet, and what the answer rests on.
///
/// The rectangle is in EMU from the sheet's top-left corner. Read
/// [`row_source`](Self::row_source), [`column_source`](Self::column_source) and
/// [`column_metrics`](Self::column_metrics) before treating it as a measurement — see this module's
/// own documentation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedAnchorBounds {
    /// The object's top-left corner, in EMU from the sheet origin.
    pub position: Position,
    /// The object's size, in EMU.
    pub size: Size,
    /// The coarsest source the **vertical** half of this answer needed.
    pub row_source: GeometrySource,
    /// The coarsest source the **horizontal** half needed.
    pub column_source: GeometrySource,
    /// The font measurement the horizontal half was computed through. Always present, because a
    /// column width is never a length until one is chosen.
    pub column_metrics: ColumnMetrics,
}

impl ResolvedAnchorBounds {
    /// Whether every row and column this answer crossed stated its own size.
    ///
    /// **Not the same as "exact".** Even an all-[`Stated`](GeometrySource::Stated) answer's
    /// horizontal half went through [`column_metrics`](Self::column_metrics), because `col@width` is
    /// a character count rather than a length. A caller reporting this rectangle to a person should
    /// say which font it assumed either way.
    #[must_use]
    pub fn is_fully_stated(&self) -> bool {
        self.row_source == GeometrySource::Stated && self.column_source == GeometrySource::Stated
    }
}

/// One sheet's row and column geometry, prepared once so that several anchors can be resolved
/// against it.
///
/// Borrows the [`WorksheetPart`] it reads, so there is no way to hold geometry whose sheet has moved
/// on beneath it.
#[derive(Debug)]
pub struct SheetAnchors<'a> {
    part: &'a WorksheetPart,
    metrics: ColumnMetrics,
    /// `sheetFormatPr@defaultRowHeight`, in points. `None` when the sheet writes no
    /// `x:sheetFormatPr` — the attribute is `use="required"`, so there is no other way for it to be
    /// absent, and there is nothing to fall back to.
    default_row_height: Option<f64>,
    /// `sheetFormatPr@defaultColWidth`, in characters, when the sheet states one.
    default_column_width: Option<f64>,
    /// `sheetFormatPr@baseColWidth`, defaulting to the schema's own `8`.
    base_column_width: u32,
    /// `sheetFormatPr@zeroHeight` — whether a row that says nothing is hidden.
    rows_hidden_by_default: bool,
}

impl<'a> SheetAnchors<'a> {
    /// Reads `part`'s `x:sheetFormatPr` once and prepares to resolve anchors against it.
    ///
    /// `metrics` is the font measurement the column half of every answer is computed through; see
    /// [`ColumnMetrics`].
    #[must_use]
    pub fn new(part: &'a WorksheetPart, metrics: ColumnMetrics) -> Self {
        let interner = part.interner();
        let format = part.format_properties();
        Self {
            part,
            metrics,
            default_row_height: format.and_then(|f| f.default_row_height(interner).ok()),
            default_column_width: format
                .and_then(|f| f.default_column_width(interner).ok().flatten()),
            base_column_width: format
                .and_then(|f| f.base_column_character_width(interner).ok())
                .unwrap_or(8),
            rows_hidden_by_default: format
                .and_then(|f| f.rows_hidden_by_default(interner).ok())
                .unwrap_or(false),
        }
    }

    /// The metrics this geometry was built with.
    #[must_use]
    pub fn column_metrics(&self) -> ColumnMetrics {
        self.metrics
    }

    /// The height of the **zero-based** `row`, in EMU, and where the number came from.
    ///
    /// Zero-based throughout this type, because that is what a `xdr:from`/`xdr:to` marker names: the
    /// row a file writes as `<row r="3">` is `row = 2` here. `SheetData::row` is keyed on the
    /// one-based `@r` and the conversion happens once, here.
    ///
    /// `None` when the sheet states neither a height for that row nor a default — see this module's
    /// own documentation.
    #[must_use]
    pub fn row_height(&self, row: u32) -> Option<(f64, GeometrySource)> {
        match self.stated_row_height(row) {
            Some(height) => Some((height, GeometrySource::Stated)),
            None if self.rows_hidden_by_default => Some((0.0, GeometrySource::SheetDefault)),
            None => Some((
                self.default_row_height? * EMU_PER_POINT,
                GeometrySource::SheetDefault,
            )),
        }
    }

    /// The height the sheet states for the zero-based `row`, in EMU, or `None` when it states none.
    ///
    /// A hidden row is `0`: `row@hidden` is layout, not decoration, and every object below a hidden
    /// row moves up by its height.
    fn stated_row_height(&self, row: u32) -> Option<f64> {
        let entry = self.part.sheet_data()?.row(row.checked_add(1)?)?;
        if entry.is_hidden() {
            return Some(0.0);
        }
        entry.height().map(|points| points * EMU_PER_POINT)
    }

    /// The distance from the sheet's top edge to the top of the zero-based `row`, in EMU.
    ///
    /// Linear in the rows that state their own `@ht` or `@hidden`, not in `row`: the offset is
    /// `row × default` plus the difference every stated row above it makes.
    #[must_use]
    pub fn row_offset(&self, row: i32) -> Option<(f64, GeometrySource)> {
        let row = u32::try_from(row).ok()?;
        if row == 0 {
            return Some((0.0, GeometrySource::Stated));
        }
        let default = match (self.default_row_height, self.rows_hidden_by_default) {
            // `zeroHeight` says a row that states nothing of its own is hidden, so its height is
            // zero whatever `defaultRowHeight` says.
            (_, true) => 0.0,
            (Some(points), false) => points * EMU_PER_POINT,
            // Nothing states a default. Every row above must state its own height, or there is no
            // answer at all — and proving that means visiting each of them.
            (None, false) => return self.summed_row_offset(row),
        };
        let mut total = f64::from(row) * default;
        let mut stated_rows = 0u32;
        for above in 0..row {
            if let Some(height) = self.stated_row_height(above) {
                total += height - default;
                stated_rows += 1;
            }
        }
        let source = if stated_rows == row {
            GeometrySource::Stated
        } else {
            GeometrySource::SheetDefault
        };
        Some((total, source))
    }

    /// [`row_offset`](Self::row_offset) for a sheet that states no default row height: every row
    /// above must state its own, and this answers `None` when one does not.
    fn summed_row_offset(&self, row: u32) -> Option<(f64, GeometrySource)> {
        let mut total = 0.0;
        for above in 0..row {
            total += self.stated_row_height(above)?;
        }
        Some((total, GeometrySource::Stated))
    }

    /// The width of the zero-based `column`, in EMU, and where the number came from.
    ///
    /// Never `None`: a column always has a width, because `sheetFormatPr@baseColWidth`'s own schema
    /// default of `8` is the end of the fallback chain. What varies is how far down that chain the
    /// answer came from, which is the second half of the return value.
    ///
    /// A hidden column is `0`, for the reason a hidden row is.
    #[must_use]
    pub fn column_width(&self, column: u16) -> (f64, GeometrySource) {
        if let Ok(Some(run)) = self.part.column_run_covering(column) {
            let interner = self.part.interner();
            if run.hidden(interner).unwrap_or(false) {
                return (0.0, GeometrySource::Stated);
            }
            if let Ok(Some(width)) = run.width(interner) {
                return (
                    self.metrics.characters_to_emu(width),
                    GeometrySource::Stated,
                );
            }
        }
        match self.default_column_width {
            Some(width) => (
                self.metrics.characters_to_emu(width),
                GeometrySource::SheetDefault,
            ),
            None => (
                self.metrics.characters_to_emu(
                    self.metrics
                        .base_column_width_characters(self.base_column_width),
                ),
                GeometrySource::BaseColumnWidth,
            ),
        }
    }

    /// The distance from the sheet's left edge to the left of the zero-based `column`, in EMU.
    ///
    /// Never `None`, for the reason [`column_width`](Self::column_width) never is.
    #[must_use]
    pub fn column_offset(&self, column: i32) -> (f64, GeometrySource) {
        let Ok(column) = u16::try_from(column.max(0)) else {
            // Past the grid's last column: an anchor may name one, and clamping is what the rest of
            // this module does with an index it cannot honour.
            return self.column_offset(i32::from(u16::MAX));
        };
        let mut total = 0.0;
        let mut source = GeometrySource::Stated;
        for index in 0..column {
            let (width, from) = self.column_width(index);
            total += width;
            source = source.max(from);
        }
        (total, source)
    }

    /// The absolute position of one `xdr:from`/`xdr:to` marker, in EMU.
    #[must_use]
    pub fn marker_position(
        &self,
        marker: CellMarker,
    ) -> Option<(Position, GeometrySource, GeometrySource)> {
        let (y, row_source) = self.row_offset(marker.row)?;
        let (x, column_source) = self.column_offset(marker.column);
        Some((
            Position::from_emu(
                round_emu(x + marker.column_offset.emu() as f64),
                round_emu(y + marker.row_offset.emu() as f64),
            ),
            row_source,
            column_source,
        ))
    }

    /// Resolves `anchor` to a rectangle on the sheet.
    ///
    /// * a **`xdr:twoCellAnchor`** takes its size from the two markers, so both are resolved and the
    ///   size is their difference;
    /// * a **`xdr:oneCellAnchor`** takes its position from `xdr:from` and its size from `xdr:ext`,
    ///   which is already EMU and needs no geometry at all;
    /// * a **`xdr:absoluteAnchor`** needs nothing from the sheet: its `xdr:pos` and `xdr:ext` are
    ///   both already absolute, and the answer is reported [`Stated`](GeometrySource::Stated) on
    ///   both axes because no default was consulted.
    ///
    /// `None` when the anchor does not state enough to be placed — a marker missing one of its four
    /// children, or a row whose height nothing in the sheet states.
    ///
    /// # `interner` is the **drawing part's**, not the sheet's
    ///
    /// An [`Anchor`] was parsed out of `xl/drawings/drawingN.xml` and every name in it is a symbol
    /// of *that* document's interner; this type borrows a [`WorksheetPart`], whose interner is a
    /// different one. Resolving a symbol in the wrong interner does not fail loudly — it resolves to
    /// whatever string happens to sit at that index — so the caller passes the one the anchor came
    /// from and there is no way to leave it implicit.
    #[must_use]
    pub fn resolve(
        &self,
        anchor: &Anchor,
        interner: &mjx_ooxml_core::Interner,
    ) -> Option<ResolvedAnchorBounds> {
        match anchor {
            Anchor::TwoCell(two) => {
                let from = two.from_marker(interner)?;
                let to = two.to_marker(interner)?;
                let (start, from_rows, from_columns) = self.marker_position(from)?;
                let (end, to_rows, to_columns) = self.marker_position(to)?;
                Some(ResolvedAnchorBounds {
                    position: start,
                    size: Size::from_emu(
                        (end.x.emu() - start.x.emu()).max(0),
                        (end.y.emu() - start.y.emu()).max(0),
                    ),
                    row_source: from_rows.max(to_rows),
                    column_source: from_columns.max(to_columns),
                    column_metrics: self.metrics,
                })
            }
            Anchor::OneCell(one) => {
                let from = one.from_marker(interner)?;
                let size = one.extent(interner)?;
                let (start, row_source, column_source) = self.marker_position(from)?;
                Some(ResolvedAnchorBounds {
                    position: start,
                    size,
                    row_source,
                    column_source,
                    column_metrics: self.metrics,
                })
            }
            Anchor::Absolute(absolute) => Some(ResolvedAnchorBounds {
                position: absolute.position(interner)?,
                size: absolute.extent(interner)?,
                // Nothing was defaulted because nothing was consulted: an absolute anchor states its
                // own rectangle in EMU and names no cell.
                row_source: GeometrySource::Stated,
                column_source: GeometrySource::Stated,
                column_metrics: self.metrics,
            }),
        }
    }

    /// The marker naming the cell that contains the point `emu` from the sheet's origin — the
    /// inverse of [`marker_position`](Self::marker_position).
    ///
    /// Walks the grid from the origin, so it is linear in the answer's own column index and in the
    /// stated rows above it. `None` when the sheet does not state enough to place the point, exactly
    /// as [`resolve`](Self::resolve) does; the search stops at the last row and column the grid has,
    /// so a point past the end of the sheet answers the last cell rather than running away.
    #[must_use]
    pub fn marker_at(
        &self,
        position: Position,
        last_row: u32,
        last_column: u16,
    ) -> Option<CellMarker> {
        let mut column = 0u16;
        let mut x = 0.0;
        while column < last_column {
            let (width, _) = self.column_width(column);
            if x + width > position.x.emu() as f64 {
                break;
            }
            x += width;
            column += 1;
        }
        let mut row = 0u32;
        let mut y = 0.0;
        while row < last_row {
            let (height, _) = self.row_height(row)?;
            if y + height > position.y.emu() as f64 {
                break;
            }
            y += height;
            row += 1;
        }
        Some(CellMarker::new(
            i32::from(column),
            round_emu(position.x.emu() as f64 - x),
            i32::try_from(row).ok()?,
            round_emu(position.y.emu() as f64 - y),
        ))
    }
}

/// A length in EMU, rounded to the nearest whole one — EMU are integers on the wire.
fn round_emu(value: f64) -> i64 {
    if !value.is_finite() {
        return 0;
    }
    let rounded = value.round();
    if rounded > i64::MAX as f64 {
        i64::MAX
    } else if rounded < i64::MIN as f64 {
        i64::MIN
    } else {
        rounded as i64
    }
}
