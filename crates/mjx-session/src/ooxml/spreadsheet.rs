//! [`SpreadsheetSession`] — an `.xlsx` held open, worksheet models and all.

use std::collections::BTreeMap;

use mjx_layout::SourceRef;
use mjx_ooxml_types::spreadsheetml::CellType;
use mjx_sml::{CellReference, CellValue, InlineString, WorksheetPart};
use mjx_xlsx::Workbook;

use crate::document::{Applied, Committed, Invalidation, ResidentDocument};
use crate::error::SessionError;
use crate::ooxml::DirtyEstimate;
use crate::operation::{Operation, OperationKind, Value};

/// A workbook held open, with the worksheet models resident between operations.
///
/// # This is the residency the ticket was written about
///
/// `crates/mjx-xlsx/docs/guide/large_workbooks.md` says it in its own words: *"A worksheet is cheap
/// to hold and expensive to open, and this library holds nothing between calls… every call that
/// reaches into a sheet's cells parses that sheet's part again."* `Workbook::set_cell_value` is
/// exactly that shape — it reads the part's bytes, parses a `WorksheetPart`, edits one cell, emits
/// the whole sheet back over the part, and drops the model. Twenty cell edits are twenty parses and
/// **twenty serialisations of a whole worksheet**, which is the most expensive thing in the
/// workspace to do twenty times.
///
/// So this type keeps the [`WorksheetPart`] for every sheet it has touched. An edit parses nothing
/// and serialises nothing; the commit writes each touched sheet back **once**. That is where the
/// batching actually pays, and `tests/batching.rs` measures it against the naive policy on this very
/// residency.
///
/// # The addressing scheme
///
/// | Piece | Meaning |
/// |---|---|
/// | [`PartId`](mjx_layout::PartId) | which sheet, in the order the workbook lists its tabs |
/// | path segment `0` | the zero-based row |
/// | path segment `1` | the zero-based column |
///
/// So `[6, 2]` under part `0` is `C7` on the first tab. A cell operation needs exactly two segments.
/// [`OperationKind::SetBounds`] is refused: a cell has no box, and a sheet's drawings are addressed
/// through their own anchors rather than through the grid.
///
/// # Exactness, and the two cells this refuses
///
/// Every [`Value`] maps onto a `CellValue` and back with no loss — including a shared-string index,
/// which is restored as the *index* rather than as its text, and an exact numeric spelling. Two
/// cells are refused rather than approximated:
///
/// * one carrying a **formula**, because a value operation cannot say what should happen to the
///   `<f>`, and silently keeping a formula whose cached value the user just replaced writes a file
///   that says two different things; and
/// * one holding a **rich inline string** — an `<is>` with formatting runs — because
///   [`Value::Text`] can only put back the plain text. The check is the honest one: the markup is
///   compared against what a plain inline string of the same text would emit, so an `<is>` that is
///   plain is edited and one that is not is refused.
pub struct SpreadsheetSession {
    workbook: Workbook,
    /// The parsed model of every sheet this session has touched, by tab index.
    resident: BTreeMap<usize, WorksheetPart>,
    /// Which of those have unwritten edits — the dirty set the commit walks.
    dirty_sheets: Vec<usize>,
    dirty: DirtyEstimate,
}

impl std::fmt::Debug for SpreadsheetSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpreadsheetSession")
            .field("resident_sheets", &self.resident.keys().collect::<Vec<_>>())
            .field("dirty_sheets", &self.dirty_sheets)
            .field("dirty_bytes", &self.dirty.bytes())
            .finish_non_exhaustive()
    }
}

impl SpreadsheetSession {
    /// Holds `workbook` open.
    #[must_use]
    pub fn new(workbook: Workbook) -> Self {
        Self {
            workbook,
            resident: BTreeMap::new(),
            dirty_sheets: Vec::new(),
            dirty: DirtyEstimate::default(),
        }
    }

    /// Opens a workbook from its container bytes and holds it open.
    ///
    /// # Errors
    /// [`SessionError::Document`] carrying the `XlsxError` if the package is unreadable.
    pub fn open(bytes: &[u8]) -> Result<Self, SessionError> {
        Workbook::open(bytes)
            .map(Self::new)
            .map_err(SessionError::document)
    }

