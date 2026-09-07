//! **The range crossing** — how a block of cells gets from a worksheet to a caller, and back,
//! without paying for a parse per cell.
//!
//! Everything else in this facade is a signature translation. This file is a design decision, and
//! [the module documentation above](super) states the measurements behind it. In one sentence:
//! `mjx_xlsx::Workbook` holds no parsed worksheet, so a per-cell call costs a whole-part parse
//! *every time*, and on a 300,000-cell sheet that is 387 ms to read one cell — so the cell door here
//! is a **range**, parsed once in each direction, and there is no per-cell door beside it.
//!
//! # The three calls
//!
//! | call | parses | serializes |
//! |---|---:|---:|
//! | [`Workbook::read_range`] | 1 | 0 |
//! | [`Workbook::read_sheet`] | 1 | 0 |
//! | [`Workbook::write_cells`] | 1 | 1 |
//!
//! Those counts are the contract, they do not depend on how many cells were asked for, and
//! `crates/mjx-ooxml/tests/workbook_boundary.rs` holds them to an **allocation** bound rather than a
//! stopwatch — a fallback to per-cell would allocate a fresh parse per cell and be caught on any
//! machine, in a debug build, with no timing flake.
//!
//! # Why a dense block and not a sparse list
//!
//! A [`CellBlock`] is row-major over the whole requested rectangle, blanks included. That is the
//! shape a binding wants (`list[list[…]]` in Python, an array of arrays in JavaScript) and the shape
//! a caller iterating a table wants. The cost is bounded by the caller's own range, and the two
//! open-ended range forms — `"A:C"` and `"1:3"` — are clamped to the sheet's populated extent
//! before anything is allocated, so `"A:A"` reads the column a file actually has rather than
//! reserving 1,048,576 slots for one that it does not.

use std::borrow::Cow;

use mjx_ooxml_types::spreadsheetml::CellType;
use mjx_sml::{CellRange, CellReference, CellValue, SharedStringTable, WorksheetPart};

use crate::error::{Error, ErrorCode};
use crate::index::index;

use super::Workbook;

/// One cell's value, as the file states it.
///
/// **Stored, not displayed.** A number comes back as the number `xl/worksheets/sheetN.xml` holds;
/// this library runs no number-format engine, so a cell formatted as `£1,234.00` and a cell
/// formatted as `1234` both answer [`Number(1234.0)`](Self::Number). A shared-string index *is*
/// resolved through `xl/sharedStrings.xml`, because that is a reference inside the package rather
/// than a rendering decision. See
/// [*Deliberate limitations*](mjx_xlsx::guide::deliberate_limitations).
///
/// **Deliberately exhaustive**, like the error enumerations below it and for the same reason: both
/// bindings map every variant onto their own language's values with a `match` that has no wildcard
/// arm, so a sixth kind of cell would be a compile error there rather than a value that silently
/// arrives as a blank.
#[derive(Debug, Clone, PartialEq)]
pub enum CellData {
    /// The cell is not populated, or holds no value element. It may still carry a style.
    Blank,
    /// `t="n"`, the schema default — a number, exactly as the file spelled it, parsed.
    Number(f64),
    /// A string: `t="s"` resolved through the shared-string table, `t="inlineStr"` read from the
    /// cell's own `<is>`, or `t="str"` — a formula's cached string result.
    Text(String),
    /// `t="b"` — `1` or `0` in the file.
    Boolean(bool),
    /// `t="e"` — an error code such as `#DIV/0!` or `#N/A`, carried verbatim. Nothing here evaluates
    /// or produces one.
    Error(String),
}

impl CellData {
    /// Whether this is [`Blank`](Self::Blank).
    #[must_use]
    pub fn is_blank(&self) -> bool {
        matches!(self, Self::Blank)
    }

    /// The number this cell holds, or `None` for every other kind.
    #[must_use]
    pub fn number(&self) -> Option<f64> {
        match self {
            Self::Number(value) => Some(*value),
            _ => None,
        }
    }

    /// The string this cell holds, or `None` for every other kind. An error code is **not** a
    /// string here: it is [`Error`](Self::Error), because `#N/A` is a value and not text somebody
    /// typed.
    #[must_use]
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Text(value) => Some(value),
            _ => None,
        }
    }

    /// The boolean this cell holds, or `None` for every other kind.
    #[must_use]
    pub fn boolean(&self) -> Option<bool> {
        match self {
            Self::Boolean(value) => Some(*value),
            _ => None,
        }
    }

    /// The error code this cell holds, or `None` for every other kind.
    #[must_use]
    pub fn error_code(&self) -> Option<&str> {
        match self {
            Self::Error(value) => Some(value),
            _ => None,
        }
    }
}

