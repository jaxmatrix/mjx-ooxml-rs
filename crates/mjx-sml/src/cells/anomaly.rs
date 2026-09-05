//! What a file got wrong, reported rather than repaired.
//!
//! # Why a report exists at all
//!
//! Every input here is a file somebody else wrote, and a worksheet can say things that are not true:
//! rows out of order, the same row number twice, a `c@r` naming a different row than the `row@r`
//! above it, a `t="s"` on a cell holding an `<is>`. The rule this store follows is that **none of
//! these is repaired**: sorting the rows, dropping a duplicate or correcting a reference would change
//! the bytes of a part nobody asked to edit, which is the one thing this library exists not to do.
//!
//! But "preserved as read" is indistinguishable from "not noticed" unless something can say what was
//! preserved. That is what this is: a read-only description, computed on demand, that changes
//! nothing. A caller who wants to know whether a workbook is well-formed can ask; a caller who only
//! wants to read a cell never pays for it.

use crate::address::CellReference;

use super::record::{PayloadShape, RowFlags};
use super::store::SheetData;

/// Something a worksheet said that is true of the file and not of a well-formed sheet.
///
/// Every one of these is **preserved and written back as it stands**. None is an error, and none
/// changes what the store answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SheetDataAnomaly {
    /// A row's `r` is not greater than the row before it. Lookups fall back to a scan, which stays
    /// correct; nothing is sorted.
    RowsOutOfOrder {
        /// The `r` of the row that broke the ascent.
        row: u32,
        /// The `r` of the row before it.
        previous_row: u32,
    },
    /// Two rows carry the same `r`. Both are kept; a lookup answers with the first.
    DuplicateRowNumber {
        /// The repeated `r`.
        row: u32,
    },
    /// A row wrote no `r` at all, so its number is its position. Legal, and worth naming, because
    /// every lookup by number for such a row is a lookup by position.
    RowWithoutNumber {
        /// The position the row would be found at.
        position: u32,
    },
    /// A cell's `c@r` names a different row than the `row@r` containing it does.
    CellRowDisagreesWithRow {
        /// The reference the cell wrote.
        cell: CellReference,
        /// The `r` of the row it was written inside.
        row: u32,
    },
    /// Two cells in one row name the same column. Both are kept; a lookup answers with one of them.
    DuplicateCellReference {
        /// The repeated address.
        cell: CellReference,
    },
    /// A row's cells are not in ascending column order. Lookups in that row fall back to a scan.
    CellsOutOfOrder {
        /// The `r` of the row.
        row: u32,
    },
    /// A cell's `t` does not match the value element it holds — `t="inlineStr"` with a `<v>`, or any
    /// other `t` with an `<is>`.
    ///
    /// The value is readable through the accessor its `t` names and preserved either way; this is
    /// the store declining to guess which of the two the producer meant.
    CellTypeDisagreesWithContent {
        /// The cell.
        cell: CellReference,
    },
}

impl SheetData {
    /// Everything this worksheet said that a well-formed one would not, in document order.
    ///
    /// Computed on demand and never cached: it is a description of the store, not a part of it, and a
    /// caller who never asks pays nothing. Empty for every file a producer writes.
    #[must_use]
    pub fn anomalies(&self) -> Vec<SheetDataAnomaly> {
        use mjx_ooxml_types::spreadsheetml::CellType;

        let mut found = Vec::new();
        let mut seen_rows: Vec<u32> = Vec::new();
        let mut previous: Option<u32> = None;
        for (position, row) in self.rows.iter().enumerate() {
            if !row.has(RowFlags::HAS_NUMBER) {
                found.push(SheetDataAnomaly::RowWithoutNumber {
                    position: position as u32,
                });
            }
            if let Some(previous) = previous {
                if row.number <= previous {
                    found.push(SheetDataAnomaly::RowsOutOfOrder {
                        row: row.number,
                        previous_row: previous,
                    });
                }
            }
            if seen_rows.contains(&row.number) {
                found.push(SheetDataAnomaly::DuplicateRowNumber { row: row.number });
            }
            seen_rows.push(row.number);
            previous = Some(row.number);

            if !row.has(RowFlags::CELLS_ASCENDING) && row.cell_count > 1 {
                found.push(SheetDataAnomaly::CellsOutOfOrder { row: row.number });
            }

            let mut seen_columns: Vec<u16> = Vec::new();
            for index in row.cell_range() {
                let cell = &self.cells[index];
                if row.has(RowFlags::HAS_NUMBER)
                    && cell.reference.row().saturating_add(1) != row.number
                {
                    found.push(SheetDataAnomaly::CellRowDisagreesWithRow {
                        cell: cell.reference,
                        row: row.number,
                    });
                }
                if seen_columns.contains(&cell.reference.column()) {
                    found.push(SheetDataAnomaly::DuplicateCellReference {
                        cell: cell.reference,
                    });
                }
                seen_columns.push(cell.reference.column());

                let declared = cell.written_cell_type().unwrap_or(CellType::Number);
                let disagrees = match cell.payload_shape() {
                    PayloadShape::InlineString => declared != CellType::InlineString,
                    PayloadShape::ValueText => declared == CellType::InlineString,
                    PayloadShape::Absent => false,
                };
                if disagrees {
                    found.push(SheetDataAnomaly::CellTypeDisagreesWithContent {
                        cell: cell.reference,
                    });
                }
            }
        }
        found
    }
}
