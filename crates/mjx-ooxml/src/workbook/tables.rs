//! Worksheet tables — `x:table`, the structured region a formula references as `Sales[Region]`.
//!
//! A **report**, not a second model. `mjx_xlsx::SheetTable` is already owned, but carries an
//! [`mjx_opc::PartName`] and an [`mjx_sml::CellRange`]; the two types here restate it with the part
//! and the range as text. Authoring a table takes an [`mjx_sml::WorksheetTableSpec`] tree, which
//! [the module documentation above](super) leaves to [`Workbook::workbook_mut`].

use mjx_ooxml_types::spreadsheetml::TotalsRowFunction;
use mjx_sml::TableStyleOrigin;

use crate::error::Error;
use crate::index::index;

use super::Workbook;

/// One column of a [`SheetTableInfo`], decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetTableColumnInfo {
    /// `@id`, unique within the table.
    pub id: u32,
    /// `@name` — the heading text, and what a structured reference (`Sales[Region]`) names.
    pub name: String,
    /// `@totalsRowFunction`, or `None` when the column writes none.
    pub totals_row_function: Option<TotalsRowFunction>,
    /// `@totalsRowLabel` — the literal text a totals cell shows instead of an aggregate.
    pub totals_row_label: Option<String>,
    /// `x:calculatedColumnFormula`'s text, **exactly as the file wrote it**. Never expanded into
    /// per-cell formulas and never evaluated.
    pub calculated_column_formula: Option<String>,
    /// `x:totalsRowFormula`'s text, on the same terms.
    pub totals_row_formula: Option<String>,
}

/// One table on a sheet, resolved to its part and decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetTableInfo {
    /// The part the table lives in — `"/xl/tables/table1.xml"` in everything a real producer writes,
    /// though nothing requires that spelling.
    pub part: String,
    /// The `tablePart@r:id` the sheet reached it through.
    pub relationship_id: String,
    /// `@id` — **workbook-unique**, and never renumbered by anything here.
    pub id: u32,
    /// `@displayName` — what a formula references the table by.
    pub display_name: String,
    /// `@name`, the programmatic name, or `None` when the table writes none.
    pub name: Option<String>,
    /// `@ref` — the whole region as A1 text, **header and totals rows included**.
    pub range: String,
    /// `@headerRowCount`, or the schema default 1.
    pub header_row_count: u32,
    /// `@totalsRowCount`, or the schema default 0.
    pub totals_row_count: u32,
    /// How many rows of [`range`](Self::range) are data rows, or `None` when the header and totals
    /// counts do not fit inside it — a report about the file, not a repair of it.
    pub data_row_count: Option<u32>,
    /// `tableStyleInfo@name`, or `None` when the table names no style at all — a different statement
    /// from naming one the file does not define. Ask
    /// [`table_style_origin`](Workbook::table_style_origin) which of the three that is.
    pub style_name: Option<String>,
    /// The columns, left to right.
    pub columns: Vec<SheetTableColumnInfo>,
}

impl Workbook {
    /// Every table on one sheet, in the order the sheet's `tableParts` names them.
    ///
    /// # Errors
    /// [`ErrorCode::IndexOutOfRange`](crate::ErrorCode::IndexOutOfRange) if `sheet` names no tab,
    /// [`ErrorCode::NothingToRead`](crate::ErrorCode::NothingToRead) if it holds no worksheet, or
    /// [`ErrorCode::MalformedDocument`](crate::ErrorCode::MalformedDocument) if a table part is not
    /// well-formed.
    pub fn sheet_tables(&self, sheet: u32) -> Result<Vec<SheetTableInfo>, Error> {
        Ok(self
            .workbook
            .sheet_tables(index(sheet))?
            .into_iter()
            .map(info)
            .collect())
    }

    /// Where the table style `name` comes from: this workbook's `tableStyles`, Excel's built-in set,
    /// or nowhere at all.
    ///
    /// The third answer is the one worth asking for: a table naming a style no consumer defines
    /// renders unstyled, and nothing in the markup says so.
    ///
    /// # Errors
    /// [`ErrorCode::MalformedDocument`](crate::ErrorCode::MalformedDocument) if the styles part
    /// cannot be read.
    pub fn table_style_origin(&self, name: &str) -> Result<TableStyleOrigin, Error> {
        Ok(self.workbook.table_style_origin(name)?)
    }

    /// The lowest `@id` no table in the workbook uses — what a new table must be given.
    ///
    /// `@id` is workbook-unique rather than sheet-unique, so this walks every sheet's tables and not
    /// only one's.
    ///
    /// # Errors
    /// [`ErrorCode::MalformedDocument`](crate::ErrorCode::MalformedDocument) if a table part is not
    /// well-formed.
    pub fn next_table_id(&self) -> Result<u32, Error> {
        Ok(self.workbook.next_table_id()?)
    }
}

/// One model table as the facade states it.
fn info(table: mjx_xlsx::SheetTable) -> SheetTableInfo {
    let data_row_count = table.data_row_count();
    SheetTableInfo {
        part: table.part.as_str().to_owned(),
        relationship_id: table.relationship_id,
        id: table.id,
        display_name: table.display_name,
        name: table.name,
        range: table.range.text().as_str().to_owned(),
        header_row_count: table.header_row_count,
        totals_row_count: table.totals_row_count,
        data_row_count,
        style_name: table.style_name,
        columns: table
            .columns
            .into_iter()
            .map(|column| SheetTableColumnInfo {
                id: column.id,
                name: column.name,
                totals_row_function: column.totals_row_function,
                totals_row_label: column.totals_row_label,
                calculated_column_formula: column.calculated_column_formula,
                totals_row_formula: column.totals_row_formula,
            })
            .collect(),
    }
}
