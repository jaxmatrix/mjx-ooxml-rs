//! `a:tbl` (`CT_Table`) — the table itself.

use mjx_derive::{FromXml, ToXml};
use mjx_ooxml_core::{
    FromXml as _, FromXmlError, Interner, RawAttribute, RawElement, RawName, RawNode,
};

use crate::geometry::Emu;

use super::cell::TableCell;
use super::grid::{TableColumn, TableGrid};
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

    // ---------------------------------------------------------------------------------------------
    // Structural edits — insert and remove rows and columns.
    //
    // A row edit touches every cell of one row; a column edit touches the grid *and* one cell in
    // every row, which must stay in step. Merges are **adjusted**, never refused: a merge the new
    // line falls inside grows by one; a merge whose anchor is removed promotes the next cell of the
    // region, which takes the anchor's text and properties so the table looks unchanged. A span that
    // falls back to 1 loses its attribute (`set_spans`), so insert-then-remove round-trips exactly.
    //
    // Each edit reads its classification from the pre-edit table, then mutates — indices shift as
    // cells move, so decisions are made first and applied second.
    // ---------------------------------------------------------------------------------------------

    /// Inserts a new row so it becomes the table's `at`-th row (0-based); `at == row_count` appends.
    /// The new row copies the height of the row beside it and holds one fresh cell per column, each
    /// built by `make_cell`. A vertical merge the new row falls **strictly inside** grows by one row,
    /// and the new cell in that column is born covered so the region stays rectangular.
    ///
    /// `at` must be `<= row_count` — the caller checks that. Returns [`FromXmlError`] only if a cell
    /// `make_cell` builds fails to parse, which a well-formed builder never does.
    pub fn insert_row(
        &mut self,
        interner: &mut Interner,
        at: usize,
        mut make_cell: impl FnMut(&mut Interner) -> RawElement,
    ) -> Result<(), FromXmlError> {
        let rows = self.row_count();
        let columns = self.column_count();

        let neighbour = if at < rows { at } else { at.saturating_sub(1) };
        let height = self.row(neighbour).and_then(|row| row.height(interner));

        let mut cells = Vec::with_capacity(columns);
        for column in 0..columns {
            let mut cell = TableCell::from_xml(&make_cell(interner), interner)?;
            // The cell the new row pushes down: if it is covered from above, the new row is interior
            // to that vertical region, so the new cell is covered too (and horizontally too when the
            // region is also merged across columns, keeping the rectangle intact).
            if at < rows {
                if let Some(below) = self.cell(at, column) {
                    if below.merged_vertically(interner) {
                        let horizontally = below.merged_horizontally(interner);
                        cell.set_merged(interner, horizontally, true);
                    }
                }
            }
            cells.push(cell);
        }
        let row = TableRow::new(interner, height, cells);

        for (anchor_row, anchor_column, row_span) in self.vertical_anchors_crossing(interner, at) {
            let column_span = self
                .cell(anchor_row, anchor_column)
                .map_or(1, |cell| cell.column_span(interner));
            if let Some(anchor) = self.cell_mut(anchor_row, anchor_column) {
                anchor.set_spans(interner, column_span, row_span + 1);
            }
        }

        self.insert_row_element(at, row);
        Ok(())
    }

    /// Removes the table's `at`-th row. A vertical merge the row lies **strictly inside** shrinks by
    /// one; a vertical merge whose **anchor** is in this row promotes the cell directly below it —
    /// which keeps the horizontal span, loses one row, and takes the anchor's `a:txBody` and
    /// `a:tcPr`, while the region's other columns simply stop being covered from above.
    ///
    /// `at` must be `< row_count`, and the caller refuses removing the last row — the checks live
    /// where the dimensions are already known.
    pub fn remove_row(&mut self, interner: &mut Interner, at: usize) {
        for (anchor_row, anchor_column, row_span) in self.vertical_anchors_crossing(interner, at) {
            let column_span = self
                .cell(anchor_row, anchor_column)
                .map_or(1, |cell| cell.column_span(interner));
            if let Some(anchor) = self.cell_mut(anchor_row, anchor_column) {
                anchor.set_spans(interner, column_span, row_span - 1);
            }
        }

        for (anchor_column, column_span, row_span) in self.vertical_anchors_at_row(interner, at) {
            let (body, properties) = self.cell(at, anchor_column).map_or((None, None), |cell| {
                (cell.text_body().cloned(), cell.properties().cloned())
            });
            for offset in 0..column_span {
                let column = anchor_column + offset;
                let Some(cell) = self.cell_mut(at + 1, column) else {
                    continue;
                };
                if offset == 0 {
                    cell.set_body_and_properties(body.clone(), properties.clone());
                    cell.set_spans(interner, column_span, row_span - 1);
                    cell.set_merged(interner, false, false);
                } else {
                    // Was covered from above and (as column > anchor) from the left; becomes the top
                    // row of the region now, so it keeps the horizontal cover and drops the vertical.
                    cell.set_merged(interner, true, false);
                }
            }
        }

        self.remove_row_element(at);
    }

    /// Inserts a new column so it becomes the table's `at`-th column (0-based); `at == column_count`
    /// appends. The grid gains one `a:gridCol` (width copied from the column beside it) and every row
    /// gains one fresh cell, so grid and rows stay in step. A horizontal merge the new column falls
    /// **strictly inside** grows by one, and the new cell in that row is born covered.
    ///
    /// `at` must be `<= column_count`. Returns [`FromXmlError`] only if a freshly built cell fails to
    /// parse.
    pub fn insert_column(
        &mut self,
        interner: &mut Interner,
        at: usize,
        mut make_cell: impl FnMut(&mut Interner) -> RawElement,
    ) -> Result<(), FromXmlError> {
        let rows = self.row_count();
        let columns = self.column_count();

        let neighbour = if at < columns {
            at
        } else {
            at.saturating_sub(1)
        };
        let width = self
            .grid()
            .and_then(|grid| grid.column(neighbour))
            .and_then(|column| column.width(interner))
            .unwrap_or(Emu::from_emu(0));

        for (anchor_row, anchor_column, column_span) in
            self.horizontal_anchors_crossing(interner, at)
        {
            let row_span = self
                .cell(anchor_row, anchor_column)
                .map_or(1, |cell| cell.row_span(interner));
            if let Some(anchor) = self.cell_mut(anchor_row, anchor_column) {
                anchor.set_spans(interner, column_span + 1, row_span);
            }
        }

        for row in 0..rows {
            let mut cell = TableCell::from_xml(&make_cell(interner), interner)?;
            if at < columns {
                if let Some(right) = self.cell(row, at) {
                    if right.merged_horizontally(interner) {
                        let vertically = right.merged_vertically(interner);
                        cell.set_merged(interner, true, vertically);
                    }
                }
            }
            if let Some(row_element) = self.row_mut(row) {
                row_element.insert_cell_at(at, cell);
            }
        }

        if let Some(grid) = self.grid_mut() {
            grid.insert_column_at(at, TableColumn::new(interner, width));
        }
        Ok(())
    }

    /// Removes the table's `at`-th column: the grid's `a:gridCol` and one `a:tc` from every row. A
    /// horizontal merge the column lies **strictly inside** shrinks by one; a horizontal merge whose
    /// **anchor** is in this column promotes the cell to its right — which keeps the vertical span,
    /// loses one column, and takes the anchor's `a:txBody` and `a:tcPr`, while the region's other
    /// rows stop being covered from the left.
    ///
    /// `at` must be `< column_count`, and the caller refuses removing the last column.
    pub fn remove_column(&mut self, interner: &mut Interner, at: usize) {
        for (anchor_row, anchor_column, column_span) in
            self.horizontal_anchors_crossing(interner, at)
        {
            let row_span = self
                .cell(anchor_row, anchor_column)
                .map_or(1, |cell| cell.row_span(interner));
            if let Some(anchor) = self.cell_mut(anchor_row, anchor_column) {
                anchor.set_spans(interner, column_span - 1, row_span);
            }
        }

        for (anchor_row, column_span, row_span) in self.horizontal_anchors_at_column(interner, at) {
            let (body, properties) = self.cell(anchor_row, at).map_or((None, None), |cell| {
                (cell.text_body().cloned(), cell.properties().cloned())
            });
            for offset in 0..row_span {
                let row = anchor_row + offset;
                let Some(cell) = self.cell_mut(row, at + 1) else {
                    continue;
                };
                if offset == 0 {
                    cell.set_body_and_properties(body.clone(), properties.clone());
                    cell.set_spans(interner, column_span - 1, row_span);
                    cell.set_merged(interner, false, false);
                } else {
                    // Was covered from the left and above; becomes the new anchor column, so it keeps
                    // the vertical cover and drops the horizontal.
                    cell.set_merged(interner, false, true);
                }
            }
        }

        for row in 0..self.row_count() {
            if let Some(row_element) = self.row_mut(row) {
                row_element.remove_cell_at(at);
            }
        }
        if let Some(grid) = self.grid_mut() {
            grid.remove_column_at(at);
        }
    }

    // --- Structural helpers -----------------------------------------------------------------------

    /// Inserts `row` at typed row position `at`, past any opaque node between the rows.
    fn insert_row_element(&mut self, at: usize, row: TableRow) {
        let index = super::typed_insert_index(&self.content, at, |item| {
            matches!(item, TableContent::Row(_))
        });
        self.content.insert(index, TableContent::Row(row));
        self.empty = false;
    }

    /// Removes the typed row at position `at`, returning it, or `None` if there are fewer.
    fn remove_row_element(&mut self, at: usize) -> Option<TableRow> {
        let index = super::nth_typed_index(&self.content, at, |item| {
            matches!(item, TableContent::Row(_))
        })?;
        // `nth_typed_index` only points at a `Row`; the fallback re-inserts rather than drop.
        match self.content.remove(index) {
            TableContent::Row(row) => Some(row),
            other => {
                self.content.insert(index, other);
                None
            }
        }
    }

    /// Anchors of vertical merges (`rowSpan > 1`, cell not itself covered) whose span strictly
    /// straddles the line at row `at` — `row < at < row + rowSpan`. `(row, column, rowSpan)`.
    fn vertical_anchors_crossing(
        &self,
        interner: &Interner,
        at: usize,
    ) -> Vec<(usize, usize, usize)> {
        let mut found = Vec::new();
        for row in 0..self.row_count() {
            for column in 0..self.column_count() {
                let Some(cell) = self.cell(row, column) else {
                    continue;
                };
                if cell.is_covered_by_merge(interner) {
                    continue;
                }
                let row_span = cell.row_span(interner);
                if row_span > 1 && row < at && at < row + row_span {
                    found.push((row, column, row_span));
                }
            }
        }
        found
    }

    /// Anchors of vertical merges whose anchor sits in row `at`. `(column, columnSpan, rowSpan)`.
    fn vertical_anchors_at_row(
        &self,
        interner: &Interner,
        at: usize,
    ) -> Vec<(usize, usize, usize)> {
        let mut found = Vec::new();
        for column in 0..self.column_count() {
            let Some(cell) = self.cell(at, column) else {
                continue;
            };
            if cell.is_covered_by_merge(interner) {
                continue;
            }
            if cell.row_span(interner) > 1 {
                found.push((column, cell.column_span(interner), cell.row_span(interner)));
            }
        }
        found
    }

    /// Anchors of horizontal merges (`gridSpan > 1`, not covered) whose span strictly straddles the
    /// line at column `at` — `column < at < column + gridSpan`. `(row, column, gridSpan)`.
    fn horizontal_anchors_crossing(
        &self,
        interner: &Interner,
        at: usize,
    ) -> Vec<(usize, usize, usize)> {
        let mut found = Vec::new();
        for row in 0..self.row_count() {
            for column in 0..self.column_count() {
                let Some(cell) = self.cell(row, column) else {
                    continue;
                };
                if cell.is_covered_by_merge(interner) {
                    continue;
                }
                let column_span = cell.column_span(interner);
                if column_span > 1 && column < at && at < column + column_span {
                    found.push((row, column, column_span));
                }
            }
        }
        found
    }

    /// Anchors of horizontal merges whose anchor sits in column `at`. `(row, columnSpan, rowSpan)`.
    fn horizontal_anchors_at_column(
        &self,
        interner: &Interner,
        at: usize,
    ) -> Vec<(usize, usize, usize)> {
        let mut found = Vec::new();
        for row in 0..self.row_count() {
            let Some(cell) = self.cell(row, at) else {
                continue;
            };
            if cell.is_covered_by_merge(interner) {
                continue;
            }
            if cell.column_span(interner) > 1 {
                found.push((row, cell.column_span(interner), cell.row_span(interner)));
            }
        }
        found
    }
}
