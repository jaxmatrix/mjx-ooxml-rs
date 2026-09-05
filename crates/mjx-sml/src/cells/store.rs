//! [`SheetData`] itself: the rows, the cell arena, the sparse index and the edit surface.

use std::sync::Arc;

use mjx_ooxml_core::{Interner, RawDocument, RawElement, RawNode};
use mjx_xml::text::escape_attribute;

use crate::address::{CellReference, CellSpans};
use crate::error::SmlError;

use super::attributes;
use super::record::{
    CellExtras, CellFlags, CellTypeCode, PackedCell, PackedRow, PayloadShape, RowFlags, NO_EXTRAS,
};
use super::text::{TextArena, TextSpan};
use super::view::{Cell, CellValue, Row};

/// `CT_SheetData` — every row and every cell of one worksheet, held as packed records over one byte
/// arena.
///
/// See the [module docs](super) for the representation and what it was chosen against, and
/// `cells/record.rs` for the byte count of each record and the alternatives that were costed.
///
/// # What "sparse" means here, exactly
///
/// Rows are stored **by their `r`, never by grid position**. A sheet whose only cell is
/// `XFD1048576` holds one row record and one cell record — not 1,048,576 row slots — and the
/// allocation gate in `tests/cell_store_allocation.rs` asserts that as a byte bound with a counting
/// global allocator rather than by inspection.
///
/// The index that answers *"the cell at B7"* is the ordering itself: rows are kept in the order the
/// file wrote them, that order is ascending for every file any producer writes, and a flag records
/// whether it actually is. When it is, a lookup is a binary search over the rows and a second over
/// the row's cells — `O(log rows + log columns)`. When a file wrote its rows or its cells out of
/// order, the flag is cleared and the lookup falls back to a scan of that one dimension: **the
/// answer stays right and nothing is re-sorted**, because sorting them would change the bytes of a
/// part nobody asked to edit.
///
/// # Copy-on-write, three deep
///
/// The sheet, each row and each cell carry a byte range of their own — the extent they occupied in
/// the part's source — and a range that is still present means "these bytes still say exactly this".
/// Writing walks outward-in: an untouched sheet is one `memcpy`; a sheet with one edited cell copies
/// every other row verbatim, and inside the edited row copies every other *cell* verbatim. Every
/// mutation clears the extent on the cell it touched, on its row and on the sheet — which is the
/// same rule `RawElement`'s [`DerefMut`](mjx_ooxml_core::RawElement) enforces for a tree, applied to
/// a store that is not one.
#[derive(Debug)]
pub struct SheetData {
    /// Every byte the store preserves, source and edited alike.
    pub(super) arena: TextArena,
    /// The rows, in the order the file wrote them.
    pub(super) rows: Vec<PackedRow>,
    /// Every row's cells, end to end. A row owns the slice [`PackedRow::cell_range`] names.
    pub(super) cells: Vec<PackedCell>,
    /// The rare per-cell data. Indices into this are stable across insertions.
    pub(super) cell_extras: Vec<CellExtras>,
    /// The whole `<sheetData …>…</sheetData>`, or [`TextSpan::NONE`] once anything is edited.
    pub(super) extent: TextSpan,
    /// The `sheetData` start tag's attribute run — everything between `<sheetData` and the `>` that
    /// closes it. `CT_SheetData` declares no attributes, so this is empty for every file anyone
    /// writes; it is kept because "declares none" and "carries none" are different claims, and the
    /// second is the one a round-trip depends on.
    pub(super) attributes: TextSpan,
    /// Bytes after the last row and before the end tag. When there are no rows, all of the content.
    pub(super) trailing: TextSpan,
    /// The namespace prefix new elements are written with — `Some("x")` for `<x:row>`, `None` for a
    /// default-namespace document. Learned from the `sheetData` element itself.
    pub(super) prefix: Option<Box<str>>,
    /// Whether [`Self::rows`] is in strictly ascending `r` order, so a row can be found by binary
    /// search.
    pub(super) rows_ascending: bool,
    /// Whether the element was written `<sheetData/>`.
    pub(super) self_closing: bool,
}