/// A value a caller writes into a cell.
///
/// Separate from [`CellData`] on purpose: reading and writing a string are not the same decision.
/// A file stores a string either in the shared-string table or in the cell, and the two are
/// different bytes with different consequences — so the caller says which, rather than this library
/// picking one and calling it the default.
///
/// **Deliberately exhaustive**, for the reason [`CellData`] states.
#[derive(Debug, Clone, PartialEq)]
pub enum CellInput {
    /// Remove the value, keeping the cell (and therefore its style).
    Blank,
    /// A number. `NaN` and the infinities are refused with [`ErrorCode::InvalidArgument`]:
    /// SpreadsheetML has no spelling for them, and Excel writes `Error("#NUM!")` instead.
    Number(f64),
    /// A string interned into `xl/sharedStrings.xml` and referenced by index — **what Excel itself
    /// writes**, and what to reach for when the same text appears in many cells.
    SharedText(String),
    /// A string stored in the cell itself as an `inlineStr`. No other part is touched, which is what
    /// to reach for when a workbook has no shared-string table and should not gain one.
    InlineText(String),
    /// A boolean, written `1` or `0`.
    Boolean(bool),
    /// An error code — `#DIV/0!`, `#N/A`. Carried verbatim.
    Error(String),
}

/// One entry of a [`Workbook::write_cells`] batch: where, and what.
#[derive(Debug, Clone, PartialEq)]
pub struct CellWrite {
    /// The cell, in A1 text: `"B7"`, or `"$B$7"` — the anchoring is data the file keeps, and either
    /// spelling names the same cell.
    pub reference: String,
    /// What to write there.
    pub value: CellInput,
}

impl CellWrite {
    /// A write of `value` into the cell `reference` names.
    #[must_use]
    pub fn new(reference: impl Into<String>, value: CellInput) -> Self {
        Self {
            reference: reference.into(),
            value,
        }
    }
}

/// A rectangular block of cell values, read in one pass over one parse of the worksheet.
///
/// Row-major over the whole rectangle, blanks included, so `row`/`column` below are **offsets into
/// the block** and not sheet coordinates — [`first_row`](Self::first_row) and
/// [`first_column`](Self::first_column) say where the block sits on the sheet.
#[derive(Debug, Clone, PartialEq)]
pub struct CellBlock {
    first_row: u32,
    first_column: u32,
    row_count: u32,
    column_count: u32,
    values: Vec<CellData>,
    /// The offset into `values`, and the `<f>` text, for every cell that carries a formula, in offset
    /// order. Sparse, because most cells in most sheets carry none, and a parallel dense `Vec` would
    /// double a block's cost to say "no" a hundred thousand times.
    formulas: Vec<(u32, String)>,
}

impl CellBlock {
    /// The zero-based sheet row the block's first row is: `0` is the row a file spells `1`.
    #[must_use]
    pub fn first_row(&self) -> u32 {
        self.first_row
    }

    /// The zero-based sheet column the block's first column is: `0` is `A`.
    #[must_use]
    pub fn first_column(&self) -> u32 {
        self.first_column
    }

    /// How many rows the block covers.
    #[must_use]
    pub fn row_count(&self) -> u32 {
        self.row_count
    }

    /// How many columns the block covers.
    #[must_use]
    pub fn column_count(&self) -> u32 {
        self.column_count
    }

    /// Whether the block covers no cells at all — an empty sheet read by
    /// [`Workbook::read_sheet`], above all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// The value at `row`/`column`, **as offsets into the block**.
    ///
    /// # Errors
    /// [`ErrorCode::IndexOutOfRange`] if either offset is outside the block.
    pub fn value(&self, row: u32, column: u32) -> Result<&CellData, Error> {
        self.values
            .get(self.offset(row, column)?)
            .ok_or_else(|| self.out_of_range(row, column))
    }

