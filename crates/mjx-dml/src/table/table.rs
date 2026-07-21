//! `a:tbl` (`CT_Table`) — the table itself.

use mjx_derive::{FromXml, ToXml};
use mjx_ooxml_core::{Interner, RawAttribute, RawName, RawNode};

use super::cell::TableCell;
use super::grid::TableGrid;
use super::properties::TableProperties;
use super::row::TableRow;

/// One ordered child of a [`Table`]: its typed properties, grid or a row, or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableContent {
    /// The table's properties (`a:tblPr`).
    Properties(TableProperties),
    /// The table's column definitions (`a:tblGrid`).
    Grid(TableGrid),
    /// A row (`a:tr`).
    Row(TableRow),
    /// Any other child — whitespace or an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `a:tbl` (`CT_Table`) — an optional `a:tblPr`, a required `a:tblGrid`, then the rows.
///
/// A table is reached through a `p:graphicFrame`: the frame positions it (see
/// `Presentation::shape_bounds`), and `a:graphic > a:graphicData > a:tbl` is what it frames.
///
/// # Dimensions
///
/// The column count is the **grid's** — `a:tblGrid` is where a table declares how wide it is, and
/// every row is expected to carry exactly that many cells, merged ones included. [`column_count`]
/// reads the grid; a row that disagrees with it is malformed, and the accessors here report what is
/// there rather than papering over it.
///
/// [`column_count`]: Table::column_count
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_MAIN)]
pub struct Table {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "tblPr", variant = Properties, ty = TableProperties),
        child(local = "tblGrid", variant = Grid, ty = TableGrid),
        child(local = "tr", variant = Row, ty = TableRow)
    )]
    content: Vec<TableContent>,
}

impl Table {
    /// The table's properties (`a:tblPr`), or `None` if it declares none.
    #[must_use]
    pub fn properties(&self) -> Option<&TableProperties> {
        self.content.iter().find_map(|item| match item {
            TableContent::Properties(properties) => Some(properties),
            _ => None,
        })
    }

    /// The table's properties, mutably.
    pub fn properties_mut(&mut self) -> Option<&mut TableProperties> {
        self.content.iter_mut().find_map(|item| match item {
            TableContent::Properties(properties) => Some(properties),
            _ => None,
        })
    }

    /// The table's column definitions (`a:tblGrid`), or `None` if it has none — which the schema
    /// forbids, but a file may still present.
    #[must_use]
    pub fn grid(&self) -> Option<&TableGrid> {
        self.content.iter().find_map(|item| match item {
            TableContent::Grid(grid) => Some(grid),
            _ => None,
        })
    }

    /// The table's column definitions, mutably.
    pub fn grid_mut(&mut self) -> Option<&mut TableGrid> {
        self.content.iter_mut().find_map(|item| match item {
            TableContent::Grid(grid) => Some(grid),
            _ => None,
        })
    }

    /// The table's rows, in order (opaque children are skipped).
    pub fn rows(&self) -> impl Iterator<Item = &TableRow> {
        self.content.iter().filter_map(|item| match item {
            TableContent::Row(row) => Some(row),
            _ => None,
        })
    }

    /// The table's rows, mutably, in order.
    pub fn rows_mut(&mut self) -> impl Iterator<Item = &mut TableRow> {
        self.content.iter_mut().filter_map(|item| match item {
            TableContent::Row(row) => Some(row),
            _ => None,
        })
    }

    /// The number of rows.
    #[must_use]
    pub fn row_count(&self) -> usize {
        self.rows().count()
    }

    /// The number of columns, as the **grid** declares it. `0` when the table has no `a:tblGrid`.
    #[must_use]
    pub fn column_count(&self) -> usize {
        self.grid().map_or(0, TableGrid::column_count)
    }

    /// The row at `index`, or `None`.
    #[must_use]
    pub fn row(&self, index: usize) -> Option<&TableRow> {
        self.rows().nth(index)
    }

    /// The row at `index`, mutably.
    pub fn row_mut(&mut self, index: usize) -> Option<&mut TableRow> {
        self.rows_mut().nth(index)
    }

    /// The cell at `(row, column)`, or `None` if the table is smaller than that.
    ///
    /// A cell covered by a merge is returned like any other — it is a real cell, and reports its
    /// covered state through [`TableCell::is_covered_by_merge`].
    #[must_use]
    pub fn cell(&self, row: usize, column: usize) -> Option<&TableCell> {
        self.row(row)?.cell(column)
    }

    /// The cell at `(row, column)`, mutably.
    pub fn cell_mut(&mut self, row: usize, column: usize) -> Option<&mut TableCell> {
        self.row_mut(row)?.cell_mut(column)
    }

    /// The anchor of the merged region covering `(row, column)` — the cell that actually renders
    /// there — or the cell itself when it is not covered.
    ///
    /// Walks left while the cell states `hMerge`, then up while it states `vMerge`, which is how a
    /// covered cell names its anchor: the attributes say *something to my left/above owns me*, not
    /// which cell that is.
    #[must_use]
    pub fn merge_anchor(
        &self,
        interner: &Interner,
        row: usize,
        column: usize,
    ) -> Option<(usize, usize)> {
        let mut at_row = row;
        let mut at_column = column;

        while self.cell(at_row, at_column)?.merged_horizontally(interner) {
            at_column = at_column.checked_sub(1)?;
        }
        while self.cell(at_row, at_column)?.merged_vertically(interner) {
            at_row = at_row.checked_sub(1)?;
        }
        Some((at_row, at_column))
    }

    /// The table's ordered content (typed children interleaved with opaque nodes).
    #[must_use]
    pub fn content(&self) -> &[TableContent] {
        &self.content
    }

    /// The table's ordered content, mutably — for inserting or removing a row.
    pub fn content_mut(&mut self) -> &mut Vec<TableContent> {
        &mut self.content
    }
}
