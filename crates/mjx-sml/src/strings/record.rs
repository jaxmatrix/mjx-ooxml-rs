//! The packed records a shared-string table is made of, and what each byte of them buys.
//!
//! # Why a table of strings is not a `Vec<String>`
//!
//! `PLAN.md` line 26 names **two** bulk-data cases for the arena half of the hybrid model —
//! *"spreadsheet cells, shared strings"* — and this is the second. A workbook's `sharedStrings.xml`
//! is one entry per *distinct* string in the whole file: a 300,000-cell sheet of mostly-text columns
//! routinely has tens of thousands of them, and Excel's own limit is 1,048,576 unique strings.
//!
//! A `Vec<String>` costs one heap allocation per entry plus 24 bytes of `String` header, before the
//! text. Worse, it costs a **copy** of every byte of a part the package already holds — for a table
//! nobody has edited, that is the whole part duplicated for nothing. The measurement that decided
//! the cell store applies unchanged: the per-entry allocation is the cost.
//!
//! So an entry is 48 bytes of `u32`s over the same one address space
//! ([`crate::arena`]) the cell store uses, and a table nobody has touched owns no bytes at all.
//!
//! | Record | Bytes | One per |
//! |---|---|---|
//! | [`PackedStringItem`] | 48 | `si` (or the `is` of an inline-string cell) |
//! | [`PackedRun`] | 36 | `r` — a rich-text run |
//! | [`PackedPhoneticRun`] | 24 | `rPh` — an East Asian ruby annotation |
//! | [`ItemExtras`] | 16 | only an item that carries phonetic markup |
//!
//! # Why every record keeps its own extent
//!
//! `CT_Rst` is a small type with a large fidelity surface: `xml:space="preserve"` on a `t` changes
//! the string, an `rPr` can carry markup this workspace does not model, and the whitespace between
//! two `si` elements is bytes a pretty-printed file wrote. **Every one of those survives because an
//! item is written by copying its extent**, not by re-serializing it from these fields. The decoded
//! spans exist so a caller can *read* the item and so an edit can splice one `<t>` element inside
//! it; they are not how it is reproduced.
//!
//! That is why an item's extent is always present — a source range for an item read from a part, and
//! a range over bytes this table authored for one it built. There is no "no bytes yet" state, and so
//! no second write path that could disagree with the first.

use crate::arena::TextSpan;

/// The index [`PackedStringItem::extras`] carries when the item needs no side record.
pub(super) const NO_EXTRAS: u32 = u32::MAX;

/// One `si` of a `sst`, or the `is` of an inline-string cell — `CT_Rst`.
///
/// **`CT_` symbol:** `CT_Rst` (`sml.xsd` line 1845). Wire children: `t?`, `r*`, `rPh*`,
/// `phoneticPr?`.
#[derive(Debug, Clone, Copy)]
pub(super) struct PackedStringItem {
    /// The whole `<si>…</si>` — **always present**, in the source for an item read from a part and
    /// in the table's own bytes for one it authored. This is what a write copies.
    pub(super) extent: TextSpan,
    /// The bytes between the previous item and this one: the newline a pretty-printer wrote, a
    /// comment, or an element that is not an `si`.
    pub(super) leading: TextSpan,
    /// The item-level `<t>…</t>` element, or [`TextSpan::NONE`] for an item made only of runs.
    ///
    /// Kept as well as [`text`](Self::text) because an edit replaces the whole element — a new
    /// string may need an `xml:space="preserve"` the old one did not have, and that lives in the
    /// start tag.
    pub(super) text_element: TextSpan,
    /// The still-escaped inner text of that `<t>`. Present and empty for `<t></t>`, absent for an
    /// item with no `t` at all — a distinction [`TextSpan::NONE`] exists to keep.
    pub(super) text: TextSpan,
    /// The first of this item's runs in the table's flat run vector.
    pub(super) first_run: u32,
    /// How many runs this item has.
    pub(super) run_count: u32,
    /// Index into the table's side table, or [`NO_EXTRAS`].
    pub(super) extras: u32,
    /// [`ItemFlags`].
    pub(super) flags: u32,
}

/// The bit flags [`PackedStringItem::flags`] carries.
pub(super) struct ItemFlags;

impl ItemFlags {
    /// The item's `<t>` carried `xml:space="preserve"`.
    pub(super) const TEXT_PRESERVES_SPACE: u32 = 1 << 0;
    /// The item is a bare `<si><t>…</t></si>`: no runs, no phonetic markup, no attributes and no
    /// children but the one `t`. **Only such an item may be reused by
    /// [`SharedStringTable::intern`](super::SharedStringTable::intern)** — see its documentation for
    /// why an item with runs is not interchangeable with the same text.
    pub(super) const INTERNABLE: u32 = 1 << 1;
}

impl PackedStringItem {
    /// Whether `flag` is set.
    pub(super) fn has(self, flag: u32) -> bool {
        self.flags & flag != 0
    }

    /// The half-open range of this item's runs in the flat run vector.
    pub(super) fn run_range(self) -> core::ops::Range<usize> {
        let start = self.first_run as usize;
        start..start + self.run_count as usize
    }
}

impl Default for PackedStringItem {
    fn default() -> Self {
        Self {
            extent: TextSpan::NONE,
            leading: TextSpan::NONE,
            text_element: TextSpan::NONE,
            text: TextSpan::NONE,
            first_run: 0,
            run_count: 0,
            extras: NO_EXTRAS,
            flags: 0,
        }
    }
}

