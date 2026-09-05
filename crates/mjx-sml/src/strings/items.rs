//! The packed store of `CT_Rst` values, shared by the shared-string table and by inline strings.
//!
//! # One store, two callers
//!
//! `CT_Rst` is reached from two places and must read to the same value from both: as an `si` of
//! `sharedStrings.xml`, and as the `is` of a `t="inlineStr"` cell. MJXOFF-97's "done when" states
//! that as a requirement — *"reading a shared-string cell and reading an inline-string cell produce
//! the same value type"* — and the way to satisfy it is not to write the reader twice and hope. It
//! is for both to be this store, differing only in how many items they hold and what the element is
//! called: [`SharedStringTable`](super::SharedStringTable) is this plus the `sst` wrapper and its
//! two count attributes, and [`InlineString`](super::InlineString) is this holding exactly one item.
//!
//! # The invariant everything else rests on
//!
//! **Every item's [`extent`](PackedStringItem::extent) is present.** An item read from a part points
//! at the part's own bytes; an item this store authored points at bytes it appended to the arena;
//! an item read from a tree that had lost its source ranges is serialized into the arena on the way
//! in. There is no "not backed by bytes yet" state.
//!
//! That buys three things at once:
//!
//! * **Writing an item is a `memcpy`**, always, so the whitespace inside a start tag, an `rPr` this
//!   workspace does not model, a comment between two runs and the exact spelling of
//!   `xml:space="preserve"` all come back because nothing ever re-serializes them.
//! * **Editing is a splice**, not a rebuild: replacing a run's text replaces the bytes of that one
//!   `<t>` element inside the item's bytes and leaves every other byte of the item alone. An `rPr`
//!   survives an edit to the text beside it *exactly*, which a rebuild from a decoded model could
//!   only approximate.
//! * **Authoring and reading share one path.** An authored item is serialized to bytes, stored, and
//!   then read back through the same reader as a file's — so the records and the bytes cannot
//!   disagree, because they were never produced separately.

use std::sync::Arc;

use mjx_ooxml_core::{Interner, RawElement};

use crate::arena::{TextArena, TextSpan};
use crate::error::SmlError;

use super::read;
use super::record::{ItemExtras, PackedPhoneticRun, PackedRun, PackedStringItem, NO_EXTRAS};

/// Which `<t>` of an item an edit is aimed at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TextTarget {
    /// The item's own `t` — `CT_Rst`'s first child.
    Item,
    /// The `t` of the run at this position among the item's runs.
    Run(usize),
}

/// A flat store of `CT_Rst` items over one byte arena.
#[derive(Debug)]
pub(super) struct StringItems {
    /// The byte space: the part's own bytes, then whatever this store authored.
    pub(super) arena: TextArena,
    /// The items, in document order.
    pub(super) items: Vec<PackedStringItem>,
    /// Every item's runs, concatenated in item order.
    pub(super) runs: Vec<PackedRun>,
    /// Every item's phonetic runs, concatenated in item order.
    pub(super) phonetics: Vec<PackedPhoneticRun>,
    /// The side records, for the items that need one.
    pub(super) extras: Vec<ItemExtras>,
    /// The prefix this store writes new markup with — the one the `sst` or the worksheet used.
    pub(super) prefix: Option<Box<str>>,
    /// The local name of an item element: `si` in a shared-string table, `is` in a cell.
    pub(super) item_local: &'static str,
}

impl StringItems {
    /// An empty store over `source`, authoring items named `item_local`.
    ///
    /// # Errors
    ///
    /// [`SmlError::PackedStoreTooLarge`] if `source` is too large for a `u32` address space.
    pub(super) fn new(
        source: Option<Arc<[u8]>>,
        prefix: Option<&str>,
        item_local: &'static str,
    ) -> Result<Self, SmlError> {
        Ok(Self {
            arena: TextArena::new(source)?,
            items: Vec::new(),
            runs: Vec::new(),
            phonetics: Vec::new(),
            extras: Vec::new(),
            prefix: prefix.map(Box::from),
            item_local,
        })
    }

    /// The bytes a span covers.
    pub(super) fn bytes(&self, span: TextSpan) -> &[u8] {
        self.arena.bytes(span)
    }

    /// How many bytes of its own this store has authored. Zero for a table nobody has edited.
    pub(super) fn edited_bytes(&self) -> usize {
        self.arena.edited_bytes()
    }

    /// The qualified name an item element is written with — `si`, `x:si`, `is`, …
    pub(super) fn item_qname(&self) -> Vec<u8> {
        let mut qname = Vec::new();
        if let Some(prefix) = &self.prefix {
            qname.extend_from_slice(prefix.as_bytes());
            qname.push(b':');
        }
        qname.extend_from_slice(self.item_local.as_bytes());
        qname
    }

