//! Tables (`w:tbl`/`w:tr`/`w:tc`) — a top-level table addressed by a plain `u32` index, `(row,
//! column)` addressing inside it, mirroring [`crate::deck::tables`]'s own naming and argument order.

use crate::error::Error;
use crate::index::{count, index};

impl super::Document {
    /// How many top-level tables the document body holds, or `0` if it declares no body.
    ///
    /// # Errors
    /// [`ErrorCode::MalformedDocument`](crate::ErrorCode::MalformedDocument) if the main document
    /// part cannot be read.
    pub fn table_count(&mut self) -> Result<u32, Error> {
        Ok(count(self.document.table_count()?))
    }

    /// The shape of the table at top-level index `table`, as `(rows, columns)`.
    ///
    /// # Errors
    /// [`ErrorCode::NothingToRead`](crate::ErrorCode::NothingToRead) if the document declares no
    /// body, or [`ErrorCode::IndexOutOfRange`](crate::ErrorCode::IndexOutOfRange) if `table` does
    /// not address a table.
    pub fn table_dimensions(&mut self, table: u32) -> Result<(u32, u32), Error> {
        let (rows, columns) = self.document.table_dimensions(index(table))?;
        Ok((count(rows), count(columns)))
    }

    /// How many rows and columns the cell at `(row, column)` of table `table` spans, as `(rows,
    /// columns)`.
    ///
    /// # Errors
    /// As [`table_dimensions`](Self::table_dimensions), plus
    /// [`ErrorCode::IndexOutOfRange`](crate::ErrorCode::IndexOutOfRange) if `(row, column)` is out of
    /// range.
    pub fn cell_span(&mut self, table: u32, row: u32, column: u32) -> Result<(u32, u32), Error> {
        let (rows, columns) = self
            .document
            .cell_span(index(table), index(row), index(column))?;
        Ok((count(rows), count(columns)))
    }

    /// Which cell actually renders at `(row, column)` of table `table` — resolving any vertical or
    /// horizontal merge.
    ///
    /// # Errors
    /// As [`cell_span`](Self::cell_span).
    pub fn merged_cell_anchor(
        &mut self,
        table: u32,
        row: u32,
        column: u32,
    ) -> Result<(u32, u32), Error> {
        let (row, column) =
            self.document
                .merged_cell_anchor(index(table), index(row), index(column))?;
        Ok((count(row), count(column)))
    }

    /// Every grid discrepancy table `table` currently has (a row whose cells' own spans do not sum
    /// to the table's `w:tblGrid` column count).
    ///
    /// # Errors
    /// As [`table_dimensions`](Self::table_dimensions).
    pub fn table_grid_discrepancies(
        &mut self,
        table: u32,
    ) -> Result<Vec<mjx_docx::GridDiscrepancy>, Error> {
        Ok(self.document.table_grid_discrepancies(index(table))?)
    }

    /// The text of the cell at `(row, column)` of table `table` — its direct paragraphs' text,
    /// joined by a newline.
    ///
    /// # Errors
    /// As [`cell_span`](Self::cell_span).
    pub fn cell_text(&mut self, table: u32, row: u32, column: u32) -> Result<String, Error> {
        Ok(self
            .document
            .cell_text(index(table), index(row), index(column))?)
    }

    /// Sets the text of the cell at `(row, column)` of table `table`: replaces its first direct
    /// paragraph's runs with a single run holding `text`.
    ///
    /// # Errors
    /// As [`cell_span`](Self::cell_span).
    pub fn set_cell_text(
        &mut self,
        table: u32,
        row: u32,
        column: u32,
        text: &str,
    ) -> Result<(), Error> {
        Ok(self
            .document
            .set_cell_text(index(table), index(row), index(column), text)?)
    }

    /// Sets (or, given `None`/`Some(1)`, removes) the `w:gridSpan` of the cell at `(row, column)` of
    /// table `table` — how many grid columns it covers.
    ///
    /// # Errors
    /// As [`cell_span`](Self::cell_span).
    pub fn set_cell_span(
        &mut self,
        table: u32,
        row: u32,
        column: u32,
        span: Option<u32>,
    ) -> Result<(), Error> {
        Ok(self
            .document
            .set_cell_span(index(table), index(row), index(column), span.map(index))?)
    }

    /// Sets (or, given `None`, removes) the `w:vMerge` of the cell at `(row, column)` of table
    /// `table`.
    ///
    /// # Errors
    /// As [`cell_span`](Self::cell_span).
    pub fn set_cell_vertical_merge(
        &mut self,
        table: u32,
        row: u32,
        column: u32,
        kind: Option<mjx_docx::MergedCellType>,
    ) -> Result<(), Error> {
        Ok(self
            .document
            .set_cell_vertical_merge(index(table), index(row), index(column), kind)?)
    }

    /// Appends a new `rows` x `columns` table as the body's new last top-level table, and returns
    /// its new index. Every cell starts with one empty paragraph.
    ///
    /// # Errors
    /// [`ErrorCode::InvalidArgument`](crate::ErrorCode::InvalidArgument) if either dimension is
    /// zero, or [`ErrorCode::NothingToRead`](crate::ErrorCode::NothingToRead) if the document
    /// declares no body.
    pub fn append_table(&mut self, rows: u32, columns: u32) -> Result<u32, Error> {
        Ok(count(
            self.document.append_table(index(rows), index(columns))?,
        ))
    }

    /// Removes the top-level table at `index`.
    ///
    /// # Errors
    /// [`ErrorCode::NothingToRead`](crate::ErrorCode::NothingToRead) if the document declares no
    /// body, or [`ErrorCode::IndexOutOfRange`](crate::ErrorCode::IndexOutOfRange) if `table` does
    /// not address a table.
    pub fn remove_table(&mut self, table: u32) -> Result<(), Error> {
        Ok(self.document.remove_table(index(table))?)
    }

    /// Inserts a row into table `table` so it becomes row `at`; `at` equal to the current row count
    /// appends. A vertical merge the new row falls inside grows to include it.
    ///
    /// # Errors
    /// As [`table_dimensions`](Self::table_dimensions), plus
    /// [`ErrorCode::IndexOutOfRange`](crate::ErrorCode::IndexOutOfRange) if `at` is past the end.
    pub fn insert_row(&mut self, table: u32, at: u32) -> Result<(), Error> {
        Ok(self.document.insert_row(index(table), index(at))?)
    }

    /// Removes row `at` from table `table`.
    ///
    /// # Errors
    /// [`ErrorCode::InvalidArgument`](crate::ErrorCode::InvalidArgument) if `at` is the table's only
    /// row, plus the errors of [`insert_row`](Self::insert_row).
    pub fn remove_row(&mut self, table: u32, at: u32) -> Result<(), Error> {
        Ok(self.document.remove_row(index(table), index(at))?)
    }

    /// Inserts a column into table `table` so it becomes column `at`; `at` equal to the current
    /// column count appends. A horizontal merge the new column falls inside grows to include it.
    ///
    /// # Errors
    /// As [`insert_row`](Self::insert_row).
    pub fn insert_column(&mut self, table: u32, at: u32) -> Result<(), Error> {
        Ok(self.document.insert_column(index(table), index(at))?)
    }

    /// Removes column `at` from table `table`.
    ///
    /// # Errors
    /// As [`remove_row`](Self::remove_row).
    pub fn remove_column(&mut self, table: u32, at: u32) -> Result<(), Error> {
        Ok(self.document.remove_column(index(table), index(at))?)
    }
}
