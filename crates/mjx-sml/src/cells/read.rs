//! Reading a `sheetData` element into the packed store.
//!
//! # Where the byte ranges come from
//!
//! `mjx-xml`'s reader records, for every element it parses, the range that element occupied in the
//! part's buffer, and `RawElement` enforces one invariant on it: **a mutation clears the range on the
//! node it touched and on every ancestor.** This module leans on that invariant hard enough to be
//! worth stating.
//!
//! A row's gap — the whitespace, comment or foreign element that sits between the previous row and
//! this one — is not read off the nodes at all. It is *derived*: `[end of the last row, start of this
//! one)` in the parent's own range. That is exact, allocates nothing, and it is only sound because a
//! `sheetData` that still has a range is a `sheetData` in which **every** parsed descendant still has
//! one — an authored or edited child would have cleared the ancestor's. So the two cases are:
//!
//! * The `sheetData` element still has a usable range → every gap is a range, nothing is copied, and
//!   an untouched worksheet's store owns no bytes at all.
//! * It does not → the nodes between rows are serialized into the store's own byte arena, which is
//!   the same answer arrived at the slow way. This is the path an authored sheet, or one whose tree
//!   was edited before the store read it, takes.
//!
//! # What is an error and what is preserved
//!
//! Exactly one thing a file can say is refused: a `c@r` that is not a cell reference. The store is
//! *keyed* on that value, and a key it cannot parse is not a key. Everything else — rows out of
//! order, duplicated row numbers, a `c@r` naming a different row than its `row@r` does, a `t` that
//! disagrees with the child element present, cells out of column order — is read as it stands,
//! written back as it stands, and reported by [`SheetData::anomalies`](super::SheetData::anomalies).
//! Nothing here sorts, deduplicates or repairs.

use std::sync::Arc;

use mjx_ooxml_core::{Interner, RawElement, RawNode};
use mjx_ooxml_types::spreadsheetml::CellType;

use crate::address::{AddressError, Anchoring, CellReference};
use crate::error::SmlError;

use super::record::{
    CellExtras, CellFlags, CellTypeCode, PackedCell, PackedRow, PayloadShape, RowFlags, NO_EXTRAS,
};
use super::store::SheetData;
use super::text::{TextArena, TextSpan};

/// Reads a `sheetData` element into a [`SheetData`].
pub(super) fn read_sheet_data(
    element: &RawElement,
    interner: &Interner,
    source: Option<&Arc<[u8]>>,
) -> Result<SheetData, SmlError> {
    let mut reader = Reader {
        arena: TextArena::new(source.cloned())?,
        interner,
        source: source.map(Arc::clone),
        qname: Vec::new(),
        scratch: Vec::new(),
    };

    let prefix = element
        .name
        .prefix
        .map(|prefix| Box::<str>::from(interner.resolve(prefix)));
    let extent = reader.extent_of(element);
    let layout = reader.layout_of(element, extent);

    let mut sheet = SheetData::authored(prefix.as_deref());
    sheet.self_closing = layout.as_ref().map_or(element.empty, |l| l.self_closing);
    sheet.extent = extent;
    sheet.attributes = match &layout {
        Some(layout) => layout.attribute_run,
        None => reader.rebuild_attribute_run(element)?,
    };

    // The cursor is the position just past whatever has been accounted for, in the parent's own
    // range. It only exists on the range-backed path.
    let mut cursor = layout.as_ref().map(|layout| layout.inner_start);
    let mut pending = Vec::new();

    for child in element.children.iter() {
        let is_row = matches!(child, RawNode::Element(child) if reader.local(child) == "row");
        if !is_row {
            if cursor.is_none() {
                reader.serialize_into(child, &mut pending);
            }
            // On the range-backed path a non-row node needs no attention at all: it lies inside the
            // gap the next row derives from the cursor.
            continue;
        }
        let RawNode::Element(row_element) = child else {
            continue;
        };
        let row_extent = reader.extent_of(row_element);
        let leading = match (&mut cursor, row_extent.is_none()) {
            (Some(at), false) if row_extent.start() >= *at => {
                let leading = TextSpan::new(*at, row_extent.start() - *at);
                *at = row_extent.end();
                leading
            }
            (Some(_), _) => {
                // A row with no usable range under a parent that has one should be unreachable —
                // `RawElement` clears an ancestor's range whenever a descendant is touched — but a
                // gap derived from a cursor that cannot advance would duplicate this row's bytes
                // into the next row's leading run. Give up the derivation rather than get it wrong.
                cursor = None;
                TextSpan::NONE
            }
            (None, _) => {
                let span = reader.arena.store(&pending)?;
                pending.clear();
                span
            }
        };
        let leading = if leading.is_none() || leading.length() > 0 {
            leading
        } else {
            // An empty gap is nothing to replay; leave the record saying so.
            TextSpan::NONE
        };
        reader.read_row(row_element, leading, sheet.rows.len(), &mut sheet)?;
    }

    sheet.trailing = match (cursor, &layout) {
        (Some(cursor), Some(layout)) if layout.inner_end >= cursor => {
            let span = TextSpan::new(cursor, layout.inner_end - cursor);
            if span.length() == 0 {
                TextSpan::NONE
            } else {
                span
            }
        }
        _ if pending.is_empty() => TextSpan::NONE,
        _ => reader.arena.store(&pending)?,
    };

    sheet.rows_ascending = sheet
        .rows
        .windows(2)
        .all(|pair| pair[0].number < pair[1].number);
    // The three arenas were grown by `push`, so each one's capacity is up to twice what it holds —
    // which on a 300,000-cell worksheet is eight megabytes of nothing. Reading a worksheet happens
    // once and editing it happens rarely, so the trade is the right way round: pay one reallocation
    // here to stop holding the doubling overshoot for as long as the workbook is open.
    sheet.rows.shrink_to_fit();
    sheet.cells.shrink_to_fit();
    sheet.cell_extras.shrink_to_fit();
    sheet.arena = reader.arena;
    Ok(sheet)
}