    /// The formula text at `row`/`column`, or `None` when that cell carries none.
    ///
    /// **Exactly as the file wrote it**, never expanded and never evaluated — a shared formula's
    /// follower answers `None`, because the follower writes no `<f>` text of its own. See
    /// [*Formulas and cached values*](mjx_xlsx::guide::formulas_and_cached_values); the value a
    /// formula last produced is the [`value`](Self::value) beside it.
    ///
    /// # Errors
    /// [`ErrorCode::IndexOutOfRange`] if either offset is outside the block.
    pub fn formula(&self, row: u32, column: u32) -> Result<Option<&str>, Error> {
        let offset = u32::try_from(self.offset(row, column)?).unwrap_or(u32::MAX);
        Ok(self
            .formulas
            .binary_search_by_key(&offset, |(at, _)| *at)
            .ok()
            .map(|found| self.formulas[found].1.as_str()))
    }

    /// The whole block as rows of values, top to bottom, left to right.
    ///
    /// The shape a binding hands to its own language — a `list[list[…]]`, an array of arrays — and
    /// the reason this type exists rather than a flat `Vec`.
    #[must_use]
    pub fn into_rows(self) -> Vec<Vec<CellData>> {
        let columns = index(self.column_count).max(1);
        self.values
            .chunks(columns)
            .map(<[CellData]>::to_vec)
            .collect()
    }

    /// The A1 text of the range this block covers, or `None` when it covers nothing.
    #[must_use]
    pub fn range(&self) -> Option<String> {
        if self.values.is_empty() {
            return None;
        }
        let start = CellReference::relative(column_of(self.first_column), self.first_row).ok()?;
        let end = CellReference::relative(
            column_of(self.first_column + self.column_count - 1),
            self.first_row + self.row_count - 1,
        )
        .ok()?;
        Some(format!("{}:{}", start.text().as_str(), end.text().as_str()))
    }

    fn offset(&self, row: u32, column: u32) -> Result<usize, Error> {
        if row >= self.row_count || column >= self.column_count {
            return Err(self.out_of_range(row, column));
        }
        Ok(index(row) * index(self.column_count) + index(column))
    }

    fn out_of_range(&self, row: u32, column: u32) -> Error {
        Error::with_cell(
            ErrorCode::IndexOutOfRange,
            format!(
                "({row}, {column}) is outside a block of {} row(s) by {} column(s)",
                self.row_count, self.column_count
            ),
            row,
            column,
        )
    }
}

/// A `u32` column index as the `u16` the grid addresses with, saturating at the last column.
fn column_of(value: u32) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}

impl Workbook {
    /// Reads every cell of one rectangle, parsing the worksheet **once**.
    ///
    /// `range` is A1 text: `"A1"` for one cell, `"A1:C3"` for a rectangle, `"A:C"` for whole
    /// columns, `"1:3"` for whole rows. The two open-ended forms are clamped to the sheet's
    /// populated extent, so `"A:A"` costs the rows the file actually has rather than the 1,048,576
    /// it could have.
    ///
    /// This is **the** way to read cells through this facade, and there is deliberately no per-cell
    /// call beside it: see [the module documentation](super) for the measurements that decided that.
    /// Reading never dirties the package — [`save`](Self::save) still re-emits every part verbatim.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), mjx_ooxml::Error> {
    /// # let workbook = mjx_ooxml::Workbook::blank()?;
    /// let block = workbook.read_range(0, "A1:B2")?;
    /// for row in block.into_rows() {
    ///     println!("{row:?}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    /// - [`ErrorCode::IndexOutOfRange`] if `sheet` names no tab.
    /// - [`ErrorCode::InvalidArgument`] if `range` is not an A1 range.
    /// - [`ErrorCode::NothingToRead`] if that tab is a chartsheet or a dialogsheet, or reaches no
    ///   part at all — there are no cells there to read.
    /// - [`ErrorCode::MalformedDocument`] if the worksheet part is not well-formed SpreadsheetML.
    pub fn read_range(&self, sheet: u32, range: &str) -> Result<CellBlock, Error> {
        let range = CellRange::parse(range)?;
        let markup = self.worksheet_or_refuse(sheet)?;
        let bounds = range.normalized_bounds();
        let (first_row, last_row, first_column, last_column) = match range {
            CellRange::Cell(_) | CellRange::Cells { .. } => (
                bounds.first_row(),
                bounds.last_row(),
                u32::from(bounds.first_column()),
                u32::from(bounds.last_column()),
            ),
            // A whole-column or whole-row form spans an axis the file does not fill. Clamp the open
            // axis to what is populated before anything is sized, so `"A:A"` is a column and not a
            // million blanks.
            CellRange::Columns { .. } => {
                let Some((first, last)) = populated_rows(&markup) else {
                    return Ok(empty_block());
                };
                (
                    first,
                    last,
                    u32::from(bounds.first_column()),
                    u32::from(bounds.last_column()),
                )
            }
            CellRange::Rows { .. } => {
                let Some((first, last)) = populated_columns(&markup) else {
                    return Ok(empty_block());
                };
                (
                    bounds.first_row(),
                    bounds.last_row(),
                    u32::from(first),
                    u32::from(last),
                )
            }
        };
        self.block_between(&markup, first_row, last_row, first_column, last_column)
    }