    /// Appends one item read from `element`, whose source ranges are offset by `base`.
    ///
    /// `base` is zero when the element was parsed from the buffer this arena was built over, and the
    /// address of the stored markup when the element was parsed from a fragment this store just
    /// authored. That one parameter is what lets authoring and reading be the same code.
    ///
    /// # Errors
    ///
    /// [`SmlError::PackedStoreTooLarge`] if the arena cannot hold what has to be stored, or
    /// [`SmlError::Xml`] if markup that had to be re-parsed is not well-formed.
    pub(super) fn push_from_element(
        &mut self,
        element: &RawElement,
        interner: &Interner,
        source: Option<&Arc<[u8]>>,
        base: u32,
        leading: TextSpan,
    ) -> Result<usize, SmlError> {
        let item = read::read_item(self, element, interner, source, base, leading)?;
        self.items.push(item);
        Ok(self.items.len() - 1)
    }

    /// Appends one item authored from `markup`, which must be exactly one element.
    ///
    /// Stores the bytes, then reads them back through [`push_from_element`](Self::push_from_element)
    /// so that the records describe the bytes rather than being built beside them.
    ///
    /// # Errors
    ///
    /// [`SmlError::Xml`] if `markup` is not well-formed, [`SmlError::PackedStoreTooLarge`] if the
    /// arena is full.
    pub(super) fn push_markup(
        &mut self,
        markup: &[u8],
        leading: TextSpan,
    ) -> Result<usize, SmlError> {
        let stored = self.arena.store(markup)?;
        let document = mjx_xml::fidelity::parse(markup)?;
        let index = self.push_from_element(
            &document.root,
            &document.interner,
            None,
            stored.start(),
            leading,
        )?;
        Ok(index)
    }

    /// Replaces the text of one `<t>` inside item `index`, splicing the new element into the item's
    /// own bytes and leaving every other byte of it alone.
    ///
    /// When the target is [`TextTarget::Item`] and the item has no `t` at all, the item's whole
    /// *content* is replaced instead — its start tag survives, and its runs and phonetic markup do
    /// not, because an item whose text is now this string has nothing else to say.
    ///
    /// # Errors
    ///
    /// [`SmlError::PackedStoreTooLarge`] if the arena is full, [`SmlError::Xml`] if the rebuilt item
    /// does not re-parse — which would mean this store had written markup it cannot read.
    pub(super) fn set_text(
        &mut self,
        index: usize,
        target: TextTarget,
        text: &str,
    ) -> Result<(), SmlError> {
        let Some(item) = self.items.get(index).copied() else {
            return Ok(());
        };
        let region = match target {
            TextTarget::Item if !item.text_element.is_none() => item.text_element,
            TextTarget::Item => self.content_region(index),
            TextTarget::Run(run) => {
                let Some(run) = self.runs_of(index).get(run).copied() else {
                    return Ok(());
                };
                run.text_element
            }
        };
        if region.is_none() {
            return Ok(());
        }

        let mut replacement = Vec::new();
        write_text_element(&mut replacement, self.prefix.as_deref(), text);

        // Rebuild the item's bytes with the region replaced, store them, and re-read the result.
        // Re-reading rather than re-anchoring every span by hand is the same decision authoring
        // makes: one reader, so the records cannot drift from the bytes they describe.
        let old = self.bytes(item.extent);
        let offset = (region.start() - item.extent.start()) as usize;
        let tail = offset + region.length() as usize;
        let mut rebuilt = Vec::with_capacity(old.len() + replacement.len());
        rebuilt.extend_from_slice(&old[..offset.min(old.len())]);
        rebuilt.extend_from_slice(&replacement);
        rebuilt.extend_from_slice(old.get(tail..).unwrap_or_default());

        self.replace_item(index, &rebuilt)
    }

    /// Replaces item `index` wholesale with `markup`, re-reading it into the same slot.
    ///
    /// # Errors
    ///
    /// As [`set_text`](Self::set_text).
    pub(super) fn replace_item(&mut self, index: usize, markup: &[u8]) -> Result<(), SmlError> {
        let leading = self.items[index].leading;
        let stored = self.arena.store(markup)?;
        let document = mjx_xml::fidelity::parse(markup)?;
        let replacement = read::read_item(
            self,
            &document.root,
            &document.interner,
            None,
            stored.start(),
            leading,
        )?;
        self.items[index] = replacement;
        Ok(())
    }

    /// The byte range between an item's start tag and its end tag, recomputed from its bytes.
    ///
    /// Only used on the path where an item has no `t` to splice, which is why it is worth
    /// recomputing rather than worth eight bytes on every record.
    fn content_region(&self, index: usize) -> TextSpan {
        let item = self.items[index];
        let qname = self.item_qname();
        let Some(layout) =
            crate::arena::layout_in_arena(self.bytes(item.extent), &qname, item.extent)
        else {
            return TextSpan::NONE;
        };
        crate::arena::span_present_between(layout.inner_start, layout.inner_end)
    }

    /// The runs of the item at `index`.
    pub(super) fn runs_of(&self, index: usize) -> &[PackedRun] {
        self.runs
            .get(self.items[index].run_range())
            .unwrap_or_default()
    }

