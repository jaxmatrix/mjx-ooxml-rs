//! The reading surface: [`StringItem`], [`RichTextRun`], [`PhoneticRun`] and [`PhoneticProperties`].
//!
//! Two-word handles onto the packed records, exactly as [`Cell`](crate::Cell) and
//! [`Row`](crate::Row) are onto the cell store's. Nothing is decoded until somebody asks, and a
//! caller that never asks for text never pays for unescaping it.
//!
//! **[`StringItem`] is the answer to MJXOFF-97's same-value-type clause.** A `t="s"` cell's index
//! resolves to one through
//! [`SharedStringTable::item`](super::SharedStringTable::item); a `t="inlineStr"` cell's `<is>`
//! resolves to one through [`InlineString::item`](super::InlineString::item). The two paths differ
//! in where the bytes came from and in nothing else.

use std::borrow::Cow;

use mjx_ooxml_types::spreadsheetml::{PhoneticAlignment, PhoneticType};

use crate::arena::{attribute_run_of, attributes};
use crate::font::{FontProperties, FontPropertyOwner};

use super::items::StringItems;
use super::record::{ItemFlags, PackedPhoneticRun, PackedRun, RunFlags};

/// One string value — `CT_Rst`, reached as a table's `si` or as a cell's `is`.
///
/// **`CT_` symbol:** `CT_Rst` (`sml.xsd` line 1845). Wire children: `t?`, `r*`, `rPh*`,
/// `phoneticPr?`.
///
/// A `CT_Rst` says its text in one of two ways and Office writes both: a plain `t` for unformatted
/// text, and a sequence of `r` runs when parts of the string are formatted differently.
/// [`text`](Self::text) answers for either, so a caller that only wants the string never has to
/// know which it got.
#[derive(Debug, Clone, Copy)]
pub struct StringItem<'a> {
    items: &'a StringItems,
    index: usize,
}

impl<'a> StringItem<'a> {
    pub(super) fn new(items: &'a StringItems, index: usize) -> Self {
        Self { items, index }
    }

    /// This item's position in the table it came from — the number a `t="s"` cell's `<v>` holds.
    ///
    /// Zero for an [`InlineString`](super::InlineString), which holds exactly one item and is
    /// indexed by nobody.
    #[must_use]
    pub fn index(&self) -> u32 {
        self.index as u32
    }

    /// The whole string this item says, plain `t` and every run concatenated, unescaped.
    ///
    /// Borrowed from the part's bytes for the overwhelmingly common shape — a single `t` with no
    /// entity references — and owned only where there is something to join or to decode.
    ///
    /// # Errors
    ///
    /// [`mjx_xml::XmlError`] if the text carries an entity reference that cannot be decoded, or is
    /// not UTF-8.
    pub fn text(&self) -> Result<Cow<'a, str>, mjx_xml::XmlError> {
        let runs = self.items.runs_of(self.index);
        let plain = self.raw_text();
        if runs.is_empty() {
            return match plain {
                Some(raw) => unescape(raw),
                None => Ok(Cow::Borrowed("")),
            };
        }
        let mut joined = match plain {
            Some(raw) => unescape(raw)?.into_owned(),
            None => String::new(),
        };
        for run in runs {
            joined.push_str(&unescape(self.items.bytes(run.text))?);
        }
        Ok(Cow::Owned(joined))
    }

    /// The still-escaped bytes inside this item's own `<t>`, or `None` when it has none.
    ///
    /// `Some(b"")` and `None` are different answers: the first is `<t/>` — an entry whose value is
    /// the empty string — and the second is an item made only of runs.
    #[must_use]
    pub fn raw_text(&self) -> Option<&'a [u8]> {
        let span = self.items.items[self.index].text;
        (!span.is_none()).then(|| self.items.bytes(span))
    }

    /// Whether this item's `<t>` carried `xml:space="preserve"`.
    ///
    /// **Load-bearing, not decoration.** Without the attribute a consumer may collapse the leading
    /// and trailing whitespace of the text, so `"  total  "` and `"total"` are the same file and
    /// different strings. It is preserved on read and written back on any item this table authors
    /// whose text needs it.
    #[must_use]
    pub fn preserves_space(&self) -> bool {
        self.items.items[self.index].has(ItemFlags::TEXT_PRESERVES_SPACE)
    }

    /// This item's rich-text runs, in document order.
    #[must_use]
    pub fn runs(&self) -> impl ExactSizeIterator<Item = RichTextRun<'a>> + 'a {
        let items = self.items;
        items
            .runs_of(self.index)
            .iter()
            .map(move |run| RichTextRun { items, run })
    }

    /// How many rich-text runs this item has. Zero for a plain entry.
    #[must_use]
    pub fn run_count(&self) -> usize {
        self.items.runs_of(self.index).len()
    }

    /// This item's phonetic runs — the East Asian ruby readings shown above its base text.
    #[must_use]
    pub fn phonetic_runs(&self) -> impl ExactSizeIterator<Item = PhoneticRun<'a>> + 'a {
        let items = self.items;
        items
            .phonetics_of(self.index)
            .iter()
            .map(move |run| PhoneticRun { items, run })
    }

    /// `phoneticPr` — how this item's ruby text is rendered, decoded, or `None` when it wrote none.
    #[must_use]
    pub fn phonetic_properties(&self) -> Option<PhoneticProperties> {
        let span = self.items.phonetic_properties_of(self.index);
        (!span.is_none())
            .then(|| PhoneticProperties::read(self.items.bytes(span)))
            .flatten()
    }

    /// The `<phoneticPr …/>` element verbatim, or `None`.
    #[must_use]
    pub fn phonetic_properties_markup(&self) -> Option<&'a [u8]> {
        let span = self.items.phonetic_properties_of(self.index);
        (!span.is_none()).then(|| self.items.bytes(span))
    }

    /// Whether this item may be reused for a plain string by
    /// [`SharedStringTable::intern`](super::SharedStringTable::intern).
    ///
    /// True only for a bare `<si><t>…</t></si>`. An item with runs or phonetic markup displays the
    /// same characters and is **not** the same value — pointing a new cell at it would give that
    /// cell formatting or ruby text nobody asked for.
    #[must_use]
    pub fn is_internable(&self) -> bool {
        self.items.items[self.index].has(ItemFlags::INTERNABLE)
    }

    /// This item's own bytes — `<si>…</si>`, or `<is>…</is>` for an inline string.
    ///
    /// Always available, because an item is written by copying these. This is the assertion an
    /// edit-isolation test is written against: after one entry's text changes, every other entry
    /// must still answer with the bytes it was read from.
    #[must_use]
    pub fn markup(&self) -> &'a [u8] {
        self.items.bytes(self.items.items[self.index].extent)
    }

    /// The bytes between the previous item and this one — the newline a pretty-printer wrote, a
    /// comment, or an element that is not an item.
    #[must_use]
    pub fn leading_markup(&self) -> &'a [u8] {
        self.items.bytes(self.items.items[self.index].leading)
    }
}

