//! What the sheet's *grid* says that a well-formed sheet would not — reported, never repaired.
//!
//! [`SheetDataAnomaly`](crate::SheetDataAnomaly) does this for the cells; this does it for
//! everything around them: the merges, the column runs, and the outline maxima. The two are separate
//! types because they are separate questions, computed over separate structures, and a caller that
//! only wants one should not pay for the other.
//!
//! # Why a report exists rather than a repair
//!
//! Every one of these is real in files Excel wrote or repaired, and every one of them is
//! **preserved exactly as read**. A merge overlapping another, a merge laid over populated cells, an
//! `outlineLevelRow` that no row reaches, two `col` runs claiming the same column: correcting any of
//! them would rewrite the bytes of a part nobody asked to edit, which is the one thing this library
//! exists not to do. *A helpful correction the caller did not ask for is the defect.*
//!
//! But "preserved as read" is indistinguishable from "not noticed" unless something can say what was
//! preserved. This is that something: computed on demand, changing nothing, and paid for only by a
//! caller who asks.
//!
//! # The mutation surface is where the refusals live
//!
//! Nothing here is an error, and that is not a softening. The same shapes *are* refused where this
//! library would be the author: [`WorksheetPart::merge_cells`](crate::WorksheetPart::merge_cells)
//! returns [`SmlError::MergeOverlapsExistingMerge`](crate::SmlError) rather than writing an overlap,
//! and [`set_row_outline_level`](crate::WorksheetPart::set_row_outline_level) raises the maximum
//! rather than writing a level past it. Refusing to *author* a shape and refusing to *read* one are
//! different acts, and only the first is this library's business.

use crate::address::{CellRange, CellReference};
use crate::error::SmlError;

use super::frame::WorksheetPart;

/// Something the sheet's grid says that a well-formed one would not.
///
/// Every one is preserved and written back as it stands. None is an error, and none changes what the
/// worksheet answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum GridAnomaly {
    /// A `mergeCell@ref` is absent or does not parse, so that merge cannot be reasoned about at all.
    ///
    /// The element is preserved; every merge query
    /// ([`merged_ranges`](crate::WorksheetPart::merged_ranges) and the four built on it) reports a
    /// typed error instead of quietly answering from a shortened list.
    MergeReferenceUnreadable {
        /// The position of the offending `mergeCell` among its siblings.
        index: usize,
    },
    /// Two merged ranges share at least one cell. Excel repairs such a file on open; both are kept
    /// here, and the queries answer with whichever comes first in document order.
    MergesOverlap {
        /// The earlier of the two, in document order.
        first: CellRange,
        /// The later.
        second: CellRange,
    },
    /// A merged range covers exactly one cell, which merges nothing.
    DegenerateMerge {
        /// The range.
        range: CellRange,
    },
    /// A cell inside a merged range, other than its top-left, holds a value.
    ///
    /// ECMA-376 Part 1 §18.3.1.55: *"The formatting and content for the merged range is always stored
    /// in the top left cell."* Excel shows only the top-left cell's content and the rest is
    /// unreachable in its user interface — but it is in the file, and it is readable here.
    MergeInteriorCellHasValue {
        /// The merged range.
        merge: CellRange,
        /// The covered cell that holds a value.
        cell: CellReference,
    },
    /// `mergeCells@count` disagrees with the number of `mergeCell` children.
    MergeCountDisagrees {
        /// What the attribute claims.
        declared: u32,
        /// How many children there are.
        actual: usize,
    },
    /// A `col` run writes `@min` greater than `@max`, so the run it describes is empty.
    ColumnRunBoundsInverted {
        /// `@min`, as written.
        first_column: u32,
        /// `@max`, as written.
        last_column: u32,
    },
    /// Two `col` runs claim the same column — within one `cols` block or across two of them.
    ///
    /// Which one wins is not something `sml.xsd` or ECMA-376 Part 1 settles, so nothing here
    /// chooses: [`column_run_covering`](crate::WorksheetPart::column_run_covering) answers with the
    /// first in document order and says so.
    ColumnRunsOverlap {
        /// The earlier run's `@min` and `@max`, one-based as the wire writes them.
        first: (u32, u32),
        /// The later run's.
        second: (u32, u32),
    },
    /// A row states an outline level deeper than `sheetFormatPr@outlineLevelRow` declares.
    ///
    /// ECMA-376 Part 1 says of that attribute that *"these values shall be in synch with the actual
    /// sheet outline levels"*. Reported only when the sheet has a `sheetFormatPr` at all: a sheet
    /// with none has declared nothing to disagree with.
    RowOutlineLevelPastDeclaredMaximum {
        /// The deepest level a row states.
        deepest: u8,
        /// What `outlineLevelRow` claims.
        declared: u8,
    },
    /// A `col` run states an outline level deeper than `sheetFormatPr@outlineLevelCol` declares.
    ColumnOutlineLevelPastDeclaredMaximum {
        /// The deepest level a run states.
        deepest: u8,
        /// What `outlineLevelCol` claims.
        declared: u8,
    },
}

