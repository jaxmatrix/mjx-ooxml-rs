//! Borrowed views onto the packed records: [`Row`], [`Cell`], and the [`CellValue`] a caller writes.
//!
//! The store is three flat arrays and a byte arena, which is the right shape for a million cells and
//! the wrong shape to hand to a caller. These are the reading surface: two-word handles that resolve
//! a record and its byte ranges on demand, so nothing is decoded until somebody asks and nothing is
//! copied when they do.

use std::borrow::Cow;

use mjx_ooxml_types::spreadsheetml::CellType;

use crate::address::{AddressError, CellReference, CellSpans};

use super::attributes;
use super::record::{CellFlags, PayloadShape, RowFlags, NO_EXTRAS};
use super::store::SheetData;
use crate::arena::TextSpan;

/// One row of a [`SheetData`] — `CT_Row`.
///
/// # The twelve attributes, and where they live
///
/// `CT_Row` declares twelve. This store decodes exactly one of them, `r`, because `r` is the key it
/// is indexed by; the other eleven are read straight out of the bytes the file wrote, by the
/// accessors below. That is not laziness for its own sake — it is what lets a row's start tag come
/// back byte-identical, unmodelled attributes, quote characters, order and spacing included.
#[derive(Debug, Clone, Copy)]
pub struct Row<'a> {
    sheet: &'a SheetData,
    index: usize,
}

impl<'a> Row<'a> {
    pub(super) fn new(sheet: &'a SheetData, index: usize) -> Self {
        Self { sheet, index }
    }

    /// `row@r` — the one-based row number the file wrote, or `None` if it wrote none.
    ///
    /// A row without an `r` is legal: its position in the sheet is its number. It is returned as
    /// `None` rather than filled in, because a row that gained an `r` on the way out would be a
    /// silent repair.
    #[must_use]
    pub fn number(&self) -> Option<u32> {
        let record = self.record();
        record.has(RowFlags::HAS_NUMBER).then_some(record.number)
    }