/// Everything the walk carries: the arena being filled, the interner names resolve through, and two
/// reusable buffers so that reading a million cells allocates nothing per cell.
struct Reader<'a> {
    arena: TextArena,
    interner: &'a Interner,
    source: Option<Arc<[u8]>>,
    /// A scratch buffer for the qualified name currently being matched.
    qname: Vec<u8>,
    /// A scratch buffer for markup being serialized on the way into the arena.
    scratch: Vec<u8>,
}

/// Where an element's start tag ends and its content begins and ends, in arena addresses.
struct Layout {
    attribute_run: TextSpan,
    inner_start: u32,
    inner_end: u32,
    self_closing: bool,
}

impl Reader<'_> {
    fn local(&self, element: &RawElement) -> &str {
        self.interner.resolve(element.name.local)
    }

    /// The element's own byte range, as an arena span, or [`TextSpan::NONE`].
    fn extent_of(&self, element: &RawElement) -> TextSpan {
        match element.source_span() {
            Some(range) => self.arena.span_in_source(range.start, range.end),
            None => TextSpan::NONE,
        }
    }

    /// Decomposes `extent` into the element's attribute run and its content, or `None` when the
    /// bytes do not describe this element after all.
    ///
    /// The range is **untrusted on the way out as well as in**, exactly as `mjx-xml`'s writer treats
    /// it: it must open with `<` and this element's qualified name, and close the way the element
    /// says it closes. Anything else falls back to building from the model — a reflow, never wrong
    /// bytes.
    fn layout_of(&mut self, element: &RawElement, extent: TextSpan) -> Option<Layout> {
        if extent.is_none() {
            return None;
        }
        let bytes = self.arena.bytes(extent);
        if bytes.is_empty() {
            return None;
        }
        self.qname.clear();
        if let Some(prefix) = element.name.prefix {
            self.qname
                .extend_from_slice(self.interner.resolve(prefix).as_bytes());
            self.qname.push(b':');
        }
        self.qname
            .extend_from_slice(self.interner.resolve(element.name.local).as_bytes());
        let parts = decompose(bytes, &self.qname)?;
        let base = extent.start();
        Some(Layout {
            attribute_run: TextSpan::new(
                base + parts.attribute_run.start as u32,
                (parts.attribute_run.end - parts.attribute_run.start) as u32,
            ),
            inner_start: base + parts.inner.start as u32,
            inner_end: base + parts.inner.end as u32,
            self_closing: parts.self_closing,
        })
    }

    /// Serializes one node into `out`, for the path where there are no byte ranges to point at.
    fn serialize_into(&mut self, node: &RawNode, out: &mut Vec<u8>) {
        mjx_xml::fidelity::serialize_node(node, self.interner, self.source.as_deref(), out);
    }

    /// The attribute run for an element with no usable range: rebuilt from the model, in the one
    /// spelling this crate writes.
    fn rebuild_attribute_run(&mut self, element: &RawElement) -> Result<TextSpan, SmlError> {
        self.scratch.clear();
        let mut scratch = core::mem::take(&mut self.scratch);
        for attribute in element.attributes.iter() {
            scratch.push(b' ');
            if let Some(prefix) = attribute.name.prefix {
                scratch.extend_from_slice(self.interner.resolve(prefix).as_bytes());
                scratch.push(b':');
            }
            scratch.extend_from_slice(self.interner.resolve(attribute.name.local).as_bytes());
            scratch.push(b'=');
            scratch.push(attribute.quote.byte());
            scratch.extend_from_slice(&attribute.value);
            scratch.push(attribute.quote.byte());
        }
        let span = self.arena.store(&scratch)?;
        self.scratch = scratch;
        Ok(span)
    }

    /// The raw, still-escaped value of `name` on `element`.
    fn raw_attribute<'e>(&self, element: &'e RawElement, name: &str) -> Option<&'e [u8]> {
        element
            .attributes
            .iter()
            .find(|attribute| {
                attribute.name.prefix.is_none()
                    && self.interner.resolve(attribute.name.local) == name
            })
            .map(|attribute| &attribute.value[..])
    }

    /// The value of `name` with entity references resolved, as text.
    fn attribute_text(&self, element: &RawElement, name: &str) -> Option<String> {
        let raw = core::str::from_utf8(self.raw_attribute(element, name)?).ok()?;
        Some(mjx_xml::text::unescape_text(raw).ok()?.into_owned())
    }

    fn read_row(
        &mut self,
        element: &RawElement,
        leading: TextSpan,
        row_position: usize,
        sheet: &mut SheetData,
    ) -> Result<(), SmlError> {
        let extent = self.extent_of(element);
        let layout = self.layout_of(element, extent);
        let attributes = match &layout {
            Some(layout) => layout.attribute_run,
            None => self.rebuild_attribute_run(element)?,
        };
        let number = self
            .attribute_text(element, "r")
            .and_then(|text| text.parse::<u32>().ok());
        let mut flags = 0u8;
        if number.is_some() {
            flags |= RowFlags::HAS_NUMBER;
        }
        if layout.as_ref().map_or(element.empty, |l| l.self_closing) {
            flags |= RowFlags::SELF_CLOSING;
        }

        let first_cell = sheet.cells.len() as u32;
        let row_number = number.unwrap_or_else(|| (row_position as u32).saturating_add(1));

        let mut cursor = layout.as_ref().map(|layout| layout.inner_start);
        let mut pending = Vec::new();
        let mut previous_column: Option<u16> = None;
        for child in element.children.iter() {
            let is_cell = matches!(child, RawNode::Element(child) if self.local(child) == "c");
            if !is_cell {
                if cursor.is_none() {
                    self.serialize_into(child, &mut pending);
                }
                continue;
            }
            let RawNode::Element(cell_element) = child else {
                continue;
            };
            let cell_extent = self.extent_of(cell_element);
            let cell_leading = match (&mut cursor, cell_extent.is_none()) {
                (Some(at), false) if cell_extent.start() >= *at => {
                    let leading = TextSpan::new(*at, cell_extent.start() - *at);
                    *at = cell_extent.end();
                    if leading.length() == 0 {
                        TextSpan::NONE
                    } else {
                        leading
                    }
                }
                (Some(_), _) => {
                    cursor = None;
                    TextSpan::NONE
                }
                (None, _) => {
                    if pending.is_empty() {
                        TextSpan::NONE
                    } else {
                        let span = self.arena.store(&pending)?;
                        pending.clear();
                        span
                    }
                }
            };
            let cell = self.read_cell(
                cell_element,
                cell_extent,
                cell_leading,
                row_number,
                previous_column,
                sheet,
            )?;
            previous_column = Some(cell.reference.column());
            sheet.cells.push(cell);
        }

        let trailing = match (cursor, &layout) {
            (Some(at), Some(layout)) if layout.inner_end > at => {
                TextSpan::new(at, layout.inner_end - at)
            }
            (Some(_), Some(_)) => TextSpan::NONE,
            _ if pending.is_empty() => TextSpan::NONE,
            _ => self.arena.store(&pending)?,
        };

        let cell_count = sheet.cells.len() as u32 - first_cell;
        let ascending = sheet.cells[first_cell as usize..]
            .windows(2)
            .all(|pair| pair[0].reference.column() < pair[1].reference.column());
        if ascending {
            flags |= RowFlags::CELLS_ASCENDING;
        }
        sheet.rows.push(PackedRow {
            number: number.unwrap_or(0),
            first_cell,
            cell_count,
            leading,
            extent,
            attributes,
            trailing,
            flags,
        });
        Ok(())
    }

    fn read_cell(
        &mut self,
        element: &RawElement,
        extent: TextSpan,
        leading: TextSpan,
        row_number: u32,
        previous_column: Option<u16>,
        sheet: &mut SheetData,
    ) -> Result<PackedCell, SmlError> {
        let layout = self.layout_of(element, extent);
        let run = match &layout {
            Some(layout) => layout.attribute_run,
            None => self.rebuild_attribute_run(element)?,
        };

        let written_reference = self.attribute_text(element, "r");
        let reference = match &written_reference {
            Some(text) => CellReference::parse(text)?,
            None => {
                let column = match previous_column {
                    Some(previous) => previous
                        .checked_add(1)
                        .ok_or(AddressError::ColumnOutOfGrid)?,
                    None => 0,
                };
                CellReference::new(
                    column,
                    row_number.saturating_sub(1),
                    Anchoring::Relative,
                    Anchoring::Relative,
                )?
            }
        };

        let style = self
            .attribute_text(element, "s")
            .and_then(|text| text.parse::<u32>().ok());
        let written_type = self
            .attribute_text(element, "t")
            .and_then(|text| CellType::from_wire(&text));

        let mut flags = 0u8;
        if written_reference.is_some() {
            flags |= CellFlags::HAS_REFERENCE;
        }
        if style.is_some() {
            flags |= CellFlags::HAS_STYLE;
        }
        if layout.as_ref().map_or(element.empty, |l| l.self_closing) {
            flags |= CellFlags::SELF_CLOSING;
        }

        let mut cell = PackedCell {
            reference,
            extent,
            payload: TextSpan::NONE,
            style: style.unwrap_or(0),
            extra: NO_EXTRAS,
            kind: written_type.map_or(CellTypeCode::ABSENT, CellTypeCode::of),
            flags,
        };

        let mut extras = CellExtras::default();
        // The attribute run is kept only when regenerating it would not reproduce it. The test is
        // the regeneration itself, byte for byte — not a guess about which shapes are canonical.
        if !self.run_is_canonical(run, &cell) {
            extras.attributes = run;
        }
        extras.leading = leading;

        self.read_cell_content(element, &layout, &mut cell, &mut extras)?;

        if !extras.is_empty() {
            cell.extra = sheet.cell_extras.len() as u32;
            sheet.cell_extras.push(extras);
        }
        Ok(cell)
    }

    /// Whether the bytes at `run` are exactly what [`super::write`] would write for this cell.
    fn run_is_canonical(&mut self, run: TextSpan, cell: &PackedCell) -> bool {
        self.scratch.clear();
        let mut scratch = core::mem::take(&mut self.scratch);
        super::write::write_canonical_attribute_run(cell, &mut scratch);
        let same = self.arena.bytes(run) == scratch.as_slice();
        self.scratch = scratch;
        same
    }

    /// Splits a cell's content into "before the value", "the value" and "after the value".
    fn read_cell_content(
        &mut self,
        element: &RawElement,
        layout: &Option<Layout>,
        cell: &mut PackedCell,
        extras: &mut CellExtras,
    ) -> Result<(), SmlError> {
        // Find the value element and the formula, by local name and in document order.
        let mut payload_element = None;
        let mut payload_shape = PayloadShape::Absent;
        let mut formula_element = None;
        for child in element.children.iter() {
            let RawNode::Element(child) = child else {
                continue;
            };
            match self.local(child) {
                "v" if payload_element.is_none() => {
                    payload_element = Some(child);
                    payload_shape = PayloadShape::ValueText;
                }
                "is" if payload_element.is_none() => {
                    payload_element = Some(child);
                    payload_shape = PayloadShape::InlineString;
                }
                "f" if formula_element.is_none() => formula_element = Some(child),
                _ => {}
            }
        }

        // A `<v>` whose content is anything but text — a CDATA section, a comment — is not something
        // the value accessors can answer for, so the whole cell content is kept opaque instead. It
        // still round-trips; it simply has no decoded value.
        if payload_shape == PayloadShape::ValueText
            && payload_element.is_some_and(|element| {
                element
                    .children
                    .iter()
                    .any(|child| !matches!(child, RawNode::Text(_)))
            })
        {
            payload_element = None;
            payload_shape = PayloadShape::Absent;
        }

        let Some(layout) = layout else {
            return self.read_cell_content_from_model(
                element,
                payload_element,
                payload_shape,
                formula_element,
                cell,
                extras,
            );
        };

        if let Some(formula) = formula_element {
            extras.formula = self.extent_of(formula);
        }

        let Some(payload_element) = payload_element else {
            extras.before_payload = span_between(layout.inner_start, layout.inner_end);
            return Ok(());
        };
        let payload_extent = self.extent_of(payload_element);
        if payload_extent.is_none() {
            extras.before_payload = span_between(layout.inner_start, layout.inner_end);
            return Ok(());
        }
        extras.before_payload = span_between(layout.inner_start, payload_extent.start());
        extras.after_payload = span_between(payload_extent.end(), layout.inner_end);

        cell.payload = match payload_shape {
            PayloadShape::InlineString => payload_extent,
            PayloadShape::ValueText => {
                let Some(value_layout) = self.layout_of(payload_element, payload_extent) else {
                    // The `<v>` range does not describe a `<v>`; keep the whole content opaque.
                    extras.before_payload = span_between(layout.inner_start, layout.inner_end);
                    extras.after_payload = TextSpan::NONE;
                    return Ok(());
                };
                span_present_between(value_layout.inner_start, value_layout.inner_end)
            }
            PayloadShape::Absent => TextSpan::NONE,
        };
        cell.set_payload_shape(payload_shape);
        Ok(())
    }

    /// The same split, for a cell with no usable byte range: every run is serialized into the arena.
    fn read_cell_content_from_model(
        &mut self,
        element: &RawElement,
        payload_element: Option<&RawElement>,
        payload_shape: PayloadShape,
        formula_element: Option<&RawElement>,
        cell: &mut PackedCell,
        extras: &mut CellExtras,
    ) -> Result<(), SmlError> {
        if let Some(formula) = formula_element {
            let mut bytes = Vec::new();
            mjx_xml::fidelity::serialize_element(
                formula,
                self.interner,
                self.source.as_deref(),
                &mut bytes,
            );
            extras.formula = self.arena.store(&bytes)?;
        }

        let mut before = Vec::new();
        let mut after = Vec::new();
        let mut seen_payload = false;
        for child in element.children.iter() {
            let is_payload = matches!(child, RawNode::Element(child)
                if payload_element.is_some_and(|payload| core::ptr::eq(payload, child)));
            if is_payload {
                seen_payload = true;
                continue;
            }
            let target = if seen_payload {
                &mut after
            } else {
                &mut before
            };
            self.serialize_into(child, target);
        }
        extras.before_payload = if before.is_empty() {
            TextSpan::NONE
        } else {
            self.arena.store(&before)?
        };
        extras.after_payload = if after.is_empty() {
            TextSpan::NONE
        } else {
            self.arena.store(&after)?
        };

        if let Some(payload_element) = payload_element {
            let mut bytes = Vec::new();
            match payload_shape {
                PayloadShape::InlineString => {
                    mjx_xml::fidelity::serialize_element(
                        payload_element,
                        self.interner,
                        self.source.as_deref(),
                        &mut bytes,
                    );
                }
                _ => {
                    for child in payload_element.children.iter() {
                        self.serialize_into(child, &mut bytes);
                    }
                }
            }
            cell.payload = self.arena.store(&bytes)?;
            cell.set_payload_shape(payload_shape);
        }
        Ok(())
    }
}