    /// Reads every populated cell of one sheet, parsing the worksheet **once**.
    ///
    /// The block is the sheet's populated extent — the smallest rectangle containing every cell the
    /// file actually holds — so an empty sheet answers a block with no rows and no columns rather
    /// than an error.
    ///
    /// # Errors
    /// As [`read_range`](Self::read_range), minus the range-parsing case.
    pub fn read_sheet(&self, sheet: u32) -> Result<CellBlock, Error> {
        let markup = self.worksheet_or_refuse(sheet)?;
        let (Some((first_row, last_row)), Some((first_column, last_column))) =
            (populated_rows(&markup), populated_columns(&markup))
        else {
            return Ok(empty_block());
        };
        self.block_between(
            &markup,
            first_row,
            last_row,
            u32::from(first_column),
            u32::from(last_column),
        )
    }

    /// The A1 range of one sheet's populated extent, or `None` when nothing is populated.
    ///
    /// Computed from the cells the file holds, **not** read from `x:dimension`: a producer's cached
    /// box is preserved by this library exactly as written and may be stale, so answering from it
    /// would report what some other application once believed rather than what is there.
    ///
    /// # Errors
    /// As [`read_range`](Self::read_range), minus the range-parsing case.
    pub fn used_range(&self, sheet: u32) -> Result<Option<String>, Error> {
        let markup = self.worksheet_or_refuse(sheet)?;
        let (Some((first_row, last_row)), Some((first_column, last_column))) =
            (populated_rows(&markup), populated_columns(&markup))
        else {
            return Ok(None);
        };
        let start = CellReference::relative(first_column, first_row)?;
        let end = CellReference::relative(last_column, last_row)?;
        Ok(Some(format!(
            "{}:{}",
            start.text().as_str(),
            end.text().as_str()
        )))
    }

    /// Writes every entry of `cells` into one sheet, parsing the worksheet **once** and serializing
    /// it **once**.
    ///
    /// This is **the** way to write cells through this facade, and there is deliberately no
    /// per-cell call beside it. Writing four thousand cells one
    /// [`mjx_xlsx::Workbook::set_cell_value`] at a time takes 18.39 s on the workbook
    /// `docs/BENCHMARKS.md` describes, against 1.12 ms for one read, four thousand edits and one
    /// write — the same file, byte for byte, 16,427× apart.
    ///
    /// Entries are applied **in the order given**, so a reference written twice keeps the last
    /// value. Prefer top-to-bottom, left-to-right order: that is append-only in the cell arena,
    /// where an out-of-order insertion moves the arena's tail.
    ///
    /// Every row, cell and worksheet child the batch does not name is left byte-identical — the
    /// isolation is [`mjx_sml::WorksheetPart`]'s slot-level copy-on-write, not this method's doing.
    ///
    /// # Writing over a formula
    ///
    /// A cell that carries an `<f>` **keeps it**, and only its cached `<v>` is replaced. So writing
    /// `50` into a cell holding `=A2*2` leaves a file that still says `=A2*2` and now caches `50`,
    /// and Excel computes the formula's own answer over that cache the next time it recalculates —
    /// the written value does not survive.
    ///
    /// That is the same decision every other formula question on this surface follows, and the
    /// reason is that the alternative is worse: dropping the `<f>` would destroy a formula the
    /// caller did not name in a file they opened to change a number, and it cannot be undone.
    /// [`CellBlock::formula`] is how to see one before writing over it, and
    /// [*Deliberate limitations*](mjx_xlsx::guide::deliberate_limitations) is why there is no
    /// calculation engine behind either.
    ///
    /// # Errors
    /// - [`ErrorCode::IndexOutOfRange`] if `sheet` names no tab.
    /// - [`ErrorCode::InvalidArgument`] if a reference is not an A1 cell, or a
    ///   [`CellInput::Number`] is `NaN` or infinite. **Nothing is written** in either case: every
    ///   reference is parsed before the worksheet is opened.
    /// - [`ErrorCode::NothingToRead`] if that tab reaches no worksheet part.
    /// - [`ErrorCode::MalformedDocument`] if the worksheet part is not well-formed SpreadsheetML.
    pub fn write_cells(&mut self, sheet: u32, cells: &[CellWrite]) -> Result<(), Error> {
        // Parse every address first: a batch that would fail halfway is refused before the package
        // is touched at all, so a failed `write_cells` leaves the workbook exactly as it was.
        let references = cells
            .iter()
            .map(|write| CellReference::parse(&write.reference).map_err(Error::from))
            .collect::<Result<Vec<_>, Error>>()?;

        // Interning happens before the worksheet is read, because it edits a *different* part
        // (`xl/sharedStrings.xml`) and would otherwise need the workbook borrowed twice.
        let mut shared = Vec::with_capacity(cells.len());
        for write in cells {
            shared.push(match &write.value {
                CellInput::SharedText(text) => Some(self.workbook.intern_shared_string(text)?),
                _ => None,
            });
        }

        let mut markup = self.worksheet_or_refuse(sheet)?;
        for ((write, reference), interned) in cells.iter().zip(references).zip(shared) {
            let value = match (&write.value, interned) {
                (CellInput::Blank, _) => CellValue::Blank,
                (CellInput::Number(number), _) => CellValue::Number(*number),
                (CellInput::SharedText(_), Some(index)) => CellValue::SharedString(index),
                // Unreachable: `interned` is `Some` for exactly the `SharedText` entries, built in
                // the same order two loops above. Written as a value rather than an `unwrap` because
                // this crate takes untrusted input and no path in it may panic.
                (CellInput::SharedText(text), None) => CellValue::InlineString(text),
                (CellInput::InlineText(text), _) => CellValue::InlineString(text),
                (CellInput::Boolean(value), _) => CellValue::Boolean(*value),
                (CellInput::Error(code), _) => CellValue::Error(code),
            };
            markup
                .set_cell_value(reference, value)
                .map_err(Error::from)?;
        }
        self.workbook
            .write_worksheet_markup(index(sheet), &markup)?;
        Ok(())
    }

