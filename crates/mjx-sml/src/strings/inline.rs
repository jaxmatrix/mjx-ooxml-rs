//! `t="inlineStr"` — a cell that carries its own `CT_Rst` instead of pointing at the table.
//!
//! # Why this is here and not in [`cells`](crate::cells)
//!
//! An inline string is the *same complex type* a shared string is. `CT_Cell`'s `is` child is a
//! `CT_Rst`, character for character the type an `sst`'s `si` is, and MJXOFF-97's "done when" makes
//! the consequence explicit: *"reading a shared-string cell and reading an inline-string cell
//! produce the same value type"*. So both are [`StringItem`], both come out of the same reader, and
//! there is nothing here but the wrapper that says "one item, called `is`".
//!
//! # The other half of that clause
//!
//! *"each writes back in its original form."* That is not this type's doing and is worth saying so:
//! the cell store holds an inline-string cell's `<is>…</is>` as bytes and its `t="inlineStr"` as a
//! decoded field, and re-emits both. Nothing here or in the store ever moves a cell's text into the
//! table or the other way — a conversion between the two forms is a decision about the workbook, not
//! a thing a reader does on the way past.
//!
//! This type is how a caller *reads* those preserved bytes:
//! [`Cell::inline_string_markup`](crate::Cell::inline_string_markup) hands them over, and
//! [`InlineString::parse`] decodes them.

use crate::arena::TextSpan;
use crate::error::SmlError;

use super::items::{write_text_element, StringItems};
use super::view::StringItem;

/// The `CT_Rst` inside a `t="inlineStr"` cell's `<is>` element.
///
/// **`CT_` symbol:** `CT_Rst` (`sml.xsd` line 1845), reached through `CT_Cell`'s `is` child.
#[derive(Debug)]
pub struct InlineString {
    items: StringItems,
}

impl InlineString {
    /// Decodes an `<is>…</is>` element from its own bytes.
    ///
    /// The bytes need declare no namespaces: they are a fragment lifted out of a worksheet that
    /// declared them, and this reader matches on local names. A prefix the fragment cannot resolve
    /// is kept as it stands and re-emitted, which is what [`markup`](Self::markup) returns.
    ///
    /// # Errors
    ///
    /// [`SmlError::Xml`] if `markup` is not well-formed XML, or
    /// [`SmlError::PackedStoreTooLarge`] if it is beyond a `u32` address space.
    pub fn parse(markup: &[u8]) -> Result<Self, SmlError> {
        let mut items = StringItems::new(None, None, "is")?;
        items.push_markup(markup, TextSpan::NONE)?;
        Ok(Self { items })
    }

    /// An inline string holding one plain `<t>`.
    ///
    /// `xml:space="preserve"` is written exactly when dropping it would change the string — the same
    /// rule the shared-string table authors by, because it is the same element.
    ///
    /// # Errors
    ///
    /// As [`parse`](Self::parse), for the markup this builds and immediately reads back.
    pub fn plain(text: &str) -> Result<Self, SmlError> {
        let mut markup = b"<is>".to_vec();
        write_text_element(&mut markup, None, text);
        markup.extend_from_slice(b"</is>");
        Self::parse(&markup)
    }

    /// The value this cell holds.
    ///
    /// The same [`StringItem`] a shared-string index resolves to, so a caller reading a column of
    /// text never has to know which form each cell used.
    #[must_use]
    pub fn item(&self) -> StringItem<'_> {
        StringItem::new(&self.items, 0)
    }

    /// This inline string's own bytes — `<is>…</is>`, exactly as they were parsed or authored.
    #[must_use]
    pub fn markup(&self) -> &[u8] {
        self.item().markup()
    }
}
