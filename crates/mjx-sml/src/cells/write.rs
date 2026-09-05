//! Writing the store back out — the copy-on-write half of the design.
//!
//! # Outward-in, three levels deep
//!
//! Every level asks the same question first: *does this record still carry the byte range it was
//! read from?* A range that is still present means nothing under it has been touched, so the bytes
//! are copied and the walk stops there.
//!
//! * The **sheet** — an untouched worksheet is one `memcpy`, and the walk below never runs.
//! * A **row** — after an edit somewhere else in the sheet, every other row is copied whole. This is
//!   the clause the edit-isolation gate is written against.
//! * A **cell** — inside the one row that *was* rewritten, every cell but the edited one is still
//!   copied from its own range, so the row comes back differing only where it was changed.
//!
//! That is `RawElement::source_span`'s rule at three granularities, in a store that holds no tree.
//! What the tree gets from `DerefMut` — a mutation that cannot happen without dropping the range —
//! this store gets from every edit going through
//! [`SheetData::dirty_cell`](super::SheetData), which clears the range on the record, on its row and
//! on the sheet.
//!
//! # What a rebuild costs, and what it does not
//!
//! When a record *is* rebuilt, almost nothing is lost, because almost nothing was decoded: a row's
//! start tag is its original attribute bytes, a cell's is either its original bytes or a
//! regeneration the reader **proved equal to them**, and every child the store does not model is a
//! byte range replayed in place. What a rebuild does not reproduce is the whitespace an *end* tag
//! was allowed to carry (`</v >` comes back `</v>`), and the qualified name of a row or cell whose
//! prefix differed from the `sheetData` element's. Both are reflows on a record somebody edited,
//! which is the same contract `mjx-xml`'s writer states for a rewritten element — and neither can
//! reach a record nobody touched, because that record is copied rather than rebuilt.

use super::record::{CellFlags, PayloadShape, RowFlags};
use super::store::SheetData;

/// Appends the whole `<sheetData>` element.
pub(super) fn write_sheet_data(sheet: &SheetData, out: &mut Vec<u8>) {
    if let Some(verbatim) = sheet.verbatim_sheet() {
        out.extend_from_slice(verbatim);
        return;
    }
    out.push(b'<');
    write_qualified_name(sheet, "sheetData", out);
    out.extend_from_slice(sheet.attribute_run());
    let trailing = sheet.trailing_bytes();
    if sheet.row_count() == 0 && trailing.is_empty() && sheet.was_self_closing() {
        out.extend_from_slice(b"/>");
        return;
    }
    out.push(b'>');
    for index in 0..sheet.row_count() {
        out.extend_from_slice(sheet.row_leading_bytes(index));
        write_row(sheet, index, out);
    }
    out.extend_from_slice(sheet.trailing_bytes());
    out.extend_from_slice(b"</");
    write_qualified_name(sheet, "sheetData", out);
    out.push(b'>');
}

/// Appends one row element — **not** the bytes that precede it, so that a caller comparing a row
/// against the one it was read from compares the row.
pub(super) fn write_row(sheet: &SheetData, index: usize, out: &mut Vec<u8>) {
    let row = sheet.row_record(index);
    if let Some(verbatim) = sheet.verbatim_row(index) {
        out.extend_from_slice(verbatim);
        return;
    }
    out.push(b'<');
    write_qualified_name(sheet, "row", out);
    out.extend_from_slice(sheet.arena_bytes(row.attributes));
    let trailing = sheet.arena_bytes(row.trailing);
    if row.cell_count == 0 && trailing.is_empty() && row.has(RowFlags::SELF_CLOSING) {
        out.extend_from_slice(b"/>");
        return;
    }
    out.push(b'>');
    for cell in row.cell_range() {
        out.extend_from_slice(sheet.cell_leading_bytes(cell));
        write_cell(sheet, cell, out);
    }
    out.extend_from_slice(trailing);
    out.extend_from_slice(b"</");
    write_qualified_name(sheet, "row", out);
    out.push(b'>');
}

