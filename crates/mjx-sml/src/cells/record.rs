//! The packed records: what a row and a cell cost, and why they cost that.
//!
//! # The decision, in one table
//!
//! `docs/BENCHMARKS.md` measures a 300,000-cell worksheet at **≈ 913 bytes of peak resident set per
//! cell** once its part is a [`RawElement`](mjx_ooxml_core::RawElement) tree, and says where the cost
//! is: not the 72-byte element struct but the two small heap allocations each element carries. Every
//! candidate representation was judged against that figure.
//!
//! | Representation | Bytes per cell | Why not |
//! |---|---|---|
//! | `RawElement` tree, as PowerPoint and Word hold their parts | **913** (measured) | The baseline this child exists to beat. A million-cell workbook costs about a gigabyte. |
//! | An owned typed tree — `Vec<Row>` of `Vec<Cell>`, each cell owning `String`s | ≥ 200, unmeasured | Same shape as the baseline with different type names: two allocations per cell for its value and its unknown bucket, plus one per row. It is the answer this ticket names as the one not to arrive at by default. |
//! | `BTreeMap<CellReference, Cell>` | ~60 + node overhead | Sparse and ordered, but it dissolves the **row**, and the row is the unit the file is written in and the unit an untouched part is re-emitted in. A store that cannot say "these bytes are row 7" cannot re-emit row 7 verbatim. |
//! | A dense grid over the addressable range | 17 GB at one byte a slot | 1,048,576 × 16,384 slots for a sheet that may hold one cell. This is what the allocation gate is written against, and what the mutation in the pull request turns on to prove the gate can fail. |
//! | **Row-major flat arenas — what this is** | **[`PackedCell`] = 36, [`PackedRow`] = 48** | — |
//!
//! Two further shapes were costed and rejected on the record's own terms rather than on the table's:
//!
//! * **32 bytes**, by storing a cell's column and anchoring (4 bytes) instead of its whole
//!   [`CellReference`] (8) and recovering the row from the row record. It saves 11%, and it pays for
//!   it by making *"`c@r` disagrees with `row@r`"* — one of the untrusted-input cases this store is
//!   required to preserve rather than repair — a special case routed through a side table, instead of
//!   the same field every other cell uses.
//! * **32 bytes**, by moving [`PackedCell::extra`] into a side table keyed by cell position. The
//!   ticket suggests exactly this, and it is right that the common cell should pay nothing — but a
//!   key that is a *position* is invalidated by every insertion into the middle of the arena, and
//!   4 bytes is not worth an index that has to be rebased on write. The side table is still here;
//!   what the cell holds is a stable index into it rather than a position that shifts.
//!
//! # What the 36 bytes buy
//!
//! Two of them are [`TextSpan`]s, and the second is why an untouched cell inside a *rewritten* row
//! comes back byte-identical: [`PackedCell::extent`] is the cell's own `<c …>…</c>` range in the
//! part's bytes, so the write path copies it rather than rebuilding it. Copy-on-write at subtree
//! granularity is what `RawElement::source_span` does for a tree; this is the same rule for a store
//! that is not one.

use mjx_ooxml_types::spreadsheetml::CellType;

use crate::address::CellReference;

use crate::arena::TextSpan;

/// Where a cell's value lives, and in what shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadShape {
    /// The cell carries neither `<v>` nor `<is>` — a blank cell, which is still a cell because it
    /// may carry a style.
    Absent,
    /// The cell record's payload span is the raw, still-escaped text **inside** a `<v>` element.
    ValueText,
    /// The cell record's payload span is a whole `<is>…</is>` element, kept verbatim.
    ///
    /// `CT_Rst` is a rich-text structure — `t`, `r*`, `rPh*`, `phoneticPr?` — and modelling it is
    /// MJXOFF-97's (D05), which owns the shared-string table this shares its type with. Until then
    /// the store's contract for it is preservation, which is exact.
    InlineString,
}

