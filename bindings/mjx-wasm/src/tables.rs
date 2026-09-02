//! Table selections, cell formatting, and table styles.
//!
//! [`Cells`] is the selection — one cell, a row, a column, a rectangle, or all of them — and
//! [`CellFormat`] is the change. `deck.formatCells(0, table, Cells.row(0), header)` is one call
//! for a whole header row, and it is the reason the per-cell setters are not the only way in.

use wasm_bindgen::prelude::*;

use mjx_ooxml as ooxml;

use crate::enums::{
    CellBorder, OnOffStyle, PresetMaterial, TableStyleBorder, TableStylePart, TextAnchoring,
    TextDirection, TextHorizontalOverflow,
};
use crate::geometry::CellMargins;
use crate::paint::{ColorSpec, FillSpec, LineSpec};
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

#[wasm_bindgen]
impl Cells {
    /// One cell.
    pub fn one(row: u32, column: u32) -> Self {
        Self(ooxml::Cells::one(row as usize, column as usize))
    }

    /// Every cell of one row.
    pub fn row(row: u32) -> Self {
        Self(ooxml::Cells::row(row as usize))
    }

    /// Every cell of one column.
    pub fn column(column: u32) -> Self {
        Self(ooxml::Cells::column(column as usize))
    }

    /// A rectangular block, given as two half-open ranges:
    /// `Cells.rectangle(0, 2, 1, 4)` is rows 0–1 and columns 1–3.
    ///
    /// Four numbers rather than two ranges, because JavaScript has no half-open range and an object
    /// literal would type as `any` in the `.d.ts`.
    #[wasm_bindgen(js_name = "rectangle")]
    pub fn rectangle(row_start: u32, row_end: u32, column_start: u32, column_end: u32) -> Cells {
        Self(ooxml::Cells::Rectangle {
            rows: row_start as usize..row_end as usize,
            columns: column_start as usize..column_end as usize,
        })
    }

    /// Every cell of the table.
    pub fn all() -> Self {
        Self(ooxml::Cells::all())
    }

    /// Which kind of selection this is: `"one"`, `"row"`, `"column"`, `"rectangle"` or `"all"`.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> String {
        match &self.0 {
            ooxml::Cells::One { .. } => "one".to_owned(),
            ooxml::Cells::Row(_) => "row".to_owned(),
            ooxml::Cells::Column(_) => "column".to_owned(),
            ooxml::Cells::Rectangle { .. } => "rectangle".to_owned(),
            ooxml::Cells::All => "all".to_owned(),
            // `Cells` is `#[non_exhaustive]`: a selection this build does not name is still a
            // selection, and reporting it as unknown beats guessing which one it is.
            _ => "unknown".to_owned(),
        }
    }

    /// The first row this selection covers, or `undefined` when it names no rows.
    #[wasm_bindgen(getter, js_name = "rowStart")]
    pub fn row_start(&self) -> Option<u32> {
        self.row_bounds().map(|(start, _)| start)
    }

    /// One past the last row this selection covers, or `undefined` when it names no rows.
    #[wasm_bindgen(getter, js_name = "rowEnd")]
    pub fn row_end(&self) -> Option<u32> {
        self.row_bounds().map(|(_, end)| end)
    }

    /// The first column this selection covers, or `undefined` when it names no columns.
    #[wasm_bindgen(getter, js_name = "columnStart")]
    pub fn column_start(&self) -> Option<u32> {
        self.column_bounds().map(|(start, _)| start)
    }

    /// One past the last column this selection covers, or `undefined` when it names no columns.
    #[wasm_bindgen(getter, js_name = "columnEnd")]
    pub fn column_end(&self) -> Option<u32> {
        self.column_bounds().map(|(_, end)| end)
    }
}

impl Cells {
    /// The half-open row range this selection covers, when it names any.
    fn row_bounds(&self) -> Option<(u32, u32)> {
        match &self.0 {
            ooxml::Cells::One { row, .. } | ooxml::Cells::Row(row) => {
                Some((*row as u32, *row as u32 + 1))
            }
            ooxml::Cells::Rectangle { rows, .. } => Some((rows.start as u32, rows.end as u32)),
            _ => None,
        }
    }

    /// The half-open column range this selection covers, when it names any.
    fn column_bounds(&self) -> Option<(u32, u32)> {
        match &self.0 {
            ooxml::Cells::One { column, .. } | ooxml::Cells::Column(column) => {
                Some((*column as u32, *column as u32 + 1))
            }
            ooxml::Cells::Rectangle { columns, .. } => {
                Some((columns.start as u32, columns.end as u32))
            }
            _ => None,
        }
    }
}

