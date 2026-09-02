//! Table selections, cell formatting, and table styles.
//!
//! [`Cells`] is the selection — one cell, a row, a column, a rectangle, or all of them — and
//! [`CellFormat`] is the change. `deck.format_cells(0, table, Cells.row(0), header)` is one call
//! for a whole header row, and it is the reason the per-cell setters are not the only way in.

use pyo3::prelude::*;
use pyo3::types::PyModule;

use mjx_ooxml as ooxml;

use crate::enums::{
    CellBorder, OnOffStyle, PresetMaterial, TableStyleBorder, TableStylePart, TextAnchoring,
    TextDirection, TextHorizontalOverflow,
};
use crate::geometry::CellMargins;
use crate::paint::{ColorSpec, FillSpec, LineSpec};
use crate::support::RangeArg;
use crate::three_d::{Bevel, LightRig};

value_class! {
    /// Which cells of a table a call is about.
    Cells(ooxml::Cells), derive(PartialEq, Eq);

    /// A change to apply to a selection of cells: fill, borders, margins, anchoring, and the 3-D
    /// properties a cell can carry.
    CellFormat(ooxml::CellFormat), derive(PartialEq);

    /// The formatting one part of a table style states.
    TableStyleFormat(ooxml::TableStyleFormat), derive(PartialEq);

    /// A whole table style: an identifier, a name, and formatting for each of its thirteen parts.
    TableStyleDefinition(ooxml::TableStyleDefinition), derive(PartialEq);

    /// Which of a table style's six banding and heading parts a table has turned on.
    TableStyleFlags(ooxml::TableStyleFlags), derive(Copy, PartialEq, Eq);
}

#[pymethods]
impl Cells {
    /// One cell.
    #[staticmethod]
    fn one(row: u32, column: u32) -> Self {
        Self(ooxml::Cells::one(row as usize, column as usize))
    }

    /// Every cell of one row.
    #[staticmethod]
    fn row(row: u32) -> Self {
        Self(ooxml::Cells::row(row as usize))
    }

    /// Every cell of one column.
    #[staticmethod]
    fn column(column: u32) -> Self {
        Self(ooxml::Cells::column(column as usize))
    }

    /// A rectangular block, given as two `range`s: `Cells.rectangle(range(0, 2), range(1, 4))`.
    #[staticmethod]
    fn rectangle(rows: RangeArg, columns: RangeArg) -> Self {
        Self(ooxml::Cells::Rectangle {
            rows: rows.0.start as usize..rows.0.end as usize,
            columns: columns.0.start as usize..columns.0.end as usize,
        })
    }

    /// Every cell of the table.
    #[staticmethod]
    fn all() -> Self {
        Self(ooxml::Cells::all())
    }

