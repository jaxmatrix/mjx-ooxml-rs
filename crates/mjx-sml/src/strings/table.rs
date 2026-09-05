//! `xl/sharedStrings.xml` — the interned string table, and the two hints on its root element.
//!
//! # What the table is for
//!
//! A `t="s"` cell holds an *index*, not text. `<c r="A1" t="s"><v>3</v></c>` says "the string at
//! position 3", and nothing in the worksheet says what that is. This table is the other half of that
//! contract: [`Cell::shared_string_index`](crate::Cell::shared_string_index) gives the index,
//! [`SharedStringTable::item`] gives the value, and the cell store deliberately holds no strings at
//! all so that a million shared-string cells cost four bytes each rather than a copy each.
//!
//! # `count` and `uniqueCount` are hints, not facts
//!
//! `CT_Sst` (`sml.xsd` line 1789) declares two optional attributes, and the difference between them
//! is the difference between what this table can know and what it cannot:
//!
//! * **`uniqueCount`** is how many `si` entries there are. The table *is* the entries, so it knows.
//! * **`count`** is how many `t="s"` **cells** in the whole workbook point into it. The table cannot
//!   see a single cell, so it does not know and never guesses.
//!
//! Both round-trip **as read**. A file whose `uniqueCount` disagrees with its own entry count — and
//! real files do, because the attributes are producer hints rather than derived values — comes back
//! saying exactly what it said. `tests/fixtures/shared_strings_rich_text.xlsx` is authored to be
//! such a file precisely so that a writer which recomputed the value unconditionally would be caught
//! by a test rather than by a user.
//!
//! The one thing that *does* move them is a change to the entry list. Appending an entry makes an
//! old `uniqueCount` definitely wrong, so it is recomputed then — and only then, and only if the
//! file wrote the attribute at all, because a file that wrote no `uniqueCount` must not gain one.
//! `count` is still not recomputed even then: adding a table entry does not add a cell.
//! [`set_reference_count`](SharedStringTable::set_reference_count) is how a caller that *can* see the
//! cells — MJXOFF-112's package writer — says so.
//!
//! # Entry lifetime: nothing is ever renumbered
//!
//! See [`SharedStringTable::compact`]. The short version is that an index is a public address: the
//! moment a cell in some sheet holds it, removing an earlier entry rewrites the meaning of every
//! cell after it. So entries are **append-only**, an entry nothing references any more stays, and
//! compaction is an explicit call that hands the caller the remapping it must then apply to every
//! sheet itself.

use std::collections::HashMap;
use std::sync::Arc;

use mjx_ooxml_core::{Interner, RawDocument, RawElement, RawNode, Symbol};
use mjx_ooxml_types::namespaces::SML;

use crate::arena::{attributes, layout_in_arena, span_between, span_present_between, TextSpan};
use crate::error::SmlError;
use crate::font::FontProperties;

use super::items::{write_text_element, StringItems, TextTarget};
use super::view::StringItem;

/// The XML declaration and newline this crate writes ahead of an authored `sharedStrings.xml`.
///
/// Byte-identical to what `mjx-chart`'s minimal workbook writer emits, because MJXOFF-112's parity
/// gate compares the two part for part and MJXOFF-99 then deletes the other one.
const AUTHORED_PROLOGUE: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n";

/// One rich-text run to author — the input side of [`SharedStringTable::push_rich_text`].
#[derive(Debug, Clone, Default)]
pub struct RichTextRunSpec {
    /// The run's text. `xml:space="preserve"` is added for it exactly when dropping the attribute
    /// would change the string.
    pub text: String,
    /// The run's formatting, or `None` for a run that inherits the cell's.
    pub properties: Option<FontProperties>,
}