    /// The workbook, for reading.
    ///
    /// **A sheet this session has edited will not read back through here until the next commit**,
    /// because the edit is in the resident model and the package still holds the bytes it was opened
    /// with. That is residency working, not a bug; read edited cells through
    /// [`cell_value`](Self::cell_value).
    #[must_use]
    pub const fn workbook(&self) -> &Workbook {
        &self.workbook
    }

    /// The workbook, mutably. Anything done through here is not journalled — see
    /// [`Session::document_mut`](crate::Session::document_mut).
    pub fn workbook_mut(&mut self) -> &mut Workbook {
        &mut self.workbook
    }

    /// Gives the workbook back, after writing every resident sheet into it.
    ///
    /// # Errors
    /// [`SessionError::Document`] if a sheet will not write back.
    pub fn into_workbook(mut self) -> Result<Workbook, SessionError> {
        self.write_back_dirty_sheets()?;
        Ok(self.workbook)
    }

    /// How many sheets are held parsed in memory.
    #[must_use]
    pub fn resident_sheets(&self) -> usize {
        self.resident.len()
    }

    /// The value of one cell, read from the resident model when there is one.
    ///
    /// # Errors
    /// [`SessionError`] if the sheet does not exist or its part will not parse.
    pub fn cell_value(
        &mut self,
        sheet: usize,
        row: u32,
        column: u16,
    ) -> Result<Value, SessionError> {
        let reference = reference(row, column)?;
        let markup = self.sheet_mut(sheet)?;
        prior_value(markup, reference, sheet, row, column)
    }

    /// The resident model for `sheet`, parsing the part on first use.
    fn sheet_mut(&mut self, sheet: usize) -> Result<&mut WorksheetPart, SessionError> {
        if !self.resident.contains_key(&sheet) {
            let markup = self
                .workbook
                .worksheet_markup(sheet)
                .map_err(SessionError::document)?
                .ok_or_else(|| {
                    SessionError::no_such_node(format!(
                        "sheet {sheet} — the tab reaches no worksheet part"
                    ))
                })?;
            self.resident.insert(sheet, markup);
        }
        self.resident
            .get_mut(&sheet)
            .ok_or_else(|| SessionError::no_such_node(format!("sheet {sheet}")))
    }

    fn mark_dirty(&mut self, sheet: usize) {
        if !self.dirty_sheets.contains(&sheet) {
            self.dirty_sheets.push(sheet);
        }
    }

    /// Writes every dirty resident sheet back over its part, and reports how many it wrote.
    ///
    /// **This is the serialisation the whole design defers**, and it happens once per dirty sheet
    /// per commit however many cells were edited.
    fn write_back_dirty_sheets(&mut self) -> Result<usize, SessionError> {
        let dirty = std::mem::take(&mut self.dirty_sheets);
        let written = dirty.len();
        for sheet in dirty {
            let Some(markup) = self.resident.get(&sheet) else {
                continue;
            };
            self.workbook
                .write_worksheet_markup(sheet, markup)
                .map_err(SessionError::document)?;
        }
        Ok(written)
    }
}

impl ResidentDocument for SpreadsheetSession {
    fn apply(&mut self, operation: &Operation) -> Result<Applied, SessionError> {
        let address = operation.address();
        let OperationKind::SetValue(value) = operation.kind() else {
            return Err(SessionError::unsupported(
                "place a box on a spreadsheet cell — a cell has no rectangle",
            ));
        };
        let (sheet, row, column) = cell_address(address)?;
        let reference = reference(row, column)?;
        let markup = self.sheet_mut(sheet)?;
        let was = prior_value(markup, reference, sheet, row, column)?;
        markup
            .set_cell_value(reference, cell_value(value))
            .map_err(SessionError::document)?;
        self.mark_dirty(sheet);
        self.dirty.note(operation.heap_bytes());
        Ok(Applied {
            inverse: Operation::set_value(address.clone(), was),
            invalidation: Invalidation::at(address),
        })
    }

    fn dirty_bytes(&self) -> usize {
        self.dirty.bytes()
    }