#[wasm_bindgen]
impl CellFormat {
    /// A change that changes nothing. Add to it with the `with_…` methods.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self(ooxml::CellFormat::new())
    }

    /// This change, also setting the cells' fill.
    #[wasm_bindgen(js_name = "withFill")]
    pub fn with_fill(&self, fill: &FillSpec) -> Self {
        Self(self.0.clone().with_fill(fill.0.clone()))
    }

    /// This change, also clearing the cells' fill so they inherit the table style's.
    #[wasm_bindgen(js_name = "withoutFill")]
    pub fn without_fill(&self) -> Self {
        Self(self.0.clone().without_fill())
    }

    /// This change, also setting one edge's border.
    #[wasm_bindgen(js_name = "withBorder")]
    pub fn with_border(&self, edge: CellBorder, line: &LineSpec) -> Self {
        Self(self.0.clone().with_border(edge.into(), line.0.clone()))
    }

    /// This change, also setting all four outer edges to the same line.
    #[wasm_bindgen(js_name = "withOutline")]
    pub fn with_outline(&self, line: &LineSpec) -> Self {
        Self(self.0.clone().with_outline(line.0.clone()))
    }

    /// This change, also clearing one edge's border.
    #[wasm_bindgen(js_name = "withoutBorder")]
    pub fn without_border(&self, edge: CellBorder) -> Self {
        Self(self.0.clone().without_border(edge.into()))
    }

    /// This change, also clearing every border.
    #[wasm_bindgen(js_name = "withoutBorders")]
    pub fn without_borders(&self) -> Self {
        Self(self.0.clone().without_borders())
    }

    /// This change, also setting the cells' inner margins.
    #[wasm_bindgen(js_name = "withMargins")]
    pub fn with_margins(&self, margins: &CellMargins) -> Self {
        Self(self.0.clone().with_margins(margins.0))
    }

    /// This change, also setting how text sits vertically in the cells.
    #[wasm_bindgen(js_name = "withAnchor")]
    pub fn with_anchor(&self, anchor: TextAnchoring) -> Self {
        Self(self.0.clone().with_anchor(anchor.into()))
    }

    /// This change, also setting the cells' text direction.
    #[wasm_bindgen(js_name = "withTextDirection")]
    pub fn with_text_direction(&self, direction: TextDirection) -> Self {
        Self(self.0.clone().with_text_direction(direction.into()))
    }

    /// This change, also setting whether text that does not fit is clipped.
    #[wasm_bindgen(js_name = "withHorizontalOverflow")]
    pub fn with_horizontal_overflow(&self, overflow: TextHorizontalOverflow) -> Self {
        Self(self.0.clone().with_horizontal_overflow(overflow.into()))
    }

    /// This change, also setting the cells' 3-D surface material.
    #[wasm_bindgen(js_name = "withCellMaterial")]
    pub fn with_cell_material(&self, material: PresetMaterial) -> Self {
        Self(self.0.clone().with_cell_material(material.into()))
    }

    /// This change, also setting the cells' bevel.
    #[wasm_bindgen(js_name = "withCellBevel")]
    pub fn with_cell_bevel(&self, bevel: &Bevel) -> Self {
        Self(self.0.clone().with_cell_bevel(bevel.0))
    }

    /// This change, also setting the cells' light rig.
    #[wasm_bindgen(js_name = "withCellLightRig")]
    pub fn with_cell_light_rig(&self, light_rig: &LightRig) -> Self {
        Self(self.0.clone().with_cell_light_rig(light_rig.0))
    }

    /// Whether this change would change nothing.
    #[wasm_bindgen(getter, js_name = "isEmpty")]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[wasm_bindgen]
