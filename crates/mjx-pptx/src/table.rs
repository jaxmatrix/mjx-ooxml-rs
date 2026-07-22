//! Addressing and formatting many table cells at once: [`Cells`] and [`CellFormat`].
//!
//! The per-cell, per-property setters say exactly one thing each, which is right when a caller means
//! exactly one thing and wrong the rest of the time — a navy header row with a rule under it is one
//! intention, not nine calls in a loop:
//!
//! ```no_run
//! # use mjx_pptx::{CellFormat, CellMargins, Cells, Presentation};
//! # use mjx_dml::{CellBorder, ColorSpec, Emu, FillSpec, LineSpec, TextAnchoring};
//! # fn f(deck: &mut Presentation, table: usize, rule: LineSpec) -> Result<(), mjx_pptx::PptxError> {
//! deck.format_cells(
//!     0,
//!     table,
//!     Cells::row(0),
//!     &CellFormat::new()
//!         .with_fill(FillSpec::Solid(ColorSpec::Srgb("1F3864".to_owned())))
//!         .with_border(CellBorder::Bottom, rule)
//!         .with_anchor(TextAnchoring::Center),
//! )?;
//! deck.format_cells(
//!     0,
//!     table,
//!     Cells::all(),
//!     &CellFormat::new().with_margins(CellMargins::uniform(Emu::from_points(6.0))),
//! )?;
//! # Ok(())
//! # }
//! ```
//!
//! Both halves are patterns this crate already uses: a **spec built with `with_`-prefixed setters**
//! (as `CharacterPropertiesSpec` and `LineSpec` are), applied to a **named scope** (as
//! `set_paragraph_run_properties` and `set_shape_run_properties` already mean "every run in this
//! much of the shape"). Nothing here is a table-specific dialect.
//!
//! A [`CellFormat`] writes only what it names, so formatting a region cannot disturb a property the
//! caller did not mention — the same rule the transform and cell-property writers follow.

use mjx_dml::{
    CellBorder, ColorSpec, FillSpec, LineSpec, OnOffStyle, TablePartStyle, TableStyleBorder,
    TableStyleCellStyle, TableStylePart, TableStyleTextStyle, TextAnchoring, TextDirection,
    TextHorizontalOverflow,
};
use mjx_ooxml_core::Interner;

use crate::geometry::CellMargins;

/// Which cells of a table an operation is about.
///
/// Every position within the table is addressable, merged ones included — merging covers a cell, it
/// never removes it — so a selection never has holes and a rectangle always means what it says.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Cells {
    /// One cell, at `(row, column)`.
    One {
        /// The cell's row.
        row: usize,
        /// The cell's column.
        column: usize,
    },
    /// Every cell of one row — a header row, a total row.
    Row(usize),
    /// Every cell of one column.
    Column(usize),
    /// A rectangular block, `rows` down by `columns` across (both half-open).
    Rectangle {
        /// The rows covered.
        rows: core::ops::Range<usize>,
        /// The columns covered.
        columns: core::ops::Range<usize>,
    },
    /// Every cell in the table.
    All,
}

impl Cells {
    /// One cell.
    #[must_use]
    pub fn one(row: usize, column: usize) -> Self {
        Self::One { row, column }
    }

    /// Every cell of one row.
    #[must_use]
    pub fn row(row: usize) -> Self {
        Self::Row(row)
    }

    /// Every cell of one column.
    #[must_use]
    pub fn column(column: usize) -> Self {
        Self::Column(column)
    }

    /// A rectangular block.
    #[must_use]
    pub fn rectangle(rows: core::ops::Range<usize>, columns: core::ops::Range<usize>) -> Self {
        Self::Rectangle { rows, columns }
    }

    /// Every cell in the table.
    #[must_use]
    pub fn all() -> Self {
        Self::All
    }

    /// The rectangle this selection covers in a `rows` x `columns` table, or the first position
    /// that falls outside it.
    ///
    /// Every selection is a rectangle — which is why a `Cells` can also describe a merge, the only
    /// shape a merged region can take. An empty range covers nothing and is not an error.
    pub(crate) fn bounds(
        &self,
        rows: usize,
        columns: usize,
    ) -> Result<(core::ops::Range<usize>, core::ops::Range<usize>), (usize, usize)> {
        let (row_range, column_range) = match self {
            Self::One { row, column } => (*row..row + 1, *column..column + 1),
            Self::Row(row) => (*row..row + 1, 0..columns),
            Self::Column(column) => (0..rows, *column..column + 1),
            Self::Rectangle { rows, columns } => (rows.clone(), columns.clone()),
            Self::All => (0..rows, 0..columns),
        };

        // An empty range covers nothing, so it cannot be out of range; a non-empty one must fit.
        if !row_range.is_empty() && row_range.end > rows {
            return Err((row_range.end - 1, column_range.start));
        }
        if !column_range.is_empty() && column_range.end > columns {
            return Err((row_range.start, column_range.end - 1));
        }
        Ok((row_range, column_range))
    }

