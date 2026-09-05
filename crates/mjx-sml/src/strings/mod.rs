//! The shared string table: `sharedStrings.xml`, rich-text runs, phonetic runs and inline strings.
//!
//! # Why this exists
//!
//! A `t="s"` cell holds an **index**, not text. Until this module, [`crate::cells`] could hold every
//! cell of a workbook and tell nobody what any of them said.
//! [`Cell::shared_string_index`](crate::Cell::shared_string_index) is the contract's one half and
//! [`SharedStringTable::item`] is the other; between them a cell costs four bytes for its value
//! rather than a copy of it, which is the whole reason `sharedStrings.xml` is a part at all.
//!
//! # What is modelled here
//!
//! | Type | `CT_` symbol | `sml.xsd` |
//! |---|---|---|
//! | [`SharedStringTable`] | `CT_Sst` | 1789 |
//! | [`StringItem`] | `CT_Rst` | 1845 |
//! | [`RichTextRun`] | `CT_RElt` | 1820 |
//! | [`PhoneticRun`] | `CT_PhoneticRun` | 1813 |
//! | [`PhoneticProperties`] | `CT_PhoneticPr` | 1853 |
//! | [`InlineString`] | `CT_Rst`, reached through `CT_Cell`'s `is` | 1845 |
//!
//! A run's `rPr` is `CT_RPrElt` (line 1826) and decodes to
//! [`FontProperties`](crate::FontProperties), which lives in [`crate::font`] because
//! `styles.xml`'s `CT_Font` is the same fifteen slots and MJXOFF-105 reuses it rather than copying
//! it.
//!
//! # Three things a naive model gets wrong
//!
//! **Whitespace is load-bearing, and the file will not warn you.** `xml:space="preserve"` on a `t`
//! is the difference between `"  total  "` and `"total"`, and `sml.xsd` types a `t` as a *simple*
//! type that can carry no attribute at all — so the attribute every producer writes does not
//! validate, and a model that "cleaned it up" would be schema-correct and wrong. It is preserved on
//! read, and written on an authored entry exactly when its absence would change the string. See
//! [`StringItem::preserves_space`].
//!
//! **`count` and `uniqueCount` are hints, not derived values.** Both round-trip as read; only a
//! change to the entry list moves `uniqueCount`, and nothing here ever computes `count`, because the
//! table cannot see a cell. [`SharedStringTable`]'s own documentation has the full policy.
//!
//! **An index is a public address.** Removing an unreferenced entry renumbers every later one and
//! silently changes the text of every cell that held those numbers. Entries are therefore
//! append-only, and [`SharedStringTable::compact`] is an explicit call that returns the remapping
//! the caller must apply to every sheet.
//!
//! # Phonetic markup is not decoration
//!
//! `rPh` and `phoneticPr` carry the *reading* of East Asian text — the kana printed above a run of
//! kanji, which the author typed and which cannot be recovered from the base text. A model that
//! dropped them would silently damage every Japanese workbook it touched, so they are preserved
//! byte for byte like everything else and decoded by [`PhoneticRun`] and [`PhoneticProperties`].
//!
//! # Memory
//!
//! An entry is 48 bytes of `u32` over the arena in `crate::arena`, the same one address space the
//! cell store uses: a table nobody has edited owns no bytes of its own and shares the part's buffer
//! with the package. `strings/record.rs` carries the accounting;
//! `crates/mjx-sml/tests/shared_string_allocation.rs` measures a per-entry bound against it with a
//! counting global allocator rather than by inspection.

mod inline;
mod items;
mod read;
mod record;
mod table;
mod view;

pub use inline::InlineString;
pub use table::{RichTextRunSpec, SharedStringTable};
pub use view::{PhoneticProperties, PhoneticRun, RichTextRun, StringItem};