impl Default for SheetData {
    fn default() -> Self {
        Self::authored(None)
    }
}

impl SheetData {
    /// An empty sheet, authored rather than read, whose elements will be written with `prefix`.
    ///
    /// Nothing here points at a source buffer, so every row it gains is written from the model.
    #[must_use]
    pub fn authored(prefix: Option<&str>) -> Self {
        Self {
            arena: TextArena::default(),
            rows: Vec::new(),
            cells: Vec::new(),
            cell_extras: Vec::new(),
            extent: TextSpan::NONE,
            attributes: TextSpan::NONE,
            trailing: TextSpan::NONE,
            prefix: prefix.map(Box::from),
            rows_ascending: true,
            self_closing: false,
        }
    }

    /// Reads the `sheetData` of a parsed worksheet part.
    ///
    /// Finds the one `sheetData` child of the document's root and reads it, sharing the document's
    /// source buffer so that untouched rows re-emit from the part's own bytes. Returns `Ok(None)`
    /// for a worksheet that has no `sheetData` at all, which the schema permits.
    ///
    /// # Errors
    ///
    /// [`SmlError`] as [`read`](Self::read).
    pub fn read_worksheet(document: &RawDocument) -> Result<Option<Self>, SmlError> {
        for child in document.root.children.iter() {
            let RawNode::Element(element) = child else {
                continue;
            };
            if document.interner.resolve(element.name.local) != "sheetData" {
                continue;
            }
            return Self::read(element, &document.interner, document.shared_source()).map(Some);
        }
        Ok(None)
    }

    /// Reads a `sheetData` element into the store.
    ///
    /// `source` is the buffer the element's [source
    /// ranges](mjx_ooxml_core::RawElement::source_span) were measured against — pass
    /// [`RawDocument::shared_source`], which hands over the `Arc` the package already holds rather
    /// than a copy. Without it every row is still read correctly; it simply has no bytes to re-emit
    /// verbatim from, so a save reflows.
    ///
    /// # Errors
    ///
    /// [`SmlError::Address`] for a `c@r` that is not a cell reference — the store is *keyed* on that
    /// value, and a key it cannot parse is not something it can preserve into a working index.
    /// [`SmlError::SheetDataTooLarge`] for a part beyond the `u32` byte space. Everything else a file
    /// can get wrong — a row out of order, a duplicate reference, a `c@r` that names a different row
    /// than its `row@r` does, a `t` that disagrees with the child element present — is **preserved
    /// as read** and reported by [`anomalies`](Self::anomalies), never repaired and never a panic.
    pub fn read(
        element: &RawElement,
        interner: &Interner,
        source: Option<&Arc<[u8]>>,
    ) -> Result<Self, SmlError> {
        super::read::read_sheet_data(element, interner, source)
    }

    /// Appends this sheet's `<sheetData>` element, bytes and all, to `out`.
    ///
    /// A sheet nobody edited is one copy out of the part's buffer. A sheet with one edited cell
    /// copies every untouched row verbatim and, inside the edited row, every untouched cell.
    pub fn write_into(&self, out: &mut Vec<u8>) {
        super::write::write_sheet_data(self, out);
    }

    /// This sheet's `<sheetData>` element as bytes.
    #[must_use]
    pub fn to_markup(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.write_into(&mut out);
        out
    }

    // -------------------------------------------------------------------------------------------
    // Reading
    // -------------------------------------------------------------------------------------------