/// `xl/sharedStrings.xml` — `CT_Sst`.
///
/// **`CT_` symbol:** `CT_Sst` (`sml.xsd` line 1789), wire element `sst`. Wire children: `si*`,
/// `extLst?`. Wire attributes: `count`, `uniqueCount`.
#[derive(Debug)]
pub struct SharedStringTable {
    items: StringItems,
    /// The whole `<sst>…</sst>`, or [`TextSpan::NONE`] once anything about the table has changed.
    extent: TextSpan,
    /// The `sst` start tag's attribute run, exactly as the file wrote it.
    attributes: TextSpan,
    /// The bytes after the last `si` and before `</sst>` — an `extLst`, a trailing newline.
    trailing: TextSpan,
    /// The part's bytes before the `sst` element: the XML declaration and whatever follows it.
    prologue: TextSpan,
    /// The part's bytes after the `sst` element.
    epilogue: TextSpan,
    /// Whether the file wrote `<sst/>`.
    self_closing: bool,
    /// `@count` as it will be written, or `None` to write no attribute.
    reference_count: Option<u32>,
    /// `@uniqueCount` as it will be written, or `None` to write no attribute.
    unique_count: Option<u32>,
    /// Whether [`Self::reference_count`] follows the entry count, which is true only for a table
    /// this crate authored and has no cells to count.
    reference_count_tracks_entries: bool,
    /// Whether the two counts above still agree with the preserved attribute run.
    counts_are_stale: bool,
    /// The interning index, built on first use and dropped by any edit that could invalidate it.
    lookup: Option<HashMap<Symbol, u32>>,
    /// The workspace interner the lookup's keys live in.
    interner: Interner,
}

impl SharedStringTable {
    /// An empty table this crate authored, writing markup with `prefix`.
    ///
    /// Writes both `count` and `uniqueCount`, each equal to the entry count, until
    /// [`set_reference_count`](Self::set_reference_count) says otherwise. That is what a table every
    /// entry of which is referenced exactly once would say, and it is what `mjx-chart`'s writer
    /// emits — the shape MJXOFF-112's parity gate compares against.
    ///
    /// # Errors
    ///
    /// [`SmlError::PackedStoreTooLarge`] is impossible here in practice and returned rather than
    /// unwrapped, because this constructor stores the XML declaration into the arena.
    pub fn authored(prefix: Option<&str>) -> Result<Self, SmlError> {
        let mut items = StringItems::new(None, prefix, "si")?;
        let prologue = items.arena.store(AUTHORED_PROLOGUE)?;
        let attributes = items
            .arena
            .store(format!(" xmlns=\"{}\"", SML.transitional).as_bytes())?;
        Ok(Self {
            items,
            extent: TextSpan::NONE,
            attributes,
            trailing: TextSpan::NONE,
            prologue,
            epilogue: TextSpan::NONE,
            self_closing: false,
            reference_count: Some(0),
            unique_count: Some(0),
            reference_count_tracks_entries: true,
            counts_are_stale: true,
            lookup: None,
            interner: Interner::new(),
        })
    }

    /// Reads a whole `xl/sharedStrings.xml` part.
    ///
    /// `Ok(None)` when the document's root is not an `sst` — the caller handed over a different
    /// part, which is a question rather than an error.
    ///
    /// # Errors
    ///
    /// [`SmlError::PackedStoreTooLarge`] for a part beyond the `u32` byte space, or
    /// [`SmlError::Xml`] if markup with no byte range behind it fails to re-parse. Nothing a
    /// well-formed file can *say* is refused.
    pub fn read_part(document: &RawDocument) -> Result<Option<Self>, SmlError> {
        if document.interner.resolve(document.root.name.local) != "sst" {
            return Ok(None);
        }
        Self::read(&document.root, &document.interner, document.shared_source()).map(Some)
    }