    /// The worksheet behind `sheet`, or the typed refusal a caller can act on.
    ///
    /// `mjx_xlsx::Workbook::worksheet_markup` answers `Ok(None)` for two different situations — a
    /// tab reaching no part, and a tab that is a chartsheet or a dialogsheet — and both mean the
    /// same thing to a cell reader: there are no cells there. [`ErrorCode::NothingToRead`] is that
    /// statement, and it is a different code from the [`ErrorCode::IndexOutOfRange`] a tab that does
    /// not exist gets.
    pub(super) fn worksheet_or_refuse(&self, sheet: u32) -> Result<WorksheetPart, Error> {
        self.workbook
            .worksheet_markup(index(sheet))?
            .ok_or_else(|| {
                Error::with_index(
                    ErrorCode::NothingToRead,
                    format!(
                        "the tab at index {sheet} holds no worksheet: it reaches no part, or the \
                         part it reaches is a chartsheet or a dialogsheet"
                    ),
                    sheet,
                )
            })
    }

    /// Fills one rectangle from an already-parsed worksheet — the half of [`read_range`] and
    /// [`read_sheet`] that is common to both, so neither can drift from the other.
    fn block_between(
        &self,
        markup: &WorksheetPart,
        first_row: u32,
        last_row: u32,
        first_column: u32,
        last_column: u32,
    ) -> Result<CellBlock, Error> {
        if last_row < first_row || last_column < first_column {
            return Ok(empty_block());
        }
        let row_count = last_row - first_row + 1;
        let column_count = last_column - first_column + 1;
        let cells = index(row_count)
            .checked_mul(index(column_count))
            .ok_or_else(|| {
                Error::new(
                    ErrorCode::InvalidArgument,
                    "the requested range has more cells than this platform can address",
                )
            })?;
        let mut values = vec![CellData::Blank; cells];
        let mut formulas = Vec::new();

        // The shared-string table is read **at most once per call**, and only when a `t="s"` cell is
        // actually in the rectangle: a numeric block must not pay for a part it never reads.
        let mut table: Option<Option<SharedStringTable>> = None;

        // One pass over the populated cells, rather than a lookup per position in the rectangle: a
        // sheet holds far fewer cells than a rectangle has positions, and a lookup per position
        // would make a sparse read cost the rectangle's area.
        for cell in markup.cells() {
            let reference = cell.reference();
            let row = reference.row();
            let column = u32::from(reference.column());
            if row < first_row || row > last_row || column < first_column || column > last_column {
                continue;
            }
            let offset =
                index(row - first_row) * index(column_count) + index(column - first_column);
            let Some(slot) = values.get_mut(offset) else {
                continue;
            };
            *slot = match cell.cell_type() {
                CellType::Number => match cell.number() {
                    Some(number) => CellData::Number(number),
                    None => CellData::Blank,
                },
                CellType::Boolean => match cell.boolean() {
                    Some(value) => CellData::Boolean(value),
                    None => CellData::Blank,
                },
                CellType::Error => match cell.value().map_err(mjx_sml::SmlError::from)? {
                    Some(code) => CellData::Error(code.into_owned()),
                    None => CellData::Blank,
                },
                CellType::FormulaString => match cell.value().map_err(mjx_sml::SmlError::from)? {
                    Some(text) => CellData::Text(text.into_owned()),
                    None => CellData::Blank,
                },
                CellType::InlineString => match cell.inline_string_markup() {
                    Some(markup) => {
                        let string = mjx_sml::InlineString::parse(markup)?;
                        let text = string.item().text().map_err(mjx_sml::SmlError::from)?;
                        CellData::Text(text.into_owned())
                    }
                    None => CellData::Blank,
                },
                CellType::SharedString => {
                    let Some(at) = cell.shared_string_index() else {
                        continue;
                    };
                    let table = match &table {
                        Some(table) => table.as_ref(),
                        None => {
                            table = Some(self.workbook.shared_strings()?);
                            table.as_ref().and_then(Option::as_ref)
                        }
                    };
                    // A `t="s"` naming no entry is a defect in the file, reported as a blank rather
                    // than repaired — the same reading `mjx_xlsx::Workbook::cell_text` gives it.
                    match table.and_then(|table| table.item(at)) {
                        Some(item) => CellData::Text(
                            item.text().map_err(mjx_sml::SmlError::from)?.into_owned(),
                        ),
                        None => CellData::Blank,
                    }
                }
            };
            if let Some(formula) = cell.formula() {
                if let Some(text) = formula_text(&formula) {
                    formulas.push((u32::try_from(offset).unwrap_or(u32::MAX), text));
                }
            }
        }
        // `markup.cells()` walks rows in document order and cells within a row in document order, so
        // offsets arrive ascending for a well-ordered file — but a file may write its rows out of
        // order, and `CellBlock::formula` binary-searches. Sorting is what makes the search sound.
        formulas.sort_unstable_by_key(|(at, _)| *at);
        Ok(CellBlock {
            first_row,
            first_column,
            row_count,
            column_count,
            values,
            formulas,
        })
    }
}

