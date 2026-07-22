//! `a:tr` (`CT_TableRow`) — one row of a table.

use mjx_derive::{FromXml, ToXml};
use mjx_ooxml_core::{Interner, RawAttribute, RawName, RawNode};

use crate::build::{attr_emu, dml_attr, dml_name, set_attr};
use crate::geometry::Emu;

use super::cell::TableCell;

/// One ordered child of a [`TableRow`]: a typed [`TableCell`], or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableRowContent {
    /// A cell (`a:tc`).
    Cell(TableCell),
    /// Any other child — `extLst`, whitespace, or an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `a:tr` (`CT_TableRow`) — a row's height and its cells, in column order.
///
/// A row carries one `a:tc` per column of the table's `a:tblGrid`, **including** the cells covered
/// by a merge, so the row's cell count is the table's column count and a cell's position in the row
/// is its column index.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_MAIN)]
pub struct TableRow {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "tc", variant = Cell, ty = TableCell))]
    content: Vec<TableRowContent>,
}

impl TableRow {
    /// A fresh `a:tr` of `cells`, carrying `height` when one is given (a new row copies its
    /// neighbour's height, and a row beside one that states none states none too).
    #[must_use]
    pub fn new(interner: &mut Interner, height: Option<Emu>, cells: Vec<TableCell>) -> Self {
        let attributes = match height {
            Some(height) => vec![dml_attr(interner, "h", &height.emu().to_string())],
            None => Vec::new(),
        };
        let content: Vec<TableRowContent> = cells.into_iter().map(TableRowContent::Cell).collect();
        let empty = content.is_empty();
        Self {
            name: dml_name(interner, "tr"),
            attributes,
            empty,
            content,
        }
    }

    /// The row's height (`@h`, EMU).
    ///
    /// This is the height the row *asks* for; PowerPoint grows a row whose content does not fit, so
    /// a rendered row is never shorter than this but may be taller.
    #[must_use]
    pub fn height(&self, interner: &Interner) -> Option<Emu> {
        attr_emu(&self.attributes, interner, "h")
    }

    /// Sets the row's height (`@h`), rewriting the attribute in place.
    pub fn set_height(&mut self, interner: &mut Interner, height: Emu) {
        set_attr(
            &mut self.attributes,
            interner,
            "h",
            &height.emu().to_string(),
        );
    }

    /// The row's cells, in column order (opaque children are skipped).
    pub fn cells(&self) -> impl Iterator<Item = &TableCell> {
        self.content.iter().filter_map(|item| match item {
            TableRowContent::Cell(cell) => Some(cell),
            _ => None,
        })
    }

    /// The row's cells, mutably, in column order.
    pub fn cells_mut(&mut self) -> impl Iterator<Item = &mut TableCell> {
        self.content.iter_mut().filter_map(|item| match item {
            TableRowContent::Cell(cell) => Some(cell),
            _ => None,
        })
    }

    /// How many cells the row holds. On a well-formed table this equals the grid's column count.
    #[must_use]
    pub fn cell_count(&self) -> usize {
        self.cells().count()
    }

    /// The cell in column `n`, or `None` if the row is shorter than that.
    #[must_use]
    pub fn cell(&self, n: usize) -> Option<&TableCell> {
        self.cells().nth(n)
    }

    /// The cell in column `n`, mutably.
    pub fn cell_mut(&mut self, n: usize) -> Option<&mut TableCell> {
        self.cells_mut().nth(n)
    }

    /// The row's ordered content (typed cells interleaved with opaque nodes).
    #[must_use]
    pub fn content(&self) -> &[TableRowContent] {
        &self.content
    }

    /// The row's ordered content, mutably — for inserting or removing a cell, which must stay in
    /// step with the table's grid.
    pub fn content_mut(&mut self) -> &mut Vec<TableRowContent> {
        &mut self.content
    }

    /// Inserts `cell` so it becomes the row's `at`-th cell (0-based); `at == cell_count` appends it.
    pub fn insert_cell_at(&mut self, at: usize, cell: TableCell) {
        let index = super::typed_insert_index(&self.content, at, |item| {
            matches!(item, TableRowContent::Cell(_))
        });
        self.content.insert(index, TableRowContent::Cell(cell));
        self.empty = false;
    }

    /// Removes the row's `at`-th cell and returns it, or `None` if the row has fewer.
    pub fn remove_cell_at(&mut self, at: usize) -> Option<TableCell> {
        let index = super::nth_typed_index(&self.content, at, |item| {
            matches!(item, TableRowContent::Cell(_))
        })?;
        // `nth_typed_index` only points at a `Cell`; the fallback re-inserts rather than drop.
        match self.content.remove(index) {
            TableRowContent::Cell(cell) => Some(cell),
            other => {
                self.content.insert(index, other);
                None
            }
        }
    }
}