    /// Reads an `sst` element, with `source` being the buffer its byte ranges index into.
    ///
    /// # Errors
    ///
    /// As [`read_part`](Self::read_part).
    pub fn read(
        element: &RawElement,
        interner: &Interner,
        source: Option<&Arc<[u8]>>,
    ) -> Result<Self, SmlError> {
        let prefix = element.name.prefix.map(|prefix| interner.resolve(prefix));
        let mut items = StringItems::new(source.cloned(), prefix, "si")?;

        let extent = match element.source_span() {
            Some(span) => items.arena.span_in_source(span.start, span.end),
            None => TextSpan::NONE,
        };
        let mut qname = Vec::new();
        if let Some(prefix) = prefix {
            qname.extend_from_slice(prefix.as_bytes());
            qname.push(b':');
        }
        qname.extend_from_slice(b"sst");
        let layout = layout_in_arena(items.arena.bytes(extent), &qname, extent);

        let attributes = match &layout {
            Some(layout) => layout.attribute_run,
            None => items
                .arena
                .store(&rebuild_attribute_run(element, interner))?,
        };
        let self_closing = layout.as_ref().map_or(element.empty, |l| l.self_closing);

        // The cursor is the position just past whatever has been accounted for, in the `sst`'s own
        // range. It exists only on the range-backed path; without a range, the bytes between two
        // items are serialized into the arena instead, which reaches the same answer the slow way.
        let mut cursor = layout.as_ref().map(|layout| layout.inner_start);
        let mut pending: Vec<u8> = Vec::new();
        let mut trailing = TextSpan::NONE;

        for child in element.children.iter() {
            let is_item = matches!(child, RawNode::Element(child) if interner.resolve(child.name.local) == "si");
            if !is_item {
                if cursor.is_none() {
                    mjx_xml::fidelity::serialize_node(
                        child,
                        interner,
                        source.map(Arc::as_ref),
                        &mut pending,
                    );
                }
                continue;
            }
            let RawNode::Element(child) = child else {
                continue;
            };
            let leading = match &mut cursor {
                Some(cursor) => {
                    let start = *cursor;
                    let end = child
                        .source_span()
                        .map_or(start, |span| span.start.max(start));
                    *cursor = child.source_span().map_or(*cursor, |span| span.end);
                    span_between(start, end)
                }
                None => {
                    let stored = if pending.is_empty() {
                        TextSpan::NONE
                    } else {
                        items.arena.store(&pending)?
                    };
                    pending.clear();
                    stored
                }
            };
            items.push_from_element(child, interner, source, 0, leading)?;
        }

        match cursor {
            Some(cursor) => {
                if let Some(layout) = &layout {
                    trailing = span_between(cursor, layout.inner_end);
                }
            }
            None => {
                if !pending.is_empty() {
                    trailing = items.arena.store(&pending)?;
                }
            }
        }

        let run = items.arena.bytes(attributes);
        let reference_count = attributes::value(run, "count").and_then(parse_count);
        let unique_count = attributes::value(run, "uniqueCount").and_then(parse_count);

        let (prologue, epilogue) = part_bounds(extent, source);

        Ok(Self {
            items,
            extent,
            attributes,
            trailing,
            prologue,
            epilogue,
            self_closing,
            reference_count,
            unique_count,
            reference_count_tracks_entries: false,
            counts_are_stale: false,
            lookup: None,
            interner: Interner::new(),
        })
    }

    // ---------------------------------------------------------------------------------------
    // Reading
    // ---------------------------------------------------------------------------------------