    /// The positions this selection covers in a `rows` x `columns` table, in row-major order, or
    /// the first position that falls outside it.
    ///
    /// An empty range selects nothing and is not an error — `Cells::rectangle(0..0, 0..0)` on any
    /// table is a well-formed way to select nothing at all.
    pub(crate) fn resolve(
        &self,
        rows: usize,
        columns: usize,
    ) -> Result<Vec<(usize, usize)>, (usize, usize)> {
        let (row_range, column_range) = self.bounds(rows, columns)?;
        let mut positions = Vec::new();
        for row in row_range {
            for column in column_range.clone() {
                positions.push((row, column));
            }
        }
        Ok(positions)
    }
}

/// How a cell should be drawn — fill, borders, insets, and how its text is framed.
///
/// A builder: every `with_` method names one property, and a property the format does **not** name
/// is left exactly as the cell had it. So a format that only sets a fill can be applied to a region
/// whose cells have different borders without flattening them.
///
/// The `without_` methods are the other half of that: they state *remove this*, which is not the
/// same as leaving it alone, and not the same as setting an explicit "none". Removing a fill lets
/// the table style decide again; [`FillSpec::None`] says the cell is deliberately unfilled.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CellFormat {
    /// The fill to write, or `Some(None)` to remove whatever fill the cell has.
    fill: Option<Option<FillSpec>>,
    /// Borders to write or remove, in the order they were named.
    borders: Vec<(CellBorder, Option<LineSpec>)>,
    /// The insets to write; each field of it is itself optional.
    margins: CellMargins,
    anchor: Option<TextAnchoring>,
    text_direction: Option<TextDirection>,
    horizontal_overflow: Option<TextHorizontalOverflow>,
}

impl CellFormat {
    /// A format that names nothing, and so changes nothing.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Fills the cell.
    #[must_use]
    pub fn with_fill(mut self, fill: FillSpec) -> Self {
        self.fill = Some(Some(fill));
        self
    }

    /// Removes the cell's own fill, so the table style decides how it is filled.
    #[must_use]
    pub fn without_fill(mut self) -> Self {
        self.fill = Some(None);
        self
    }

    /// Draws a border on one edge.
    #[must_use]
    pub fn with_border(mut self, edge: CellBorder, line: LineSpec) -> Self {
        self.borders.push((edge, Some(line)));
        self
    }

    /// Draws the same border on all four outer edges — what a caller means by "box these cells".
    ///
    /// Note this outlines **each cell** in the selection, not the selection's perimeter.
    #[must_use]
    pub fn with_outline(mut self, line: LineSpec) -> Self {
        for edge in [
            CellBorder::Left,
            CellBorder::Right,
            CellBorder::Top,
            CellBorder::Bottom,
        ] {
            self.borders.push((edge, Some(line.clone())));
        }
        self
    }

    /// Removes the border on one edge.
    #[must_use]
    pub fn without_border(mut self, edge: CellBorder) -> Self {
        self.borders.push((edge, None));
        self
    }

    /// Removes the border on all six edges, diagonals included.
    #[must_use]
    pub fn without_borders(mut self) -> Self {
        for edge in CellBorder::all() {
            self.borders.push((edge, None));
        }
        self
    }

    /// Sets the insets between the cell's edges and its text. A field of `margins` left `None` is
    /// still left alone, so this can name one inset.
    #[must_use]
    pub fn with_margins(mut self, margins: CellMargins) -> Self {
        self.margins = margins;
        self
    }

    /// Sets where the text sits vertically in the cell.
    #[must_use]
    pub fn with_anchor(mut self, anchor: TextAnchoring) -> Self {
        self.anchor = Some(anchor);
        self
    }

    /// Sets which way the text flows — how a rotated header is made.
    #[must_use]
    pub fn with_text_direction(mut self, direction: TextDirection) -> Self {
        self.text_direction = Some(direction);
        self
    }

    /// Sets what a character too wide for the cell does.
    #[must_use]
    pub fn with_horizontal_overflow(mut self, overflow: TextHorizontalOverflow) -> Self {
        self.horizontal_overflow = Some(overflow);
        self
    }