    /// The phonetic runs of the item at `index`.
    pub(super) fn phonetics_of(&self, index: usize) -> &[PackedPhoneticRun] {
        let extras = self.items[index].extras;
        if extras == NO_EXTRAS {
            return &[];
        }
        self.phonetics
            .get(self.extras[extras as usize].phonetic_range())
            .unwrap_or_default()
    }

    /// The `<phoneticPr …/>` of the item at `index`, verbatim.
    pub(super) fn phonetic_properties_of(&self, index: usize) -> TextSpan {
        let extras = self.items[index].extras;
        if extras == NO_EXTRAS {
            return TextSpan::NONE;
        }
        self.extras[extras as usize].phonetic_properties
    }

    /// Writes item `index` — its leading bytes, then its own bytes, both copied.
    pub(super) fn write_item(&self, index: usize, out: &mut Vec<u8>) {
        let item = self.items[index];
        out.extend_from_slice(self.bytes(item.leading));
        out.extend_from_slice(self.bytes(item.extent));
    }
}

/// Writes `<t>text</t>`, adding `xml:space="preserve"` exactly when dropping it would change the
/// string.
///
/// # Why the attribute is conditional, and why it is written at all
///
/// `sml.xsd` types a `t` as the *simple* type `s:ST_Xstring`, which can carry no attribute at all —
/// so `xml:space` on a `t` does not validate against ECMA-376 Transitional. It is nonetheless what
/// **both** Excel and LibreOffice write, and `crates/mjx-schema-gate/src/tolerances.rs` already
/// records that as a producer-wide divergence in inputs this workspace preserves.
///
/// Authoring it is a different question from preserving it, and the answer is decided by what the
/// alternative costs: without the attribute, a consumer is free to collapse the leading and trailing
/// whitespace, and `"  total  "` comes back `"total"`. **Losing the string is worse than diverging
/// from a schema every producer diverges from**, so it is written — and written *only* where its
/// absence would change the value, so that a table of ordinary strings is schema-valid markup and
/// byte-identical to what `mjx-chart`'s writer produces.
pub(super) fn write_text_element(out: &mut Vec<u8>, prefix: Option<&str>, text: &str) {
    let preserve = needs_space_preservation(text);
    out.push(b'<');
    crate::font::value::write_qualified_name(out, prefix, "t");
    if preserve {
        out.extend_from_slice(b" xml:space=\"preserve\"");
    }
    if text.is_empty() {
        out.extend_from_slice(b"/>");
        return;
    }
    out.push(b'>');
    out.extend_from_slice(mjx_xml::text::escape_text(text).as_bytes());
    out.extend_from_slice(b"</");
    crate::font::value::write_qualified_name(out, prefix, "t");
    out.push(b'>');
}

/// Whether `text` would come back different if `xml:space="preserve"` were dropped.
///
/// XML's default whitespace handling lets a consumer normalize leading and trailing whitespace, so
/// that — and not the presence of whitespace anywhere — is the test.
pub(super) fn needs_space_preservation(text: &str) -> bool {
    text != text.trim_matches(|c: char| c == ' ' || c == '\t' || c == '\n' || c == '\r')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_attribute_is_written_exactly_when_dropping_it_would_change_the_string() {
        let mut out = Vec::new();
        write_text_element(&mut out, None, "total");
        assert_eq!(out, b"<t>total</t>".to_vec());

        out.clear();
        write_text_element(&mut out, None, "  total  ");
        assert_eq!(
            out,
            br#"<t xml:space="preserve">  total  </t>"#.to_vec(),
            "leading and trailing whitespace is the whole reason the attribute exists"
        );

        out.clear();
        write_text_element(&mut out, None, "one two");
        assert_eq!(
            out,
            b"<t>one two</t>".to_vec(),
            "whitespace *inside* the string is not at risk and needs no attribute"
        );
    }

    #[test]
    fn an_empty_string_writes_a_self_closing_element() {
        // The spelling `mjx-chart`'s writer produces for an empty entry, so that table and this one
        // agree byte for byte — MJXOFF-112's parity gate rests on it.
        let mut out = Vec::new();
        write_text_element(&mut out, None, "");
        assert_eq!(out, b"<t/>".to_vec());
    }

    #[test]
    fn the_prefix_reaches_both_tags() {
        let mut out = Vec::new();
        write_text_element(&mut out, Some("x"), " a ");
        assert_eq!(out, br#"<x:t xml:space="preserve"> a </x:t>"#.to_vec());
    }

    #[test]
    fn text_is_escaped_as_character_data() {
        let mut out = Vec::new();
        write_text_element(&mut out, None, "a < b & c");
        assert_eq!(out, b"<t>a &lt; b &amp; c</t>".to_vec());
    }

    #[test]
    fn a_newline_at_the_edge_counts_as_whitespace_that_must_be_preserved() {
        assert!(needs_space_preservation("\nline"));
        assert!(needs_space_preservation("line\t"));
        assert!(!needs_space_preservation("li\nne"));
    }
}