/// One run of a rich-text string — `CT_RElt`.
///
/// **`CT_` symbol:** `CT_RElt` (`sml.xsd` line 1820). Wire children: `rPr?`, `t`.
#[derive(Debug, Clone, Copy)]
pub struct RichTextRun<'a> {
    items: &'a StringItems,
    run: &'a PackedRun,
}

impl<'a> RichTextRun<'a> {
    /// This run's text, unescaped.
    ///
    /// # Errors
    ///
    /// [`mjx_xml::XmlError`] if the text carries an entity reference that cannot be decoded, or is
    /// not UTF-8.
    pub fn text(&self) -> Result<Cow<'a, str>, mjx_xml::XmlError> {
        unescape(self.raw_text())
    }

    /// This run's still-escaped text bytes.
    #[must_use]
    pub fn raw_text(&self) -> &'a [u8] {
        self.items.bytes(self.run.text)
    }

    /// Whether this run's `<t>` carried `xml:space="preserve"`.
    #[must_use]
    pub fn preserves_space(&self) -> bool {
        self.run.has(RunFlags::TEXT_PRESERVES_SPACE)
    }

    /// This run's `<rPr>…</rPr>` verbatim, or `None` when it carries none.
    ///
    /// **This is the preservation contract, and [`properties`](Self::properties) is the reading
    /// one.** Everything a producer wrote inside the `rPr` is in these bytes — attribute order,
    /// quote style, prefixes, and any element `mjx-sml` does not model — and an edit to the run's
    /// text splices around them rather than through them.
    #[must_use]
    pub fn properties_markup(&self) -> Option<&'a [u8]> {
        (!self.run.properties.is_none()).then(|| self.items.bytes(self.run.properties))
    }

    /// This run's formatting, decoded, or `None` when it carries no `rPr`.
    ///
    /// The same [`FontProperties`] a `styles.xml` font-table entry decodes to (MJXOFF-105) — see
    /// [`crate::font`] for why the two are one type.
    ///
    /// # Errors
    ///
    /// [`mjx_xml::XmlError`] if the preserved `rPr` bytes do not re-parse, which would mean this
    /// crate had written markup it cannot read.
    pub fn properties(&self) -> Result<Option<FontProperties>, mjx_xml::XmlError> {
        let Some(markup) = self.properties_markup() else {
            return Ok(None);
        };
        FontProperties::from_markup(markup, FontPropertyOwner::RichTextRun).map(Some)
    }

    /// This run's own bytes — `<r>…</r>`.
    #[must_use]
    pub fn markup(&self) -> &'a [u8] {
        self.items.bytes(self.run.extent)
    }
}

