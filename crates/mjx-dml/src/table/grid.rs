//! `a:tblGrid` (`CT_TableGrid`) and `a:gridCol` (`CT_TableCol`) — how wide each column is.
//!
//! The grid is where a table's column count is *declared*. Every row is expected to carry one
//! `a:tc` per `a:gridCol`, so the grid is the authority a reader counts columns from.

use mjx_derive::{FromXml, ToXml};
use mjx_ooxml_core::{Interner, RawAttribute, RawName, RawNode};

use crate::build::{attr_emu, dml_attr, dml_name, fidelity_element_impls, set_attr};
use crate::geometry::Emu;

/// `a:gridCol` (`CT_TableCol`) — one column of the table grid, carrying its width.
///
/// The width is `use="required"` in the schema, but a file in the wild is read as it is written:
/// a column whose `@w` is absent or unparsable reads as `None` rather than failing the parse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableColumn {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

fidelity_element_impls!(TableColumn);

impl TableColumn {
    /// A fresh `a:gridCol` carrying just its width — what a newly inserted column is born as, with no
    /// inherited `extLst` or unknown attribute from the neighbour it copied the width from.
    #[must_use]
    pub fn new(interner: &mut Interner, width: Emu) -> Self {
        Self {
            name: dml_name(interner, "gridCol"),
            attributes: vec![dml_attr(interner, "w", &width.emu().to_string())],
            children: Vec::new(),
            empty: true,
        }
    }

    /// The column's width (`@w`, EMU).
    #[must_use]
    pub fn width(&self, interner: &Interner) -> Option<Emu> {
        attr_emu(&self.attributes, interner, "w")
    }

    /// Sets the column's width (`@w`), rewriting the attribute in place so anything else the element
    /// carries — an `extLst`, an unknown attribute — is untouched.
    pub fn set_width(&mut self, interner: &mut Interner, width: Emu) {
        set_attr(
            &mut self.attributes,
            interner,
            "w",
            &width.emu().to_string(),
        );
    }
}

/// One ordered child of a [`TableGrid`]: a typed [`TableColumn`], or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableGridContent {
    /// A column definition (`a:gridCol`).
    Column(TableColumn),
    /// Any other child — whitespace or an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `a:tblGrid` (`CT_TableGrid`) — the table's column definitions, in order.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_MAIN)]
pub struct TableGrid {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "gridCol", variant = Column, ty = TableColumn))]
    content: Vec<TableGridContent>,
}

impl TableGrid {
    /// The grid's columns, in order (opaque children are skipped).
    pub fn columns(&self) -> impl Iterator<Item = &TableColumn> {
        self.content.iter().filter_map(|item| match item {
            TableGridContent::Column(column) => Some(column),
            _ => None,
        })
    }

    /// The grid's columns, mutably, in order.
    pub fn columns_mut(&mut self) -> impl Iterator<Item = &mut TableColumn> {
        self.content.iter_mut().filter_map(|item| match item {
            TableGridContent::Column(column) => Some(column),
            _ => None,
        })
    }

    /// The number of columns the table declares — the width every row is expected to match.
    #[must_use]
    pub fn column_count(&self) -> usize {
        self.columns().count()
    }

    /// The `n`-th column, or `None` if the grid declares fewer.
    #[must_use]
    pub fn column(&self, n: usize) -> Option<&TableColumn> {
        self.columns().nth(n)
    }

    /// The `n`-th column, mutably.
    pub fn column_mut(&mut self, n: usize) -> Option<&mut TableColumn> {
        self.columns_mut().nth(n)
    }

    /// The grid's ordered content (typed columns interleaved with opaque nodes).
    #[must_use]
    pub fn content(&self) -> &[TableGridContent] {
        &self.content
    }

    /// The grid's ordered content, mutably — for inserting or removing a column, which must stay in
    /// step with every row's cells.
    pub fn content_mut(&mut self) -> &mut Vec<TableGridContent> {
        &mut self.content
    }

    /// Inserts `column` so it becomes the grid's `at`-th column (0-based); `at == column_count`
    /// appends it at the end. The new column lands beside its typed siblings, past any opaque node.
    pub fn insert_column_at(&mut self, at: usize, column: TableColumn) {
        let index = super::typed_insert_index(&self.content, at, |item| {
            matches!(item, TableGridContent::Column(_))
        });
        self.content.insert(index, TableGridContent::Column(column));
        self.empty = false;
    }

    /// Removes the grid's `at`-th column and returns it, or `None` if the grid has fewer.
    pub fn remove_column_at(&mut self, at: usize) -> Option<TableColumn> {
        let index = super::nth_typed_index(&self.content, at, |item| {
            matches!(item, TableGridContent::Column(_))
        })?;
        // `nth_typed_index` only ever points at a `Column`, so the match below always takes the first
        // arm; the fallback re-inserts rather than dropping anything if that ever changed.
        match self.content.remove(index) {
            TableGridContent::Column(column) => Some(column),
            other => {
                self.content.insert(index, other);
                None
            }
        }
    }
}