    fn commit(&mut self) -> Result<Committed, SessionError> {
        let sheets = self.write_back_dirty_sheets()?;
        // Anything edited through the package's own trees — a workbook part, a relationship — is
        // settled here, so it is serialised once rather than on every save for the rest of the
        // session.
        let settled = self.workbook.settle_dirty_parts().len();
        let bytes = self.workbook.save().map_err(SessionError::document)?;
        self.dirty.clear();
        Ok(Committed {
            parts_serialised: sheets + settled,
            bytes,
        })
    }
}

/// The sheet, row and column an address names.
fn cell_address(address: &SourceRef) -> Result<(usize, u32, u16), SessionError> {
    let segments = address.path().segments();
    let [row, column] = segments else {
        return Err(SessionError::no_such_node(format!(
            "part {} path {segments:?} — a cell is addressed by exactly a row and a column",
            address.part().number()
        )));
    };
    let column = u16::try_from(*column).map_err(|_| {
        SessionError::no_such_node(format!("column {column} — outside the spreadsheet grid"))
    })?;
    Ok((address.part().number() as usize, *row, column))
}

fn reference(row: u32, column: u16) -> Result<CellReference, SessionError> {
    CellReference::relative(column, row).map_err(SessionError::document)
}

/// What a cell holds, exactly, or a refusal.
fn prior_value(
    markup: &WorksheetPart,
    reference: CellReference,
    sheet: usize,
    row: u32,
    column: u16,
) -> Result<Value, SessionError> {
    let where_it_is = || format!("sheet {sheet}, row {row}, column {column}");
    let Some(cell) = markup.cell(reference) else {
        return Ok(Value::Empty);
    };
    if cell.has_formula() {
        return Err(SessionError::no_exact_inverse(format!(
            "{} — the cell carries a formula, and setting its value would leave the two disagreeing",
            where_it_is()
        )));
    }
    if let Some(inline) = cell.inline_string_markup() {
        let parsed = InlineString::parse(inline).map_err(SessionError::document)?;
        let text = parsed
            .item()
            .text()
            .map_err(|error| SessionError::document(mjx_sml::SmlError::from(error)))?
            .into_owned();
        // Exact iff a plain inline string of this text is byte-for-byte what is there.
        let plain = InlineString::plain(&text).map_err(SessionError::document)?;
        if plain.markup() != inline {
            return Err(SessionError::no_exact_inverse(format!(
                "{} — the cell holds a rich inline string, and text alone cannot put it back",
                where_it_is()
            )));
        }
        return Ok(Value::text(text));
    }
    let raw = cell
        .value()
        .map_err(|error| SessionError::document(mjx_sml::SmlError::from(error)))?;
    let Some(text) = raw else {
        return Ok(Value::Empty);
    };
    match cell.cell_type() {
        // The exact spelling, not the parsed number: `1.0` and `1` are different bytes and putting
        // back the wrong one is this library authoring over the file's own.
        CellType::Number => Ok(Value::NumberText(text.into_owned().into_boxed_str())),
        CellType::SharedString => text.parse::<u32>().map(Value::PooledText).map_err(|_| {
            SessionError::no_exact_inverse(format!(
                "{} — the cell states a shared-string index that is not a number",
                where_it_is()
            ))
        }),
        CellType::Boolean => match text.as_ref() {
            "1" => Ok(Value::Boolean(true)),
            "0" => Ok(Value::Boolean(false)),
            _ => Err(SessionError::no_exact_inverse(format!(
                "{} — the cell states a boolean spelled `{text}`",
                where_it_is()
            ))),
        },
        CellType::Error => Ok(Value::Error(text.into_owned().into_boxed_str())),
        CellType::FormulaString => Ok(Value::ComputedText(text.into_owned().into_boxed_str())),
        CellType::InlineString => Ok(Value::Empty),
    }
}

/// A value as SpreadsheetML spells it.
fn cell_value(value: &Value) -> CellValue<'_> {
    match value {
        Value::Empty => CellValue::Blank,
        Value::Text(text) => CellValue::InlineString(text),
        Value::PooledText(index) => CellValue::SharedString(*index),
        Value::Number(number) => CellValue::Number(*number),
        Value::NumberText(text) => CellValue::NumberText(text),
        Value::Boolean(flag) => CellValue::Boolean(*flag),
        Value::Error(text) => CellValue::Error(text),
        Value::ComputedText(text) => CellValue::FormulaString(text),
    }
}