/// A span over `start..end`, or [`TextSpan::NONE`] when that is empty or inverted.
fn span_between(start: u32, end: u32) -> TextSpan {
    if end > start {
        TextSpan::new(start, end - start)
    } else {
        TextSpan::NONE
    }
}

/// A span over `start..end` that stays **present** when empty — `<v></v>` is a value, and an absent
/// one is not the same thing.
fn span_present_between(start: u32, end: u32) -> TextSpan {
    if end >= start {
        TextSpan::new(start, end - start)
    } else {
        TextSpan::NONE
    }
}

/// Where an element's start tag ends and its content begins and ends, as offsets into its own bytes.
struct Decomposed {
    attribute_run: core::ops::Range<usize>,
    inner: core::ops::Range<usize>,
    self_closing: bool,
}

/// Splits `bytes` — which must be exactly one element — into its attribute run and its content.
///
/// Returns `None` unless the bytes open with `<` and `qname` followed by a delimiter, and close the
/// way an element closes. That is the same check `mjx-xml`'s writer makes before trusting a range,
/// and for the same reason: the range is a claim about somebody else's buffer, and a claim that does
/// not check out must degrade to a rebuild rather than to wrong bytes.
fn decompose(bytes: &[u8], qname: &[u8]) -> Option<Decomposed> {
    if bytes.first() != Some(&b'<') || !bytes.get(1..)?.starts_with(qname) {
        return None;
    }
    let run_start = 1 + qname.len();
    match bytes.get(run_start) {
        Some(b'>' | b'/') => {}
        Some(byte) if byte.is_ascii_whitespace() => {}
        _ => return None,
    }

    // Scan to the `>` that closes the start tag, stepping over quoted attribute values — `>` is
    // perfectly legal inside one, so the first `>` is not necessarily the tag's.
    let mut at = run_start;
    let mut quote = 0u8;
    let tag_end = loop {
        let byte = *bytes.get(at)?;
        if quote != 0 {
            if byte == quote {
                quote = 0;
            }
        } else if byte == b'"' || byte == b'\'' {
            quote = byte;
        } else if byte == b'>' {
            break at;
        }
        at += 1;
    };

    let self_closing = tag_end > run_start && bytes[tag_end - 1] == b'/';
    let run_end = if self_closing { tag_end - 1 } else { tag_end };
    let attribute_run = run_start..run_end;
    if self_closing {
        if tag_end + 1 != bytes.len() {
            return None;
        }
        return Some(Decomposed {
            attribute_run,
            inner: bytes.len()..bytes.len(),
            self_closing: true,
        });
    }

    // `</name >` is legal, so trim the whitespace an end tag may carry before its `>`.
    let rest = bytes.strip_suffix(b">")?;
    let mut end = rest.len();
    while end > 0 && rest[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    let rest = rest.get(..end)?.strip_suffix(qname)?.strip_suffix(b"</")?;
    let inner_end = rest.len();
    let inner_start = tag_end + 1;
    if inner_end < inner_start {
        return None;
    }
    Some(Decomposed {
        attribute_run,
        inner: inner_start..inner_end,
        self_closing: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parts(markup: &str, qname: &str) -> Option<(String, String, bool)> {
        let parsed = decompose(markup.as_bytes(), qname.as_bytes())?;
        Some((
            markup[parsed.attribute_run].to_owned(),
            markup[parsed.inner].to_owned(),
            parsed.self_closing,
        ))
    }

    #[test]
    fn splits_a_start_tag_from_its_content() {
        assert_eq!(
            parts(r#"<c r="A1"><v>12</v></c>"#, "c"),
            Some((" r=\"A1\"".to_owned(), "<v>12</v>".to_owned(), false))
        );
        assert_eq!(
            parts(r#"<x:c r="A1"/>"#, "x:c"),
            Some((" r=\"A1\"".to_owned(), String::new(), true))
        );
        assert_eq!(
            parts("<c></c>", "c"),
            Some((String::new(), String::new(), false))
        );
    }

    #[test]
    fn an_angle_bracket_inside_an_attribute_value_does_not_end_the_tag() {
        // Legal XML: `>` needs no escaping in an attribute value, and a naive scan for the first
        // `>` would cut the tag in half and read the rest of it as content.
        assert_eq!(
            parts(r#"<c note="a>b" r="A1">x</c>"#, "c"),
            Some((r#" note="a>b" r="A1""#.to_owned(), "x".to_owned(), false))
        );
    }

    #[test]
    fn whitespace_an_end_tag_is_allowed_to_carry_is_not_content() {
        assert_eq!(
            parts("<v>12</v >", "v"),
            Some((String::new(), "12".to_owned(), false))
        );
        assert_eq!(
            parts("<v >12</v>", "v"),
            Some((" ".to_owned(), "12".to_owned(), false))
        );
    }

    #[test]
    fn bytes_that_do_not_describe_this_element_are_refused() {
        // Each of these would otherwise be a way to write somebody else's markup into a cell.
        assert_eq!(parts("<cc r=\"A1\"/>", "c"), None, "a longer name");
        assert_eq!(parts("<b/>", "c"), None, "a different name");
        assert_eq!(parts("c/>", "c"), None, "no opening angle bracket");
        assert_eq!(parts("<c><v>1</v></b>", "c"), None, "a mismatched end tag");
        assert_eq!(parts("<c/><c/>", "c"), None, "more than one element");
        assert_eq!(parts("<c>", "c"), None, "no end tag at all");
    }
}