/// One cell, packed. **36 bytes**, and it is multiplied by a million — see the [module docs](self)
/// for what each field buys and what the alternatives cost.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PackedCell {
    /// `c@r`, or the position-derived address when the file left it out.
    pub(crate) reference: CellReference,
    /// The whole `<c …>…</c>` in the part's bytes, or [`TextSpan::NONE`] once the cell is edited or
    /// if it was authored. **This is the cell's copy-on-write state**: present means "these bytes
    /// still say exactly this", and every mutation clears it.
    pub(crate) extent: TextSpan,
    /// The value, in the shape [`Self::payload_shape`] names.
    pub(crate) payload: TextSpan,
    /// `c@s`, the `cellXfs` index. Zero when the attribute is absent, which is also its schema
    /// default; [`CellFlags::HAS_STYLE`] is what distinguishes "wrote `s="0"`" from "wrote nothing".
    pub(crate) style: u32,
    /// Index into the sheet's cell-extras table, or [`NO_EXTRAS`]. Stable across insertions.
    pub(crate) extra: u32,
    /// `c@t` as [`CellTypeCode`].
    pub(crate) kind: u8,
    /// [`CellFlags`].
    pub(crate) flags: u8,
}

/// The value of [`PackedCell::extra`] meaning "this cell has no side-table entry". A real index can
/// never reach it: the arena would have to hold four billion cells.
pub(crate) const NO_EXTRAS: u32 = u32::MAX;

/// `c@t`, stored as one byte with a distinct code for "the attribute was absent".
///
/// The schema default is `n`, so an absent `t` and `t="n"` *mean* the same thing — and must not be
/// written the same way, because a file that said nothing must come back saying nothing. That is the
/// whole reason this is a code rather than a [`CellType`].
pub(crate) struct CellTypeCode;

impl CellTypeCode {
    /// `c@t` was not written. The cell's type is [`CellType::Number`] by schema default.
    pub(crate) const ABSENT: u8 = 0;

    /// The code for a written `c@t`.
    pub(crate) fn of(cell_type: CellType) -> u8 {
        1 + match cell_type {
            CellType::Boolean => 0,
            CellType::Number => 1,
            CellType::Error => 2,
            CellType::SharedString => 3,
            CellType::FormulaString => 4,
            CellType::InlineString => 5,
        }
    }

    /// The type a code names, or `None` when the attribute was absent.
    pub(crate) fn cell_type(code: u8) -> Option<CellType> {
        Some(match code {
            1 => CellType::Boolean,
            2 => CellType::Number,
            3 => CellType::Error,
            4 => CellType::SharedString,
            5 => CellType::FormulaString,
            6 => CellType::InlineString,
            _ => return None,
        })
    }
}

/// The bit flags [`PackedCell::flags`] carries.
pub(crate) struct CellFlags;

impl CellFlags {
    /// `c@r` was written. A cell without one takes its address from its position, and must not gain
    /// the attribute on the way out.
    pub(crate) const HAS_REFERENCE: u8 = 1 << 0;
    /// `c@s` was written, even if it was written as the schema default `0`.
    pub(crate) const HAS_STYLE: u8 = 1 << 1;
    /// The cell was written `<c …/>` rather than `<c …></c>`.
    pub(crate) const SELF_CLOSING: u8 = 1 << 2;
    /// Bits 3–4 hold the [`PayloadShape`].
    pub(crate) const PAYLOAD_SHIFT: u8 = 3;
    /// The mask for those two bits.
    pub(crate) const PAYLOAD_MASK: u8 = 0b11 << Self::PAYLOAD_SHIFT;
}

impl PackedCell {
    /// The shape of [`Self::payload`].
    pub(crate) fn payload_shape(&self) -> PayloadShape {
        match (self.flags & CellFlags::PAYLOAD_MASK) >> CellFlags::PAYLOAD_SHIFT {
            1 => PayloadShape::ValueText,
            2 => PayloadShape::InlineString,
            _ => PayloadShape::Absent,
        }
    }

    /// Records the shape of [`Self::payload`].
    pub(crate) fn set_payload_shape(&mut self, shape: PayloadShape) {
        let code = match shape {
            PayloadShape::Absent => 0,
            PayloadShape::ValueText => 1,
            PayloadShape::InlineString => 2,
        };
        self.flags = (self.flags & !CellFlags::PAYLOAD_MASK) | (code << CellFlags::PAYLOAD_SHIFT);
    }

    /// Whether `flag` is set.
    pub(crate) fn has(&self, flag: u8) -> bool {
        self.flags & flag != 0
    }

    /// `c@t`, or `None` when the attribute was absent (meaning [`CellType::Number`]).
    pub(crate) fn written_cell_type(&self) -> Option<CellType> {
        CellTypeCode::cell_type(self.kind)
    }
}

