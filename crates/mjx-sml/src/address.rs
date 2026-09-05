//! Cell addressing: cell references, ranges, `sqref` lists, and the A1 and R1C1 grammars.
//!
//! **Filled by MJXOFF-93 (D03).** A cell reference is the single most reused value in
//! SpreadsheetML, and `sml.xsd` declares it four separate ways — `ST_CellRef`, `ST_Ref`, `ST_Sqref`
//! and the `ST_CellSpans` list on `CT_Row` — all four of them `xsd:string` restrictions with no
//! facets, so the grammar below is the schema's whole contribution plus ECMA-376 Part 1's prose.
//! It appears on `c@r`, `dimension@ref`, `mergeCell@ref`, `hyperlink@ref`,
//! `conditionalFormatting@sqref`, `dataValidation@sqref`, `autoFilter@ref`, `table@ref`,
//! `selection@sqref`, `cellWatch@r`, `f@ref`/`f@r1`/`f@r2`, and inside every defined name.
//!
//! # The two properties everything here is built around
//!
//! **No allocation on the hot path.** [`CellReference`] is eight bytes and `Copy`; parsing one is a
//! single forward pass over `&str` with no backtracking and no intermediate `String`, and formatting
//! one writes into [`AddressText`], a stack-resident `Copy` buffer. `Copy` is not decoration here —
//! a `Copy` type cannot own a heap allocation, so the compiler itself is the proof that reading a
//! million cell references allocates nothing.
//!
//! **Round-trip is exact, not canonical.** `A1` read writes `A1`; `$A$1` writes `$A$1`; `A1` never
//! becomes `A1:A1` and `C3:A1` never becomes `A1:C3`. Every absolute marker, every degenerate form
//! and every separator run a producer wrote is preserved, and the *ordered* view a caller usually
//! wants is a separate method ([`CellRange::normalized_bounds`]) rather than a rewrite of the value.
//! A reference-formatting "improvement" is an edit-isolation failure — a part this library was not
//! asked to edit would come back changed — which is the fidelity tier the whole project exists for.
//!
//! # Indices are zero-based; the wire is one-based
//!
//! Every index in this module is **zero-based**: column `0` is `A`, row `0` is the row a file spells
//! `1`. The `+ 1` happens once, in the formatter, and the `- 1` once, in the parser. `ST_CellSpan`
//! (`"1:3"`) and R1C1's absolute positions (`R5C2`) are one-based on the wire and zero-based here,
//! for the same reason.
//!
//! # Untrusted input
//!
//! Every parser here returns [`AddressError`] and never panics — not on `""`, `"$"`, `"A0"`,
//! `"1A"`, `"AAAAAAAA1"`, a row number that overflows `u64`, or a quoted sheet name containing `!`.
//! Slicing goes through `str::get`, so no boundary miscalculation could reach a panicking index.
//! Out-of-grid input is a typed error, never a wrap-around and never a clamp: `XFE` is rejected, not
//! quietly read as `XFD`.
//!
//! # What is deliberately not here
//!
//! * **Formula parsing.** A reference inside a formula string is MJXOFF-115 (D11)'s; formulas are
//!   carried as the text their producer wrote.
//! * **Translation between the two syntaxes.** [`ReferenceMode`] is `calcPr@refMode`, and it is
//!   *reported*, never applied: rewriting an A1 formula into R1C1 (or the reverse) would change
//!   bytes in a part nobody asked to edit. There is deliberately no `R1C1Reference::to_a1`.
//! * **Defined-name resolution** (MJXOFF-100, D06) and **cell values** (MJXOFF-95, D04).

use core::fmt;
use std::borrow::Cow;

/// `ST_RefMode` — which reference syntax a workbook's formulas are written in (`calcPr@refMode`).
///
/// Re-exported from the generated simple types rather than declared again here: `sml.xsd` declares
/// it as an enumeration, and the generator is the workspace's single source for those.
///
/// **Reported, never applied.** A workbook that says `refMode="R1C1"` has its formulas stored in
/// R1C1; reading one never rewrites it into A1, and writing one never rewrites it back.
pub use mjx_ooxml_types::spreadsheetml::ReferenceMode;

/// How many columns Excel's grid has: `A` through `XFD`.
pub const COLUMN_COUNT: u32 = 16_384;

/// The zero-based index of the last column, `XFD`.
pub const LAST_COLUMN_INDEX: u16 = 16_383;

/// How many rows Excel's grid has: `1` through `1048576`.
pub const ROW_COUNT: u32 = 1_048_576;

/// The zero-based index of the last row, the one a file spells `1048576`.
pub const LAST_ROW_INDEX: u32 = 1_048_575;

/// How many bytes [`AddressText`] holds.
///
/// The longest thing this module renders is an R1C1 range at the grid's extremes —
/// `R[-1048575]C[-16383]:R[-1048575]C[-16383]`, 41 bytes — and the longest A1 range is
/// `$XFD$1048576:$XFD$1048576`, 25. The capacity is checked against both by
/// `every_rendering_fits_the_inline_buffer`, computed from the constants rather than written down
/// twice.
pub const ADDRESS_TEXT_CAPACITY: usize = 48;

/// Why an address failed to parse.
///
/// Every variant names what was wrong with the *input*, not where the parser was, so an error is
/// worth showing a user. The enum is `Copy` and borrows nothing: an error path that allocates is an
/// error path that can fail, and these come from files somebody else wrote.
///
/// **Deliberately exhaustive**, for the reason [`crate::SmlError`] states on itself: the facade
/// classifies every variant through a `match` with no wildcard arm, so a new one fails to compile
/// until somebody decides what it means.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AddressError {
    /// There was nothing to parse — `""`, or an empty component such as the text after the colon in
    /// `"A1:"`.
    #[error("an address is empty")]
    Empty,

    /// Column letters were required and none were found: `"$"`, `"1A"` in a cell position, or the
    /// lowercase `"a1"` (see [`column_index_from_letters`] on why case is not folded).
    #[error(
        "no column letters where a column was required (columns are spelled A to XFD, uppercase)"
    )]
    MissingColumnLetters,

    /// A row number was required and none was found: `"A"`, `"$A$"`.
    #[error("no row number where a row was required")]
    MissingRowNumber,

    /// A character that cannot appear where it appeared: `"A1x"`, `"A 1"`, `"1A"`.
    #[error("unexpected character {0:?} in an address")]
    UnexpectedCharacter(char),

    /// Column letters naming a column past `XFD`: `"XFE"`, `"AAAAAAAA"`.
    ///
    /// Never clamped. A reference outside the grid is a defect in the file, and answering `XFD` for
    /// it would silently move somebody's data one column.
    #[error("a column past XFD: the grid has {COLUMN_COUNT} columns, A to XFD")]
    ColumnOutOfGrid,

    /// A row number of `0`, or past `1048576`: `"A0"`, `"A1048577"`.
    ///
    /// `row_number` is the one-based number as written, saturating at [`u64::MAX`] for an absurd
    /// digit run, so an error message can quote what the file said.
    #[error("row {row_number} is outside the grid: rows are numbered 1 to {ROW_COUNT}")]
    RowOutOfGrid {
        /// The one-based row number the file carried.
        row_number: u64,
    },

    /// A range with more than two ends: `"A1:B2:C3"`.
    #[error("a range has more than two ends")]
    TooManyRangeEnds,

    /// A range whose two ends are different kinds: `"A:1"`, `"A1:B"`.
    #[error("a range mixes kinds of end (cell, whole column, whole row)")]
    MismatchedRangeEnds,

    /// A quoted sheet name with no closing apostrophe: `"'My Sheet!A1"`.
    #[error("a quoted sheet name has no closing apostrophe")]
    UnterminatedSheetName,

    /// A sheet-qualified reference whose sheet name is empty: `"!A1"`, `"''!A1"`.
    #[error("a sheet-qualified reference names no sheet")]
    EmptySheetName,

    /// A sheet-qualified reference with no `!` outside quotes: `"Sheet1A1"`.
    #[error("a sheet-qualified reference has no `!` separating the sheet from the reference")]
    MissingSheetSeparator,

    /// An external-book index that is not a number, is empty, or has no closing `]`:
    /// `"[x]Sheet1!A1"`, `"[1Sheet1!A1"`.
    #[error("an external-book index is not a number in brackets")]
    InvalidExternalBookIndex,

    /// An R1C1 reference missing its `R` or its `C`: `"1C1"`, `"R1"`.
    #[error("an R1C1 reference is not `R`…`C`…")]
    MissingRowColumnMarker,

    /// An R1C1 relative offset with no closing `]`: `"R[-1C1"`.
    #[error("an R1C1 offset has no closing bracket")]
    UnterminatedOffset,

    /// An R1C1 relative offset that is empty, not a number, or larger than the grid: `"R[]C1"`,
    /// `"R[x]C1"`, `"R[9999999]C1"`.
    #[error("an R1C1 offset is not a whole number within the grid")]
    InvalidOffset,

    /// A `spans` entry with no `:` between its two column numbers: `"13"`.
    #[error("a `spans` entry is not `first:last`")]
    MissingSpanSeparator,
}

// ---------------------------------------------------------------------------------------------
// AddressText — the allocation-free rendering
// ---------------------------------------------------------------------------------------------

/// An address rendered as text, in a fixed inline buffer — a `Copy` value that never allocates.
///
/// This is what makes "formatting writes into a small stack array" a fact rather than an intention:
/// `AddressText` is `Copy`, so it can own no heap allocation, and [`ADDRESS_TEXT_CAPACITY`] is
/// large enough for the longest address this module can produce (checked by a test computed from
/// [`COLUMN_COUNT`] and [`ROW_COUNT`], not from a literal).
///
/// ```
/// use mjx_sml::CellReference;
/// let cell = CellReference::parse("$B$7").expect("a cell reference");
/// assert_eq!(cell.text().as_str(), "$B$7");
/// ```
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AddressText {
    bytes: [u8; ADDRESS_TEXT_CAPACITY],
    length: u8,
}

impl AddressText {
    /// An empty buffer.
    fn new() -> Self {
        Self {
            bytes: [0; ADDRESS_TEXT_CAPACITY],
            length: 0,
        }
    }

    /// Appends one byte, ignoring it if the buffer is full.
    ///
    /// Silently dropping would be a defect if it could happen; it cannot, because every caller
    /// renders a grid-validated value and the capacity is proved sufficient by test. Ignoring is
    /// still the right failure: this runs on a path that must not panic.
    fn push(&mut self, byte: u8) {
        let index = self.length as usize;
        if index < ADDRESS_TEXT_CAPACITY {
            self.bytes[index] = byte;
            self.length = self.length.saturating_add(1);
        }
    }

    /// Appends ASCII text.
    fn push_str(&mut self, text: &str) {
        for byte in text.as_bytes() {
            self.push(*byte);
        }
    }

    /// Appends a decimal number.
    fn push_number(&mut self, value: u32) {
        let mut digits = [0u8; 10];
        let mut count = 0;
        let mut remaining = value;
        loop {
            digits[count] = b'0' + (remaining % 10) as u8;
            count += 1;
            remaining /= 10;
            if remaining == 0 || count == digits.len() {
                break;
            }
        }
        for position in (0..count).rev() {
            self.push(digits[position]);
        }
    }

    /// Appends a signed decimal number.
    fn push_signed(&mut self, value: i32) {
        if value < 0 {
            self.push(b'-');
        }
        self.push_number(value.unsigned_abs());
    }

    /// The rendered text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        // Every byte pushed is ASCII, so this is always valid UTF-8; `unwrap_or` keeps the promise
        // that nothing here panics even if that ever stopped being true.
        self.bytes
            .get(..self.length as usize)
            .and_then(|bytes| core::str::from_utf8(bytes).ok())
            .unwrap_or("")
    }

    /// How many bytes the rendering is.
    #[must_use]
    pub fn len(&self) -> usize {
        self.length as usize
    }

    /// Whether nothing has been rendered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }
}