    /// The still-escaped bytes of the attribute `name`, exactly as the file wrote them between the
    /// quotes.
    #[must_use]
    pub fn raw_attribute(&self, name: &str) -> Option<&'a str> {
        let run = self.sheet.arena_bytes(self.record().attributes);
        attributes::value(run, name).and_then(|value| core::str::from_utf8(value).ok())
    }

    /// The attribute `name` with its entity references resolved.
    ///
    /// # Errors
    ///
    /// [`mjx_xml::XmlError`] if the value carries a reference that cannot be decoded.
    pub fn attribute(&self, name: &str) -> Result<Option<Cow<'a, str>>, mjx_xml::XmlError> {
        self.raw_attribute(name)
            .map(mjx_xml::text::unescape_text)
            .transpose()
    }

    /// `row@spans` — the advisory hint naming the columns this row occupies, or `None` if the file
    /// wrote none.
    ///
    /// **Never derived.** LibreOffice writes no `spans` at all and Excel writes one on every row;
    /// computing one for a row that had none would change the bytes of a part nobody asked to edit,
    /// so there is no call here that does it.
    ///
    /// # Errors
    ///
    /// [`AddressError`] if the value is not a `ST_CellSpans` list.
    pub fn spans(&self) -> Result<Option<CellSpans>, AddressError> {
        self.raw_attribute("spans")
            .map(CellSpans::parse)
            .transpose()
    }

    /// `row@s` — the `cellXfs` index this row's cells inherit. Zero, the schema default, when the
    /// attribute is absent or unreadable.
    #[must_use]
    pub fn style(&self) -> u32 {
        self.raw_attribute("s")
            .and_then(|value| value.parse().ok())
            .unwrap_or(0)
    }

    /// `row@ht` — the row's height in points, or `None` when the file left it to the sheet default.
    #[must_use]
    pub fn height(&self) -> Option<f64> {
        self.raw_attribute("ht")?.parse().ok()
    }

    /// `row@outlineLevel` — the outline group depth. Zero, the schema default, when absent.
    #[must_use]
    pub fn outline_level(&self) -> u8 {
        self.raw_attribute("outlineLevel")
            .and_then(|value| value.parse().ok())
            .unwrap_or(0)
    }

    /// `row@customFormat` — whether the row carries a format of its own rather than the column's.
    #[must_use]
    pub fn uses_custom_format(&self) -> bool {
        self.boolean_attribute("customFormat")
    }

    /// `row@hidden`.
    #[must_use]
    pub fn is_hidden(&self) -> bool {
        self.boolean_attribute("hidden")
    }

    /// `row@customHeight` — whether [`height`](Self::height) was set by hand rather than fitted.
    #[must_use]
    pub fn uses_custom_height(&self) -> bool {
        self.boolean_attribute("customHeight")
    }

    /// `row@collapsed` — whether the outline group this row ends is collapsed.
    #[must_use]
    pub fn is_collapsed(&self) -> bool {
        self.boolean_attribute("collapsed")
    }

    /// `row@thickTop` — whether the row shows a thick border above it.
    #[must_use]
    pub fn has_thick_top_border(&self) -> bool {
        self.boolean_attribute("thickTop")
    }

    /// `row@thickBot` — whether the row shows a thick border below it.
    #[must_use]
    pub fn has_thick_bottom_border(&self) -> bool {
        self.boolean_attribute("thickBot")
    }

    /// `row@ph` — whether phonetic guides are shown for this row's East Asian text.
    #[must_use]
    pub fn shows_phonetic(&self) -> bool {
        self.boolean_attribute("ph")
    }

    /// This row's cells, in the order the file wrote them.
    pub fn cells(&self) -> impl ExactSizeIterator<Item = Cell<'a>> + 'a {
        let sheet = self.sheet;
        self.record()
            .cell_range()
            .map(move |index| Cell::new(sheet, index))
    }

    /// How many cells this row holds.
    #[must_use]
    pub fn cell_count(&self) -> usize {
        self.record().cell_count as usize
    }

    /// The cell in this row at `column` (zero-based), or `None`.
    #[must_use]
    pub fn cell(&self, column: u16) -> Option<Cell<'a>> {
        self.sheet
            .cell_position(self.index, column)
            .map(|index| Cell::new(self.sheet, index))
    }

    /// Whether this row can still be written straight out of the part's bytes — nothing in it has
    /// been edited since it was read.
    #[must_use]
    pub fn is_verbatim(&self) -> bool {
        !self.record().extent.is_none()
    }

    /// The bytes this row occupied in the part, or `None` for a row that was authored or has been
    /// edited.
    ///
    /// This is the row's copy-on-write state made visible, and the assertion an edit-isolation test
    /// is written against: after one cell elsewhere in the sheet changes, every other row must still
    /// answer with the same bytes it was read from.
    #[must_use]
    pub fn source_markup(&self) -> Option<&'a [u8]> {
        let extent = self.record().extent;
        (!extent.is_none()).then(|| self.sheet.arena_bytes(extent))
    }

    /// This row as it would be written — a copy of the part's bytes when the row is untouched, and a
    /// rebuild otherwise.
    #[must_use]
    pub fn markup(&self) -> Vec<u8> {
        let mut out = Vec::new();
        super::write::write_row(self.sheet, self.index, &mut out);
        out
    }

    fn record(&self) -> &'a super::record::PackedRow {
        self.sheet.row_record(self.index)
    }

    /// An `xsd:boolean` attribute, defaulting to `false` when absent or unreadable.
    ///
    /// `xsd:boolean` is exactly `true`, `false`, `1` and `0` — not the wider `ST_OnOff` family,
    /// which also accepts `on`/`off`. `CT_Row`'s seven flags are declared `xsd:boolean`, so this
    /// reads that and nothing else; a value outside the four is left to the byte-level preservation
    /// that keeps it in the file, and read here as the schema default.
    fn boolean_attribute(&self, name: &str) -> bool {
        matches!(self.raw_attribute(name), Some("true" | "1"))
    }
}

/// One cell of a [`SheetData`] — `CT_Cell`.
#[derive(Debug, Clone, Copy)]
pub struct Cell<'a> {
    sheet: &'a SheetData,
    index: usize,
}

impl<'a> Cell<'a> {
    pub(super) fn new(sheet: &'a SheetData, index: usize) -> Self {
        Self { sheet, index }
    }

    /// The cell's address.
    ///
    /// `c@r` when the file wrote one — anchoring and all, so `$B$7` comes back `$B$7` — and the
    /// address its position implies when it did not.
    #[must_use]
    pub fn reference(&self) -> CellReference {
        self.record().reference
    }

    /// Whether `c@r` was written. A cell without one takes its address from its position, and must
    /// not gain the attribute on the way out.
    #[must_use]
    pub fn has_written_reference(&self) -> bool {
        self.record().has(CellFlags::HAS_REFERENCE)
    }

    /// `c@t`, with the schema default applied — [`CellType::Number`] for a cell that wrote none.
    #[must_use]
    pub fn cell_type(&self) -> CellType {
        self.record()
            .written_cell_type()
            .unwrap_or(CellType::Number)
    }