    /// How many rows the sheet holds — populated rows, not addressable ones.
    #[must_use]
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// How many cells the sheet holds, across every row.
    #[must_use]
    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }

    /// Every row, in the order the file wrote them.
    pub fn rows(&self) -> impl ExactSizeIterator<Item = Row<'_>> + '_ {
        (0..self.rows.len()).map(move |index| Row::new(self, index))
    }

    /// Every cell of the sheet, row by row and in each row's own order.
    pub fn cells(&self) -> impl ExactSizeIterator<Item = Cell<'_>> + '_ {
        (0..self.cells.len()).map(move |index| Cell::new(self, index))
    }

    /// The row a file numbered `number` (one-based, as `row@r` writes it).
    ///
    /// A binary search when the rows are ascending, which they are for every file a producer writes;
    /// a scan otherwise. When a file wrote the same `r` twice, this answers with the **first** of
    /// them — both are kept, and [`anomalies`](Self::anomalies) names the duplicate.
    #[must_use]
    pub fn row(&self, number: u32) -> Option<Row<'_>> {
        self.row_position(number).map(|index| Row::new(self, index))
    }

    /// The cell at `reference`, or `None`.
    ///
    /// `O(log rows + log columns)` for an ordered sheet. The row is found by the reference's row,
    /// and the column within it by the reference's column; **anchoring is not part of the key**,
    /// because `$B$7` and `B7` name the same cell and differ only in how a formula copies.
    #[must_use]
    pub fn cell(&self, reference: CellReference) -> Option<Cell<'_>> {
        let row = self.row_position(reference.row().saturating_add(1))?;
        let index = self.cell_position(row, reference.column())?;
        Some(Cell::new(self, index))
    }

    /// Whether the whole sheet can still be written straight out of the part's bytes.
    #[must_use]
    pub fn is_verbatim(&self) -> bool {
        !self.extent.is_none()
    }

    /// How many bytes of its own the store has authored — zero until something is edited.
    ///
    /// This is the copy-on-write rule stated as a number: a worksheet read and saved without an edit
    /// owns no bytes at all beyond its records, because every value it preserves is a range into the
    /// part's buffer.
    #[must_use]
    pub fn edited_bytes(&self) -> usize {
        self.arena.edited_bytes()
    }

    /// The store's own heap footprint, in bytes, for **documentation** — the record arenas plus the
    /// bytes it has authored.
    ///
    /// # This is not the memory gate, and cannot be
    ///
    /// It reports capacity the store *asked for*, which is close to the truth here because the store
    /// is three flat vectors — but it is still the store describing itself. The gate this crate is
    /// held to is `tests/cell_store_allocation.rs`, which installs a counting global allocator and
    /// measures what the process actually asked the allocator for while a sparse sheet is built.
    /// `size_of_val` and its relatives are explicitly *not* an acceptable substitute: a `Vec` that
    /// reserved a million slots reports the same twenty-four bytes as an empty one, so an assertion
    /// on them passes against precisely the defect the gate exists to catch. Use this to explain a
    /// figure, never to prove one.
    #[must_use]
    pub fn reserved_bytes(&self) -> usize {
        self.rows.capacity() * core::mem::size_of::<PackedRow>()
            + self.cells.capacity() * core::mem::size_of::<PackedCell>()
            + self.cell_extras.capacity() * core::mem::size_of::<CellExtras>()
            + self.arena.edited_bytes()
    }

    // -------------------------------------------------------------------------------------------
    // Editing
    // -------------------------------------------------------------------------------------------

    /// Sets the value of the cell at `reference`, creating the cell — and the row — if neither
    /// exists.
    ///
    /// The row and cell are placed in ascending order, so building a sheet the way a file is written
    /// (top to bottom, left to right) only ever appends. Inserting into the middle moves the cell
    /// arena's tail, which is a `memmove` proportional to the cells after the insertion point; that
    /// is the price of one flat arena, and it is paid by a caller who inserts backwards.
    ///
    /// # Errors
    ///
    /// [`SmlError::SheetDataTooLarge`] if the store's byte space cannot hold the new value.
    pub fn set_cell_value(
        &mut self,
        reference: CellReference,
        value: CellValue<'_>,
    ) -> Result<(), SmlError> {
        let (row_index, index) = self.cell_slot(reference)?;
        let (payload, shape, cell_type) = self.encode_value(value)?;
        let previous_type = self.cells[index].kind;
        let cell = &mut self.cells[index];
        cell.payload = payload;
        cell.set_payload_shape(shape);
        cell.kind = match cell_type {
            Some(cell_type) => CellTypeCode::of(cell_type),
            None => CellTypeCode::ABSENT,
        };
        let new_type = cell.kind;
        self.dirty_cell(row_index, index);
        if new_type != previous_type {
            let wire = CellTypeCode::cell_type(new_type).map(|cell_type| cell_type.to_wire());
            self.rewrite_cell_attribute(index, "t", wire.map(str::as_bytes))?;
        }
        Ok(())
    }

    /// Sets `c@s`, the `cellXfs` index, on the cell at `reference`, creating a blank cell if there
    /// is none. `None` removes the attribute.
    ///
    /// # Errors
    ///
    /// As [`set_cell_value`](Self::set_cell_value).
    pub fn set_cell_style(
        &mut self,
        reference: CellReference,
        style: Option<u32>,
    ) -> Result<(), SmlError> {
        let (row_index, index) = self.cell_slot(reference)?;
        let cell = &mut self.cells[index];
        match style {
            Some(style) => {
                cell.style = style;
                cell.flags |= CellFlags::HAS_STYLE;
            }
            None => {
                cell.style = 0;
                cell.flags &= !CellFlags::HAS_STYLE;
            }
        }
        self.dirty_cell(row_index, index);
        let rendered = style.map(|style| style.to_string());
        self.rewrite_cell_attribute(index, "s", rendered.as_deref().map(str::as_bytes))
    }

    /// Removes the cell at `reference`, and reports whether there was one.
    ///
    /// The row survives an empty of its cells: a `<row>` carries height, style and hidden-ness of
    /// its own, and removing the last cell from a row is not the same statement as removing the row.
    pub fn remove_cell(&mut self, reference: CellReference) -> bool {
        let Some(row_index) = self.row_position(reference.row().saturating_add(1)) else {
            return false;
        };
        let Some(index) = self.cell_position(row_index, reference.column()) else {
            return false;
        };
        self.cells.remove(index);
        self.rows[row_index].cell_count -= 1;
        if row_index + 1 < self.rows.len() {
            for row in self.rows.iter_mut().skip(row_index + 1) {
                row.first_cell -= 1;
            }
        }
        self.dirty_row(row_index);
        true
    }

    /// Removes the row a file numbered `number`, with every cell in it, and reports whether there
    /// was one.
    pub fn remove_row(&mut self, number: u32) -> bool {
        let Some(index) = self.row_position(number) else {
            return false;
        };
        let range = self.rows[index].cell_range();
        let removed = range.len();
        self.cells.drain(range);
        self.rows.remove(index);
        if index < self.rows.len() {
            for row in self.rows.iter_mut().skip(index) {
                row.first_cell -= removed as u32;
            }
        }
        self.extent = TextSpan::NONE;
        true
    }

    /// Sets one of `CT_Row`'s attributes on the row a file numbered `number`, creating the row if
    /// there is none. `None` removes the attribute.
    ///
    /// # Why one setter rather than twelve
    ///
    /// `CT_Row` declares twelve attributes and this store decodes exactly one of them, `r`, because
    /// `r` is the key it is indexed by. The other eleven are kept as the bytes the file wrote and
    /// read back out of them on demand ([`Row`]'s typed accessors cover all eleven), so writing one
    /// is *editing a byte range in place* — which is what preserves the order, the prefixes, the
    /// quote characters and any attribute beside them that this workspace does not model. A typed
    /// setter per attribute would be eleven wrappers around this one call; what it would add is a
    /// name, and what it would cost is eleven places for the in-place rule to be forgotten.
    ///
    /// `value` is escaped as an attribute value before it is written.
    ///
    /// # Errors
    ///
    /// [`SmlError::SheetDataTooLarge`] if the store's byte space cannot hold the rewritten run.
    pub fn set_row_attribute(
        &mut self,
        number: u32,
        name: &str,
        value: Option<&str>,
    ) -> Result<(), SmlError> {
        let index = self.row_slot(number)?;
        let escaped = value.map(escape_attribute);
        let mut rewritten = Vec::new();
        attributes::set_attribute(
            self.arena.bytes(self.rows[index].attributes),
            name,
            escaped.as_deref().map(str::as_bytes),
            &mut rewritten,
        );
        let span = self.arena.store(&rewritten)?;
        self.rows[index].attributes = span;
        if name == "r" {
            // `r` is the key this store is indexed by, so writing it moves the row rather than just
            // its bytes. Re-derive both the decoded number and the ordering flag rather than letting
            // the index quietly disagree with the markup.
            match value.and_then(|value| value.parse::<u32>().ok()) {
                Some(number) => {
                    self.rows[index].number = number;
                    self.rows[index].flags |= RowFlags::HAS_NUMBER;
                }
                None => {
                    self.rows[index].number = 0;
                    self.rows[index].flags &= !RowFlags::HAS_NUMBER;
                }
            }
            self.rows_ascending = self
                .rows
                .windows(2)
                .all(|pair| pair[0].number < pair[1].number);
        }
        self.dirty_row(index);
        Ok(())
    }

    /// Sets `row@spans` on the row a file numbered `number`, or removes it with `None`.
    ///
    /// **Never derived.** There is deliberately no "compute the spans from the cells" call, here or
    /// on [`CellSpans`]: `spans` is an advisory hint that Excel writes and LibreOffice does not, so a
    /// row whose source carried none must not gain one.
    ///
    /// # Errors
    ///
    /// As [`set_row_attribute`](Self::set_row_attribute).
    pub fn set_row_spans(
        &mut self,
        number: u32,
        spans: Option<&CellSpans>,
    ) -> Result<(), SmlError> {
        let rendered = spans.map(CellSpans::to_string);
        self.set_row_attribute(number, "spans", rendered.as_deref())
    }

    /// Sets `row@ht`, the row's height in points, or removes it with `None`.
    ///
    /// # Errors
    ///
    /// As [`set_row_attribute`](Self::set_row_attribute).
    pub fn set_row_height(&mut self, number: u32, height: Option<f64>) -> Result<(), SmlError> {
        let rendered = height.map(|height| height.to_string());
        self.set_row_attribute(number, "ht", rendered.as_deref())
    }

    /// Sets `row@hidden`, or removes it with `None`.
    ///
    /// # Errors
    ///
    /// As [`set_row_attribute`](Self::set_row_attribute).
    pub fn set_row_hidden(&mut self, number: u32, hidden: Option<bool>) -> Result<(), SmlError> {
        self.set_row_attribute(number, "hidden", hidden.map(boolean_wire_value))
    }

    /// Sets `row@s`, the row's `cellXfs` index, or removes it with `None`.
    ///
    /// # Errors
    ///
    /// As [`set_row_attribute`](Self::set_row_attribute).
    pub fn set_row_style(&mut self, number: u32, style: Option<u32>) -> Result<(), SmlError> {
        let rendered = style.map(|style| style.to_string());
        self.set_row_attribute(number, "s", rendered.as_deref())
    }

    // -------------------------------------------------------------------------------------------
    // The index, and the slots the edit surface writes through
    // -------------------------------------------------------------------------------------------

    /// The position of the row a file numbered `number`.
    pub(super) fn row_position(&self, number: u32) -> Option<usize> {
        if self.rows_ascending {
            return self
                .rows
                .binary_search_by_key(&number, |row| row.number)
                .ok();
        }
        self.rows.iter().position(|row| row.number == number)
    }

    /// The position in the cell arena of `column` within the row at `row_index`.
    pub(super) fn cell_position(&self, row_index: usize, column: u16) -> Option<usize> {
        let row = self.rows.get(row_index)?;
        let range = row.cell_range();
        let slice = self.cells.get(range.clone())?;
        if row.has(RowFlags::CELLS_ASCENDING) {
            return slice
                .binary_search_by_key(&column, |cell| cell.reference.column())
                .ok()
                .map(|offset| range.start + offset);
        }
        slice
            .iter()
            .position(|cell| cell.reference.column() == column)
            .map(|offset| range.start + offset)
    }

    /// The position of the row a file numbered `number`, creating one in ascending position if there
    /// is none.
    fn row_slot(&mut self, number: u32) -> Result<usize, SmlError> {
        if let Some(index) = self.row_position(number) {
            return Ok(index);
        }
        // `partition_point` is a binary search, and it is what keeps building a sheet the way a
        // file is written — top to bottom — linear rather than quadratic. A sheet whose rows are
        // *not* ascending has no such ordering to search, so it falls back to a scan; that is one
        // more consequence of preserving a file's order instead of sorting it.
        let at = if self.rows_ascending {
            self.rows.partition_point(|row| row.number < number)
        } else {
            self.rows
                .iter()
                .position(|row| row.number > number)
                .unwrap_or(self.rows.len())
        };
        let still_ascending = self.rows_ascending
            && at
                .checked_sub(1)
                .is_none_or(|before| self.rows[before].number < number)
            && self.rows.get(at).is_none_or(|after| number < after.number);
        let first_cell = self
            .rows
            .get(at)
            .map_or(self.cells.len() as u32, |row| row.first_cell);
        let attributes = self.arena.store(format!(" r=\"{number}\"").as_bytes())?;
        self.rows.insert(
            at,
            PackedRow {
                number,
                first_cell,
                cell_count: 0,
                leading: TextSpan::NONE,
                extent: TextSpan::NONE,
                attributes,
                trailing: TextSpan::NONE,
                flags: RowFlags::HAS_NUMBER | RowFlags::CELLS_ASCENDING,
            },
        );
        self.rows_ascending = still_ascending;
        // No fixup loop: an empty row owns no cells, so no other row's slice moved.
        self.extent = TextSpan::NONE;
        Ok(at)
    }

    /// The position of the cell at `reference`, creating a blank one in ascending position — and the
    /// row it belongs to — if there is none.
    fn cell_slot(&mut self, reference: CellReference) -> Result<(usize, usize), SmlError> {
        let row_index = self.row_slot(reference.row().saturating_add(1))?;
        if let Some(index) = self.cell_position(row_index, reference.column()) {
            return Ok((row_index, index));
        }
        let range = self.rows[row_index].cell_range();
        let slice = &self.cells[range.clone()];
        let offset = if self.rows[row_index].has(RowFlags::CELLS_ASCENDING) {
            slice.partition_point(|cell| cell.reference.column() < reference.column())
        } else {
            slice
                .iter()
                .position(|cell| cell.reference.column() > reference.column())
                .unwrap_or(range.len())
        };
        let at = range.start + offset;
        self.cells.insert(
            at,
            PackedCell {
                reference,
                extent: TextSpan::NONE,
                payload: TextSpan::NONE,
                style: 0,
                extra: NO_EXTRAS,
                kind: CellTypeCode::ABSENT,
                flags: CellFlags::HAS_REFERENCE,
            },
        );
        let row = &mut self.rows[row_index];
        row.cell_count += 1;
        // The insertion was placed in ascending position, so a row that was ordered still is.
        if !row.has(RowFlags::CELLS_ASCENDING) {
            let cells = &self.cells[self.rows[row_index].cell_range()];
            let ascending = cells
                .windows(2)
                .all(|pair| pair[0].reference.column() < pair[1].reference.column());
            if ascending {
                self.rows[row_index].flags |= RowFlags::CELLS_ASCENDING;
            }
        }
        // Skipped entirely when the row is the last one, which is what makes building a sheet in
        // the order a file writes it — top to bottom, left to right — an append rather than a walk
        // of every row after it. A caller who inserts backwards pays the walk, and a `memmove` of
        // the cell arena's tail besides; that is the price of one flat arena, and it is stated on
        // `set_cell_value` rather than hidden here.
        if row_index + 1 < self.rows.len() {
            for row in self.rows.iter_mut().skip(row_index + 1) {
                row.first_cell += 1;
            }
        }
        self.dirty_row(row_index);
        Ok((row_index, at))
    }

    /// Clears the copy-on-write extent of the cell at `index`, of its row, and of the sheet.
    ///
    /// **This is the invariant, in one place.** An extent is a claim that a stretch of the part's
    /// bytes *is* this record's markup, and every mutation makes that claim false for the record it
    /// touched and for everything that contains it. `RawElement` enforces the same rule by putting
    /// its content behind a `DerefMut` that drops the range; a store made of flat arrays has no such
    /// seam, so it has this function instead and every edit goes through it.
    fn dirty_cell(&mut self, row_index: usize, index: usize) {
        self.cells[index].extent = TextSpan::NONE;
        self.dirty_row(row_index);
    }

    /// Clears the copy-on-write extent of the row at `index` and of the sheet.
    fn dirty_row(&mut self, index: usize) {
        if let Some(row) = self.rows.get_mut(index) {
            row.extent = TextSpan::NONE;
        }
        self.extent = TextSpan::NONE;
    }

    /// Rewrites one attribute of the cell at `index`, in place, when that cell replays a verbatim
    /// attribute run.
    ///
    /// A cell whose start tag this store would have written identically keeps no run at all — its
    /// tag is regenerated from `r`, `s` and `t` at write time, so there is nothing to rewrite and
    /// this returns immediately. A cell that *does* keep one carries something the regeneration
    /// would lose — an unmodelled attribute, a single-quoted value, an unusual order — so its run is
    /// edited byte-wise rather than replaced.
    fn rewrite_cell_attribute(
        &mut self,
        index: usize,
        name: &str,
        value: Option<&[u8]>,
    ) -> Result<(), SmlError> {
        let extra = self.cells[index].extra;
        if extra == NO_EXTRAS {
            return Ok(());
        }
        let run = self.cell_extras[extra as usize].attributes;
        if run.is_none() {
            return Ok(());
        }
        let mut rewritten = Vec::new();
        attributes::set_attribute(self.arena.bytes(run), name, value, &mut rewritten);
        let span = self.arena.store(&rewritten)?;
        self.cell_extras[extra as usize].attributes = span;
        Ok(())
    }

    /// Turns a [`CellValue`] into the payload span, its shape, and the `c@t` the cell should carry.
    fn encode_value(
        &mut self,
        value: CellValue<'_>,
    ) -> Result<
        (
            TextSpan,
            PayloadShape,
            Option<mjx_ooxml_types::spreadsheetml::CellType>,
        ),
        SmlError,
    > {
        use mjx_ooxml_types::spreadsheetml::CellType;
        Ok(match value {
            CellValue::Blank => (TextSpan::NONE, PayloadShape::Absent, None),
            // A number carries no `t`: `n` is the schema default, and a file that would not have
            // written the attribute must not gain it.
            CellValue::Number(number) => (
                self.arena.store(number.to_string().as_bytes())?,
                PayloadShape::ValueText,
                None,
            ),
            CellValue::NumberText(text) => (
                self.arena.store(escape_text_bytes(text).as_ref())?,
                PayloadShape::ValueText,
                None,
            ),
            CellValue::SharedString(index) => (
                self.arena.store(index.to_string().as_bytes())?,
                PayloadShape::ValueText,
                Some(CellType::SharedString),
            ),
            CellValue::Boolean(flag) => (
                self.arena.store(if flag { b"1" } else { b"0" })?,
                PayloadShape::ValueText,
                Some(CellType::Boolean),
            ),
            CellValue::Error(text) => (
                self.arena.store(escape_text_bytes(text).as_ref())?,
                PayloadShape::ValueText,
                Some(CellType::Error),
            ),
            CellValue::FormulaString(text) => (
                self.arena.store(escape_text_bytes(text).as_ref())?,
                PayloadShape::ValueText,
                Some(CellType::FormulaString),
            ),
            CellValue::InlineString(text) => {
                let mut markup = Vec::new();
                let prefix = self.prefix.clone();
                super::write::write_inline_string(prefix.as_deref(), text, &mut markup);
                (
                    self.arena.store(&markup)?,
                    PayloadShape::InlineString,
                    Some(CellType::InlineString),
                )
            }
        })
    }
}

/// The canonical `xsd:boolean` spelling this crate writes.
fn boolean_wire_value(flag: bool) -> &'static str {
    if flag {
        "true"
    } else {
        "false"
    }
}

/// `text`, escaped for XML character data, as bytes.
fn escape_text_bytes(text: &str) -> std::borrow::Cow<'_, [u8]> {
    match mjx_xml::text::escape_text(text) {
        std::borrow::Cow::Borrowed(text) => std::borrow::Cow::Borrowed(text.as_bytes()),
        std::borrow::Cow::Owned(text) => std::borrow::Cow::Owned(text.into_bytes()),
    }
}