impl fmt::Debug for AddressText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl fmt::Display for AddressText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for AddressText {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl core::ops::Deref for AddressText {
    type Target = str;

    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq<str> for AddressText {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for AddressText {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

// ---------------------------------------------------------------------------------------------
// Anchoring
// ---------------------------------------------------------------------------------------------

/// Whether a coordinate carries a `$`.
///
/// Excel calls this absolute versus relative addressing: `$A$1` keeps naming column `A` row `1`
/// wherever the formula is copied, while `A1` moves with it. Nothing in this crate *acts* on the
/// distinction — it is preserved because a file that said `$A$1` must still say `$A$1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Anchoring {
    /// No `$` — the coordinate moves when the formula is copied.
    Relative,
    /// A `$` — the coordinate is fixed.
    Absolute,
}

impl Anchoring {
    /// Whether this is [`Absolute`](Self::Absolute).
    #[must_use]
    pub fn is_absolute(self) -> bool {
        matches!(self, Self::Absolute)
    }

    /// The wire marker: `"$"` when absolute, `""` when relative.
    #[must_use]
    pub fn marker(self) -> &'static str {
        match self {
            Self::Relative => "",
            Self::Absolute => "$",
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Column letters, both ways
// ---------------------------------------------------------------------------------------------

/// The zero-based column index for a run of column letters: `A` → `0`, `Z` → `25`, `AA` → `26`,
/// `XFD` → `16383`.
///
/// This is **bijective base-26** — there is no zero digit, so `Z` is followed by `AA` rather than by
/// `BA`, and the value of a run of *n* letters starts where the run of *n-1* letters ended. That one
/// property is where every off-by-one in this conversion lives, which is why the test suite walks
/// `A`, `Z`, `AA`, `AZ`, `BA`, `ZZ`, `AAA`, `XFD` and rejects `XFE`.
///
/// # Case is not folded
///
/// Only `A`–`Z` are accepted; `"a1"` is [`AddressError::MissingColumnLetters`]. Excel and every
/// producer this workspace has read write uppercase, and folding case would mean a file that said
/// `a1` came back saying `A1` — a canonicalization, which is the class of change this module exists
/// to avoid. A caller that meets a lowercase reference gets a typed error and can keep the source
/// text verbatim, which loses nothing.
///
/// # Errors
///
/// [`AddressError::MissingColumnLetters`] for an empty run, [`AddressError::UnexpectedCharacter`]
/// for anything that is not `A`–`Z`, and [`AddressError::ColumnOutOfGrid`] for a run naming a column
/// past `XFD`.
///
/// ```
/// use mjx_sml::address::column_index_from_letters;
/// assert_eq!(column_index_from_letters("A"), Ok(0));
/// assert_eq!(column_index_from_letters("AA"), Ok(26));
/// assert_eq!(column_index_from_letters("XFD"), Ok(16_383));
/// assert!(column_index_from_letters("XFE").is_err());
/// ```
pub fn column_index_from_letters(letters: &str) -> Result<u16, AddressError> {
    if letters.is_empty() {
        return Err(AddressError::MissingColumnLetters);
    }
    // One-based while accumulating (`A` is 1), because bijective base-26 has no zero digit.
    let mut ordinal: u32 = 0;
    for byte in letters.bytes() {
        if !byte.is_ascii_uppercase() {
            return Err(AddressError::UnexpectedCharacter(char::from(byte)));
        }
        ordinal = ordinal * 26 + u32::from(byte - b'A') + 1;
        if ordinal > COLUMN_COUNT {
            // Checked inside the loop, so an arbitrarily long run of letters cannot overflow the
            // accumulator: it leaves at the fourth letter at the latest.
            return Err(AddressError::ColumnOutOfGrid);
        }
    }
    u16::try_from(ordinal - 1).map_err(|_| AddressError::ColumnOutOfGrid)
}

/// The column letters for a zero-based column index: `0` → `A`, `25` → `Z`, `26` → `AA`,
/// `16383` → `XFD`.
///
/// Allocation-free — the answer is an [`AddressText`], a `Copy` stack buffer. This supersedes
/// `mjx_chart::workbook::column_letters`, which returns a `String` per call; MJXOFF-112 (D10)
/// switches `mjx-chart` over and MJXOFF-99 (E1) retires the copy.
///
/// # Errors
///
/// [`AddressError::ColumnOutOfGrid`] for an index past [`LAST_COLUMN_INDEX`].
///
/// ```
/// use mjx_sml::address::column_letters;
/// assert_eq!(column_letters(0).unwrap().as_str(), "A");
/// assert_eq!(column_letters(26).unwrap().as_str(), "AA");
/// assert_eq!(column_letters(16_383).unwrap().as_str(), "XFD");
/// assert!(column_letters(16_384).is_err());
/// ```
pub fn column_letters(column: u16) -> Result<AddressText, AddressError> {
    if column > LAST_COLUMN_INDEX {
        return Err(AddressError::ColumnOutOfGrid);
    }
    let mut text = AddressText::new();
    write_column_letters(column, &mut text);
    Ok(text)
}

/// Renders a grid-validated column index into `out`, most significant letter first.
///
/// The `- 1` before each division is the bijective part: without it `AA` would render as `BA`.
fn write_column_letters(column: u16, out: &mut AddressText) {
    let mut letters = [0u8; 4];
    let mut count = 0;
    // One-based, so that `A` is 1 and the "no zero digit" arithmetic below works.
    let mut ordinal = u32::from(column) + 1;
    while ordinal > 0 && count < letters.len() {
        let remainder = (ordinal - 1) % 26;
        letters[count] = b'A' + remainder as u8;
        count += 1;
        ordinal = (ordinal - 1) / 26;
    }
    for position in (0..count).rev() {
        out.push(letters[position]);
    }
}

// ---------------------------------------------------------------------------------------------
// CellReference — ST_CellRef
// ---------------------------------------------------------------------------------------------

/// `ST_CellRef` — one cell's address: a column, a row, and each one's `$`.
///
/// Wire forms: `A1`, `$A$1`, `A$1`, `$A1`. All four round-trip exactly; the absolute markers are
/// data, not formatting.
///
/// **Eight bytes and `Copy`**, laid out `u32` row + `u16` column + two one-byte anchorings with no
/// padding waste. MJXOFF-95 (D04) parses one of these for every cell of a sheet that may hold
/// 1,048,576 × 16,384 of them, so the representation is the design: parsing is one forward pass
/// producing a value the caller can hold by copy, and nothing on the path allocates.
///
/// ```
/// use mjx_sml::{Anchoring, CellReference};
/// let cell = CellReference::parse("$B$7").expect("a cell reference");
/// assert_eq!(cell.column(), 1);
/// assert_eq!(cell.row(), 6);
/// assert_eq!(cell.column_anchoring(), Anchoring::Absolute);
/// assert_eq!(cell.text().as_str(), "$B$7"); // exactly what was read
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CellReference {
    /// Zero-based row index: `0` is the row a file spells `1`.
    row: u32,
    /// Zero-based column index: `0` is `A`.
    column: u16,
    column_anchoring: Anchoring,
    row_anchoring: Anchoring,
}

impl CellReference {
    /// A reference to `column`/`row` (both zero-based) with the given anchoring.
    ///
    /// # Errors
    ///
    /// [`AddressError::ColumnOutOfGrid`] or [`AddressError::RowOutOfGrid`] if either index is
    /// outside the grid.
    pub fn new(
        column: u16,
        row: u32,
        column_anchoring: Anchoring,
        row_anchoring: Anchoring,
    ) -> Result<Self, AddressError> {
        if column > LAST_COLUMN_INDEX {
            return Err(AddressError::ColumnOutOfGrid);
        }
        if row > LAST_ROW_INDEX {
            return Err(AddressError::RowOutOfGrid {
                row_number: u64::from(row) + 1,
            });
        }
        Ok(Self {
            row,
            column,
            column_anchoring,
            row_anchoring,
        })
    }

    /// A wholly relative reference — what `A1` means.
    ///
    /// # Errors
    ///
    /// As [`new`](Self::new).
    pub fn relative(column: u16, row: u32) -> Result<Self, AddressError> {
        Self::new(column, row, Anchoring::Relative, Anchoring::Relative)
    }

    /// A wholly absolute reference — what `$A$1` means.
    ///
    /// # Errors
    ///
    /// As [`new`](Self::new).
    pub fn absolute(column: u16, row: u32) -> Result<Self, AddressError> {
        Self::new(column, row, Anchoring::Absolute, Anchoring::Absolute)
    }

    /// Parses `A1`, `$A$1`, `A$1` or `$A1`.
    ///
    /// # Errors
    ///
    /// [`AddressError`], never a panic. `""` is [`Empty`](AddressError::Empty); `"$"` and `"a1"` are
    /// [`MissingColumnLetters`](AddressError::MissingColumnLetters); `"A"` is
    /// [`MissingRowNumber`](AddressError::MissingRowNumber); `"A0"` and a row past `1048576` are
    /// [`RowOutOfGrid`](AddressError::RowOutOfGrid); `"XFE1"` is
    /// [`ColumnOutOfGrid`](AddressError::ColumnOutOfGrid).
    pub fn parse(text: &str) -> Result<Self, AddressError> {
        match parse_range_end(text)? {
            RangeEnd::Cell(cell) => Ok(cell),
            RangeEnd::Column(_) => Err(AddressError::MissingRowNumber),
            RangeEnd::Row(_) => Err(AddressError::MissingColumnLetters),
        }
    }

    /// The zero-based column index — `0` for `A`.
    #[must_use]
    pub fn column(self) -> u16 {
        self.column
    }

    /// The zero-based row index — `0` for the row a file spells `1`.
    #[must_use]
    pub fn row(self) -> u32 {
        self.row
    }

    /// Whether the column carries a `$`.
    #[must_use]
    pub fn column_anchoring(self) -> Anchoring {
        self.column_anchoring
    }

    /// Whether the row carries a `$`.
    #[must_use]
    pub fn row_anchoring(self) -> Anchoring {
        self.row_anchoring
    }

    /// The same cell with different anchoring — the one edit that changes a reference's text
    /// without moving it.
    #[must_use]
    pub fn with_anchoring(self, column_anchoring: Anchoring, row_anchoring: Anchoring) -> Self {
        Self {
            column_anchoring,
            row_anchoring,
            ..self
        }
    }

    /// This reference as text, in a stack buffer — exactly the spelling it was parsed from.
    #[must_use]
    pub fn text(self) -> AddressText {
        let mut out = AddressText::new();
        self.write_into(&mut out);
        out
    }

    fn write_into(self, out: &mut AddressText) {
        out.push_str(self.column_anchoring.marker());
        write_column_letters(self.column, out);
        out.push_str(self.row_anchoring.marker());
        out.push_number(self.row.saturating_add(1));
    }
}

impl fmt::Display for CellReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.text().as_str())
    }
}

impl core::str::FromStr for CellReference {
    type Err = AddressError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::parse(text)
    }
}

// ---------------------------------------------------------------------------------------------
// The shared scanner
// ---------------------------------------------------------------------------------------------

/// What one end of a range turned out to be.
enum RangeEnd {
    /// `A1`, `$A$1` — a cell.
    Cell(CellReference),
    /// `A`, `$A` — a whole column, as in `A:A`.
    Column(ColumnBound),
    /// `1`, `$1` — a whole row, as in `1:1`.
    Row(RowBound),
}

/// Reads `[$]LETTERS[[$]DIGITS]` or `[$]DIGITS`, consuming the whole of `text`.
///
/// One scanner for every A1-syntax address in the module, so a cell reference and a range end
/// cannot disagree about what `"$"` or `"1A"` means.
fn parse_range_end(text: &str) -> Result<RangeEnd, AddressError> {
    let bytes = text.as_bytes();
    if bytes.is_empty() {
        return Err(AddressError::Empty);
    }
    let mut position = 0;
    let first_anchoring = take_anchoring(bytes, &mut position);

    let letters_start = position;
    while matches!(bytes.get(position), Some(byte) if byte.is_ascii_uppercase()) {
        position += 1;
    }
    let letters = text.get(letters_start..position).unwrap_or("");

    if letters.is_empty() {
        // No letters: this can only be a whole-row end, `[$]DIGITS`. The `$` already read anchors
        // the row. Anything that is not a digit here is not an address at all.
        if !matches!(bytes.get(position), Some(byte) if byte.is_ascii_digit()) {
            return Err(AddressError::MissingColumnLetters);
        }
        let row_number = take_row_number(bytes, &mut position)?;
        expect_end(text, position)?;
        return Ok(RangeEnd::Row(RowBound {
            row: row_number - 1,
            anchoring: first_anchoring,
        }));
    }

    let column = column_index_from_letters(letters)?;
    if position == bytes.len() {
        return Ok(RangeEnd::Column(ColumnBound {
            column,
            anchoring: first_anchoring,
        }));
    }

    let row_anchoring = take_anchoring(bytes, &mut position);
    let row_number = take_row_number(bytes, &mut position)?;
    expect_end(text, position)?;
    Ok(RangeEnd::Cell(CellReference {
        row: row_number - 1,
        column,
        column_anchoring: first_anchoring,
        row_anchoring,
    }))
}

/// Consumes a `$` if there is one.
fn take_anchoring(bytes: &[u8], position: &mut usize) -> Anchoring {
    if bytes.get(*position) == Some(&b'$') {
        *position += 1;
        Anchoring::Absolute
    } else {
        Anchoring::Relative
    }
}

/// Consumes a run of digits and validates it as a one-based row number in `1..=ROW_COUNT`.
///
/// The accumulator saturates rather than wrapping, so `"A99999999999999999999999"` is a reported
/// [`AddressError::RowOutOfGrid`] and never a wrapped-around row 7.
fn take_row_number(bytes: &[u8], position: &mut usize) -> Result<u32, AddressError> {
    let digits_start = *position;
    let mut row_number: u64 = 0;
    while let Some(byte) = bytes.get(*position) {
        if !byte.is_ascii_digit() {
            break;
        }
        row_number = row_number
            .saturating_mul(10)
            .saturating_add(u64::from(byte - b'0'));
        *position += 1;
    }
    if *position == digits_start {
        return Err(AddressError::MissingRowNumber);
    }
    if row_number == 0 || row_number > u64::from(ROW_COUNT) {
        return Err(AddressError::RowOutOfGrid { row_number });
    }
    u32::try_from(row_number).map_err(|_| AddressError::RowOutOfGrid { row_number })
}

/// Fails with the character at `position` unless the whole input has been consumed.
fn expect_end(text: &str, position: usize) -> Result<(), AddressError> {
    match text.get(position..).and_then(|rest| rest.chars().next()) {
        None => Ok(()),
        Some(character) => Err(AddressError::UnexpectedCharacter(character)),
    }
}

// ---------------------------------------------------------------------------------------------
// CellRange — ST_Ref
// ---------------------------------------------------------------------------------------------

/// One end of a whole-column range: `A` or `$A` in `A:A`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColumnBound {
    column: u16,
    anchoring: Anchoring,
}