    /// How many `si` entries the table holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.items.len()
    }

    /// Whether the table holds no entries at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.items.is_empty()
    }

    /// The entry at `index` — what a `t="s"` cell holding that number says.
    ///
    /// `None` for an index past the end, which a file can perfectly well write. That is reported as
    /// absence rather than repaired, exactly as
    /// [`Cell::shared_string_index`](crate::Cell::shared_string_index) reports an unparseable `<v>`.
    #[must_use]
    pub fn item(&self, index: u32) -> Option<StringItem<'_>> {
        let index = index as usize;
        (index < self.items.items.len()).then(|| StringItem::new(&self.items, index))
    }

    /// Every entry, in the order the file wrote them — which is the order their indices run in.
    pub fn items(&self) -> impl ExactSizeIterator<Item = StringItem<'_>> + '_ {
        (0..self.items.items.len()).map(|index| StringItem::new(&self.items, index))
    }

    /// `@count` — how many `t="s"` cells the producer said point into this table, or `None` if it
    /// wrote no such attribute.
    ///
    /// **A hint, never derived here.** The table cannot see a cell; see [`SharedStringTable`]'s own
    /// documentation.
    #[must_use]
    pub fn reference_count(&self) -> Option<u32> {
        self.reference_count
    }

    /// `@uniqueCount` — how many entries the producer said there are, or `None` if it wrote no such
    /// attribute.
    ///
    /// May disagree with [`len`](Self::len), and comes back as written when it does. It is
    /// recomputed only when the entry list itself changes.
    #[must_use]
    pub fn unique_count(&self) -> Option<u32> {
        self.unique_count
    }

    /// Whether the whole part can still be written straight out of its own bytes.
    #[must_use]
    pub fn is_verbatim(&self) -> bool {
        !self.extent.is_none()
    }

    /// How many bytes of its own this table has authored. Zero for one nobody has edited.
    #[must_use]
    pub fn edited_bytes(&self) -> usize {
        self.items.edited_bytes()
    }

    // ---------------------------------------------------------------------------------------
    // Interning
    // ---------------------------------------------------------------------------------------

    /// The index of `text` in the table, appending an entry if it is not already there.
    ///
    /// This is the operation `sharedStrings.xml` exists for, and the one `mjx-chart`'s minimal
    /// workbook writer has its own copy of (`crates/mjx-chart/src/workbook.rs`) pending MJXOFF-112
    /// and MJXOFF-99. It reproduces that copy's behaviour exactly for the case it covers — plain
    /// entries, in first-use order — so the switch is a deletion rather than a rewrite.
    ///
    /// # What may be reused
    ///
    /// Only an entry [`StringItem::is_internable`] answers `true` for: a bare
    /// `<si><t>…</t></si>`. An entry carrying rich-text runs or phonetic markup displays the same
    /// characters and is **not** the same value, so a plain string never resolves to it — pointing a
    /// cell there would give that cell formatting or ruby text nobody asked for. The comparison is
    /// on the **decoded** text, so `&amp;` and `&#38;` are the same entry, and it is exact, so
    /// `"total"` and `" total "` are not.
    ///
    /// # Cost
    ///
    /// The index is built on first call, in one pass over the entries, and maintained from then on.
    /// A table nobody interns into never builds it. Any edit that could invalidate it drops it, and
    /// the next call rebuilds.
    ///
    /// # Errors
    ///
    /// [`SmlError::PackedStoreTooLarge`] if the arena cannot hold the new entry, or
    /// [`SmlError::Xml`] if the entry this crate just wrote does not re-parse.
    pub fn intern(&mut self, text: &str) -> Result<u32, SmlError> {
        self.build_lookup()?;
        let symbol = self.interner.intern(text);
        if let Some(index) = self.lookup.as_ref().and_then(|lookup| lookup.get(&symbol)) {
            return Ok(*index);
        }
        let index = self.push_plain_text(text)?;
        if let Some(lookup) = self.lookup.as_mut() {
            lookup.insert(symbol, index);
        }
        Ok(index)
    }

    /// The index `text` already has, without appending anything.
    ///
    /// # Errors
    ///
    /// As [`intern`](Self::intern), for the index build.
    pub fn index_of(&mut self, text: &str) -> Result<Option<u32>, SmlError> {
        self.build_lookup()?;
        let Some(symbol) = self.interner.get(text) else {
            return Ok(None);
        };
        Ok(self
            .lookup
            .as_ref()
            .and_then(|lookup| lookup.get(&symbol))
            .copied())
    }

    /// Builds the interning index if it is not already there.
    fn build_lookup(&mut self) -> Result<(), SmlError> {
        if self.lookup.is_some() {
            return Ok(());
        }
        let mut lookup = HashMap::with_capacity(self.items.items.len());
        for index in 0..self.items.items.len() {
            let item = StringItem::new(&self.items, index);
            if !item.is_internable() {
                continue;
            }
            let text = item.text()?;
            let symbol = self.interner.intern(&text);
            // First use wins, which is what makes the index the *first* entry holding the text —
            // the same answer a producer's own writer gives.
            lookup.entry(symbol).or_insert(index as u32);
        }
        self.lookup = Some(lookup);
        Ok(())
    }

    // ---------------------------------------------------------------------------------------
    // Editing
    // ---------------------------------------------------------------------------------------

    /// Appends a plain entry — `<si><t>…</t></si>` — and returns its index.
    ///
    /// Appends unconditionally; [`intern`](Self::intern) is the call that reuses.
    ///
    /// # Errors
    ///
    /// As [`intern`](Self::intern).
    pub fn push_plain_text(&mut self, text: &str) -> Result<u32, SmlError> {
        let mut markup = Vec::new();
        self.open_item(&mut markup);
        write_text_element(&mut markup, self.items.prefix.as_deref(), text);
        self.close_item(&mut markup);
        self.push_item_markup(&markup)
    }

    /// Appends a rich-text entry — one `<si>` holding a run per element of `runs`.
    ///
    /// # Errors
    ///
    /// As [`intern`](Self::intern).
    pub fn push_rich_text(&mut self, runs: &[RichTextRunSpec]) -> Result<u32, SmlError> {
        let prefix = self.items.prefix.as_deref().map(str::to_owned);
        let prefix = prefix.as_deref();
        let mut markup = Vec::new();
        self.open_item(&mut markup);
        for run in runs {
            markup.push(b'<');
            crate::font::value::write_qualified_name(&mut markup, prefix, "r");
            markup.push(b'>');
            if let Some(properties) = &run.properties {
                properties.write_into(
                    &mut markup,
                    prefix,
                    "rPr",
                    crate::font::FontPropertyOwner::RichTextRun,
                );
            }
            write_text_element(&mut markup, prefix, &run.text);
            markup.extend_from_slice(b"</");
            crate::font::value::write_qualified_name(&mut markup, prefix, "r");
            markup.push(b'>');
        }
        self.close_item(&mut markup);
        self.push_item_markup(&markup)
    }

    /// Appends an entry from markup the caller wrote — one `si` element, whatever it holds.
    ///
    /// The escape hatch for a `CT_Rst` shape this crate does not author, and the path a caller uses
    /// to copy an entry from one table to another verbatim.
    ///
    /// # Errors
    ///
    /// As [`intern`](Self::intern).
    pub fn push_item_markup(&mut self, markup: &[u8]) -> Result<u32, SmlError> {
        let index = self.items.push_markup(markup, TextSpan::NONE)?;
        self.entry_list_changed();
        Ok(index as u32)
    }

    /// Replaces the text of the entry at `index`.
    ///
    /// **Every cell pointing at this entry changes with it** — that is what a shared string is. It
    /// is also the operation MJXOFF-97's Tier-3 clause is written about: every *other* entry stays
    /// byte-identical, including entries this edit leaves unreferenced.
    ///
    /// The entry's other markup survives: the new `<t>` is spliced into the entry's own bytes, so an
    /// `rPr` on a sibling run, a comment, or an element this crate does not model is exactly where
    /// it was. An entry with no `t` at all has its *content* replaced instead, which does drop its
    /// runs — an entry whose text is now this string has nothing else to say.
    ///
    /// Does nothing for an index past the end.
    ///
    /// # Errors
    ///
    /// As [`intern`](Self::intern).
    pub fn set_text(&mut self, index: u32, text: &str) -> Result<(), SmlError> {
        let index = index as usize;
        if index >= self.items.items.len() {
            return Ok(());
        }
        self.items.set_text(index, TextTarget::Item, text)?;
        self.entry_changed();
        Ok(())
    }

    /// Replaces the text of one rich-text run of the entry at `index`.
    ///
    /// The run's `rPr` and every other run are untouched, byte for byte.
    ///
    /// Does nothing when either index is past the end.
    ///
    /// # Errors
    ///
    /// As [`intern`](Self::intern).
    pub fn set_run_text(&mut self, index: u32, run: usize, text: &str) -> Result<(), SmlError> {
        let index = index as usize;
        if index >= self.items.items.len() {
            return Ok(());
        }
        self.items.set_text(index, TextTarget::Run(run), text)?;
        self.entry_changed();
        Ok(())
    }

    /// Sets `@count` — how many `t="s"` cells point into this table — or removes the attribute.
    ///
    /// The table has no way to work this out: it cannot see a cell. A caller that can — the package
    /// writer, which holds every worksheet — says so here, and nothing else ever writes it.
    pub fn set_reference_count(&mut self, count: Option<u32>) {
        if self.reference_count == count && !self.reference_count_tracks_entries {
            return;
        }
        self.reference_count = count;
        self.reference_count_tracks_entries = false;
        self.counts_are_stale = true;
        self.extent = TextSpan::NONE;
    }

    /// Removes every entry `is_referenced` answers `false` for, **renumbering the rest**, and
    /// returns the old-index-to-new-index map.
    ///
    /// # Read this before calling it
    ///
    /// An entry's index is its public address: it is written into the `<v>` of every `t="s"` cell
    /// that uses it, in every worksheet of the workbook. Removing entry 3 therefore does not merely
    /// free entry 3 — it changes what entries 4, 5, 6 … *mean*, and every cell holding one of those
    /// numbers now says something different.
    ///
    /// **This crate cannot fix that, because it cannot see the sheets.** The returned vector is
    /// indexed by old index and holds the new index, or `None` for an entry that was dropped; the
    /// caller must rewrite every shared-string cell in the workbook through it before the file is
    /// written. A caller that ignores the return value has silently changed the text of an
    /// unpredictable number of cells.
    ///
    /// This is why nothing here compacts on its own. An edit that leaves an entry unreferenced
    /// leaves it in the table, where it costs a few bytes and breaks nothing.
    pub fn compact(&mut self, is_referenced: impl Fn(u32) -> bool) -> Vec<Option<u32>> {
        let mut mapping = Vec::with_capacity(self.items.items.len());
        let mut kept = Vec::with_capacity(self.items.items.len());
        let mut next = 0u32;
        for index in 0..self.items.items.len() {
            if is_referenced(index as u32) {
                mapping.push(Some(next));
                kept.push(self.items.items[index]);
                next += 1;
            } else {
                mapping.push(None);
            }
        }
        if kept.len() != self.items.items.len() {
            self.items.items = kept;
            self.entry_list_changed();
        }
        mapping
    }

    // ---------------------------------------------------------------------------------------
    // Writing
    // ---------------------------------------------------------------------------------------

    /// The whole part: prologue, `sst` element, epilogue.
    #[must_use]
    pub fn to_part_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.write_part_into(&mut out);
        out
    }

    /// Appends the whole part to `out`.
    pub fn write_part_into(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(self.items.bytes(self.prologue));
        self.write_element_into(out);
        out.extend_from_slice(self.items.bytes(self.epilogue));
    }

    /// Appends just the `<sst>…</sst>` element to `out`.
    ///
    /// One `memcpy` for a table nothing has touched. Otherwise: the start tag, then every entry
    /// copied from its own bytes, then whatever followed the last one.
    pub fn write_element_into(&self, out: &mut Vec<u8>) {
        if !self.extent.is_none() {
            out.extend_from_slice(self.items.bytes(self.extent));
            return;
        }
        let run = self.effective_attribute_run();
        out.push(b'<');
        self.write_qname(out);
        out.extend_from_slice(&run);
        if self.self_closing && self.items.items.is_empty() && self.trailing.is_none() {
            out.extend_from_slice(b"/>");
            return;
        }
        out.push(b'>');
        for index in 0..self.items.items.len() {
            self.items.write_item(index, out);
        }
        out.extend_from_slice(self.items.bytes(self.trailing));
        out.extend_from_slice(b"</");
        self.write_qname(out);
        out.push(b'>');
    }

    /// The `sst` start tag's attributes as they will be written.
    ///
    /// The file's own bytes unless the entry list changed — in which case `uniqueCount` is updated,
    /// and only if the file wrote one, and `count` is left exactly as it was unless a caller who can
    /// see the cells has said otherwise.
    fn effective_attribute_run(&self) -> std::borrow::Cow<'_, [u8]> {
        let run = self.items.bytes(self.attributes);
        if !self.counts_are_stale {
            return std::borrow::Cow::Borrowed(run);
        }
        let mut with_count = Vec::new();
        attributes::set_attribute(
            run,
            "count",
            self.reference_count
                .map(|c| c.to_string())
                .as_deref()
                .map(str::as_bytes),
            &mut with_count,
        );
        let mut with_unique = Vec::new();
        attributes::set_attribute(
            &with_count,
            "uniqueCount",
            self.unique_count
                .map(|c| c.to_string())
                .as_deref()
                .map(str::as_bytes),
            &mut with_unique,
        );
        std::borrow::Cow::Owned(with_unique)
    }

    /// Writes `sst` with the table's prefix.
    fn write_qname(&self, out: &mut Vec<u8>) {
        crate::font::value::write_qualified_name(out, self.items.prefix.as_deref(), "sst");
    }

    /// Opens an authored `<si>`.
    fn open_item(&self, out: &mut Vec<u8>) {
        out.push(b'<');
        crate::font::value::write_qualified_name(out, self.items.prefix.as_deref(), "si");
        out.push(b'>');
    }

    /// Closes an authored `<si>`.
    fn close_item(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(b"</");
        crate::font::value::write_qualified_name(out, self.items.prefix.as_deref(), "si");
        out.push(b'>');
    }

    /// One entry changed, but the entry list did not: the counts stay exactly as the file wrote
    /// them, because neither the number of entries nor the number of referencing cells moved.
    fn entry_changed(&mut self) {
        self.extent = TextSpan::NONE;
        self.lookup = None;
    }

    /// The entry list changed: `uniqueCount` is now definitely wrong if the file wrote one.
    fn entry_list_changed(&mut self) {
        self.extent = TextSpan::NONE;
        self.lookup = None;
        let length = self.items.items.len() as u32;
        if self.unique_count.is_some() {
            self.unique_count = Some(length);
            self.counts_are_stale = true;
        }
        if self.reference_count_tracks_entries {
            self.reference_count = Some(length);
            self.counts_are_stale = true;
        }
    }
}