/// A cell's rare data — allocated only for a cell that has some, so the common cell pays nothing but
/// the four bytes of [`PackedCell::extra`].
///
/// Every field is a byte range, and between them they carry everything the packed fields do not:
/// a `<f>` formula, an `extLst` nobody models, foreign markup between two cells, and the whitespace a
/// pretty-printer put there. **This is the unknown bucket for a packed store.** `CLAUDE.md` writes
/// the rule as `extra: Vec<RawNode>`; a `Vec<RawNode>` per cell is the per-cell allocation this
/// store exists to not have, so the same rule is kept in the representation the store can afford —
/// and raw bytes are the *stricter* of the two, since they preserve the whitespace inside a start tag
/// that a decomposed node cannot record.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct CellExtras {
    /// The start tag's attribute run — everything between `<c` and the `>` that closes it — kept
    /// only when regenerating it from `r`, `s` and `t` would **not** reproduce it byte for byte.
    ///
    /// The test is not a heuristic: the reader writes out the attributes it would write, compares
    /// them with the bytes the file has, and keeps the file's bytes whenever the two differ. So a
    /// start tag with an unmodelled attribute, a single-quoted value, two spaces, or `t` written
    /// before `r` is replayed exactly as found, and one that this store would have written
    /// identically costs nothing at all. Editing such a cell rewrites this run **in place**
    /// ([`crate::arena::attributes::set_attribute`]) rather than regenerating it, so the unmodelled
    /// attribute survives the edit too.
    pub(crate) attributes: TextSpan,
    /// Bytes between the previous sibling's end and this cell's `<`, when there are any: whitespace
    /// a pretty-printer wrote, a comment, or an element that is not a `c`.
    pub(crate) leading: TextSpan,
    /// The cell's `<f …>…</f>`, when it has one — the "formula as an opaque handle" this child
    /// stores and MJXOFF-115 (D11) parses. It is a *view into* [`Self::before_payload`], which is
    /// what the writer copies; keeping both means a reader can ask for the formula without a scan
    /// and the writer can replay every byte before the value, formula and foreign markup alike, in
    /// the order the file wrote them.
    pub(crate) formula: TextSpan,
    /// The cell's children **before** its value element — a `<f>` formula, and anything foreign
    /// ahead of it. This is where the "formula as an opaque index" of MJXOFF-115 (D11) lives until
    /// that child models it.
    pub(crate) before_payload: TextSpan,
    /// The cell's children **after** its value element — an `extLst`, a second value element, a
    /// trailing comment.
    pub(crate) after_payload: TextSpan,
}

impl CellExtras {
    /// Whether this record carries nothing, in which case the cell need not point at one.
    pub(crate) fn is_empty(&self) -> bool {
        self.attributes.is_none()
            && self.leading.is_none()
            && self.formula.is_none()
            && self.before_payload.is_none()
            && self.after_payload.is_none()
    }
}

/// One row, packed. **48 bytes** — a sheet holds at most 1,048,576 of them against a million times
/// more cells, so a row may afford what a cell may not.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PackedRow {
    /// `row@r` exactly as written. Zero when the attribute is absent — see
    /// [`RowFlags::HAS_NUMBER`], because `r="0"` is a thing a file can say and a thing this store
    /// must give back.
    pub(crate) number: u32,
    /// The first of this row's cells in the sheet's flat cell arena.
    pub(crate) first_cell: u32,
    /// How many cells this row has.
    pub(crate) cell_count: u32,
    /// Bytes between the previous row's end and this row's `<`.
    pub(crate) leading: TextSpan,
    /// The whole `<row …>…</row>` in the part's bytes, or [`TextSpan::NONE`] once the row or any
    /// cell in it is edited. **The row's copy-on-write state.**
    pub(crate) extent: TextSpan,
    /// The start tag's attribute run — everything between `<row` and the `>` that closes it.
    ///
    /// Twelve attributes are declared on `CT_Row` and this store decodes exactly one of them, `r`,
    /// because `r` is the key it is indexed by. The other eleven — `spans`, `ht`, `hidden`,
    /// `customFormat`, `customHeight`, `outlineLevel`, `collapsed`, `thickTop`, `thickBot`, `ph`,
    /// `s` — are read out of this run on demand and written back into it in place, which preserves
    /// their order, their prefixes, their quoting and anything unmodelled beside them.
    pub(crate) attributes: TextSpan,
    /// Bytes after the last cell and before the end tag — a row-level `extLst`, or a newline.
    pub(crate) trailing: TextSpan,
    /// [`RowFlags`].
    pub(crate) flags: u8,
}