impl ColumnBound {
    /// A bound on the zero-based `column`.
    ///
    /// # Errors
    ///
    /// [`AddressError::ColumnOutOfGrid`] past [`LAST_COLUMN_INDEX`].
    pub fn new(column: u16, anchoring: Anchoring) -> Result<Self, AddressError> {
        if column > LAST_COLUMN_INDEX {
            return Err(AddressError::ColumnOutOfGrid);
        }
        Ok(Self { column, anchoring })
    }

    /// The zero-based column index.
    #[must_use]
    pub fn column(self) -> u16 {
        self.column
    }

    /// Whether the column carries a `$`.
    #[must_use]
    pub fn anchoring(self) -> Anchoring {
        self.anchoring
    }

    fn write_into(self, out: &mut AddressText) {
        out.push_str(self.anchoring.marker());
        write_column_letters(self.column, out);
    }
}

impl fmt::Display for ColumnBound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = AddressText::new();
        self.write_into(&mut out);
        f.write_str(out.as_str())
    }
}

/// One end of a whole-row range: `1` or `$1` in `1:1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RowBound {
    row: u32,
    anchoring: Anchoring,
}

impl RowBound {
    /// A bound on the zero-based `row`.
    ///
    /// # Errors
    ///
    /// [`AddressError::RowOutOfGrid`] past [`LAST_ROW_INDEX`].
    pub fn new(row: u32, anchoring: Anchoring) -> Result<Self, AddressError> {
        if row > LAST_ROW_INDEX {
            return Err(AddressError::RowOutOfGrid {
                row_number: u64::from(row) + 1,
            });
        }
        Ok(Self { row, anchoring })
    }

    /// The zero-based row index.
    #[must_use]
    pub fn row(self) -> u32 {
        self.row
    }

    /// Whether the row carries a `$`.
    #[must_use]
    pub fn anchoring(self) -> Anchoring {
        self.anchoring
    }

    fn write_into(self, out: &mut AddressText) {
        out.push_str(self.anchoring.marker());
        out.push_number(self.row.saturating_add(1));
    }
}

impl fmt::Display for RowBound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = AddressText::new();
        self.write_into(&mut out);
        f.write_str(out.as_str())
    }
}

/// The ordered rectangle a [`CellRange`] covers, whichever way round it was written.
///
/// This is the *derived* view. It is deliberately a separate type from [`CellRange`] so that
/// wanting ordered bounds never turns into reordering the value: `C3:A1` reports the same
/// `GridBounds` as `A1:C3` and still writes `C3:A1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridBounds {
    first_column: u16,
    last_column: u16,
    first_row: u32,
    last_row: u32,
}

impl GridBounds {
    /// The leftmost column, zero-based.
    #[must_use]
    pub fn first_column(self) -> u16 {
        self.first_column
    }

    /// The rightmost column, zero-based and inclusive.
    #[must_use]
    pub fn last_column(self) -> u16 {
        self.last_column
    }

    /// The topmost row, zero-based.
    #[must_use]
    pub fn first_row(self) -> u32 {
        self.first_row
    }

    /// The bottommost row, zero-based and inclusive.
    #[must_use]
    pub fn last_row(self) -> u32 {
        self.last_row
    }

    /// How many cells the rectangle covers. `u64` because the whole grid is 17,179,869,184 cells.
    #[must_use]
    pub fn cell_count(self) -> u64 {
        let columns = u64::from(self.last_column - self.first_column) + 1;
        let rows = u64::from(self.last_row - self.first_row) + 1;
        columns * rows
    }

    /// Whether `cell` falls inside the rectangle. Anchoring is not part of the question.
    #[must_use]
    pub fn contains(self, cell: CellReference) -> bool {
        (self.first_column..=self.last_column).contains(&cell.column())
            && (self.first_row..=self.last_row).contains(&cell.row())
    }
}

/// `ST_Ref` — a range of cells, in every form SpreadsheetML writes one.
///
/// | Variant | Wire | Where it appears |
/// |---|---|---|
/// | [`Cell`](Self::Cell) | `A1` | the degenerate single-cell range Excel writes for a one-cell `sqref` |
/// | [`Cells`](Self::Cells) | `A1:C3`, `$A$1:$C$3`, `C3:A1` | `dimension@ref`, `mergeCell@ref`, `table@ref` |
/// | [`Columns`](Self::Columns) | `A:A`, `$A:$C` | autofilters and defined names |
/// | [`Rows`](Self::Rows) | `1:1`, `$1:$3` | the same |
///
/// **`A1` and `A1:A1` are different values** and stay different: the first is
/// [`Cell`](Self::Cell), the second [`Cells`](Self::Cells) with equal ends. Widening one into the
/// other would change bytes in a part nobody asked to edit.
///
/// ```
/// use mjx_sml::CellRange;
/// let backwards = CellRange::parse("C3:A1").expect("a range");
/// // Preserved as written …
/// assert_eq!(backwards.text().as_str(), "C3:A1");
/// // … and ordered only when asked.
/// assert_eq!(backwards.normalized_bounds().first_column(), 0);
/// assert_eq!(backwards.normalized_bounds().last_column(), 2);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CellRange {
    /// A single cell written without a colon: `A1`.
    Cell(CellReference),
    /// Two cell ends: `A1:C3`. Either order; the order is preserved.
    Cells {
        /// The end written before the colon.
        start: CellReference,
        /// The end written after it.
        end: CellReference,
    },
    /// Whole columns: `A:C`.
    Columns {
        /// The end written before the colon.
        start: ColumnBound,
        /// The end written after it.
        end: ColumnBound,
    },
    /// Whole rows: `1:3`.
    Rows {
        /// The end written before the colon.
        start: RowBound,
        /// The end written after it.
        end: RowBound,
    },
}

impl CellRange {
    /// Parses `A1`, `A1:C3`, `A:C` or `1:3`, with any pattern of `$`.
    ///
    /// # Errors
    ///
    /// [`AddressError`], never a panic. `"A1:B2:C3"` is
    /// [`TooManyRangeEnds`](AddressError::TooManyRangeEnds); `"A:1"` and `"A1:B"` are
    /// [`MismatchedRangeEnds`](AddressError::MismatchedRangeEnds); each end otherwise fails exactly
    /// as [`CellReference::parse`] does.
    pub fn parse(text: &str) -> Result<Self, AddressError> {
        let Some((start, end)) = text.split_once(':') else {
            return Ok(Self::Cell(CellReference::parse(text)?));
        };
        if end.contains(':') {
            return Err(AddressError::TooManyRangeEnds);
        }
        match (parse_range_end(start)?, parse_range_end(end)?) {
            (RangeEnd::Cell(start), RangeEnd::Cell(end)) => Ok(Self::Cells { start, end }),
            (RangeEnd::Column(start), RangeEnd::Column(end)) => Ok(Self::Columns { start, end }),
            (RangeEnd::Row(start), RangeEnd::Row(end)) => Ok(Self::Rows { start, end }),
            _ => Err(AddressError::MismatchedRangeEnds),
        }
    }

    /// Whether this is the degenerate single-cell form (`A1`), as opposed to `A1:A1`.
    #[must_use]
    pub fn is_single_cell(self) -> bool {
        matches!(self, Self::Cell(_))
    }

    /// The ordered rectangle this covers, with a whole-column or whole-row form widened to the
    /// grid's full extent on the other axis.
    ///
    /// Ordering happens **here and only here**. The value itself keeps whatever order its producer
    /// wrote.
    #[must_use]
    pub fn normalized_bounds(self) -> GridBounds {
        let (first_column, last_column, first_row, last_row) = match self {
            Self::Cell(cell) => (cell.column(), cell.column(), cell.row(), cell.row()),
            Self::Cells { start, end } => (
                start.column().min(end.column()),
                start.column().max(end.column()),
                start.row().min(end.row()),
                start.row().max(end.row()),
            ),
            Self::Columns { start, end } => (
                start.column().min(end.column()),
                start.column().max(end.column()),
                0,
                LAST_ROW_INDEX,
            ),
            Self::Rows { start, end } => (
                0,
                LAST_COLUMN_INDEX,
                start.row().min(end.row()),
                start.row().max(end.row()),
            ),
        };
        GridBounds {
            first_column,
            last_column,
            first_row,
            last_row,
        }
    }

    /// Whether `cell` falls inside this range, by [`normalized_bounds`](Self::normalized_bounds).
    #[must_use]
    pub fn contains(self, cell: CellReference) -> bool {
        self.normalized_bounds().contains(cell)
    }

    /// This range as text, in a stack buffer — exactly the spelling it was parsed from.
    #[must_use]
    pub fn text(self) -> AddressText {
        let mut out = AddressText::new();
        self.write_into(&mut out);
        out
    }

    fn write_into(self, out: &mut AddressText) {
        match self {
            Self::Cell(cell) => cell.write_into(out),
            Self::Cells { start, end } => {
                start.write_into(out);
                out.push(b':');
                end.write_into(out);
            }
            Self::Columns { start, end } => {
                start.write_into(out);
                out.push(b':');
                end.write_into(out);
            }
            Self::Rows { start, end } => {
                start.write_into(out);
                out.push(b':');
                end.write_into(out);
            }
        }
    }
}

impl fmt::Display for CellRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.text().as_str())
    }
}

impl core::str::FromStr for CellRange {
    type Err = AddressError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::parse(text)
    }
}

impl From<CellReference> for CellRange {
    fn from(cell: CellReference) -> Self {
        Self::Cell(cell)
    }
}

// ---------------------------------------------------------------------------------------------
// CellRangeList — ST_Sqref
// ---------------------------------------------------------------------------------------------