/// A block covering nothing — what an empty sheet, or a range clamped to an empty axis, answers.
fn empty_block() -> CellBlock {
    CellBlock {
        first_row: 0,
        first_column: 0,
        row_count: 0,
        column_count: 0,
        values: Vec::new(),
        formulas: Vec::new(),
    }
}

/// The `<f>` text of a formula, or `None` when it writes none — a shared-formula follower, above
/// all, which states its group and nothing else.
fn formula_text(formula: &mjx_sml::CellFormula<'_>) -> Option<String> {
    match formula.text() {
        Ok(Cow::Borrowed("")) => None,
        Ok(text) => Some(text.into_owned()),
        Err(_) => None,
    }
}

/// The first and last populated row of a sheet, or `None` when it holds no cells.
fn populated_rows(markup: &WorksheetPart) -> Option<(u32, u32)> {
    bounds(markup, CellReference::row)
}

/// The first and last populated column of a sheet, or `None` when it holds no cells.
fn populated_columns(markup: &WorksheetPart) -> Option<(u16, u16)> {
    bounds(markup, mjx_sml::CellReference::column)
}

/// The minimum and maximum of one axis over the sheet's populated cells.
///
/// Over the **cells** rather than over the rows, because a row element with no cells in it is not a
/// populated row and must not widen the extent a caller then iterates.
fn bounds<T: Ord + Copy>(
    markup: &WorksheetPart,
    axis: impl Fn(CellReference) -> T,
) -> Option<(T, T)> {
    let mut seen: Option<(T, T)> = None;
    for cell in markup.cells() {
        let value = axis(cell.reference());
        seen = Some(match seen {
            None => (value, value),
            Some((low, high)) => (low.min(value), high.max(value)),
        });
    }
    seen
}