/// The bit flags [`PackedRow::flags`] carries.
pub(crate) struct RowFlags;

impl RowFlags {
    /// `row@r` was written.
    pub(crate) const HAS_NUMBER: u8 = 1 << 0;
    /// The row was written `<row …/>` rather than `<row …></row>`.
    pub(crate) const SELF_CLOSING: u8 = 1 << 1;
    /// This row's cells are in strictly ascending column order, so a column can be found by binary
    /// search. Cleared for a file that wrote them out of order, which is preserved, not sorted.
    pub(crate) const CELLS_ASCENDING: u8 = 1 << 2;
}

impl PackedRow {
    /// Whether `flag` is set.
    pub(crate) fn has(&self, flag: u8) -> bool {
        self.flags & flag != 0
    }

    /// The range of the cell arena this row owns.
    pub(crate) fn cell_range(&self) -> core::ops::Range<usize> {
        let first = self.first_cell as usize;
        first..first.saturating_add(self.cell_count as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The size of these two records **is** the design, so it is asserted rather than described.
    ///
    /// A change that grows [`PackedCell`] is a change that costs a million times its size on the
    /// file this store was built for, and it should have to come here and say so.
    #[test]
    fn the_packed_records_are_the_size_the_design_says_they_are() {
        assert_eq!(
            core::mem::size_of::<PackedCell>(),
            36,
            "a cell is 36 bytes: an 8-byte reference, two 8-byte spans, two 4-byte indices and \
             two bytes of discriminant. Against the 913 B/cell of a RawElement tree, that is the \
             whole point of this module."
        );
        assert_eq!(
            core::mem::size_of::<PackedRow>(),
            48,
            "a row is 48 bytes; there are at most 1,048,576 of them against a million times more \
             cells"
        );
        assert_eq!(core::mem::size_of::<CellExtras>(), 40);
        assert_eq!(
            core::mem::size_of::<CellReference>(),
            8,
            "MJXOFF-93's own assertion, restated here because this record's size depends on it"
        );
    }

    #[test]
    fn a_written_cell_type_and_an_absent_one_are_different_codes() {
        assert_eq!(CellTypeCode::cell_type(CellTypeCode::ABSENT), None);
        for cell_type in [
            CellType::Boolean,
            CellType::Number,
            CellType::Error,
            CellType::SharedString,
            CellType::FormulaString,
            CellType::InlineString,
        ] {
            let code = CellTypeCode::of(cell_type);
            assert_ne!(
                code,
                CellTypeCode::ABSENT,
                "{cell_type:?} must not share a code with `absent`"
            );
            assert_eq!(CellTypeCode::cell_type(code), Some(cell_type));
        }
        // The trap this guards: `t="n"` is the schema default, so it is tempting to store it as
        // absent — and a file that wrote it would then come back without it.
        assert_ne!(CellTypeCode::of(CellType::Number), CellTypeCode::ABSENT);
    }

    #[test]
    fn payload_shape_survives_the_bit_packing_it_shares_with_the_other_flags() {
        let mut cell = PackedCell {
            reference: CellReference::relative(0, 0).expect("A1"),
            extent: TextSpan::NONE,
            payload: TextSpan::NONE,
            style: 0,
            extra: NO_EXTRAS,
            kind: CellTypeCode::ABSENT,
            flags: CellFlags::HAS_REFERENCE | CellFlags::SELF_CLOSING,
        };
        assert_eq!(cell.payload_shape(), PayloadShape::Absent);
        for shape in [
            PayloadShape::ValueText,
            PayloadShape::InlineString,
            PayloadShape::Absent,
        ] {
            cell.set_payload_shape(shape);
            assert_eq!(cell.payload_shape(), shape);
            assert!(
                cell.has(CellFlags::HAS_REFERENCE) && cell.has(CellFlags::SELF_CLOSING),
                "setting the payload shape must not disturb the flags beside it"
            );
            assert!(!cell.has(CellFlags::HAS_STYLE));
        }
    }
}