/// `ST_Sqref` — the whitespace-separated range list on `selection@sqref`,
/// `conditionalFormatting@sqref` and `dataValidation@sqref`.
///
/// # Why this one owns a string
///
/// `xsd:list` says "separated by whitespace" and says nothing about *which* whitespace, so
/// `"A1  B2"` and `"A1 B2"` are the same list and different bytes. Re-emitting a list a producer
/// wrote therefore needs the original text, and a list is not a `Copy` value in any case. So this
/// type applies the project's copy-on-write rule at value scale: **the source text is kept and
/// replayed verbatim until the list is edited**, and only an edited list is rendered canonically
/// (single spaces). An untouched `sqref` is byte-identical; a changed one is spelled the one way
/// this crate spells it.
///
/// This is not a hot path — an `sqref` is per conditional format, not per cell — so the one `Box<str>`
/// it holds costs nothing that matters.
///
/// ```
/// use mjx_sml::CellRangeList;
/// let list = CellRangeList::parse("A1  B2:C3").expect("a sqref");
/// assert_eq!(list.len(), 2);
/// assert_eq!(list.to_string(), "A1  B2:C3"); // the double space survives
///
/// let mut edited = list;
/// edited.push(mjx_sml::CellRange::parse("D4").expect("a range"));
/// assert_eq!(edited.to_string(), "A1 B2:C3 D4"); // edited: canonical spacing
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CellRangeList {
    ranges: Vec<CellRange>,
    /// The exact text this list was parsed from, replayed while the list is untouched.
    source: Option<Box<str>>,
}

impl CellRangeList {
    /// Parses a whitespace-separated list of ranges.
    ///
    /// An empty or all-whitespace value is an empty list, which is what `xsd:list` says it is, and
    /// it still re-emits verbatim.
    ///
    /// # Errors
    ///
    /// The first [`AddressError`] any entry produces.
    pub fn parse(text: &str) -> Result<Self, AddressError> {
        let mut ranges = Vec::new();
        for entry in text.split_ascii_whitespace() {
            ranges.push(CellRange::parse(entry)?);
        }
        Ok(Self {
            ranges,
            source: Some(text.into()),
        })
    }

    /// A list built from ranges, with no source text — it renders canonically.
    pub fn from_ranges(ranges: impl IntoIterator<Item = CellRange>) -> Self {
        Self {
            ranges: ranges.into_iter().collect(),
            source: None,
        }
    }

    /// The ranges, in the order they were written.
    #[must_use]
    pub fn ranges(&self) -> &[CellRange] {
        &self.ranges
    }

    /// How many ranges the list holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.ranges.len()
    }

    /// Whether the list holds no ranges.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }

    /// Whether any range in the list contains `cell`.
    #[must_use]
    pub fn contains(&self, cell: CellReference) -> bool {
        self.ranges.iter().any(|range| range.contains(cell))
    }

    /// Whether this list still replays the text it was parsed from.
    ///
    /// `false` once anything has been added or removed — which is exactly when re-emitting the old
    /// text would be wrong.
    #[must_use]
    pub fn is_verbatim(&self) -> bool {
        self.source.is_some()
    }

    /// Appends a range, dropping the verbatim source.
    pub fn push(&mut self, range: CellRange) {
        self.ranges.push(range);
        self.source = None;
    }

    /// Removes the range at `index` and returns it, dropping the verbatim source.
    ///
    /// Returns `None` if there is no such index, and then changes nothing at all.
    pub fn remove(&mut self, index: usize) -> Option<CellRange> {
        if index >= self.ranges.len() {
            return None;
        }
        self.source = None;
        Some(self.ranges.remove(index))
    }
}

impl fmt::Display for CellRangeList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(source) = &self.source {
            return f.write_str(source);
        }
        for (position, range) in self.ranges.iter().enumerate() {
            if position > 0 {
                f.write_str(" ")?;
            }
            f.write_str(range.text().as_str())?;
        }
        Ok(())
    }
}

impl core::str::FromStr for CellRangeList {
    type Err = AddressError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::parse(text)
    }
}

// ---------------------------------------------------------------------------------------------
// CellSpans — CT_Row@spans
// ---------------------------------------------------------------------------------------------

/// One `ST_CellSpan`: the `1:3` of a `spans` list, as zero-based column indices.
///
/// The wire form is a pair of **one-based column numbers**, not letters. Both are stored zero-based
/// here and rendered `+ 1`, like every other index in this module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CellSpan {
    first_column: u16,
    last_column: u16,
}

impl CellSpan {
    /// A span over `first_column..=last_column`, both zero-based.
    ///
    /// The two are **not** reordered: a file that wrote `3:1` gets `3:1` back.
    ///
    /// # Errors
    ///
    /// [`AddressError::ColumnOutOfGrid`] if either index is past [`LAST_COLUMN_INDEX`].
    pub fn new(first_column: u16, last_column: u16) -> Result<Self, AddressError> {
        if first_column > LAST_COLUMN_INDEX || last_column > LAST_COLUMN_INDEX {
            return Err(AddressError::ColumnOutOfGrid);
        }
        Ok(Self {
            first_column,
            last_column,
        })
    }

    /// Parses one entry, `"1:3"`.
    ///
    /// # Errors
    ///
    /// [`AddressError::MissingSpanSeparator`] without a `:`,
    /// [`AddressError::ColumnOutOfGrid`] for a number of `0` or past `16384`, and
    /// [`AddressError::UnexpectedCharacter`] for anything that is not a digit.
    pub fn parse(text: &str) -> Result<Self, AddressError> {
        let Some((first, last)) = text.split_once(':') else {
            return Err(AddressError::MissingSpanSeparator);
        };
        Ok(Self {
            first_column: parse_column_number(first)?,
            last_column: parse_column_number(last)?,
        })
    }

    /// The first column, zero-based.
    #[must_use]
    pub fn first_column(self) -> u16 {
        self.first_column
    }

    /// The last column, zero-based and inclusive.
    #[must_use]
    pub fn last_column(self) -> u16 {
        self.last_column
    }

    fn write_into(self, out: &mut AddressText) {
        out.push_number(u32::from(self.first_column) + 1);
        out.push(b':');
        out.push_number(u32::from(self.last_column) + 1);
    }

    /// This span as text, in a stack buffer.
    #[must_use]
    pub fn text(self) -> AddressText {
        let mut out = AddressText::new();
        self.write_into(&mut out);
        out
    }
}

impl fmt::Display for CellSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.text().as_str())
    }
}

/// Reads a one-based column number (`"3"`) and returns it zero-based.
fn parse_column_number(text: &str) -> Result<u16, AddressError> {
    if text.is_empty() {
        return Err(AddressError::MissingColumnLetters);
    }
    let mut number: u64 = 0;
    for byte in text.bytes() {
        if !byte.is_ascii_digit() {
            return Err(AddressError::UnexpectedCharacter(char::from(byte)));
        }
        number = number
            .saturating_mul(10)
            .saturating_add(u64::from(byte - b'0'));
        if number > u64::from(COLUMN_COUNT) {
            return Err(AddressError::ColumnOutOfGrid);
        }
    }
    if number == 0 {
        return Err(AddressError::ColumnOutOfGrid);
    }
    u16::try_from(number - 1).map_err(|_| AddressError::ColumnOutOfGrid)
}

/// `ST_CellSpans` — `CT_Row@spans`, the whitespace-separated `1:3` hint list.
///
/// # An optimisation hint, and nothing more
///
/// `spans` tells a consumer which columns a row occupies so it can size its buffers before reading
/// the cells. It is **optional and advisory**: Excel writes it, LibreOffice does not, and a row's
/// real extent is its cells. So this type exists to *preserve*, and carries the rule in its shape:
///
/// * **Never derived.** There is deliberately no constructor that computes a span from a row's
///   cells. A row whose source carried no `spans` must not grow one — inventing the attribute
///   changes bytes in a part nobody asked to edit, and MJXOFF-95 (D04) models the attribute as an
///   `Option` for exactly that reason.
/// * **Never dropped.** A row whose source carried `spans` keeps it, spelled as it was found,
///   including whatever whitespace separated multiple entries — the same verbatim-until-edited rule
///   [`CellRangeList`] documents.
///
/// ```
/// use mjx_sml::CellSpans;
/// let spans = CellSpans::parse("1:3").expect("a spans list");
/// assert_eq!(spans.spans()[0].first_column(), 0); // one-based on the wire, zero-based here
/// assert_eq!(spans.to_string(), "1:3");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CellSpans {
    spans: Vec<CellSpan>,
    source: Option<Box<str>>,
}

impl CellSpans {
    /// Parses a whitespace-separated `spans` value.
    ///
    /// # Errors
    ///
    /// The first [`AddressError`] any entry produces.
    pub fn parse(text: &str) -> Result<Self, AddressError> {
        let mut spans = Vec::new();
        for entry in text.split_ascii_whitespace() {
            spans.push(CellSpan::parse(entry)?);
        }
        Ok(Self {
            spans,
            source: Some(text.into()),
        })
    }

    /// The spans, in the order they were written.
    #[must_use]
    pub fn spans(&self) -> &[CellSpan] {
        &self.spans
    }

    /// How many spans the list holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.spans.len()
    }

    /// Whether the list holds no spans.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }

    /// Whether this list still replays the text it was parsed from.
    #[must_use]
    pub fn is_verbatim(&self) -> bool {
        self.source.is_some()
    }
}

impl fmt::Display for CellSpans {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(source) = &self.source {
            return f.write_str(source);
        }
        for (position, span) in self.spans.iter().enumerate() {
            if position > 0 {
                f.write_str(" ")?;
            }
            f.write_str(span.text().as_str())?;
        }
        Ok(())
    }
}

impl core::str::FromStr for CellSpans {
    type Err = AddressError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::parse(text)
    }
}

// ---------------------------------------------------------------------------------------------
// Sheet-qualified references
// ---------------------------------------------------------------------------------------------

/// A sheet's name as one reference spells it — the source slice, plus whether it was quoted.
///
/// Borrowed and `Copy`: naming a sheet in a reference allocates nothing, and unescaping only
/// allocates when there is an escape to undo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SheetName<'a> {
    /// The name exactly as written between the apostrophes (still carrying any `''`), or bare.
    raw: &'a str,
    quoted: bool,
}

impl SheetName<'_> {
    /// The name exactly as written, without the surrounding apostrophes and with any `''` escape
    /// still doubled. Allocation-free.
    #[must_use]
    pub fn raw(&self) -> &str {
        self.raw
    }

    /// Whether the reference wrapped the name in apostrophes.
    ///
    /// Excel quotes a name containing a space, a punctuation mark or a leading digit, and leaves a
    /// simple name bare. Which it did is preserved rather than re-decided.
    #[must_use]
    pub fn is_quoted(&self) -> bool {
        self.quoted
    }
}

impl<'a> SheetName<'a> {
    /// The logical sheet name, with `''` collapsed to `'`.
    ///
    /// Borrowed unless there was an escape to undo — a name without an apostrophe in it never
    /// allocates.
    #[must_use]
    pub fn name(&self) -> Cow<'a, str> {
        if self.quoted && self.raw.contains("''") {
            Cow::Owned(self.raw.replace("''", "'"))
        } else {
            Cow::Borrowed(self.raw)
        }
    }
}

impl fmt::Display for SheetName<'_> {
    /// Writes the name as it appeared, apostrophes included when it was quoted.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.quoted {
            write!(f, "'{}'", self.raw)
        } else {
            f.write_str(self.raw)
        }
    }
}

/// A reference qualified by a sheet, and optionally by an external book: `Sheet1!$A$1`,
/// `'My Sheet'!$A$1`, `'It''s'!A1`, `[1]Sheet1!A1`, `Sheet1:Sheet3!A1`.
///
/// # Exact by construction
///
/// The qualifier — everything up to and including the `!` — is kept as the borrowed source slice
/// and replayed verbatim, so however a producer spelled the quoting, the book index and the 3-D
/// span, that spelling comes back. The parsed pieces ([`external_book`](Self::external_book),
/// [`first_sheet`](Self::first_sheet), [`last_sheet`](Self::last_sheet)) are views into that same
/// slice, so the type is `Copy` and parsing one allocates nothing.
///
/// # A `!` inside quotes is part of the name
///
/// The sheet/reference split is the first `!` **outside** apostrophes, so `'Q1!Q2'!A1` names the
/// sheet `Q1!Q2` and not the sheet `'Q1`.
///
/// ```
/// use mjx_sml::SheetQualifiedReference;
/// let reference = SheetQualifiedReference::parse("'Q1!Q2'!$A$1").expect("a reference");
/// assert_eq!(reference.first_sheet().name(), "Q1!Q2");
/// assert_eq!(reference.to_string(), "'Q1!Q2'!$A$1");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SheetQualifiedReference<'a> {
    /// Everything before and including the `!`, exactly as written.
    qualifier: &'a str,
    external_book: Option<u32>,
    first_sheet: SheetName<'a>,
    last_sheet: Option<SheetName<'a>>,
    target: CellRange,
}