    /// Whether this format names nothing — in which case applying it is a no-op, and no `a:tcPr`
    /// is created for a cell that had none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fill.is_none()
            && self.borders.is_empty()
            && self.margins == CellMargins::default()
            && self.anchor.is_none()
            && self.text_direction.is_none()
            && self.horizontal_overflow.is_none()
    }

    /// The fill this format names, if any.
    pub(crate) fn fill(&self) -> Option<Option<&FillSpec>> {
        self.fill.as_ref().map(Option::as_ref)
    }

    /// The borders this format names, in the order they were named.
    pub(crate) fn borders(&self) -> &[(CellBorder, Option<LineSpec>)] {
        &self.borders
    }

    /// The insets this format names.
    pub(crate) fn margins(&self) -> CellMargins {
        self.margins
    }

    /// The text-framing attributes this format names.
    pub(crate) fn framing(
        &self,
    ) -> (
        Option<TextAnchoring>,
        Option<TextDirection>,
        Option<TextHorizontalOverflow>,
    ) {
        (self.anchor, self.text_direction, self.horizontal_overflow)
    }
}

/// The formatting a table **style** gives one part of a table (`wholeTbl`, `firstRow`, a banded row,
/// a corner cell) — a fill, text emphasis (bold / italic as the tri-state, plus a colour), and
/// borders.
///
/// This is the style-level counterpart of [`CellFormat`]: `CellFormat` overrides one cell directly,
/// while a `TableStyleFormat` is applied to a *named part* of a table style with
/// [`format_table_style_part`](crate::Presentation::format_table_style_part), so every cell that part
/// covers picks it up. Only the properties you set are written; a part keeps whatever else it held.
#[derive(Debug, Clone, Default)]
pub struct TableStyleFormat {
    fill: Option<FillSpec>,
    bold: Option<OnOffStyle>,
    italic: Option<OnOffStyle>,
    text_color: Option<ColorSpec>,
    borders: Vec<(TableStyleBorder, LineSpec)>,
}

impl TableStyleFormat {
    /// An empty format that changes nothing.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Fills the part's cells.
    #[must_use]
    pub fn with_fill(mut self, fill: FillSpec) -> Self {
        self.fill = Some(fill);
        self
    }

    /// Takes bold on, off, or back to the default (follow the parent) for the part's text.
    #[must_use]
    pub fn with_bold(mut self, bold: OnOffStyle) -> Self {
        self.bold = Some(bold);
        self
    }

    /// Takes italic on, off, or back to the default for the part's text.
    #[must_use]
    pub fn with_italic(mut self, italic: OnOffStyle) -> Self {
        self.italic = Some(italic);
        self
    }

    /// Colours the part's text.
    #[must_use]
    pub fn with_text_color(mut self, color: ColorSpec) -> Self {
        self.text_color = Some(color);
        self
    }

    /// Draws `line` on one border `edge` of the part's cells. Repeated edges take the last line.
    #[must_use]
    pub fn with_border(mut self, edge: TableStyleBorder, line: LineSpec) -> Self {
        self.borders.retain(|(existing, _)| *existing != edge);
        self.borders.push((edge, line));
        self
    }

    /// Merges this format into `part`, creating the text and cell styles only for the facets set —
    /// so a format that touches only the fill leaves the part's text style untouched.
    pub(crate) fn apply(&self, part: &mut TablePartStyle, interner: &mut Interner) {
        if self.bold.is_some() || self.italic.is_some() || self.text_color.is_some() {
            let mut text = part
                .text_style(interner)
                .unwrap_or_else(|| TableStyleTextStyle::new(interner));
            if let Some(bold) = self.bold {
                text.set_bold(interner, bold);
            }
            if let Some(italic) = self.italic {
                text.set_italic(interner, italic);
            }
            if let Some(color) = &self.text_color {
                text.set_color(interner, color);
            }
            part.set_text_style(interner, &text);
        }
        if self.fill.is_some() || !self.borders.is_empty() {
            let mut cell = part
                .cell_style(interner)
                .unwrap_or_else(|| TableStyleCellStyle::new(interner));
            if let Some(fill) = &self.fill {
                cell.set_fill(interner, fill);
            }
            for (edge, line) in &self.borders {
                cell.set_border(interner, *edge, line);
            }
            part.set_cell_style(interner, &cell);
        }
    }
}

/// A default `styleId` for an inline style. It is schema-required (`ST_Guid`) but, being inline,
/// never referenced by anything, so a fixed placeholder is fine — a caller who cares overrides it.
pub(crate) const DEFAULT_INLINE_STYLE_ID: &str = "{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}";

/// A default gallery name for an inline style.
pub(crate) const DEFAULT_INLINE_STYLE_NAME: &str = "Custom Table Style";