impl TableStyleFormat {
    /// Formatting that states nothing. Add to it with the `with_…` methods.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self(ooxml::TableStyleFormat::new())
    }

    /// This formatting with the given cell fill.
    #[wasm_bindgen(js_name = "withFill")]
    pub fn with_fill(&self, fill: &FillSpec) -> Self {
        Self(self.0.clone().with_fill(fill.0.clone()))
    }

    /// This formatting with the given boldness. A table style's on/off values are three-valued —
    /// on, off, or "whatever the default is" — which is why this takes an [`OnOffStyle`].
    #[wasm_bindgen(js_name = "withBold")]
    pub fn with_bold(&self, bold: OnOffStyle) -> Self {
        Self(self.0.clone().with_bold(bold.into()))
    }

    /// This formatting with the given italicisation.
    #[wasm_bindgen(js_name = "withItalic")]
    pub fn with_italic(&self, italic: OnOffStyle) -> Self {
        Self(self.0.clone().with_italic(italic.into()))
    }

    /// This formatting with the given text colour.
    #[wasm_bindgen(js_name = "withTextColor")]
    pub fn with_text_color(&self, color: &ColorSpec) -> Self {
        Self(self.0.clone().with_text_color(color.0.clone()))
    }

    /// This formatting with the given border on one edge. A table style has eight edges, including
    /// the two *inside* ones a single cell does not have.
    #[wasm_bindgen(js_name = "withBorder")]
    pub fn with_border(&self, edge: TableStyleBorder, line: &LineSpec) -> Self {
        Self(self.0.clone().with_border(edge.into(), line.0.clone()))
    }

    /// This formatting with the given 3-D surface material.
    #[wasm_bindgen(js_name = "withCellMaterial")]
    pub fn with_cell_material(&self, material: PresetMaterial) -> Self {
        Self(self.0.clone().with_cell_material(material.into()))
    }

    /// This formatting with the given bevel.
    #[wasm_bindgen(js_name = "withCellBevel")]
    pub fn with_cell_bevel(&self, bevel: &Bevel) -> Self {
        Self(self.0.clone().with_cell_bevel(bevel.0))
    }

    /// This formatting with the given light rig.
    #[wasm_bindgen(js_name = "withCellLightRig")]
    pub fn with_cell_light_rig(&self, light_rig: &LightRig) -> Self {
        Self(self.0.clone().with_cell_light_rig(light_rig.0))
    }
}

#[wasm_bindgen]
impl TableStyleDefinition {
    /// A style that states nothing. Add to it with the `with_…` methods.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self(ooxml::TableStyleDefinition::new())
    }

    /// This style with the given identifier — a GUID in braces, as `tableStyles.xml` writes them.
    #[wasm_bindgen(js_name = "withId")]
    pub fn with_id(&self, style_id: &str) -> Self {
        Self(self.0.clone().with_id(style_id))
    }

    /// This style with the given display name.
    #[wasm_bindgen(js_name = "withName")]
    pub fn with_name(&self, style_name: &str) -> Self {
        Self(self.0.clone().with_name(style_name))
    }

    /// This style with formatting for one of its thirteen parts.
    #[wasm_bindgen(js_name = "withPart")]
    pub fn with_part(&self, part: TableStylePart, format: &TableStyleFormat) -> Self {
        Self(self.0.clone().with_part(part.into(), format.0.clone()))
    }
}

#[wasm_bindgen]
impl TableStyleFlags {
    /// Which banding and heading parts a table has turned on.
    #[wasm_bindgen(constructor)]
    pub fn new(
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
    #[wasm_bindgen(getter, js_name = "firstRow")]
    pub fn first_row(&self) -> bool {
        self.0.first_row
    }

    /// Whether the last row is formatted as a total.
    #[wasm_bindgen(getter, js_name = "lastRow")]
    pub fn last_row(&self) -> bool {
        self.0.last_row
    }

    /// Whether the first column is formatted as a header.
    #[wasm_bindgen(getter, js_name = "firstColumn")]
    pub fn first_column(&self) -> bool {
        self.0.first_column
    }

    /// Whether the last column is formatted as a total.
    #[wasm_bindgen(getter, js_name = "lastColumn")]
    pub fn last_column(&self) -> bool {
        self.0.last_column
    }

    /// Whether rows alternate between the two banding parts.
    #[wasm_bindgen(getter, js_name = "bandedRows")]
    pub fn banded_rows(&self) -> bool {
        self.0.banded_rows
    }

    /// Whether columns alternate between the two banding parts.
    #[wasm_bindgen(getter, js_name = "bandedColumns")]
    pub fn banded_columns(&self) -> bool {
        self.0.banded_columns
    }
}

impl Default for CellFormat {
    /// The same value the no-argument constructor builds.
    fn default() -> Self {
        Self::new()
    }
}

impl Default for TableStyleFormat {
    /// The same value the no-argument constructor builds.
    fn default() -> Self {
        Self::new()
    }
}

impl Default for TableStyleDefinition {
    /// The same value the no-argument constructor builds.
    fn default() -> Self {
        Self::new()
    }
}