impl<'a> SheetQualifiedReference<'a> {
    /// Parses a sheet-qualified reference or range.
    ///
    /// # Errors
    ///
    /// [`AddressError::MissingSheetSeparator`] without an unquoted `!`,
    /// [`AddressError::UnterminatedSheetName`] for an unclosed apostrophe,
    /// [`AddressError::EmptySheetName`] for `"!A1"`, [`AddressError::InvalidExternalBookIndex`] for
    /// a bracket group that is not a number — and whatever [`CellRange::parse`] says about the part
    /// after the `!`.
    pub fn parse(text: &'a str) -> Result<Self, AddressError> {
        let separator = unquoted_separator(text)?;
        let qualifier = text.get(..=separator).unwrap_or("");
        let sheets = text.get(..separator).unwrap_or("");
        let target = CellRange::parse(text.get(separator + 1..).unwrap_or(""))?;

        // The bracket group sits outside the apostrophes in `[1]Sheet1!A1` and inside them in
        // `'[1]My Sheet'!A1` — Excel writes both — so it is stripped on either side of the quote.
        let (external_book, sheets) = take_external_book_index(sheets)?;
        let (mut first_sheet, rest) = take_sheet_name(sheets)?;
        let external_book = match external_book {
            Some(index) => Some(index),
            None => {
                let (index, raw) = take_external_book_index(first_sheet.raw)?;
                if index.is_some() {
                    if raw.is_empty() {
                        return Err(AddressError::EmptySheetName);
                    }
                    first_sheet.raw = raw;
                }
                index
            }
        };
        let last_sheet = match rest {
            "" => None,
            rest => {
                // A 3-D span, `Sheet1:Sheet3!A1`. A sheet name cannot itself contain `:`, so the
                // separator is unambiguous once quoting has been accounted for.
                let Some(remainder) = rest.strip_prefix(':') else {
                    return Err(AddressError::UnexpectedCharacter(
                        rest.chars().next().unwrap_or(':'),
                    ));
                };
                let (last, trailing) = take_sheet_name(remainder)?;
                if !trailing.is_empty() {
                    return Err(AddressError::UnexpectedCharacter(
                        trailing.chars().next().unwrap_or(':'),
                    ));
                }
                Some(last)
            }
        };

        Ok(Self {
            qualifier,
            external_book,
            first_sheet,
            last_sheet,
            target,
        })
    }

    /// The `1` of `[1]Sheet1!A1` — the index into the workbook's external-link list, if the
    /// reference names another book.
    #[must_use]
    pub fn external_book(&self) -> Option<u32> {
        self.external_book
    }

    /// The sheet named before the `!`, or the first of a 3-D span.
    #[must_use]
    pub fn first_sheet(&self) -> SheetName<'a> {
        self.first_sheet
    }

    /// The second sheet of a 3-D span (`Sheet1:Sheet3!A1`), if there is one.
    #[must_use]
    pub fn last_sheet(&self) -> Option<SheetName<'a>> {
        self.last_sheet
    }

    /// The reference or range after the `!`.
    #[must_use]
    pub fn target(&self) -> CellRange {
        self.target
    }

    /// Everything before and including the `!`, exactly as written.
    #[must_use]
    pub fn qualifier(&self) -> &'a str {
        self.qualifier
    }
}

impl fmt::Display for SheetQualifiedReference<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.qualifier)?;
        f.write_str(self.target.text().as_str())
    }
}

/// The byte offset of the first `!` that is not inside apostrophes.
fn unquoted_separator(text: &str) -> Result<usize, AddressError> {
    let bytes = text.as_bytes();
    let mut position = 0;
    let mut inside_quotes = false;
    while let Some(byte) = bytes.get(position) {
        match byte {
            b'\'' => inside_quotes = !inside_quotes,
            b'!' if !inside_quotes => return Ok(position),
            _ => {}
        }
        position += 1;
    }
    if inside_quotes {
        return Err(AddressError::UnterminatedSheetName);
    }
    Err(AddressError::MissingSheetSeparator)
}

/// Splits a leading `[n]` off the sheet part.
///
/// Excel writes the bracket group *inside* the apostrophes when the name needs quoting
/// (`'[1]My Sheet'!A1`), so this is applied to the unquoted text as well as to the bare form.
fn take_external_book_index(sheets: &str) -> Result<(Option<u32>, &str), AddressError> {
    let Some(after_bracket) = sheets.strip_prefix('[') else {
        return Ok((None, sheets));
    };
    let Some((digits, rest)) = after_bracket.split_once(']') else {
        return Err(AddressError::InvalidExternalBookIndex);
    };
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(AddressError::InvalidExternalBookIndex);
    }
    let index = digits
        .parse::<u32>()
        .map_err(|_| AddressError::InvalidExternalBookIndex)?;
    Ok((Some(index), rest))
}

/// Reads one sheet name — quoted or bare — and returns it with whatever follows.
fn take_sheet_name(sheets: &str) -> Result<(SheetName<'_>, &str), AddressError> {
    if let Some(after_quote) = sheets.strip_prefix('\'') {
        // Scan to the closing apostrophe, stepping over every `''` escape.
        let bytes = after_quote.as_bytes();
        let mut position = 0;
        while let Some(byte) = bytes.get(position) {
            if *byte == b'\'' {
                if bytes.get(position + 1) == Some(&b'\'') {
                    position += 2;
                    continue;
                }
                break;
            }
            position += 1;
        }
        if position >= bytes.len() {
            return Err(AddressError::UnterminatedSheetName);
        }
        let raw = after_quote.get(..position).unwrap_or("");
        if raw.is_empty() {
            return Err(AddressError::EmptySheetName);
        }
        return Ok((
            SheetName { raw, quoted: true },
            after_quote.get(position + 1..).unwrap_or(""),
        ));
    }

    // A bare name runs to the next `:` — a sheet name cannot contain one.
    let end = sheets.find(':').unwrap_or(sheets.len());
    let raw = sheets.get(..end).unwrap_or("");
    if raw.is_empty() {
        return Err(AddressError::EmptySheetName);
    }
    Ok((
        SheetName { raw, quoted: false },
        sheets.get(end..).unwrap_or(""),
    ))
}

// ---------------------------------------------------------------------------------------------
// R1C1
// ---------------------------------------------------------------------------------------------

/// One coordinate of an [`R1C1Reference`]: `5`, `[-1]`, or nothing at all.
///
/// Excel writes three spellings and they are three different values, so all three are modelled —
/// `RC` and `R[0]C[0]` mean the same cell and are not the same text, and this crate writes back
/// whichever it read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum R1C1Coordinate {
    /// `R5` / `C2` — a fixed position, held **zero-based** (`R5` is `Absolute(4)`).
    ///
    /// Building one past the grid by hand is a caller error; every parser and constructor here
    /// rejects it, and the renderer saturates rather than panicking.
    Absolute(u32),
    /// `R[-1]` / `C[2]` — an offset from the cell the formula is in, written in brackets.
    Offset(i32),
    /// A bare `R` or `C` — the same row or column as the formula's own cell. `RC` is what Excel
    /// writes for a zero offset; `R[0]C[0]` is the other spelling of the same thing.
    Same,
}

impl R1C1Coordinate {
    /// The offset from an anchor this coordinate implies, or `None` when it is absolute.
    #[must_use]
    pub fn offset(self) -> Option<i32> {
        match self {
            Self::Absolute(_) => None,
            Self::Offset(offset) => Some(offset),
            Self::Same => Some(0),
        }
    }

    fn write_into(self, out: &mut AddressText) {
        match self {
            Self::Absolute(index) => out.push_number(index.saturating_add(1)),
            Self::Offset(offset) => {
                out.push(b'[');
                out.push_signed(offset);
                out.push(b']');
            }
            Self::Same => {}
        }
    }
}

/// An R1C1-syntax reference: `R1C1`, `R[-1]C[2]`, `RC`, `R2C[3]`.
///
/// The syntax a workbook's formulas use is `calcPr@refMode` ([`ReferenceMode`]), and it is reported
/// rather than applied: there is deliberately no conversion between this type and
/// [`CellReference`], because rewriting a formula from one syntax to the other would change bytes in
/// a part nobody asked to edit. MJXOFF-115 (D11) carries formula text as its producer wrote it.
///
/// ```
/// use mjx_sml::{R1C1Coordinate, R1C1Reference};
/// let reference = R1C1Reference::parse("R[-1]C2").expect("an R1C1 reference");
/// assert_eq!(reference.row(), R1C1Coordinate::Offset(-1));
/// assert_eq!(reference.column(), R1C1Coordinate::Absolute(1)); // `C2` is zero-based 1
/// assert_eq!(reference.to_string(), "R[-1]C2");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct R1C1Reference {
    row: R1C1Coordinate,
    column: R1C1Coordinate,
}

impl R1C1Reference {
    /// A reference from its two coordinates.
    ///
    /// # Errors
    ///
    /// [`AddressError::RowOutOfGrid`], [`AddressError::ColumnOutOfGrid`] or
    /// [`AddressError::InvalidOffset`] if either coordinate falls outside the grid.
    pub fn new(row: R1C1Coordinate, column: R1C1Coordinate) -> Result<Self, AddressError> {
        check_r1c1_coordinate(row, LAST_ROW_INDEX, true)?;
        check_r1c1_coordinate(column, u32::from(LAST_COLUMN_INDEX), false)?;
        Ok(Self { row, column })
    }

    /// Parses `R…C…`.
    ///
    /// # Errors
    ///
    /// [`AddressError::MissingRowColumnMarker`] without both markers,
    /// [`AddressError::UnterminatedOffset`] for an unclosed `[`,
    /// [`AddressError::InvalidOffset`] for an offset that is empty, not a number, or past the grid,
    /// and [`AddressError::RowOutOfGrid`] / [`AddressError::ColumnOutOfGrid`] for an absolute
    /// position outside it.
    pub fn parse(text: &str) -> Result<Self, AddressError> {
        let bytes = text.as_bytes();
        if bytes.is_empty() {
            return Err(AddressError::Empty);
        }
        if bytes.first() != Some(&b'R') {
            return Err(AddressError::MissingRowColumnMarker);
        }
        let mut position = 1;
        let row = take_r1c1_coordinate(text, &mut position, LAST_ROW_INDEX, true)?;
        if bytes.get(position) != Some(&b'C') {
            return Err(AddressError::MissingRowColumnMarker);
        }
        position += 1;
        let column =
            take_r1c1_coordinate(text, &mut position, u32::from(LAST_COLUMN_INDEX), false)?;
        expect_end(text, position)?;
        Ok(Self { row, column })
    }

    /// The row coordinate.
    #[must_use]
    pub fn row(self) -> R1C1Coordinate {
        self.row
    }

    /// The column coordinate.
    #[must_use]
    pub fn column(self) -> R1C1Coordinate {
        self.column
    }

    /// This reference as text, in a stack buffer — exactly the spelling it was parsed from.
    #[must_use]
    pub fn text(self) -> AddressText {
        let mut out = AddressText::new();
        self.write_into(&mut out);
        out
    }

    fn write_into(self, out: &mut AddressText) {
        out.push(b'R');
        self.row.write_into(out);
        out.push(b'C');
        self.column.write_into(out);
    }
}

impl fmt::Display for R1C1Reference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.text().as_str())
    }
}

impl core::str::FromStr for R1C1Reference {
    type Err = AddressError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::parse(text)
    }
}

/// An R1C1-syntax range: `R1C1:R3C3`, or a single reference with no colon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum R1C1Range {
    /// A single reference, written without a colon.
    Reference(R1C1Reference),
    /// Two ends. Either order; the order is preserved.
    References {
        /// The end written before the colon.
        start: R1C1Reference,
        /// The end written after it.
        end: R1C1Reference,
    },
}

impl R1C1Range {
    /// Parses `R1C1` or `R1C1:R3C3`.
    ///
    /// # Errors
    ///
    /// [`AddressError::TooManyRangeEnds`] for more than two ends, and whatever
    /// [`R1C1Reference::parse`] says about either of them.
    pub fn parse(text: &str) -> Result<Self, AddressError> {
        let Some((start, end)) = text.split_once(':') else {
            return Ok(Self::Reference(R1C1Reference::parse(text)?));
        };
        if end.contains(':') {
            return Err(AddressError::TooManyRangeEnds);
        }
        Ok(Self::References {
            start: R1C1Reference::parse(start)?,
            end: R1C1Reference::parse(end)?,
        })
    }