/// Appends one cell element.
pub(super) fn write_cell(sheet: &SheetData, index: usize, out: &mut Vec<u8>) {
    let cell = sheet.cell_record(index);
    if let Some(verbatim) = sheet.verbatim_cell(index) {
        out.extend_from_slice(verbatim);
        return;
    }
    out.push(b'<');
    write_qualified_name(sheet, "c", out);
    match sheet.cell_attribute_run(index) {
        Some(run) => out.extend_from_slice(run),
        None => write_canonical_attribute_run(cell, out),
    }

    let before = sheet.cell_before_payload_bytes(index);
    let after = sheet.cell_after_payload_bytes(index);
    let shape = cell.payload_shape();
    if shape == PayloadShape::Absent
        && before.is_empty()
        && after.is_empty()
        && cell.has(CellFlags::SELF_CLOSING)
    {
        out.extend_from_slice(b"/>");
        return;
    }
    out.push(b'>');
    out.extend_from_slice(before);
    match shape {
        PayloadShape::Absent => {}
        PayloadShape::ValueText => {
            out.push(b'<');
            write_qualified_name(sheet, "v", out);
            out.push(b'>');
            out.extend_from_slice(sheet.arena_bytes(cell.payload));
            out.extend_from_slice(b"</");
            write_qualified_name(sheet, "v", out);
            out.push(b'>');
        }
        // An inline string is kept as its whole `<is>…</is>`, so it is replayed rather than rebuilt.
        PayloadShape::InlineString => out.extend_from_slice(sheet.arena_bytes(cell.payload)),
    }
    out.extend_from_slice(after);
    out.extend_from_slice(b"</");
    write_qualified_name(sheet, "c", out);
    out.push(b'>');
}

/// Writes the attribute run for a cell whose start tag this store regenerates: `r`, then `s`, then
/// `t`, each only if the file wrote it, each double-quoted and separated by one space.
///
/// **This function is also the reader's canonicality test.** The reader writes this run and compares
/// it byte for byte with the one the file has; equal means the cell needs no verbatim run at all,
/// and different means it keeps one. So the two can never drift: a change here changes which cells
/// are considered canonical, in the same pass that decides it.
pub(super) fn write_canonical_attribute_run(cell: &super::record::PackedCell, out: &mut Vec<u8>) {
    if cell.has(CellFlags::HAS_REFERENCE) {
        out.extend_from_slice(b" r=\"");
        out.extend_from_slice(cell.reference.text().as_str().as_bytes());
        out.push(b'"');
    }
    if cell.has(CellFlags::HAS_STYLE) {
        out.extend_from_slice(b" s=\"");
        out.extend_from_slice(cell.style.to_string().as_bytes());
        out.push(b'"');
    }
    if let Some(cell_type) = cell.written_cell_type() {
        out.extend_from_slice(b" t=\"");
        out.extend_from_slice(cell_type.to_wire().as_bytes());
        out.push(b'"');
    }
}

/// Writes `<is><t>text</t></is>` for a caller setting an inline string, with `text` escaped as
/// character data.
pub(super) fn write_inline_string(prefix: Option<&str>, text: &str, out: &mut Vec<u8>) {
    let escaped = mjx_xml::text::escape_text(text);
    for local in ["is", "t"] {
        out.push(b'<');
        write_prefixed_name(prefix, local, out);
        out.push(b'>');
    }
    out.extend_from_slice(escaped.as_bytes());
    for local in ["t", "is"] {
        out.extend_from_slice(b"</");
        write_prefixed_name(prefix, local, out);
        out.push(b'>');
    }
}

fn write_qualified_name(sheet: &SheetData, local: &str, out: &mut Vec<u8>) {
    write_prefixed_name(sheet.element_prefix(), local, out);
}

fn write_prefixed_name(prefix: Option<&str>, local: &str, out: &mut Vec<u8>) {
    if let Some(prefix) = prefix {
        out.extend_from_slice(prefix.as_bytes());
        out.push(b':');
    }
    out.extend_from_slice(local.as_bytes());
}