/// A whole table style, described up front — the lean way to give a table its own look, written
/// **inline** on the table (`a:tableStyle`) rather than into the shared `tableStyles.xml` part.
///
/// Each [`with_part`](Self::with_part) names a [part of the table](TableStylePart) and the
/// [`TableStyleFormat`] it takes, so an agent can spell out a whole style — a bold, filled header
/// row; a banded body; ruled cells — in one expression and apply it with
/// [`set_inline_table_style`](crate::Presentation::set_inline_table_style).
///
/// A part only *renders* when the table declares it (`a:tblPr` flags): style
/// [`FirstRow`](TableStylePart::FirstRow) **and** turn the header row on with
/// [`set_table_part`](crate::Presentation::set_table_part) (a table from
/// [`add_table`](crate::Presentation::add_table) already has `firstRow` and `bandRow` on). This keeps
/// "which parts exist" and "how they look" apart.
#[derive(Debug, Clone, Default)]
pub struct TableStyleDefinition {
    style_id: Option<String>,
    style_name: Option<String>,
    parts: Vec<(TableStylePart, TableStyleFormat)>,
}

impl TableStyleDefinition {
    /// An empty definition — a style that formats nothing.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the style's GUID (`@styleId`). Optional: an inline style's id is never referenced, so it
    /// defaults to a placeholder.
    #[must_use]
    pub fn with_id(mut self, style_id: &str) -> Self {
        self.style_id = Some(style_id.to_owned());
        self
    }

    /// Sets the style's gallery name (`@styleName`). Optional; defaults to a generic name.
    #[must_use]
    pub fn with_name(mut self, style_name: &str) -> Self {
        self.style_name = Some(style_name.to_owned());
        self
    }

    /// Gives `part` the formatting in `format`, replacing any it had named already.
    #[must_use]
    pub fn with_part(mut self, part: TableStylePart, format: TableStyleFormat) -> Self {
        self.parts.retain(|(existing, _)| *existing != part);
        self.parts.push((part, format));
        self
    }

    /// The style id, or the default placeholder.
    pub(crate) fn style_id(&self) -> &str {
        self.style_id.as_deref().unwrap_or(DEFAULT_INLINE_STYLE_ID)
    }

    /// The style name, or the default.
    pub(crate) fn style_name(&self) -> &str {
        self.style_name
            .as_deref()
            .unwrap_or(DEFAULT_INLINE_STYLE_NAME)
    }

    /// The parts and their formats, in the order named.
    pub(crate) fn parts(&self) -> &[(TableStylePart, TableStyleFormat)] {
        &self.parts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_selection_covers_the_positions_it_names() {
        assert_eq!(Cells::one(1, 2).resolve(3, 3), Ok(vec![(1, 2)]));
        assert_eq!(
            Cells::row(0).resolve(2, 3),
            Ok(vec![(0, 0), (0, 1), (0, 2)])
        );
        assert_eq!(
            Cells::column(1).resolve(3, 2),
            Ok(vec![(0, 1), (1, 1), (2, 1)])
        );
        assert_eq!(
            Cells::all().resolve(2, 2),
            Ok(vec![(0, 0), (0, 1), (1, 0), (1, 1)])
        );
        assert_eq!(
            Cells::rectangle(1..3, 0..1).resolve(3, 3),
            Ok(vec![(1, 0), (2, 0)])
        );
    }

    #[test]
    fn a_selection_past_an_edge_names_the_offending_position() {
        assert_eq!(Cells::row(5).resolve(2, 2), Err((5, 0)));
        assert_eq!(Cells::column(9).resolve(2, 2), Err((0, 9)));
        assert_eq!(Cells::one(0, 4).resolve(2, 2), Err((0, 4)));
    }

    #[test]
    fn an_empty_range_selects_nothing_rather_than_failing() {
        // Selecting no cells is a well-formed thing to ask for, whatever the table's size.
        assert_eq!(Cells::rectangle(0..0, 0..0).resolve(0, 0), Ok(vec![]));
        assert_eq!(Cells::rectangle(2..2, 0..2).resolve(2, 2), Ok(vec![]));
        assert_eq!(Cells::all().resolve(0, 0), Ok(vec![]));
    }

    #[test]
    fn a_format_that_names_nothing_changes_nothing() {
        assert!(CellFormat::new().is_empty());
        assert!(!CellFormat::new()
            .with_anchor(TextAnchoring::Center)
            .is_empty());
        assert!(!CellFormat::new().without_fill().is_empty());
    }

    #[test]
    fn an_outline_is_the_four_outer_edges_not_the_diagonals() {
        let format = CellFormat::new().with_outline(LineSpec::default());
        let edges: Vec<CellBorder> = format.borders().iter().map(|(edge, _)| *edge).collect();
        assert_eq!(
            edges,
            [
                CellBorder::Left,
                CellBorder::Right,
                CellBorder::Top,
                CellBorder::Bottom
            ]
        );
    }
}