    /// This range as text, in a stack buffer — exactly the spelling it was parsed from.
    #[must_use]
    pub fn text(self) -> AddressText {
        let mut out = AddressText::new();
        match self {
            Self::Reference(reference) => reference.write_into(&mut out),
            Self::References { start, end } => {
                start.write_into(&mut out);
                out.push(b':');
                end.write_into(&mut out);
            }
        }
        out
    }
}

impl fmt::Display for R1C1Range {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.text().as_str())
    }
}

impl core::str::FromStr for R1C1Range {
    type Err = AddressError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::parse(text)
    }
}

/// Rejects an R1C1 coordinate that falls outside the grid.
fn check_r1c1_coordinate(
    coordinate: R1C1Coordinate,
    last_index: u32,
    is_row: bool,
) -> Result<(), AddressError> {
    match coordinate {
        R1C1Coordinate::Same => Ok(()),
        R1C1Coordinate::Absolute(index) if index <= last_index => Ok(()),
        R1C1Coordinate::Absolute(index) => Err(if is_row {
            AddressError::RowOutOfGrid {
                row_number: u64::from(index) + 1,
            }
        } else {
            AddressError::ColumnOutOfGrid
        }),
        R1C1Coordinate::Offset(offset) if offset.unsigned_abs() <= last_index => Ok(()),
        R1C1Coordinate::Offset(_) => Err(AddressError::InvalidOffset),
    }
}

/// Reads one R1C1 coordinate at `position`: `[n]`, digits, or nothing.
fn take_r1c1_coordinate(
    text: &str,
    position: &mut usize,
    last_index: u32,
    is_row: bool,
) -> Result<R1C1Coordinate, AddressError> {
    let bytes = text.as_bytes();
    match bytes.get(*position) {
        Some(b'[') => {
            let start = *position + 1;
            let Some(offset) = text.get(start..).and_then(|rest| rest.find(']')) else {
                return Err(AddressError::UnterminatedOffset);
            };
            let digits = text.get(start..start + offset).unwrap_or("");
            *position = start + offset + 1;
            let value = parse_offset(digits)?;
            if value.unsigned_abs() > last_index {
                return Err(AddressError::InvalidOffset);
            }
            Ok(R1C1Coordinate::Offset(value))
        }
        Some(byte) if byte.is_ascii_digit() => {
            let mut number: u64 = 0;
            while let Some(byte) = bytes.get(*position) {
                if !byte.is_ascii_digit() {
                    break;
                }
                number = number
                    .saturating_mul(10)
                    .saturating_add(u64::from(byte - b'0'));
                *position += 1;
            }
            if number == 0 || number > u64::from(last_index) + 1 {
                return Err(if is_row {
                    AddressError::RowOutOfGrid { row_number: number }
                } else {
                    AddressError::ColumnOutOfGrid
                });
            }
            u32::try_from(number - 1)
                .map(R1C1Coordinate::Absolute)
                .map_err(|_| AddressError::InvalidOffset)
        }
        _ => Ok(R1C1Coordinate::Same),
    }
}