/// One `r` of a `CT_Rst` — a run of text with its own formatting.
///
/// **`CT_` symbol:** `CT_RElt` (`sml.xsd` line 1820). Wire children: `rPr?`, `t`.
#[derive(Debug, Clone, Copy)]
pub(super) struct PackedRun {
    /// The whole `<r>…</r>`.
    pub(super) extent: TextSpan,
    /// The whole `<rPr>…</rPr>`, verbatim, or [`TextSpan::NONE`].
    ///
    /// Bytes rather than a decoded [`FontProperties`](crate::FontProperties): a run's properties are
    /// preserved exactly this way, including anything the model does not carry, and decoding is what
    /// [`RichTextRun::properties`](super::RichTextRun::properties) does on demand for a caller that
    /// asks.
    pub(super) properties: TextSpan,
    /// The run's `<t>…</t>` element.
    pub(super) text_element: TextSpan,
    /// The still-escaped inner text of that `<t>`.
    pub(super) text: TextSpan,
    /// [`RunFlags`].
    pub(super) flags: u32,
}

/// The bit flags [`PackedRun::flags`] carries.
pub(super) struct RunFlags;

impl RunFlags {
    /// The run's `<t>` carried `xml:space="preserve"`.
    pub(super) const TEXT_PRESERVES_SPACE: u32 = 1 << 0;
}

impl PackedRun {
    /// Whether `flag` is set.
    pub(super) fn has(self, flag: u32) -> bool {
        self.flags & flag != 0
    }
}

impl Default for PackedRun {
    fn default() -> Self {
        Self {
            extent: TextSpan::NONE,
            properties: TextSpan::NONE,
            text_element: TextSpan::NONE,
            text: TextSpan::NONE,
            flags: 0,
        }
    }
}

/// One `rPh` — the reading of a span of East Asian text, shown above it as ruby.
///
/// **`CT_` symbol:** `CT_PhoneticRun` (`sml.xsd` line 1813). Wire children: `t`. Wire attributes:
/// `sb`, `eb`, both required.
#[derive(Debug, Clone, Copy)]
pub(super) struct PackedPhoneticRun {
    /// The whole `<rPh …>…</rPh>`.
    pub(super) extent: TextSpan,
    /// The still-escaped inner text of its `<t>` — the reading itself.
    pub(super) text: TextSpan,
    /// `@sb` — the first character of the base text this reading annotates, counted in UTF-16 code
    /// units from the start of the item's text.
    pub(super) start_base: u32,
    /// `@eb` — one past the last such character.
    pub(super) end_base: u32,
}

impl Default for PackedPhoneticRun {
    fn default() -> Self {
        Self {
            extent: TextSpan::NONE,
            text: TextSpan::NONE,
            start_base: 0,
            end_base: 0,
        }
    }
}

/// What an item carries beyond a `t` and its runs — allocated only for the items that have it.
///
/// Phonetic markup appears in Japanese workbooks and essentially nowhere else, so the common item
/// pays four bytes for [`PackedStringItem::extras`] and nothing more.
#[derive(Debug, Clone, Copy)]
pub(super) struct ItemExtras {
    /// The first of this item's phonetic runs in the table's flat vector.
    pub(super) first_phonetic: u32,
    /// How many phonetic runs this item has.
    pub(super) phonetic_count: u32,
    /// The whole `<phoneticPr …/>`, verbatim, or [`TextSpan::NONE`].
    pub(super) phonetic_properties: TextSpan,
}

impl ItemExtras {
    /// The half-open range of this item's phonetic runs.
    pub(super) fn phonetic_range(self) -> core::ops::Range<usize> {
        let start = self.first_phonetic as usize;
        start..start + self.phonetic_count as usize
    }

    /// Whether this record says nothing, in which case the item need not carry one.
    pub(super) fn is_empty(self) -> bool {
        self.phonetic_count == 0 && self.phonetic_properties.is_none()
    }
}

impl Default for ItemExtras {
    fn default() -> Self {
        Self {
            first_phonetic: 0,
            phonetic_count: 0,
            phonetic_properties: TextSpan::NONE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_packed_records_are_the_size_the_design_says_they_are() {
        // The figures the module documentation quotes, and the ones
        // `tests/shared_string_allocation.rs` measures a per-entry bound against. A field added
        // without a decision fails here rather than in a memory profile six children later.
        assert_eq!(core::mem::size_of::<PackedStringItem>(), 48);
        assert_eq!(core::mem::size_of::<PackedRun>(), 36);
        assert_eq!(core::mem::size_of::<PackedPhoneticRun>(), 24);
        assert_eq!(core::mem::size_of::<ItemExtras>(), 16);
    }

    #[test]
    fn a_default_item_is_absent_everywhere_rather_than_empty_at_address_zero() {
        // The same trap `TextSpan::NONE` exists for: a derived `Default` would give every span a
        // *present* zero-length range at address zero, and every item would read as one holding an
        // empty `<t>`.
        let item = PackedStringItem::default();
        assert!(item.extent.is_none());
        assert!(item.text.is_none());
        assert!(item.text_element.is_none());
        assert_eq!(item.extras, NO_EXTRAS);
        assert!(item.run_range().is_empty());
    }

    #[test]
    fn the_flags_do_not_collide() {
        assert_ne!(ItemFlags::TEXT_PRESERVES_SPACE, ItemFlags::INTERNABLE);
        let item = PackedStringItem {
            flags: ItemFlags::INTERNABLE,
            ..PackedStringItem::default()
        };
        assert!(item.has(ItemFlags::INTERNABLE));
        assert!(!item.has(ItemFlags::TEXT_PRESERVES_SPACE));
    }
}