/// The XML declaration before the root element and whatever follows it, as spans.
fn part_bounds(extent: TextSpan, source: Option<&Arc<[u8]>>) -> (TextSpan, TextSpan) {
    let (Some(source), false) = (source, extent.is_none()) else {
        return (TextSpan::NONE, TextSpan::NONE);
    };
    let Ok(length) = u32::try_from(source.len()) else {
        return (TextSpan::NONE, TextSpan::NONE);
    };
    let prologue = span_present_between(0, extent.start());
    let epilogue = span_between(extent.end(), length);
    (
        if prologue.length() == 0 {
            TextSpan::NONE
        } else {
            prologue
        },
        epilogue,
    )
}

/// The attribute run for an `sst` with no usable byte range, rebuilt from the model.
fn rebuild_attribute_run(element: &RawElement, interner: &Interner) -> Vec<u8> {
    let mut out = Vec::new();
    for attribute in element.attributes.iter() {
        out.push(b' ');
        if let Some(prefix) = attribute.name.prefix {
            out.extend_from_slice(interner.resolve(prefix).as_bytes());
            out.push(b':');
        }
        out.extend_from_slice(interner.resolve(attribute.name.local).as_bytes());
        out.push(b'=');
        out.push(attribute.quote.byte());
        out.extend_from_slice(&attribute.value);
        out.push(attribute.quote.byte());
    }
    out
}

/// Parses a `count`/`uniqueCount` value, or `None` when it is not an `xsd:unsignedInt`.
///
/// A value that does not parse is read as absent, and therefore written back from the preserved
/// bytes rather than regenerated — an unreadable hint is still a hint the file wrote.
fn parse_count(value: &[u8]) -> Option<u32> {
    core::str::from_utf8(value).ok()?.trim().parse().ok()
}