    /// Which kind of selection this is: `"one"`, `"row"`, `"column"`, `"rectangle"` or `"all"`.
    #[getter]
    fn kind(&self) -> &'static str {
        match &self.0 {
            ooxml::Cells::One { .. } => "one",
            ooxml::Cells::Row(_) => "row",
            ooxml::Cells::Column(_) => "column",
            ooxml::Cells::Rectangle { .. } => "rectangle",
            ooxml::Cells::All => "all",
            // `Cells` is `#[non_exhaustive]`: a selection this build does not name is still a
            // selection, and reporting it as unknown beats guessing which one it is.
            _ => "unknown",
        }
    }

    /// The rows this selection covers, as a `range`, when it names any.
    #[getter]
    fn rows(&self) -> Option<(u32, u32)> {
        match &self.0 {
            ooxml::Cells::One { row, .. } => Some((*row as u32, *row as u32 + 1)),
            ooxml::Cells::Row(row) => Some((*row as u32, *row as u32 + 1)),
            ooxml::Cells::Rectangle { rows, .. } => Some((rows.start as u32, rows.end as u32)),
            _ => None,
        }
    }

    /// The columns this selection covers, as a `range`, when it names any.
    #[getter]
    fn columns(&self) -> Option<(u32, u32)> {
        match &self.0 {
            ooxml::Cells::One { column, .. } => Some((*column as u32, *column as u32 + 1)),
            ooxml::Cells::Column(column) => Some((*column as u32, *column as u32 + 1)),
            ooxml::Cells::Rectangle { columns, .. } => {
                Some((columns.start as u32, columns.end as u32))
            }
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl CellFormat {
    /// A change that changes nothing. Add to it with the `with_…` methods.
    #[new]
    fn new() -> Self {
        Self(ooxml::CellFormat::new())
    }

    /// This change, also setting the cells' fill.
    fn with_fill(&self, fill: FillSpec) -> Self {
        Self(self.0.clone().with_fill(fill.0))
    }

    /// This change, also clearing the cells' fill so they inherit the table style's.
    fn without_fill(&self) -> Self {
        Self(self.0.clone().without_fill())
    }

    /// This change, also setting one edge's border.
    fn with_border(&self, edge: CellBorder, line: LineSpec) -> Self {
        Self(self.0.clone().with_border(edge.into(), line.0))
    }

    /// This change, also setting all four outer edges to the same line.
    fn with_outline(&self, line: LineSpec) -> Self {
        Self(self.0.clone().with_outline(line.0))
    }

    /// This change, also clearing one edge's border.
    fn without_border(&self, edge: CellBorder) -> Self {
        Self(self.0.clone().without_border(edge.into()))
    }

    /// This change, also clearing every border.
    fn without_borders(&self) -> Self {
        Self(self.0.clone().without_borders())
    }

    /// This change, also setting the cells' inner margins.
    fn with_margins(&self, margins: CellMargins) -> Self {
        Self(self.0.clone().with_margins(margins.0))
    }

    /// This change, also setting how text sits vertically in the cells.
    fn with_anchor(&self, anchor: TextAnchoring) -> Self {
        Self(self.0.clone().with_anchor(anchor.into()))
    }

    /// This change, also setting the cells' text direction.
    fn with_text_direction(&self, direction: TextDirection) -> Self {
        Self(self.0.clone().with_text_direction(direction.into()))
    }

    /// This change, also setting whether text that does not fit is clipped.
    fn with_horizontal_overflow(&self, overflow: TextHorizontalOverflow) -> Self {
        Self(self.0.clone().with_horizontal_overflow(overflow.into()))
    }

    /// This change, also setting the cells' 3-D surface material.
    fn with_cell_material(&self, material: PresetMaterial) -> Self {
        Self(self.0.clone().with_cell_material(material.into()))
    }

    /// This change, also setting the cells' bevel.
    fn with_cell_bevel(&self, bevel: Bevel) -> Self {
        Self(self.0.clone().with_cell_bevel(bevel.0))
    }

    /// This change, also setting the cells' light rig.
    fn with_cell_light_rig(&self, light_rig: LightRig) -> Self {
        Self(self.0.clone().with_cell_light_rig(light_rig.0))
    }

    /// Whether this change would change nothing.
    #[getter]
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl TableStyleFormat {
    /// Formatting that states nothing. Add to it with the `with_…` methods.
    #[new]
    fn new() -> Self {
        Self(ooxml::TableStyleFormat::new())
    }

    /// This formatting with the given cell fill.
    fn with_fill(&self, fill: FillSpec) -> Self {
        Self(self.0.clone().with_fill(fill.0))
    }

    /// This formatting with the given boldness. A table style's on/off values are three-valued —
    /// on, off, or "whatever the default is" — which is why this takes an [`OnOffStyle`].
    fn with_bold(&self, bold: OnOffStyle) -> Self {
        Self(self.0.clone().with_bold(bold.into()))
    }

    /// This formatting with the given italicisation.
    fn with_italic(&self, italic: OnOffStyle) -> Self {
        Self(self.0.clone().with_italic(italic.into()))
    }

    /// This formatting with the given text colour.
    fn with_text_color(&self, color: ColorSpec) -> Self {
        Self(self.0.clone().with_text_color(color.0))
    }

    /// This formatting with the given border on one edge. A table style has eight edges, including
    /// the two *inside* ones a single cell does not have.
    fn with_border(&self, edge: TableStyleBorder, line: LineSpec) -> Self {
        Self(self.0.clone().with_border(edge.into(), line.0))
    }

    /// This formatting with the given 3-D surface material.
    fn with_cell_material(&self, material: PresetMaterial) -> Self {
        Self(self.0.clone().with_cell_material(material.into()))
    }

    /// This formatting with the given bevel.
    fn with_cell_bevel(&self, bevel: Bevel) -> Self {
        Self(self.0.clone().with_cell_bevel(bevel.0))
    }

    /// This formatting with the given light rig.
    fn with_cell_light_rig(&self, light_rig: LightRig) -> Self {
        Self(self.0.clone().with_cell_light_rig(light_rig.0))
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl TableStyleDefinition {
    /// A style that states nothing. Add to it with the `with_…` methods.
    #[new]
    fn new() -> Self {
        Self(ooxml::TableStyleDefinition::new())
    }

    /// This style with the given identifier — a GUID in braces, as `tableStyles.xml` writes them.
    fn with_id(&self, style_id: &str) -> Self {
        Self(self.0.clone().with_id(style_id))
    }

    /// This style with the given display name.
    fn with_name(&self, style_name: &str) -> Self {
        Self(self.0.clone().with_name(style_name))
    }

    /// This style with formatting for one of its thirteen parts.
    fn with_part(&self, part: TableStylePart, format: TableStyleFormat) -> Self {
        Self(self.0.clone().with_part(part.into(), format.0))
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl TableStyleFlags {
    /// Which banding and heading parts a table has turned on.
    #[new]
    #[pyo3(signature = (
        first_row = false,
        last_row = false,
        first_column = false,
        last_column = false,
        banded_rows = false,
        banded_columns = false,
    ))]
    fn new(
        first_row: bool,
        last_row: bool,
        first_column: bool,
        last_column: bool,
        banded_rows: bool,
        banded_columns: bool,
    ) -> Self {
        Self(ooxml::TableStyleFlags {
            first_row,
            last_row,
            first_column,
            last_column,
            banded_rows,
            banded_columns,
        })
    }

    /// Whether the first row is formatted as a header.
    #[getter]
    fn first_row(&self) -> bool {
        self.0.first_row
    }

    /// Whether the last row is formatted as a total.
    #[getter]
    fn last_row(&self) -> bool {
        self.0.last_row
    }

    /// Whether the first column is formatted as a header.
    #[getter]
    fn first_column(&self) -> bool {
        self.0.first_column
    }

    /// Whether the last column is formatted as a total.
    #[getter]
    fn last_column(&self) -> bool {
        self.0.last_column
    }

    /// Whether rows alternate between the two banding parts.
    #[getter]
    fn banded_rows(&self) -> bool {
        self.0.banded_rows
    }

    /// Whether columns alternate between the two banding parts.
    #[getter]
    fn banded_columns(&self) -> bool {
        self.0.banded_columns
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

/// Adds every class in this module to the extension module.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<Cells>()?;
    module.add_class::<CellFormat>()?;
    module.add_class::<TableStyleFormat>()?;
    module.add_class::<TableStyleDefinition>()?;
    module.add_class::<TableStyleFlags>()
}