/// Parses the inside of an R1C1 bracket group: an optionally negative whole number.
fn parse_offset(digits: &str) -> Result<i32, AddressError> {
    if digits.is_empty() {
        return Err(AddressError::InvalidOffset);
    }
    let (negative, magnitude) = match digits.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, digits.strip_prefix('+').unwrap_or(digits)),
    };
    if magnitude.is_empty() || !magnitude.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(AddressError::InvalidOffset);
    }
    let value = magnitude
        .parse::<i32>()
        .map_err(|_| AddressError::InvalidOffset)?;
    Ok(if negative { -value } else { value })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `SplitMix64` generator, so the property cases below are a reproducible experiment rather
    /// than a different test on every run. Nine lines, no dependency — the same generator
    /// `xtask/src/fuzz/random.rs` uses, and for the same reason.
    struct Random(u64);

    impl Random {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        }

        fn below(&mut self, bound: u64) -> u64 {
            self.next() % bound
        }
    }

    const ALL_ANCHORINGS: [(Anchoring, Anchoring); 4] = [
        (Anchoring::Relative, Anchoring::Relative),
        (Anchoring::Absolute, Anchoring::Relative),
        (Anchoring::Relative, Anchoring::Absolute),
        (Anchoring::Absolute, Anchoring::Absolute),
    ];

    // -- column letters -------------------------------------------------------------------------

    /// The bijective base-26 boundaries, both ways.
    ///
    /// `Z` → `AA` is the carry that a plain base-26 conversion gets wrong (it answers `BA`), and
    /// `ZZ` → `AAA` is the same carry one digit further out, where a fix that only special-cases the
    /// first would still be wrong. `XFD` is the grid's last column and `XFE` its first invalid one.
    #[test]
    fn column_letters_and_indices_agree_at_every_boundary() {
        const BOUNDARIES: [(u16, &str); 8] = [
            (0, "A"),
            (25, "Z"),
            (26, "AA"),
            (51, "AZ"),
            (52, "BA"),
            (701, "ZZ"),
            (702, "AAA"),
            (16_383, "XFD"),
        ];
        for (index, letters) in BOUNDARIES {
            assert_eq!(
                column_letters(index).expect("in the grid").as_str(),
                letters,
                "encoding column {index}"
            );
            assert_eq!(
                column_index_from_letters(letters),
                Ok(index),
                "decoding {letters}"
            );
        }
    }

    /// Every column of the grid encodes and decodes back to itself — the exhaustive form of the
    /// boundary case above, which no single carry bug can survive.
    #[test]
    fn every_column_in_the_grid_round_trips() {
        for index in 0..=LAST_COLUMN_INDEX {
            let letters = column_letters(index).expect("in the grid");
            assert_eq!(
                column_index_from_letters(letters.as_str()),
                Ok(index),
                "column {index} rendered as {letters}"
            );
        }
    }

    /// `XFE` is past the last column, and the answer is a refusal — never `XFD`.
    ///
    /// A parser that clamps out-of-grid input would pass every test that only feeds it valid
    /// references. This one exists to fail for that parser.
    #[test]
    fn a_column_past_xfd_is_rejected_never_clamped() {
        for letters in ["XFE", "XFF", "XGA", "YAA", "ZZZ", "AAAA", "AAAAAAAA"] {
            assert_eq!(
                column_index_from_letters(letters),
                Err(AddressError::ColumnOutOfGrid),
                "{letters} names no column in the grid"
            );
        }
        assert_eq!(
            column_letters(LAST_COLUMN_INDEX + 1),
            Err(AddressError::ColumnOutOfGrid)
        );
        assert_eq!(column_letters(u16::MAX), Err(AddressError::ColumnOutOfGrid));
        assert_eq!(
            CellReference::parse("XFE1"),
            Err(AddressError::ColumnOutOfGrid)
        );
        // And the last valid one still works, so the rejection is a boundary and not a blanket.
        assert_eq!(
            CellReference::parse("XFD1048576")
                .expect("the grid's last cell")
                .column(),
            LAST_COLUMN_INDEX
        );
    }

    /// Lowercase is refused rather than folded — see [`column_index_from_letters`] on why.
    #[test]
    fn lowercase_column_letters_are_refused_not_folded() {
        assert_eq!(
            column_index_from_letters("a"),
            Err(AddressError::UnexpectedCharacter('a'))
        );
        assert_eq!(
            CellReference::parse("a1"),
            Err(AddressError::MissingColumnLetters)
        );
    }

    // -- cell references ------------------------------------------------------------------------

    #[test]
    fn all_four_absolute_forms_parse_to_the_same_cell_and_keep_their_markers() {
        const FORMS: [(&str, Anchoring, Anchoring); 4] = [
            ("B7", Anchoring::Relative, Anchoring::Relative),
            ("$B7", Anchoring::Absolute, Anchoring::Relative),
            ("B$7", Anchoring::Relative, Anchoring::Absolute),
            ("$B$7", Anchoring::Absolute, Anchoring::Absolute),
        ];
        for (text, column_anchoring, row_anchoring) in FORMS {
            let cell = CellReference::parse(text).expect("a cell reference");
            assert_eq!(cell.column(), 1, "{text}");
            assert_eq!(cell.row(), 6, "{text}");
            assert_eq!(cell.column_anchoring(), column_anchoring, "{text}");
            assert_eq!(cell.row_anchoring(), row_anchoring, "{text}");
            assert_eq!(cell.text().as_str(), text, "{text} must re-emit verbatim");
        }
    }

    /// `parse(format(x)) == x` and `format(parse(s)) == s` over random in-grid references, in
    /// **all four** absolute-flag combinations.
    ///
    /// The four combinations are the point. A generator that only ever emitted `A1`, `B2`, `C3`
    /// would pass for a formatter that silently dropped every `$`, which is the exact defect this
    /// case exists to catch — so the anchorings are drawn from
    /// [`ALL_ANCHORINGS`](self::tests::ALL_ANCHORINGS) rather than left relative, and the assertion
    /// below counts how many of the four were actually produced.
    #[test]
    fn every_absolute_flag_combination_round_trips_through_text() {
        let mut random = Random(0x5EED);
        let mut seen = [0usize; 4];
        for _ in 0..20_000 {
            let column = u16::try_from(random.below(u64::from(COLUMN_COUNT))).expect("in the grid");
            let row = u32::try_from(random.below(u64::from(ROW_COUNT))).expect("in the grid");
            let choice = usize::try_from(random.below(4)).expect("0..4");
            seen[choice] += 1;
            let (column_anchoring, row_anchoring) = ALL_ANCHORINGS[choice];
            let cell = CellReference::new(column, row, column_anchoring, row_anchoring)
                .expect("in the grid");

            let text = cell.text();
            assert_eq!(
                CellReference::parse(text.as_str()),
                Ok(cell),
                "{text} did not parse back to the reference that produced it"
            );
            // And the other direction: the text a parse consumed is the text a format produces.
            let reparsed = CellReference::parse(text.as_str()).expect("just formatted");
            assert_eq!(reparsed.text().as_str(), text.as_str());
        }
        assert!(
            seen.iter().all(|count| *count > 1_000),
            "every absolute-flag combination must actually be generated, got {seen:?}"
        );
    }

    /// The malformed set from MJXOFF-93's "Done when": each returns a typed error, none panics.
    #[test]
    fn malformed_references_are_typed_errors_and_never_panic() {
        const CASES: [(&str, AddressError); 12] = [
            ("", AddressError::Empty),
            ("$", AddressError::MissingColumnLetters),
            ("$$A1", AddressError::MissingColumnLetters),
            ("A0", AddressError::RowOutOfGrid { row_number: 0 }),
            ("1A", AddressError::UnexpectedCharacter('A')),
            ("AAAAAAAA1", AddressError::ColumnOutOfGrid),
            (
                "A99999999999",
                AddressError::RowOutOfGrid {
                    row_number: 99_999_999_999,
                },
            ),
            (
                "A99999999999999999999999999",
                AddressError::RowOutOfGrid {
                    row_number: u64::MAX,
                },
            ),
            ("A", AddressError::MissingRowNumber),
            ("A1x", AddressError::UnexpectedCharacter('x')),
            ("A 1", AddressError::MissingRowNumber),
            (
                "A1048577",
                AddressError::RowOutOfGrid {
                    row_number: 1_048_577,
                },
            ),
        ];
        for (text, expected) in CASES {
            assert_eq!(
                CellReference::parse(text),
                Err(expected),
                "parsing {text:?}"
            );
        }
        // A wider sweep, asserting only that nothing panics and nothing is silently accepted.
        for text in [
            "$$",
            ":",
            "::",
            "A1:",
            ":A1",
            "A1:B2:C3",
            "A:1",
            "A1:B",
            "$:$",
            "\u{1F600}1",
            "A\u{1F600}",
            "-1",
            "+A1",
            "A-1",
            "A1 ",
            " A1",
        ] {
            assert!(
                CellReference::parse(text).is_err(),
                "{text:?} is not a cell reference"
            );
        }
    }

    #[test]
    fn a_reference_is_eight_copyable_bytes() {
        // `Copy` is the compile-time proof that reading a million of these allocates nothing: a
        // `Copy` type cannot own a heap allocation.
        fn assert_copy<T: Copy>() {}
        assert_copy::<CellReference>();
        assert_copy::<CellRange>();
        assert_copy::<AddressText>();
        assert_copy::<R1C1Reference>();
        assert_copy::<SheetQualifiedReference<'_>>();
        assert_eq!(core::mem::size_of::<CellReference>(), 8);
    }

    /// The inline buffer is big enough for the longest thing this module can render — computed
    /// from the grid constants, not from a literal somebody could forget to update.
    #[test]
    fn every_rendering_fits_the_inline_buffer() {
        let last_cell = CellReference::new(
            LAST_COLUMN_INDEX,
            LAST_ROW_INDEX,
            Anchoring::Absolute,
            Anchoring::Absolute,
        )
        .expect("the grid's last cell");
        let widest_a1 = CellRange::Cells {
            start: last_cell,
            end: last_cell,
        };
        assert!(widest_a1.text().len() < ADDRESS_TEXT_CAPACITY);
        assert_eq!(widest_a1.text().as_str(), "$XFD$1048576:$XFD$1048576");

        let farthest = R1C1Reference::new(
            R1C1Coordinate::Offset(-(LAST_ROW_INDEX as i32)),
            R1C1Coordinate::Offset(-i32::from(LAST_COLUMN_INDEX)),
        )
        .expect("within the grid");
        let widest_r1c1 = R1C1Range::References {
            start: farthest,
            end: farthest,
        };
        assert!(widest_r1c1.text().len() < ADDRESS_TEXT_CAPACITY);
        assert_eq!(
            widest_r1c1.text().as_str(),
            "R[-1048575]C[-16383]:R[-1048575]C[-16383]"
        );
    }

    // -- ranges ---------------------------------------------------------------------------------

    #[test]
    fn every_range_form_round_trips_verbatim() {
        for text in [
            "A1",
            "A1:C3",
            "$A$1:$C$3",
            "A$1:$C3",
            "A:A",
            "$A:$C",
            "1:1",
            "$1:$3",
            "A1:A1",
            "C3:A1",
            "XFD1048576",
            "A1:XFD1048576",
        ] {
            let range = CellRange::parse(text).expect("a range");
            assert_eq!(range.text().as_str(), text, "{text} must re-emit verbatim");
        }
    }

    /// `A1` and `A1:A1` are different values, and stay different.
    #[test]
    fn the_degenerate_single_cell_form_is_not_widened() {
        let single = CellRange::parse("A1").expect("a range");
        let pair = CellRange::parse("A1:A1").expect("a range");
        assert!(single.is_single_cell());
        assert!(!pair.is_single_cell());
        assert_ne!(single, pair);
        assert_eq!(single.text().as_str(), "A1");
        assert_eq!(pair.text().as_str(), "A1:A1");
        assert_eq!(single.normalized_bounds(), pair.normalized_bounds());
    }

    /// A range Excel wrote backwards keeps its order; the ordered view is a separate answer.
    #[test]
    fn a_backwards_range_is_preserved_and_ordered_only_on_request() {
        let backwards = CellRange::parse("$C$3:$A$1").expect("a range");
        assert_eq!(backwards.text().as_str(), "$C$3:$A$1");
        let bounds = backwards.normalized_bounds();
        assert_eq!(bounds.first_column(), 0);
        assert_eq!(bounds.last_column(), 2);
        assert_eq!(bounds.first_row(), 0);
        assert_eq!(bounds.last_row(), 2);
        assert_eq!(bounds.cell_count(), 9);
        assert_eq!(
            bounds,
            CellRange::parse("A1:C3")
                .expect("a range")
                .normalized_bounds(),
            "both orders describe the same rectangle"
        );
    }

    #[test]
    fn whole_column_and_whole_row_ranges_widen_to_the_grid() {
        let columns = CellRange::parse("B:D")
            .expect("a range")
            .normalized_bounds();
        assert_eq!((columns.first_column(), columns.last_column()), (1, 3));
        assert_eq!(
            (columns.first_row(), columns.last_row()),
            (0, LAST_ROW_INDEX)
        );

        let rows = CellRange::parse("2:4")
            .expect("a range")
            .normalized_bounds();
        assert_eq!((rows.first_row(), rows.last_row()), (1, 3));
        assert_eq!(
            (rows.first_column(), rows.last_column()),
            (0, LAST_COLUMN_INDEX)
        );

        let whole_grid = CellRange::parse("A:XFD")
            .expect("a range")
            .normalized_bounds();
        assert_eq!(
            whole_grid.cell_count(),
            u64::from(COLUMN_COUNT) * u64::from(ROW_COUNT)
        );
    }

    #[test]
    fn a_range_reports_which_cells_it_contains() {
        let range = CellRange::parse("B2:D4").expect("a range");
        assert!(range.contains(CellReference::parse("C3").expect("a cell")));
        assert!(range.contains(CellReference::parse("$B$2").expect("a cell")));
        assert!(!range.contains(CellReference::parse("A1").expect("a cell")));
        assert!(!range.contains(CellReference::parse("E4").expect("a cell")));
    }

    #[test]
    fn malformed_ranges_are_typed_errors() {
        const CASES: [(&str, AddressError); 6] = [
            ("A1:B2:C3", AddressError::TooManyRangeEnds),
            ("A:1", AddressError::MismatchedRangeEnds),
            ("A1:B", AddressError::MismatchedRangeEnds),
            ("A1:", AddressError::Empty),
            (":A1", AddressError::Empty),
            ("A1:XFE1", AddressError::ColumnOutOfGrid),
        ];
        for (text, expected) in CASES {
            assert_eq!(CellRange::parse(text), Err(expected), "parsing {text:?}");
        }
    }

    // -- sqref ----------------------------------------------------------------------------------

    #[test]
    fn a_sqref_keeps_its_separator_run_until_it_is_edited() {
        let list = CellRangeList::parse("A1  B2:C3\tD4").expect("a sqref");
        assert_eq!(list.len(), 3);
        assert!(list.is_verbatim());
        assert_eq!(
            list.to_string(),
            "A1  B2:C3\tD4",
            "an untouched sqref re-emits verbatim, odd whitespace and all"
        );

        let mut edited = list.clone();
        edited.push(CellRange::parse("$E$5").expect("a range"));
        assert!(!edited.is_verbatim());
        assert_eq!(
            edited.to_string(),
            "A1 B2:C3 D4 $E$5",
            "an edited sqref is spelled the one way this crate spells one"
        );

        let mut shortened = list;
        assert_eq!(
            shortened.remove(0),
            Some(CellRange::parse("A1").expect("a range"))
        );
        assert_eq!(shortened.to_string(), "B2:C3 D4");
        assert_eq!(shortened.remove(9), None, "an absent index changes nothing");
    }

    #[test]
    fn an_empty_sqref_is_an_empty_list_and_still_verbatim() {
        for text in ["", "   "] {
            let list = CellRangeList::parse(text).expect("an empty sqref is a legal xsd:list");
            assert!(list.is_empty());
            assert_eq!(list.to_string(), text);
        }
    }

    #[test]
    fn a_sqref_reports_which_of_its_ranges_hold_a_cell() {
        let list = CellRangeList::parse("A1:B2 D4:E5").expect("a sqref");
        assert!(list.contains(CellReference::parse("B2").expect("a cell")));
        assert!(list.contains(CellReference::parse("E5").expect("a cell")));
        assert!(!list.contains(CellReference::parse("C3").expect("a cell")));
    }

    #[test]
    fn a_sqref_with_a_bad_entry_is_an_error() {
        assert_eq!(
            CellRangeList::parse("A1 XFE9"),
            Err(AddressError::ColumnOutOfGrid)
        );
    }

    // -- spans ----------------------------------------------------------------------------------

    #[test]
    fn spans_are_one_based_on_the_wire_and_zero_based_here() {
        let spans = CellSpans::parse("1:3").expect("a spans list");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans.spans()[0].first_column(), 0);
        assert_eq!(spans.spans()[0].last_column(), 2);
        assert_eq!(spans.to_string(), "1:3");
    }

    #[test]
    fn a_spans_list_keeps_its_separator_run() {
        let spans = CellSpans::parse("1:3  5:7").expect("a spans list");
        assert_eq!(spans.len(), 2);
        assert!(spans.is_verbatim());
        assert_eq!(spans.to_string(), "1:3  5:7");
    }

    /// A `spans` value is never invented and never dropped, which is why the type has no
    /// constructor that derives one from a row's cells. The half this level can assert is that a
    /// list built from nothing is empty and renders as nothing — an absent `spans` stays absent.
    #[test]
    fn an_absent_spans_value_is_not_invented() {
        let nothing = CellSpans::default();
        assert!(nothing.is_empty());
        assert!(!nothing.is_verbatim());
        assert_eq!(nothing.to_string(), "");
    }

    #[test]
    fn malformed_spans_are_typed_errors() {
        assert_eq!(
            CellSpan::parse("13"),
            Err(AddressError::MissingSpanSeparator)
        );
        assert_eq!(CellSpan::parse("0:3"), Err(AddressError::ColumnOutOfGrid));
        assert_eq!(
            CellSpan::parse("1:16385"),
            Err(AddressError::ColumnOutOfGrid)
        );
        assert_eq!(
            CellSpan::parse("A:C"),
            Err(AddressError::UnexpectedCharacter('A'))
        );
        assert_eq!(
            CellSpan::parse(":3"),
            Err(AddressError::MissingColumnLetters)
        );
        assert_eq!(
            CellSpan::parse("1:16384")
                .expect("the last column")
                .last_column(),
            LAST_COLUMN_INDEX
        );
    }

    #[test]
    fn a_span_written_backwards_is_preserved() {
        let spans = CellSpans::parse("3:1").expect("a spans list");
        assert_eq!(spans.to_string(), "3:1");
    }

    // -- sheet-qualified ------------------------------------------------------------------------

    #[test]
    fn every_sheet_qualified_form_round_trips_verbatim() {
        for text in [
            "Sheet1!A1",
            "Sheet1!$A$1",
            "'My Sheet'!$A$1",
            "'It''s'!A1",
            "'Q1!Q2'!A1",
            "[1]Sheet1!A1",
            "'[1]My Sheet'!A1",
            "Sheet1:Sheet3!A1",
            "'a b':'c d'!A1:C3",
            "Sheet1!A:A",
        ] {
            let reference = SheetQualifiedReference::parse(text).expect("a qualified reference");
            assert_eq!(reference.to_string(), text, "{text} must re-emit verbatim");
        }
    }

    #[test]
    fn a_quoted_sheet_name_is_unescaped_only_when_it_has_to_be() {
        let plain = SheetQualifiedReference::parse("'My Sheet'!A1").expect("a reference");
        assert_eq!(plain.first_sheet().name(), "My Sheet");
        assert!(matches!(plain.first_sheet().name(), Cow::Borrowed(_)));

        let escaped = SheetQualifiedReference::parse("'It''s'!A1").expect("a reference");
        assert_eq!(escaped.first_sheet().name(), "It's");
        assert!(matches!(escaped.first_sheet().name(), Cow::Owned(_)));
        assert_eq!(escaped.first_sheet().raw(), "It''s");
        assert!(escaped.first_sheet().is_quoted());
    }

    /// A `!` inside apostrophes belongs to the sheet name — the case the ticket calls out by name.
    #[test]
    fn a_bang_inside_quotes_does_not_split_the_reference() {
        let reference = SheetQualifiedReference::parse("'Q1!Q2'!$B$4").expect("a reference");
        assert_eq!(reference.first_sheet().name(), "Q1!Q2");
        assert_eq!(
            reference.target(),
            CellRange::parse("$B$4").expect("a range")
        );
        assert_eq!(reference.qualifier(), "'Q1!Q2'!");
    }

    #[test]
    fn an_external_book_index_is_read_inside_or_outside_the_quotes() {
        let bare = SheetQualifiedReference::parse("[1]Sheet1!A1").expect("a reference");
        assert_eq!(bare.external_book(), Some(1));
        assert_eq!(bare.first_sheet().name(), "Sheet1");

        let quoted = SheetQualifiedReference::parse("'[12]My Sheet'!A1").expect("a reference");
        assert_eq!(quoted.external_book(), Some(12));
        assert_eq!(quoted.first_sheet().name(), "My Sheet");
    }

    #[test]
    fn a_three_dimensional_span_names_both_sheets() {
        let span = SheetQualifiedReference::parse("Sheet1:Sheet3!A1").expect("a reference");
        assert_eq!(span.first_sheet().name(), "Sheet1");
        assert_eq!(
            span.last_sheet().map(|sheet| sheet.name().into_owned()),
            Some("Sheet3".to_owned())
        );

        let single = SheetQualifiedReference::parse("Sheet1!A1").expect("a reference");
        assert_eq!(single.last_sheet(), None);
    }

    #[test]
    fn malformed_sheet_qualified_references_are_typed_errors() {
        const CASES: [(&str, AddressError); 7] = [
            ("Sheet1A1", AddressError::MissingSheetSeparator),
            ("'My Sheet!A1", AddressError::UnterminatedSheetName),
            ("!A1", AddressError::EmptySheetName),
            ("''!A1", AddressError::EmptySheetName),
            ("[x]Sheet1!A1", AddressError::InvalidExternalBookIndex),
            ("[1Sheet1!A1", AddressError::InvalidExternalBookIndex),
            ("Sheet1!XFE1", AddressError::ColumnOutOfGrid),
        ];
        for (text, expected) in CASES {
            assert_eq!(
                SheetQualifiedReference::parse(text),
                Err(expected),
                "parsing {text:?}"
            );
        }
    }

    // -- R1C1 -----------------------------------------------------------------------------------

    #[test]
    fn every_r1c1_form_round_trips_verbatim() {
        for text in [
            "R1C1",
            "RC",
            "R[0]C[0]",
            "R[-1]C[2]",
            "R2C[3]",
            "R[-1]C",
            "RC[2]",
            "R1048576C16384",
            "R[-1048575]C[-16383]",
        ] {
            let reference = R1C1Reference::parse(text).expect("an R1C1 reference");
            assert_eq!(
                reference.text().as_str(),
                text,
                "{text} must re-emit verbatim"
            );
        }
        for text in ["R1C1:R3C3", "RC:R[1]C[1]", "R1C1"] {
            let range = R1C1Range::parse(text).expect("an R1C1 range");
            assert_eq!(range.text().as_str(), text, "{text} must re-emit verbatim");
        }
    }

    /// `RC` and `R[0]C[0]` mean the same cell and are not the same text.
    #[test]
    fn the_bare_and_bracketed_zero_offsets_stay_distinct() {
        let bare = R1C1Reference::parse("RC").expect("an R1C1 reference");
        let bracketed = R1C1Reference::parse("R[0]C[0]").expect("an R1C1 reference");
        assert_ne!(bare, bracketed);
        assert_eq!(bare.row(), R1C1Coordinate::Same);
        assert_eq!(bracketed.row(), R1C1Coordinate::Offset(0));
        assert_eq!(bare.row().offset(), Some(0));
        assert_eq!(bracketed.row().offset(), Some(0));
        assert_eq!(bare.text().as_str(), "RC");
        assert_eq!(bracketed.text().as_str(), "R[0]C[0]");
    }

    #[test]
    fn an_r1c1_absolute_position_is_one_based_on_the_wire() {
        let reference = R1C1Reference::parse("R5C2").expect("an R1C1 reference");
        assert_eq!(reference.row(), R1C1Coordinate::Absolute(4));
        assert_eq!(reference.column(), R1C1Coordinate::Absolute(1));
        assert_eq!(reference.row().offset(), None);
    }

    #[test]
    fn malformed_r1c1_references_are_typed_errors() {
        const CASES: [(&str, AddressError); 9] = [
            ("", AddressError::Empty),
            ("1C1", AddressError::MissingRowColumnMarker),
            ("R1", AddressError::MissingRowColumnMarker),
            ("R[-1C1", AddressError::UnterminatedOffset),
            ("R[]C1", AddressError::InvalidOffset),
            ("R[x]C1", AddressError::InvalidOffset),
            ("R[9999999]C1", AddressError::InvalidOffset),
            ("R0C1", AddressError::RowOutOfGrid { row_number: 0 }),
            ("R1C16385", AddressError::ColumnOutOfGrid),
        ];
        for (text, expected) in CASES {
            assert_eq!(
                R1C1Reference::parse(text),
                Err(expected),
                "parsing {text:?}"
            );
        }
        assert_eq!(
            R1C1Reference::parse("R1048577C1"),
            Err(AddressError::RowOutOfGrid {
                row_number: 1_048_577
            })
        );
        assert_eq!(
            R1C1Range::parse("R1C1:R2C2:R3C3"),
            Err(AddressError::TooManyRangeEnds)
        );
    }

    #[test]
    fn an_out_of_grid_r1c1_coordinate_is_refused_by_the_constructor() {
        assert_eq!(
            R1C1Reference::new(R1C1Coordinate::Absolute(ROW_COUNT), R1C1Coordinate::Same),
            Err(AddressError::RowOutOfGrid {
                row_number: u64::from(ROW_COUNT) + 1
            })
        );
        assert_eq!(
            R1C1Reference::new(R1C1Coordinate::Same, R1C1Coordinate::Absolute(COLUMN_COUNT)),
            Err(AddressError::ColumnOutOfGrid)
        );
        assert_eq!(
            R1C1Reference::new(R1C1Coordinate::Offset(i32::MIN), R1C1Coordinate::Same),
            Err(AddressError::InvalidOffset)
        );
    }

    /// The mode is the generated simple type, spelled with its own wire tokens — not a second
    /// enumeration declared here.
    #[test]
    fn the_reference_mode_is_the_generated_simple_type() {
        assert_eq!(ReferenceMode::from_wire("A1"), Some(ReferenceMode::A1));
        assert_eq!(ReferenceMode::from_wire("R1C1"), Some(ReferenceMode::R1C1));
        assert_eq!(ReferenceMode::A1.to_wire(), "A1");
        assert_eq!(ReferenceMode::from_wire("a1"), None);
        // The same type, not a copy of it.
        let generated: mjx_ooxml_types::spreadsheetml::ReferenceMode = ReferenceMode::R1C1;
        assert_eq!(generated.to_wire(), "R1C1");
    }

    // -- the seam with the rest of the crate ----------------------------------------------------

    #[test]
    fn an_address_error_reaches_sml_error_through_question_mark() {
        fn fail() -> Result<(), crate::SmlError> {
            CellReference::parse("XFE1")?;
            Ok(())
        }
        let error = fail().expect_err("XFE is out of the grid");
        assert!(matches!(
            error,
            crate::SmlError::Address(AddressError::ColumnOutOfGrid)
        ));
        assert_eq!(error.to_string(), AddressError::ColumnOutOfGrid.to_string());
    }

    #[test]
    fn the_string_conversions_agree_with_the_inherent_parsers() {
        assert_eq!(
            "$A$1".parse::<CellReference>(),
            CellReference::parse("$A$1")
        );
        assert_eq!("A1:C3".parse::<CellRange>(), CellRange::parse("A1:C3"));
        assert_eq!(
            "A1 B2"
                .parse::<CellRangeList>()
                .map(|list| list.to_string()),
            Ok("A1 B2".to_owned())
        );
        assert_eq!(
            "1:3".parse::<CellSpans>().map(|spans| spans.to_string()),
            Ok("1:3".to_owned())
        );
        assert_eq!("RC".parse::<R1C1Reference>(), R1C1Reference::parse("RC"));
        assert_eq!(
            "R1C1:R2C2".parse::<R1C1Range>(),
            R1C1Range::parse("R1C1:R2C2")
        );
    }

    #[test]
    fn address_text_behaves_like_the_string_it_holds() {
        let text = CellReference::parse("$B$7").expect("a cell").text();
        assert_eq!(text.len(), 4);
        assert!(!text.is_empty());
        assert_eq!(&*text, "$B$7");
        assert_eq!(text.as_ref() as &str, "$B$7");
        assert_eq!(text, "$B$7");
        assert_eq!(format!("{text}"), "$B$7");
        assert_eq!(format!("{text:?}"), "\"$B$7\"");
        assert!(AddressText::new().is_empty());
    }

    #[test]
    fn anchoring_spells_its_own_marker() {
        assert!(Anchoring::Absolute.is_absolute());
        assert!(!Anchoring::Relative.is_absolute());
        assert_eq!(Anchoring::Absolute.marker(), "$");
        assert_eq!(Anchoring::Relative.marker(), "");
        let cell = CellReference::relative(1, 6).expect("in the grid");
        assert_eq!(cell.text().as_str(), "B7");
        assert_eq!(
            cell.with_anchoring(Anchoring::Absolute, Anchoring::Absolute)
                .text()
                .as_str(),
            "$B$7"
        );
        assert_eq!(
            CellReference::absolute(1, 6)
                .expect("in the grid")
                .text()
                .as_str(),
            "$B$7"
        );
    }

    #[test]
    fn the_bounds_of_a_range_are_a_separate_value_with_its_own_accessors() {
        let bounds = CellRange::parse("B2:D5")
            .expect("a range")
            .normalized_bounds();
        assert_eq!(bounds.first_column(), 1);
        assert_eq!(bounds.last_column(), 3);
        assert_eq!(bounds.first_row(), 1);
        assert_eq!(bounds.last_row(), 4);
        assert_eq!(bounds.cell_count(), 12);
    }

    #[test]
    fn the_bound_types_refuse_an_out_of_grid_index_and_spell_themselves() {
        assert_eq!(
            ColumnBound::new(COLUMN_COUNT as u16, Anchoring::Relative),
            Err(AddressError::ColumnOutOfGrid)
        );
        assert_eq!(
            RowBound::new(ROW_COUNT, Anchoring::Relative),
            Err(AddressError::RowOutOfGrid {
                row_number: u64::from(ROW_COUNT) + 1
            })
        );
        assert_eq!(
            ColumnBound::new(0, Anchoring::Absolute)
                .expect("column A")
                .to_string(),
            "$A"
        );
        assert_eq!(
            RowBound::new(0, Anchoring::Absolute)
                .expect("row 1")
                .to_string(),
            "$1"
        );
        assert_eq!(
            ColumnBound::new(0, Anchoring::Relative)
                .expect("column A")
                .anchoring(),
            Anchoring::Relative
        );
        assert_eq!(
            RowBound::new(4, Anchoring::Relative).expect("row 5").row(),
            4
        );
        assert_eq!(
            ColumnBound::new(4, Anchoring::Relative)
                .expect("column E")
                .column(),
            4
        );
    }

    #[test]
    fn a_cell_reference_converts_into_a_range() {
        let cell = CellReference::parse("$D$4").expect("a cell");
        assert_eq!(CellRange::from(cell), CellRange::Cell(cell));
        assert_eq!(CellRange::from(cell).text().as_str(), "$D$4");
        assert_eq!(
            CellReference::new(0, 0, Anchoring::Relative, Anchoring::Relative)
                .expect("A1")
                .text()
                .as_str(),
            "A1"
        );
        assert_eq!(
            CellReference::new(0, ROW_COUNT, Anchoring::Relative, Anchoring::Relative),
            Err(AddressError::RowOutOfGrid {
                row_number: u64::from(ROW_COUNT) + 1
            })
        );
        assert_eq!(
            CellReference::new(
                COLUMN_COUNT as u16,
                0,
                Anchoring::Relative,
                Anchoring::Relative
            ),
            Err(AddressError::ColumnOutOfGrid)
        );
    }

    #[test]
    fn a_span_is_built_from_indices_and_refuses_an_out_of_grid_one() {
        let span = CellSpan::new(0, 2).expect("columns A to C");
        assert_eq!(span.text().as_str(), "1:3");
        assert_eq!(span.to_string(), "1:3");
        assert_eq!(
            CellSpan::new(0, COLUMN_COUNT as u16),
            Err(AddressError::ColumnOutOfGrid)
        );
    }

    #[test]
    fn a_range_list_can_be_built_from_ranges_and_renders_canonically() {
        let list = CellRangeList::from_ranges([
            CellRange::parse("A1").expect("a range"),
            CellRange::parse("$C$3:$D$4").expect("a range"),
        ]);
        assert!(!list.is_verbatim());
        assert_eq!(list.ranges().len(), 2);
        assert_eq!(list.to_string(), "A1 $C$3:$D$4");
        assert!(CellRangeList::default().is_empty());
    }
}