    /// `c@t` exactly as written, or `None` when the attribute was absent.
    ///
    /// The distinction is not pedantry: `n` *is* the default, so an absent `t` and `t="n"` mean the
    /// same thing and are different bytes, and a file that wrote nothing must come back writing
    /// nothing.
    #[must_use]
    pub fn written_cell_type(&self) -> Option<CellType> {
        self.record().written_cell_type()
    }

    /// `c@s` — the `cellXfs` index. Zero, the schema default, when absent.
    #[must_use]
    pub fn style(&self) -> u32 {
        self.record().style
    }

    /// Whether `c@s` was written, even as the default `0`.
    #[must_use]
    pub fn has_written_style(&self) -> bool {
        self.record().has(CellFlags::HAS_STYLE)
    }

    /// The still-escaped text of the attribute `name` — including `cm`, `vm`, `ph` and anything
    /// this workspace does not model.
    ///
    /// Borrowed from the part's bytes for a cell that keeps a verbatim attribute run, and owned for
    /// a cell whose start tag is regenerated from its decoded fields — where the value is not stored
    /// as text anywhere, because storing it would be storing it a million times.
    #[must_use]
    pub fn raw_attribute(&self, name: &str) -> Option<Cow<'a, str>> {
        if let Some(run) = self.sheet.cell_attribute_run(self.index) {
            return attributes::value(run, name)
                .and_then(|value| core::str::from_utf8(value).ok())
                .map(Cow::Borrowed);
        }
        // A cell whose start tag this store would have written identically keeps no run: its
        // attributes *are* the decoded fields, so answer from those.
        match name {
            "r" if self.has_written_reference() => {
                Some(Cow::Owned(self.reference().text().as_str().to_owned()))
            }
            "s" if self.has_written_style() => Some(Cow::Owned(self.style().to_string())),
            "t" => self
                .written_cell_type()
                .map(|cell_type| Cow::Borrowed(cell_type.to_wire())),
            _ => None,
        }
    }

    /// The raw, still-escaped text inside the cell's `<v>`, or `None` when it has none.
    #[must_use]
    pub fn raw_value(&self) -> Option<&'a [u8]> {
        let record = self.record();
        (record.payload_shape() == PayloadShape::ValueText)
            .then(|| self.sheet.arena_bytes(record.payload))
    }

    /// The cell's `<v>` text with its entity references resolved.
    ///
    /// # Errors
    ///
    /// [`mjx_xml::XmlError`] if the text carries a reference that cannot be decoded, or is not UTF-8.
    pub fn value(&self) -> Result<Option<Cow<'a, str>>, mjx_xml::XmlError> {
        let Some(raw) = self.raw_value() else {
            return Ok(None);
        };
        let text = core::str::from_utf8(raw)
            .map_err(|_| mjx_xml::XmlError::Syntax("a cell value was not UTF-8".to_owned()))?;
        mjx_xml::text::unescape_text(text).map(Some)
    }

    /// The shared-string index this cell names, for a cell whose `t` is `s`.
    ///
    /// `None` for any other type, and for an `s` cell whose `<v>` does not hold an index — which a
    /// file can do, and which this reports as absence rather than repairing. MJXOFF-97 (D05) owns
    /// the table these index into; this is the contract it is read through.
    #[must_use]
    pub fn shared_string_index(&self) -> Option<u32> {
        if self.cell_type() != CellType::SharedString {
            return None;
        }
        core::str::from_utf8(self.raw_value()?).ok()?.parse().ok()
    }

    /// The boolean this cell holds, for a cell whose `t` is `b`.
    #[must_use]
    pub fn boolean(&self) -> Option<bool> {
        if self.cell_type() != CellType::Boolean {
            return None;
        }
        match self.raw_value()? {
            b"1" | b"true" => Some(true),
            b"0" | b"false" => Some(false),
            _ => None,
        }
    }

    /// The number this cell holds, for a cell whose type is `n` — written or defaulted.
    #[must_use]
    pub fn number(&self) -> Option<f64> {
        if self.cell_type() != CellType::Number {
            return None;
        }
        core::str::from_utf8(self.raw_value()?).ok()?.parse().ok()
    }

    /// The whole `<is>…</is>` of an inline-string cell, verbatim.
    ///
    /// `CT_Rst` is rich text — `t`, `r*`, `rPh*`, `phoneticPr?` — and MJXOFF-97 (D05) models it,
    /// beside the shared-string table it shares its type with. Until then the store's contract for
    /// it is preservation, which is exact.
    #[must_use]
    pub fn inline_string_markup(&self) -> Option<&'a [u8]> {
        let record = self.record();
        (record.payload_shape() == PayloadShape::InlineString)
            .then(|| self.sheet.arena_bytes(record.payload))
    }

    /// The cell's `<f …>…</f>`, verbatim, or `None` if it has no formula.
    ///
    /// **This is the opaque handle this child promises and MJXOFF-115 (D11) parses.** A formula is
    /// preserved byte for byte — its `t`, `ref`, `si`, `ca` and text alike — and nothing here reads
    /// into it. Storing it as bytes rather than as an index into a formula table is deliberate: a
    /// table index costs four bytes on *every* cell for a feature most cells do not have, and the
    /// bytes are where a faithful round-trip needs the formula to be anyway.
    #[must_use]
    pub fn formula_markup(&self) -> Option<&'a [u8]> {
        let span = self.extras().map(|extras| extras.formula)?;
        (!span.is_none()).then(|| self.sheet.arena_bytes(span))
    }

    /// Whether this cell carries a formula.
    #[must_use]
    pub fn has_formula(&self) -> bool {
        self.formula_markup().is_some()
    }

    /// Everything the cell holds **before** its value element — its formula, and any markup this
    /// workspace does not model that the file put ahead of the value.
    #[must_use]
    pub fn markup_before_value(&self) -> &'a [u8] {
        self.extras()
            .map(|extras| self.sheet.arena_bytes(extras.before_payload))
            .unwrap_or_default()
    }

    /// Everything the cell holds **after** its value element — its `extLst`, and any other markup
    /// this workspace does not model.
    ///
    /// This is the unknown bucket, in the shape a packed store can afford: raw bytes rather than a
    /// `Vec<RawNode>` per cell. It preserves order, prefixes and the whitespace inside a start tag,
    /// which is strictly more than a decomposed node records.
    #[must_use]
    pub fn markup_after_value(&self) -> &'a [u8] {
        self.extras()
            .map(|extras| self.sheet.arena_bytes(extras.after_payload))
            .unwrap_or_default()
    }

    /// The bytes between the previous sibling and this cell — the whitespace a pretty-printer wrote,
    /// a comment, or an element that is not a `c`.
    #[must_use]
    pub fn leading_markup(&self) -> &'a [u8] {
        self.extras()
            .map(|extras| self.sheet.arena_bytes(extras.leading))
            .unwrap_or_default()
    }

    /// Whether this cell can still be written straight out of the part's bytes.
    #[must_use]
    pub fn is_verbatim(&self) -> bool {
        !self.record().extent.is_none()
    }

    /// The bytes this cell occupied in the part, or `None` once it is edited or if it was authored.
    #[must_use]
    pub fn source_markup(&self) -> Option<&'a [u8]> {
        let extent = self.record().extent;
        (!extent.is_none()).then(|| self.sheet.arena_bytes(extent))
    }

    /// This cell as it would be written.
    #[must_use]
    pub fn markup(&self) -> Vec<u8> {
        let mut out = Vec::new();
        super::write::write_cell(self.sheet, self.index, &mut out);
        out
    }

    fn record(&self) -> &'a super::record::PackedCell {
        self.sheet.cell_record(self.index)
    }

    fn extras(&self) -> Option<&'a super::record::CellExtras> {
        let extra = self.record().extra;
        (extra != NO_EXTRAS).then(|| self.sheet.cell_extras_record(extra))
    }
}