/// One East Asian ruby annotation — `CT_PhoneticRun`.
///
/// **`CT_` symbol:** `CT_PhoneticRun` (`sml.xsd` line 1813). Wire children: `t`. Wire attributes:
/// `sb`, `eb`.
///
/// Ruby text is a *reading* printed above a span of the base text — most often the kana for a run of
/// kanji. Dropping it does not make a Japanese workbook look plainer; it removes information the
/// author typed and Excel cannot recover.
#[derive(Debug, Clone, Copy)]
pub struct PhoneticRun<'a> {
    items: &'a StringItems,
    run: &'a PackedPhoneticRun,
}

impl<'a> PhoneticRun<'a> {
    /// The reading itself, unescaped.
    ///
    /// # Errors
    ///
    /// [`mjx_xml::XmlError`] if the text carries an entity reference that cannot be decoded, or is
    /// not UTF-8.
    pub fn text(&self) -> Result<Cow<'a, str>, mjx_xml::XmlError> {
        unescape(self.raw_text())
    }

    /// The reading's still-escaped text bytes.
    #[must_use]
    pub fn raw_text(&self) -> &'a [u8] {
        self.items.bytes(self.run.text)
    }

    /// `@sb` — where in the item's base text this reading starts, in UTF-16 code units.
    #[must_use]
    pub fn start_base(&self) -> u32 {
        self.run.start_base
    }

    /// `@eb` — one past where it ends, in UTF-16 code units.
    #[must_use]
    pub fn end_base(&self) -> u32 {
        self.run.end_base
    }

    /// This run's own bytes — `<rPh …>…</rPh>`.
    #[must_use]
    pub fn markup(&self) -> &'a [u8] {
        self.items.bytes(self.run.extent)
    }
}

/// How an item's ruby text is rendered — `CT_PhoneticPr`.
///
/// **`CT_` symbol:** `CT_PhoneticPr` (`sml.xsd` line 1853). Wire attributes: `fontId` (required),
/// `type` (default `fullwidthKatakana`), `alignment` (default `left`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhoneticProperties {
    /// `@fontId` — the index into `styles.xml`'s font table the ruby text is drawn with.
    pub font_id: u32,
    /// `@type` — `ST_PhoneticType`. Which script the reading is written in. Schema default
    /// [`PhoneticType::FullwidthKatakana`].
    pub script: PhoneticType,
    /// `@alignment` — `ST_PhoneticAlignment`. How the reading is distributed over its base text.
    /// Schema default [`PhoneticAlignment::Left`].
    pub alignment: PhoneticAlignment,
}

impl PhoneticProperties {
    /// Decodes a `<phoneticPr …/>` from its own bytes.
    ///
    /// `None` only when the bytes are not a start tag at all. A `fontId` that is missing or does not
    /// parse reads as zero rather than failing: the element is preserved verbatim either way, and an
    /// unreadable font index is not a reason to refuse to open a workbook.
    #[must_use]
    pub(super) fn read(markup: &[u8]) -> Option<Self> {
        let run = attribute_run_of(markup)?;
        let text = |name: &str| {
            attributes::value(run, name).and_then(|value| core::str::from_utf8(value).ok())
        };
        Some(Self {
            font_id: text("fontId")
                .and_then(|value| value.trim().parse().ok())
                .unwrap_or(0),
            script: text("type")
                .and_then(PhoneticType::from_wire)
                .unwrap_or(PhoneticType::FullwidthKatakana),
            alignment: text("alignment")
                .and_then(PhoneticAlignment::from_wire)
                .unwrap_or(PhoneticAlignment::Left),
        })
    }
}

/// Unescapes still-escaped markup bytes, borrowing when there is nothing to decode.
fn unescape(raw: &[u8]) -> Result<Cow<'_, str>, mjx_xml::XmlError> {
    let text = core::str::from_utf8(raw)
        .map_err(|_| mjx_xml::XmlError::Syntax("a shared string was not UTF-8".to_owned()))?;
    mjx_xml::text::unescape_text(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phonetic_properties_default_the_two_attributes_the_schema_defaults() {
        let properties =
            PhoneticProperties::read(br#"<phoneticPr fontId="1"/>"#).expect("a start tag");
        assert_eq!(properties.font_id, 1);
        assert_eq!(properties.script, PhoneticType::FullwidthKatakana);
        assert_eq!(properties.alignment, PhoneticAlignment::Left);
    }

    #[test]
    fn phonetic_properties_read_what_was_written_and_ignore_the_prefix() {
        let properties = PhoneticProperties::read(
            br#"<x:phoneticPr fontId="4" type="Hiragana" alignment="center"/>"#,
        )
        .expect("a start tag");
        assert_eq!(
            properties,
            PhoneticProperties {
                font_id: 4,
                script: PhoneticType::Hiragana,
                alignment: PhoneticAlignment::Center,
            }
        );
    }

    #[test]
    fn an_unreadable_font_id_is_zero_rather_than_a_failure() {
        let properties =
            PhoneticProperties::read(br#"<phoneticPr fontId="fourteen"/>"#).expect("a start tag");
        assert_eq!(properties.font_id, 0);
        assert_eq!(PhoneticProperties::read(b"phoneticPr/>"), None);
    }
}