impl WorksheetPart {
    /// Everything this sheet's grid says that a well-formed one would not, in a stable order:
    /// merges, then column runs, then the outline maxima.
    ///
    /// Computed on demand and never cached — it is a description of the part, not a part of it.
    /// Empty for every file a producer writes.
    ///
    /// # Errors
    /// Never for a shape it can describe. [`SmlError::Model`] only if a `col` is missing `@min` or
    /// `@max` entirely, which is the one thing here that stops the *description* rather than being
    /// part of it: a run with no bounds names no columns, so there is nothing to compare it against.
    /// An unreadable `mergeCell@ref` is a described anomaly rather than an error, which is the whole
    /// difference between this call and [`merged_ranges`](Self::merged_ranges).
    #[allow(clippy::missing_panics_doc)]
    pub fn grid_anomalies(&self) -> Result<Vec<GridAnomaly>, SmlError> {
        let mut found = Vec::new();
        self.describe_merges(&mut found);
        self.describe_column_runs(&mut found)?;
        self.describe_outline_maxima(&mut found)?;
        Ok(found)
    }

    /// The merge half: unreadable references, overlaps, degenerate ranges, populated interiors, and
    /// a `@count` that disagrees.
    fn describe_merges(&self, found: &mut Vec<GridAnomaly>) {
        let Some(merges) = self.merged_cells() else {
            return;
        };
        let mut ranges: Vec<CellRange> = Vec::new();
        for (index, merge) in merges.merges().enumerate() {
            match merge.range(self.interner()) {
                Ok(range) => ranges.push(range),
                Err(_) => found.push(GridAnomaly::MergeReferenceUnreadable { index }),
            }
        }

        for (position, range) in ranges.iter().enumerate() {
            let bounds = range.normalized_bounds();
            if bounds.cell_count() == 1 {
                found.push(GridAnomaly::DegenerateMerge { range: *range });
            }
            for later in &ranges[position + 1..] {
                let other = later.normalized_bounds();
                let overlaps = bounds.first_column() <= other.last_column()
                    && other.first_column() <= bounds.last_column()
                    && bounds.first_row() <= other.last_row()
                    && other.first_row() <= bounds.last_row();
                if overlaps {
                    found.push(GridAnomaly::MergesOverlap {
                        first: *range,
                        second: *later,
                    });
                }
            }
        }

        for cell in self.cells() {
            let reference = cell.reference();
            let has_value = cell.raw_value().is_some() || cell.inline_string_markup().is_some();
            if !has_value {
                continue;
            }
            for range in &ranges {
                let bounds = range.normalized_bounds();
                let covered = bounds.contains(reference)
                    && (bounds.first_column() != reference.column()
                        || bounds.first_row() != reference.row());
                if covered {
                    found.push(GridAnomaly::MergeInteriorCellHasValue {
                        merge: *range,
                        cell: reference,
                    });
                }
            }
        }

        if let Ok(Some(declared)) = merges.declared_count(self.interner()) {
            let actual = merges.len();
            if u64::from(declared) != actual as u64 {
                found.push(GridAnomaly::MergeCountDisagrees { declared, actual });
            }
        }
    }

    /// The column half: inverted bounds, and two runs claiming one column.
    fn describe_column_runs(&self, found: &mut Vec<GridAnomaly>) -> Result<(), SmlError> {
        let mut seen: Vec<(u32, u32)> = Vec::new();
        for block in self.column_blocks() {
            for run in block.runs() {
                let first = run
                    .first_column(self.interner())
                    .map_err(mjx_ooxml_core::FromXmlError::from)?;
                let last = run
                    .last_column(self.interner())
                    .map_err(mjx_ooxml_core::FromXmlError::from)?;
                if first > last {
                    found.push(GridAnomaly::ColumnRunBoundsInverted {
                        first_column: first,
                        last_column: last,
                    });
                }
                let bounds = (first.min(last), first.max(last));
                for earlier in &seen {
                    if earlier.0 <= bounds.1 && bounds.0 <= earlier.1 {
                        found.push(GridAnomaly::ColumnRunsOverlap {
                            first: *earlier,
                            second: bounds,
                        });
                    }
                }
                seen.push(bounds);
            }
        }
        Ok(())
    }

    /// The outline half: a level deeper than the maximum `sheetFormatPr` declares.
    ///
    /// Silent for a sheet with no `sheetFormatPr`: an absent element declares nothing, and treating
    /// the schema's `0` default as a claim the file made would report every outlined sheet that
    /// omits the element.
    fn describe_outline_maxima(&self, found: &mut Vec<GridAnomaly>) -> Result<(), SmlError> {
        let Some(format) = self.format_properties() else {
            return Ok(());
        };
        let (rows, columns) = self.outline_levels_in_use()?;
        let declared_rows = format
            .deepest_row_outline_level(self.interner())
            .unwrap_or(0);
        if rows > declared_rows {
            found.push(GridAnomaly::RowOutlineLevelPastDeclaredMaximum {
                deepest: rows,
                declared: declared_rows,
            });
        }
        let declared_columns = format
            .deepest_column_outline_level(self.interner())
            .unwrap_or(0);
        if columns > declared_columns {
            found.push(GridAnomaly::ColumnOutlineLevelPastDeclaredMaximum {
                deepest: columns,
                declared: declared_columns,
            });
        }
        Ok(())
    }
}