/// A value a caller writes into a cell.
///
/// The `c@t` that goes with each is decided here rather than by the caller, because the pair *is*
/// the value: a `<v>` reading `3` is a number, a shared-string index or an error code depending
/// entirely on the `t` beside it, and letting the two be set separately is how a store ends up
/// holding a cell that says one thing and means another.
///
/// [`Number`](Self::Number) writes no `t` at all, because `n` is the schema default and a file that
/// would not have written the attribute must not gain one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CellValue<'a> {
    /// No value element at all. The cell survives — a blank cell still carries a style.
    Blank,
    /// A number, written in Rust's shortest round-tripping spelling.
    ///
    /// `NaN` and the infinities are refused with
    /// [`SmlError::UnrepresentableNumber`](crate::SmlError::UnrepresentableNumber) rather than
    /// written: SpreadsheetML has no numeric spelling for them, and Excel writes an error cell —
    /// [`Error`](Self::Error) with `#NUM!` — in their place.
    Number(f64),
    /// A number written exactly as given, for a caller who has a spelling to preserve — `1.0`,
    /// `1e-7`, `0.30000000000000004`. Escaped as character data before it is written.
    NumberText(&'a str),
    /// An index into the shared-string table. Writes `t="s"`.
    SharedString(u32),
    /// A boolean, written `1` or `0`. Writes `t="b"`.
    Boolean(bool),
    /// An error code — `#DIV/0!`, `#N/A`. Writes `t="e"`.
    Error(&'a str),
    /// The string result of a formula. Writes `t="str"`.
    FormulaString(&'a str),
    /// A string stored in the cell itself rather than in the shared-string table. Writes
    /// `t="inlineStr"` and an `<is><t>…</t></is>`.
    InlineString(&'a str),
}

impl SheetData {
    /// The bytes a span covers. Crate-internal, and the one door the views resolve spans through.
    pub(super) fn arena_bytes(&self, span: TextSpan) -> &[u8] {
        self.arena.bytes(span)
    }

    pub(super) fn row_record(&self, index: usize) -> &super::record::PackedRow {
        &self.rows[index]
    }

    pub(super) fn cell_record(&self, index: usize) -> &super::record::PackedCell {
        &self.cells[index]
    }

    pub(super) fn cell_extras_record(&self, extra: u32) -> &super::record::CellExtras {
        &self.cell_extras[extra as usize]
    }

    /// The verbatim attribute run of the cell at `index`, or `None` when the cell's start tag is
    /// regenerated from its decoded fields.
    pub(super) fn cell_attribute_run(&self, index: usize) -> Option<&[u8]> {
        self.cell_extra_span(index, |extras| extras.attributes)
            .map(|span| self.arena.bytes(span))
    }

    /// The `sheetData` start tag's attribute run.
    pub(super) fn attribute_run(&self) -> &[u8] {
        self.arena.bytes(self.attributes)
    }

    /// The bytes after the last row and before the end tag.
    pub(super) fn trailing_bytes(&self) -> &[u8] {
        self.arena.bytes(self.trailing)
    }

    /// The bytes between the previous row and the row at `index`.
    pub(super) fn row_leading_bytes(&self, index: usize) -> &[u8] {
        self.arena.bytes(self.rows[index].leading)
    }

    /// The bytes between the previous sibling and the cell at `index`.
    pub(super) fn cell_leading_bytes(&self, index: usize) -> &[u8] {
        self.cell_extra_span(index, |extras| extras.leading)
            .map(|span| self.arena.bytes(span))
            .unwrap_or_default()
    }

    /// The cell's children before its value element.
    pub(super) fn cell_before_payload_bytes(&self, index: usize) -> &[u8] {
        self.cell_extra_span(index, |extras| extras.before_payload)
            .map(|span| self.arena.bytes(span))
            .unwrap_or_default()
    }

    /// The cell's children after its value element.
    pub(super) fn cell_after_payload_bytes(&self, index: usize) -> &[u8] {
        self.cell_extra_span(index, |extras| extras.after_payload)
            .map(|span| self.arena.bytes(span))
            .unwrap_or_default()
    }

    /// Whether the `sheetData` element was written `<sheetData/>`.
    pub(super) fn was_self_closing(&self) -> bool {
        self.self_closing
    }

    /// The prefix new markup is written with.
    pub(super) fn element_prefix(&self) -> Option<&str> {
        self.prefix.as_deref()
    }

    /// The whole sheet's bytes, if nothing in it has been touched.
    pub(super) fn verbatim_sheet(&self) -> Option<&[u8]> {
        (!self.extent.is_none()).then(|| self.arena.bytes(self.extent))
    }

    /// The row's bytes, if nothing in it has been touched.
    pub(super) fn verbatim_row(&self, index: usize) -> Option<&[u8]> {
        let extent = self.rows[index].extent;
        (!extent.is_none()).then(|| self.arena.bytes(extent))
    }

    /// The cell's bytes, if it has not been touched.
    pub(super) fn verbatim_cell(&self, index: usize) -> Option<&[u8]> {
        let extent = self.cells[index].extent;
        (!extent.is_none()).then(|| self.arena.bytes(extent))
    }

    /// One field of the cell's side-table record, when it has one and the field is present.
    fn cell_extra_span(
        &self,
        index: usize,
        field: impl Fn(&super::record::CellExtras) -> TextSpan,
    ) -> Option<TextSpan> {
        let extra = self.cells[index].extra;
        if extra == NO_EXTRAS {
            return None;
        }
        let span = field(&self.cell_extras[extra as usize]);
        (!span.is_none()).then_some(span)
    }
}
