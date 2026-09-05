//! The shared-string table's fidelity contract (MJXOFF-97): what survives a read, an edit and a
//! write, and what a reader is told.
//!
//! # The traps these cases are written against
//!
//! Three of this child's clauses are the shape that passes without doing anything, and each is
//! answered by the fixture rather than by an assertion that hopes.
//!
//! * *"`sample.xlsx`'s shared strings read back with their exact text and the part re-emits
//!   byte-identically"* is satisfied by a table that reads nothing and writes the bytes back. So
//!   every round-trip case here also reads the values out, and the byte comparison is against the
//!   part's own bytes rather than against a string written in this file.
//! * *"`count` and `uniqueCount` round-trip"* is satisfied trivially by **any** writer when the two
//!   already agree with the entry count, which is the only shape `sample.xlsx` has.
//!   `shared_strings_rich_text.xlsx` is authored so they do not:
//!   **`count="9"`, `uniqueCount="6"`, and seven `si` entries.** A writer that recomputed either
//!   value would change the bytes, and [`the_counts_are_hints_and_survive_an_edit_to_an_entry`]
//!   catches it.
//! * *"whitespace is load-bearing"* is satisfied by a fixture that carries no `xml:space` at all.
//!   Both fixtures carry one, and [`an_entry_that_needs_xml_space_keeps_it`] asserts on the
//!   attribute itself and not merely on the decoded text — which would come back right either way
//!   until somebody opened the file in Excel.
//!
//! The authored fixture also carries an entry **nothing references** (index 4, `"never
//! referenced"`), a duplicate of entry 0 at index 6, an empty `<t/>` at index 5, three rich-text
//! runs with two different `rPr` shapes, a phonetic run with `phoneticPr`, and — in the worksheet —
//! two `t="inlineStr"` cells, one plain and one rich, plus a `t="s"` cell whose index (99) points
//! past the end of the table.

use std::sync::Arc;

use mjx_ooxml_core::RawDocument;
use mjx_ooxml_types::spreadsheetml::{CellType, PhoneticAlignment, PhoneticType, UnderlineType};
use mjx_opc::{Package, PartName};
use mjx_sml::{Color, FontProperties, InlineString, RichTextRunSpec, SharedStringTable, SheetData};

/// The authored fixture, whose whole purpose is to disagree with the naive answer.
const DISCRIMINATING_FIXTURE: &str = "shared_strings_rich_text.xlsx";

/// An authored table carrying, on purpose, the constructs a rebuild loses.
///
/// * `</x:t >` on the first entry — whitespace inside an end tag, which `ETag ::= '</' Name S? '>'`
///   permits and which nothing but the original bytes reproduces.
/// * A comment between two entries and a `<q:note>` inside an `si`, neither of which `CT_Rst`
///   models: markup the item's extent carries and a decoded rebuild would not.
/// * An `rPr` with a foreign child and a doubled space in its start tag.
/// * A `t` whose text is written with a numeric character reference.
const DISCRIMINATING: &[u8] = br#"<x:sst xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="4"><x:si><x:t>Alpha</x:t ></x:si><!-- between --><x:si><x:t>&#65;mpersand &amp; co</x:t><q:note xmlns:q="urn:q" weight="3">kept</q:note></x:si>
  <x:si><x:r><x:rPr  foo='bar'><x:b/><q:odd xmlns:q="urn:q"/></x:rPr><x:t>Bold</x:t></x:r></x:si></x:sst>"#;

// -------------------------------------------------------------------------------------------
// Helpers
// -------------------------------------------------------------------------------------------

/// Parses `markup` and reads its `sst`.
fn read(markup: &[u8]) -> (RawDocument, SharedStringTable) {
    let document =
        mjx_xml::fidelity::parse_shared(Arc::from(markup)).expect("the string table parses");
    let table = SharedStringTable::read_part(&document)
        .expect("the table reads")
        .expect("the root is an sst");
    (document, table)
}

/// One part of a committed fixture.
fn part(fixture: &str, name: &str) -> Vec<u8> {
    let bytes = mjx_fixtures::fixture(fixture);
    let package = Package::open(&bytes).expect("a committed fixture opens");
    let part = PartName::new(name).expect("a valid part name");
    package
        .part_bytes(&part)
        .unwrap_or_else(|| panic!("{fixture} has no {name}"))
        .to_vec()
}

/// Every `sharedStrings.xml` in the committed corpus, derived from the directory rather than listed.
fn shared_string_parts() -> Vec<(String, Vec<u8>)> {
    let mut found = Vec::new();
    for name in mjx_fixtures::all_fixture_files() {
        if !name.ends_with(".xlsx") {
            continue;
        }
        let bytes = mjx_fixtures::fixture(&name);
        let package = Package::open(&bytes).expect("a committed fixture opens");
        let parts: Vec<PartName> = package
            .part_names()
            .filter(|part| part.as_str().ends_with("sharedStrings.xml"))
            .collect();
        for part in parts {
            let markup = package
                .part_bytes(&part)
                .expect("the part is there")
                .to_vec();
            found.push((format!("{name}::{}", part.as_str()), markup));
        }
    }
    assert!(
        found.len() >= 2,
        "only {} shared-string parts found in the committed corpus — a sweep that finds nothing \
         passes every assertion below",
        found.len()
    );
    found
}

// -------------------------------------------------------------------------------------------
// Tier 1 — the part re-emits byte for byte
// -------------------------------------------------------------------------------------------

/// Every committed `sharedStrings.xml` re-emits byte for byte, and its entries read back.
///
/// The second half is what stops this passing on a table that read nothing: every entry's text is
/// decoded and the entry count is checked against the markup's own `<si` count.
#[test]
fn every_committed_string_table_re_emits_byte_for_byte() {
    let parts = shared_string_parts();
    let mut entries_seen = 0usize;
    for (name, markup) in &parts {
        let (_document, table) = read(markup);
        assert_eq!(
            String::from_utf8_lossy(&table.to_part_bytes()),
            String::from_utf8_lossy(markup),
            "{name}: the part did not re-emit byte for byte"
        );
        assert_eq!(
            table.edited_bytes(),
            0,
            "{name}: a table nobody edited must own no bytes of its own"
        );
        assert!(table.is_verbatim(), "{name}");

        let declared = markup.windows(4).filter(|w| *w == b"<si>").count()
            + markup.windows(4).filter(|w| *w == b"<si/").count();
        assert_eq!(
            table.len(),
            declared,
            "{name}: the table read {} entries out of markup holding {declared}",
            table.len()
        );
        for item in table.items() {
            // Decoding every entry is what makes the byte comparison above evidence of a read.
            let _ = item.text().expect("every entry's text decodes");
            entries_seen += 1;
        }
    }
    assert!(
        entries_seen >= 10,
        "only {entries_seen} entries were read across the corpus"
    );
}

/// Markup a *rebuild* cannot reproduce comes back exactly, and its values still read.
#[test]
fn the_constructs_a_rebuild_loses_come_back_unchanged() {
    let (_document, table) = read(DISCRIMINATING);
    assert_eq!(
        String::from_utf8_lossy(&table.to_part_bytes()),
        String::from_utf8_lossy(DISCRIMINATING)
    );
    assert_eq!(table.len(), 3);
    assert_eq!(table.item(0).unwrap().text().unwrap(), "Alpha");
    assert_eq!(
        table.item(1).unwrap().text().unwrap(),
        "Ampersand & co",
        "a numeric character reference and a named one both decode"
    );
    assert_eq!(table.item(2).unwrap().text().unwrap(), "Bold");
    // The `</x:t >` is the byte only the original can produce; assert it is still there rather than
    // trusting the whole-part comparison to have covered it.
    assert!(
        table.item(0).unwrap().markup().ends_with(b"</x:t ></x:si>"),
        "{}",
        String::from_utf8_lossy(table.item(0).unwrap().markup())
    );
}

/// An entry with markup `CT_Rst` does not model keeps it, and still answers for its text.
#[test]
fn markup_the_model_does_not_carry_is_preserved_inside_an_entry() {
    let (_document, table) = read(DISCRIMINATING);
    let item = table.item(1).expect("the second entry");
    assert!(
        item.markup().windows(5).any(|w| w == b"q:not"),
        "the unmodelled child must still be in the entry's bytes"
    );
    assert!(
        !item.is_internable(),
        "an entry holding markup this crate does not model is not interchangeable with its text"
    );

    let run = table
        .item(2)
        .expect("the third entry")
        .runs()
        .next()
        .expect("its run");
    let properties = String::from_utf8(run.properties_markup().unwrap().to_vec()).unwrap();
    assert_eq!(
        properties, r#"<x:rPr  foo='bar'><x:b/><q:odd xmlns:q="urn:q"/></x:rPr>"#,
        "the doubled space, the single quotes and the foreign child are all the file's bytes"
    );
    let decoded = run.properties().unwrap().expect("an rPr");
    assert_eq!(decoded.bold, Some(true));
    assert_eq!(
        decoded.extra.len(),
        1,
        "the foreign child reaches the bucket"
    );
}

// -------------------------------------------------------------------------------------------
// Tier 2 — the values, read through the contract the cell store hands over
// -------------------------------------------------------------------------------------------

/// `sample.xlsx`'s five entries read back with their exact text, addressed the way a cell does.
#[test]
fn a_shared_string_cell_resolves_through_the_table_to_its_text() {
    let strings = part("sample.xlsx", "/xl/sharedStrings.xml");
    let (_document, table) = read(&strings);

    let sheet_bytes = part("sample.xlsx", "/xl/worksheets/sheet1.xml");
    let sheet_document =
        mjx_xml::fidelity::parse_shared(Arc::from(&sheet_bytes[..])).expect("the sheet parses");
    let sheet = SheetData::read_worksheet(&sheet_document)
        .expect("the sheet reads")
        .expect("it has a sheetData");

    // Row 1 of the fixture is the header: three `t="s"` cells naming entries 0, 1 and 2.
    let mut resolved = Vec::new();
    for cell in sheet.row(1).expect("row 1").cells() {
        assert_eq!(cell.cell_type(), CellType::SharedString);
        let index = cell
            .shared_string_index()
            .expect("a t=\"s\" cell names an index");
        let item = table.item(index).expect("the index is in range");
        resolved.push(item.text().expect("the text decodes").into_owned());
    }
    assert_eq!(resolved, ["name", "qty", "price"]);
    assert_eq!(
        table
            .items()
            .map(|item| item.text().unwrap().into_owned())
            .collect::<Vec<_>>(),
        ["name", "qty", "price", "widget", "gadget"]
    );
}

/// An index no entry answers for is reported as absence rather than repaired or panicked on.
#[test]
fn an_index_past_the_end_of_the_table_is_absence() {
    let strings = part(DISCRIMINATING_FIXTURE, "/xl/sharedStrings.xml");
    let (_document, table) = read(&strings);
    assert_eq!(table.len(), 7);
    assert!(table.item(7).is_none());
    assert!(
        table.item(99).is_none(),
        "the fixture's A4 holds 99 on purpose — a file may write an index the table has no entry \
         for, and that is read as absence"
    );
}

/// The rich-text entry decodes to its runs, their formatting and the string they spell.
#[test]
fn a_rich_text_entry_reads_as_its_runs_and_as_one_string() {
    let strings = part(DISCRIMINATING_FIXTURE, "/xl/sharedStrings.xml");
    let (_document, table) = read(&strings);
    let item = table.item(2).expect("the rich-text entry");

    assert_eq!(item.run_count(), 3);
    assert_eq!(
        item.text().unwrap(),
        "Bold and slanted",
        "a caller that only wants the string never has to know it came from runs"
    );
    assert_eq!(
        item.raw_text(),
        None,
        "an entry made of runs has no item-level `t` at all, which is not the same as an empty one"
    );

    let runs: Vec<_> = item.runs().collect();
    let first = runs[0].properties().unwrap().expect("the first run's rPr");
    assert_eq!(
        first,
        FontProperties {
            font_name: Some("Calibri".to_owned()),
            family: Some(2),
            bold: Some(true),
            color: Some(Color {
                rgb: Some("FFFF0000".to_owned()),
                ..Color::default()
            }),
            size_in_points: Some(11.0),
            scheme: Some(mjx_ooxml_types::spreadsheetml::FontScheme::Minor),
            ..FontProperties::default()
        }
    );
    assert!(
        runs[1].properties().unwrap().is_none(),
        "the middle run inherits its formatting and writes no rPr"
    );
    assert!(runs[1].preserves_space(), "` and ` is whitespace-delimited");

    let third = runs[2].properties().unwrap().expect("the third run's rPr");
    assert_eq!(third.italic, Some(true));
    assert_eq!(third.underline, Some(UnderlineType::Double));
    assert_eq!(
        third.vertical_position,
        Some(mjx_ooxml_types::shared::VerticalTextPosition::Superscript)
    );
    assert_eq!(third.character_set, Some(0));
}

/// Phonetic markup is decoded, not merely carried.
#[test]
fn an_east_asian_entry_reads_its_ruby_text_and_its_phonetic_properties() {
    let strings = part(DISCRIMINATING_FIXTURE, "/xl/sharedStrings.xml");
    let (_document, table) = read(&strings);
    let item = table.item(3).expect("the phonetic entry");

    assert_eq!(item.text().unwrap(), "東京");
    let phonetic: Vec<_> = item.phonetic_runs().collect();
    assert_eq!(phonetic.len(), 1);
    assert_eq!(phonetic[0].text().unwrap(), "とうきょう");
    assert_eq!(phonetic[0].start_base(), 0);
    assert_eq!(phonetic[0].end_base(), 2);

    let properties = item.phonetic_properties().expect("a phoneticPr");
    assert_eq!(properties.font_id, 1);
    assert_eq!(properties.script, PhoneticType::Hiragana);
    assert_eq!(properties.alignment, PhoneticAlignment::Center);
    assert!(
        !item.is_internable(),
        "reusing this entry for the plain string 東京 would give a cell ruby text nobody asked for"
    );
}

/// A `phoneticPr` that writes only its required attribute takes the schema's defaults for the rest.
#[test]
fn the_two_defaulted_phonetic_attributes_are_defaults_and_not_absences() {
    let (_document, table) = read(
        br#"<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><si><t>a</t><phoneticPr fontId="3"/></si></sst>"#,
    );
    let properties = table
        .item(0)
        .unwrap()
        .phonetic_properties()
        .expect("a phoneticPr");
    assert_eq!(properties.font_id, 3);
    assert_eq!(properties.script, PhoneticType::FullwidthKatakana);
    assert_eq!(properties.alignment, PhoneticAlignment::Left);
}

// -------------------------------------------------------------------------------------------
// Inline strings — the same value type, and each written back in its own form
// -------------------------------------------------------------------------------------------

/// A shared-string cell and an inline-string cell produce the same value type, and both read.
#[test]
fn a_shared_string_and_an_inline_string_read_to_the_same_value_type() {
    let strings = part(DISCRIMINATING_FIXTURE, "/xl/sharedStrings.xml");
    let (_document, table) = read(&strings);
    let sheet_bytes = part(DISCRIMINATING_FIXTURE, "/xl/worksheets/sheet1.xml");
    let sheet_document =
        mjx_xml::fidelity::parse_shared(Arc::from(&sheet_bytes[..])).expect("the sheet parses");
    let sheet = SheetData::read_worksheet(&sheet_document)
        .expect("the sheet reads")
        .expect("it has a sheetData");

    // Both are read into a `StringItem` and asked the same question, by the same function.
    fn describe(item: mjx_sml::StringItem<'_>) -> (String, usize) {
        (item.text().unwrap().into_owned(), item.run_count())
    }

    let shared = describe(
        table
            .item(
                sheet
                    .cell("A1".parse().unwrap())
                    .unwrap()
                    .shared_string_index()
                    .unwrap(),
            )
            .unwrap(),
    );
    assert_eq!(shared, ("Alpha".to_owned(), 0));

    let plain_inline_cell = sheet.cell("C1".parse().unwrap()).expect("C1");
    assert_eq!(plain_inline_cell.cell_type(), CellType::InlineString);
    let plain_inline = InlineString::parse(plain_inline_cell.inline_string_markup().unwrap())
        .expect("the inline string parses");
    assert_eq!(describe(plain_inline.item()), ("inline".to_owned(), 0));

    let rich_inline_cell = sheet.cell("C3".parse().unwrap()).expect("C3");
    let rich_inline = InlineString::parse(rich_inline_cell.inline_string_markup().unwrap())
        .expect("the inline string parses");
    assert_eq!(describe(rich_inline.item()), ("rich/inline".to_owned(), 2));
    assert_eq!(
        rich_inline
            .item()
            .runs()
            .next()
            .unwrap()
            .properties()
            .unwrap()
            .unwrap()
            .bold,
        Some(true),
        "an inline string's runs decode through the same rPr reader a shared string's do"
    );
}

/// A cell read as inline is written back as inline, and its `<is>` bytes are unchanged.
#[test]
fn an_inline_string_cell_is_written_back_in_its_original_form() {
    let sheet_bytes = part(DISCRIMINATING_FIXTURE, "/xl/worksheets/sheet1.xml");
    let sheet_document =
        mjx_xml::fidelity::parse_shared(Arc::from(&sheet_bytes[..])).expect("the sheet parses");
    let sheet = SheetData::read_worksheet(&sheet_document)
        .expect("the sheet reads")
        .expect("it has a sheetData");

    let cell = sheet.cell("C3".parse().unwrap()).expect("C3");
    let original = cell.inline_string_markup().expect("an <is>").to_vec();
    let inline = InlineString::parse(&original).expect("parses");
    assert_eq!(
        inline.markup(),
        original.as_slice(),
        "reading an inline string must not reflow it"
    );
    assert_eq!(
        cell.markup(),
        br#"<c r="C3" t="inlineStr"><is><r><rPr><b/></rPr><t>rich</t></r><r><t>/inline</t></r></is></c>"#
            .to_vec(),
        "the cell keeps t=\"inlineStr\" and its own `<is>`; nothing here moves it into the table"
    );
}

/// An authored inline string writes the same `<t>` an authored table entry does.
#[test]
fn an_authored_inline_string_and_an_authored_entry_write_the_same_text_element() {
    let inline = InlineString::plain("  padded  ").expect("authors");
    assert_eq!(
        inline.markup(),
        br#"<is><t xml:space="preserve">  padded  </t></is>"#,
    );
    let mut table = SharedStringTable::authored(None).expect("authors");
    table.push_plain_text("  padded  ").expect("appends");
    assert_eq!(
        table.item(0).unwrap().markup(),
        br#"<si><t xml:space="preserve">  padded  </t></si>"#,
    );
}

// -------------------------------------------------------------------------------------------
// Whitespace — the mutation target
// -------------------------------------------------------------------------------------------

/// `xml:space="preserve"` survives a read and a write, and is written where it is needed.
///
/// **The mutation this is written against:** make the writer drop `xml:space`. This case goes red
/// three times over — on the preserved attribute, on the authored one, and on the byte comparison.
#[test]
fn an_entry_that_needs_xml_space_keeps_it() {
    let strings = part(DISCRIMINATING_FIXTURE, "/xl/sharedStrings.xml");
    let (_document, table) = read(&strings);

    let padded = table.item(1).expect("the padded entry");
    assert_eq!(padded.text().unwrap(), "  padded  ");
    assert!(
        padded.preserves_space(),
        "the attribute itself, not merely the text it protects: without it a consumer may collapse \
         the whitespace and the value becomes \"padded\""
    );
    assert!(
        padded
            .markup()
            .windows(21)
            .any(|w| w == b"xml:space=\"preserve\"" as &[u8])
            || String::from_utf8_lossy(padded.markup()).contains("xml:space=\"preserve\""),
        "{}",
        String::from_utf8_lossy(padded.markup())
    );
    assert_eq!(
        String::from_utf8_lossy(&table.to_part_bytes()),
        String::from_utf8_lossy(&strings)
    );

    // And on the authoring side: written exactly when its absence would change the string.
    let mut authored = SharedStringTable::authored(None).expect("authors");
    authored.push_plain_text("total").expect("appends");
    authored.push_plain_text(" total ").expect("appends");
    authored.push_plain_text("one two").expect("appends");
    assert_eq!(authored.item(0).unwrap().markup(), b"<si><t>total</t></si>");
    assert_eq!(
        authored.item(1).unwrap().markup(),
        br#"<si><t xml:space="preserve"> total </t></si>"#
    );
    assert_eq!(
        authored.item(2).unwrap().markup(),
        b"<si><t>one two</t></si>",
        "whitespace inside the string is not at risk and must not gain an attribute"
    );
    for index in 0..3 {
        let item = authored.item(index).unwrap();
        assert_eq!(
            item.preserves_space(),
            index == 1,
            "entry {index} read back its own attribute state"
        );
    }
}

/// An edited entry gains the attribute when its new text needs it, and loses it when it does not.
#[test]
fn editing_an_entry_re_decides_whether_its_text_needs_preserving() {
    let (_document, mut table) = read(
        br#"<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><si><t>plain</t></si></sst>"#,
    );
    table.set_text(0, "  now padded  ").expect("edits");
    assert_eq!(
        table.item(0).unwrap().markup(),
        br#"<si><t xml:space="preserve">  now padded  </t></si>"#
    );
    assert!(table.item(0).unwrap().preserves_space());

    table.set_text(0, "plain again").expect("edits");
    assert_eq!(
        table.item(0).unwrap().markup(),
        b"<si><t>plain again</t></si>",
        "an attribute that is no longer needed is not kept for its own sake"
    );
}

// -------------------------------------------------------------------------------------------
// `count` / `uniqueCount` — the second mutation target
// -------------------------------------------------------------------------------------------

/// The two counts come back exactly as the file wrote them, disagreeing values and all.
///
/// **The mutation this is written against:** make the writer recompute `uniqueCount` from the entry
/// count unconditionally. The fixture says `uniqueCount="6"` over **seven** entries, so the rewrite
/// path produces `7` and the byte comparison here fails.
#[test]
fn the_counts_are_hints_and_survive_an_edit_to_an_entry() {
    let strings = part(DISCRIMINATING_FIXTURE, "/xl/sharedStrings.xml");
    let (_document, mut table) = read(&strings);

    assert_eq!(table.len(), 7);
    assert_eq!(
        table.unique_count(),
        Some(6),
        "the fixture's uniqueCount disagrees with its own entry count on purpose"
    );
    assert_eq!(
        table.reference_count(),
        Some(9),
        "and its count disagrees with its uniqueCount, which is what the attributes mean"
    );

    // An edit to an entry's text changes neither the number of entries nor the number of cells, so
    // neither hint may move.
    table.set_text(0, "Alpha edited").expect("edits");
    let written = String::from_utf8(table.to_part_bytes()).expect("utf-8");
    assert!(
        written.contains(r#"count="9" uniqueCount="6""#),
        "both hints must come back as read: {}",
        &written[..written.len().min(300)]
    );
    assert_eq!(table.unique_count(), Some(6));
    assert_eq!(table.reference_count(), Some(9));
}

/// Appending an entry *does* move `uniqueCount`, because the old value is then definitely wrong.
#[test]
fn appending_an_entry_recomputes_unique_count_and_never_count() {
    let strings = part(DISCRIMINATING_FIXTURE, "/xl/sharedStrings.xml");
    let (_document, mut table) = read(&strings);

    table.push_plain_text("appended").expect("appends");
    assert_eq!(table.len(), 8);
    assert_eq!(
        table.unique_count(),
        Some(8),
        "the entry count is the one thing the table can see, so it is the one thing it recomputes"
    );
    assert_eq!(
        table.reference_count(),
        Some(9),
        "adding a table entry adds no cell, so `count` is left exactly as the file wrote it"
    );
    let written = String::from_utf8(table.to_part_bytes()).expect("utf-8");
    assert!(
        written.contains(r#"count="9" uniqueCount="8""#),
        "{written}"
    );
}

/// A file that wrote no `uniqueCount` must not gain one.
#[test]
fn a_table_with_no_count_attributes_does_not_grow_them() {
    let (_document, mut table) = read(
        br#"<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><si><t>a</t></si></sst>"#,
    );
    assert_eq!(table.reference_count(), None);
    assert_eq!(table.unique_count(), None);
    table.push_plain_text("b").expect("appends");
    let written = String::from_utf8(table.to_part_bytes()).expect("utf-8");
    assert!(
        !written.contains("uniqueCount") && !written.contains("count="),
        "an attribute the file did not write must not appear: {written}"
    );
    assert!(written.contains("<si><t>b</t></si>"), "{written}");
}

/// `count` is settable by a caller that can see the cells, and by nothing else.
#[test]
fn only_an_explicit_call_writes_the_reference_count() {
    let strings = part(DISCRIMINATING_FIXTURE, "/xl/sharedStrings.xml");
    let (_document, mut table) = read(&strings);
    table.set_reference_count(Some(4));
    let written = String::from_utf8(table.to_part_bytes()).expect("utf-8");
    assert!(
        written.contains(r#"count="4" uniqueCount="6""#),
        "{written}"
    );

    table.set_reference_count(None);
    let written = String::from_utf8(table.to_part_bytes()).expect("utf-8");
    assert!(!written.contains("count=\"4\""), "{written}");
    assert!(written.contains(r#"uniqueCount="6""#), "{written}");
}

// -------------------------------------------------------------------------------------------
// Tier 3 — an edit leaves everything else byte-identical
// -------------------------------------------------------------------------------------------

/// Editing one entry's text leaves every other entry byte-identical, unreferenced ones included.
#[test]
fn editing_one_entry_leaves_every_other_entry_untouched() {
    let strings = part(DISCRIMINATING_FIXTURE, "/xl/sharedStrings.xml");
    let (_document, before) = read(&strings);
    let originals: Vec<Vec<u8>> = before.items().map(|item| item.markup().to_vec()).collect();
    drop(before);

    let (_document, mut table) = read(&strings);
    table.set_text(0, "Alpha edited").expect("edits");

    for (index, original) in originals.iter().enumerate() {
        let item = table.item(index as u32).expect("the entry is still there");
        if index == 0 {
            assert_ne!(item.markup(), original.as_slice(), "entry 0 was the edit");
            continue;
        }
        assert_eq!(
            String::from_utf8_lossy(item.markup()),
            String::from_utf8_lossy(original),
            "entry {index} changed while an unrelated entry was edited"
        );
    }
    // Entry 4 is referenced by no cell in the fixture and is asserted by name, because "every other
    // entry" is exactly where an eager compaction would show up.
    assert_eq!(
        table.item(4).unwrap().text().unwrap(),
        "never referenced",
        "an entry nothing points at is not garbage this table collects"
    );
    assert_eq!(table.len(), 7);
}

/// Editing one run's text leaves its own `rPr`, its siblings and the whole rest of the entry alone.
#[test]
fn editing_one_run_splices_and_does_not_rebuild() {
    let strings = part(DISCRIMINATING_FIXTURE, "/xl/sharedStrings.xml");
    let (_document, mut table) = read(&strings);
    let before: Vec<Vec<u8>> = table
        .item(2)
        .unwrap()
        .runs()
        .map(|run| run.markup().to_vec())
        .collect();

    table.set_run_text(2, 0, "Heavy").expect("edits");

    let item = table.item(2).unwrap();
    assert_eq!(item.text().unwrap(), "Heavy and slanted");
    let after: Vec<Vec<u8>> = item.runs().map(|run| run.markup().to_vec()).collect();
    assert_eq!(
        String::from_utf8_lossy(&after[1]),
        String::from_utf8_lossy(&before[1])
    );
    assert_eq!(
        String::from_utf8_lossy(&after[2]),
        String::from_utf8_lossy(&before[2])
    );
    assert_eq!(
        String::from_utf8_lossy(item.runs().next().unwrap().properties_markup().unwrap()),
        r#"<rPr><sz val="11"/><color rgb="FFFF0000"/><rFont val="Calibri"/><b/><family val="2"/><scheme val="minor"/></rPr>"#,
        "the run's own rPr — including its non-canonical child order — is untouched by an edit to \
         the text beside it"
    );
}

/// Editing an entry that carries phonetic markup keeps the ruby text.
#[test]
fn editing_the_text_of_a_phonetic_entry_keeps_its_ruby_markup() {
    let strings = part(DISCRIMINATING_FIXTURE, "/xl/sharedStrings.xml");
    let (_document, mut table) = read(&strings);
    table.set_text(3, "京都").expect("edits");

    let item = table.item(3).unwrap();
    assert_eq!(item.text().unwrap(), "京都");
    assert_eq!(item.phonetic_runs().len(), 1);
    assert_eq!(
        item.phonetic_runs().next().unwrap().text().unwrap(),
        "とうきょう",
        "a splice replaces the `t` and nothing else"
    );
    assert_eq!(
        String::from_utf8_lossy(item.phonetic_properties_markup().unwrap()),
        r#"<phoneticPr fontId="1" type="Hiragana" alignment="center"/>"#
    );
}

// -------------------------------------------------------------------------------------------
// Interning
// -------------------------------------------------------------------------------------------

/// Interning reuses the first plain entry holding the text, and appends when there is none.
#[test]
fn interning_reuses_the_first_plain_entry_and_appends_otherwise() {
    let strings = part(DISCRIMINATING_FIXTURE, "/xl/sharedStrings.xml");
    let (_document, mut table) = read(&strings);

    assert_eq!(
        table.intern("Alpha").expect("interns"),
        0,
        "`Alpha` is at 0 and again at 6; first use wins, as a producer's own writer would answer"
    );
    assert_eq!(table.intern("  padded  ").expect("interns"), 1);
    assert_eq!(
        table.intern("").expect("interns"),
        5,
        "the empty entry is a value, not an absence"
    );
    assert_eq!(table.len(), 7, "nothing was appended");

    assert_eq!(table.intern("brand new").expect("interns"), 7);
    assert_eq!(table.len(), 8);
    assert_eq!(
        table.intern("brand new").expect("interns"),
        7,
        "the entry it just appended is the entry it now finds"
    );
    assert_eq!(
        table.item(7).unwrap().markup(),
        b"<si><t>brand new</t></si>"
    );
}

/// An entry that is not interchangeable with its text is never reused for it.
#[test]
fn a_rich_or_phonetic_entry_is_never_reused_for_a_plain_string() {
    let strings = part(DISCRIMINATING_FIXTURE, "/xl/sharedStrings.xml");
    let (_document, mut table) = read(&strings);

    let rich = table.intern("Bold and slanted").expect("interns");
    assert_eq!(
        rich, 7,
        "entry 2 says those characters, but it says them in three formatted runs; a plain string \
         must get an entry of its own rather than inherit the formatting"
    );
    let phonetic = table.intern("東京").expect("interns");
    assert_eq!(
        phonetic, 8,
        "entry 3 says 東京 with ruby text above it, which is not the same value"
    );
    assert_eq!(
        table.item(7).unwrap().markup(),
        b"<si><t>Bold and slanted</t></si>"
    );
}

/// Interning matches on the decoded text, exactly.
#[test]
fn interning_compares_decoded_text_and_does_not_trim() {
    let (_document, mut table) = read(
        br#"<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><si><t>a &amp; b</t></si><si><t xml:space="preserve"> spaced </t></si></sst>"#,
    );
    assert_eq!(
        table.intern("a & b").expect("interns"),
        0,
        "the escape is not part of the value"
    );
    assert_eq!(table.intern(" spaced ").expect("interns"), 1);
    assert_eq!(
        table.intern("spaced").expect("interns"),
        2,
        "trimming would silently merge two different strings into one entry"
    );
    assert_eq!(table.index_of("a & b").expect("looks up"), Some(0));
    assert_eq!(table.index_of("absent").expect("looks up"), None);
    assert_eq!(table.len(), 3, "index_of appends nothing");
}

/// An edit that changes an entry's text is visible to the next `intern`.
#[test]
fn the_interning_index_is_dropped_by_an_edit_that_could_invalidate_it() {
    let (_document, mut table) = read(
        br#"<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><si><t>before</t></si></sst>"#,
    );
    assert_eq!(table.intern("before").expect("interns"), 0);
    table.set_text(0, "after").expect("edits");
    assert_eq!(
        table.intern("after").expect("interns"),
        0,
        "the entry now says `after`, and interning must see that"
    );
    assert_eq!(
        table.intern("before").expect("interns"),
        1,
        "and must no longer answer 0 for the text that entry used to hold"
    );
}

// -------------------------------------------------------------------------------------------
// Authoring, and parity with `mjx-chart`'s writer
// -------------------------------------------------------------------------------------------

/// An authored table is byte-identical to what `mjx-chart`'s own interner writes.
///
/// **This is the pinned half of MJXOFF-112's parity gate.** `mjx-chart` sits *above* `mjx-sml` and
/// cannot be reached from here, so the expected bytes are the literal its
/// `SharedStrings::to_part_bytes` produces for its own `bar_chart` fixture — verified against that
/// writer's live output while this child was written, and asserted in
/// `crates/mjx-chart/tests/workbook.rs` from the other side. MJXOFF-112 replaces this literal with
/// the real comparison, and MJXOFF-99 then deletes the writer it is pinned to.
#[test]
fn an_authored_table_matches_the_chart_writers_bytes_exactly() {
    let mut table = SharedStringTable::authored(None).expect("authors");
    for label in ["Sales", "Costs", "North", "South", "West", "Sales"] {
        table.intern(label).expect("interns");
    }
    assert_eq!(
        String::from_utf8(table.to_part_bytes()).expect("utf-8"),
        concat!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n",
            "<sst xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" ",
            "count=\"5\" uniqueCount=\"5\">",
            "<si><t>Sales</t></si><si><t>Costs</t></si><si><t>North</t></si>",
            "<si><t>South</t></si><si><t>West</t></si></sst>"
        )
    );
}

/// An authored table reads back through the same reader a file does.
#[test]
fn an_authored_table_reopens_as_itself() {
    let mut table = SharedStringTable::authored(None).expect("authors");
    table.intern("  padded  ").expect("interns");
    table
        .push_rich_text(&[
            RichTextRunSpec {
                text: "Bold".to_owned(),
                properties: Some(FontProperties {
                    bold: Some(true),
                    size_in_points: Some(11.0),
                    color: Some(Color::from_opaque_rgb("FF0000")),
                    ..FontProperties::default()
                }),
            },
            RichTextRunSpec {
                text: " and plain".to_owned(),
                properties: None,
            },
        ])
        .expect("appends");

    let bytes = table.to_part_bytes();
    let (_document, reread) = read(&bytes);
    assert_eq!(reread.len(), 2);
    assert_eq!(reread.item(0).unwrap().text().unwrap(), "  padded  ");
    assert!(reread.item(0).unwrap().preserves_space());
    assert_eq!(reread.item(1).unwrap().text().unwrap(), "Bold and plain");
    assert_eq!(
        reread
            .item(1)
            .unwrap()
            .runs()
            .next()
            .unwrap()
            .properties()
            .unwrap(),
        Some(FontProperties {
            bold: Some(true),
            size_in_points: Some(11.0),
            color: Some(Color::from_opaque_rgb("FF0000")),
            ..FontProperties::default()
        })
    );
    assert_eq!(
        reread.to_part_bytes(),
        bytes,
        "and re-emits byte for byte, which is the same contract a read file gets"
    );
    assert_eq!(reread.unique_count(), Some(2));
}

// -------------------------------------------------------------------------------------------
// Entry lifetime
// -------------------------------------------------------------------------------------------

/// `compact` renumbers, says so, and is the only thing that ever does.
#[test]
fn compaction_returns_the_remapping_the_caller_must_apply() {
    let strings = part(DISCRIMINATING_FIXTURE, "/xl/sharedStrings.xml");
    let (_document, mut table) = read(&strings);

    // The fixture's cells reference 0, 1, 2, 3, 5, 6 and 99; entry 4 is referenced by nothing.
    let referenced = [true, true, true, true, false, true, true];
    let mapping = table.compact(|index| referenced[index as usize]);

    assert_eq!(
        mapping,
        vec![Some(0), Some(1), Some(2), Some(3), None, Some(4), Some(5)]
    );
    assert_eq!(table.len(), 6);
    assert_eq!(
        table.item(4).unwrap().text().unwrap(),
        "",
        "what was entry 5 is now entry 4 — which is exactly why every referencing cell has to be \
         rewritten through the returned map"
    );
    assert_eq!(
        table.unique_count(),
        Some(6),
        "recomputed, because entries went"
    );
    assert_eq!(table.reference_count(), Some(9), "still never derived");
}

/// Nothing compacts on its own: an entry left unreferenced by an edit stays where it is.
#[test]
fn an_edit_never_renumbers() {
    let strings = part(DISCRIMINATING_FIXTURE, "/xl/sharedStrings.xml");
    let (_document, mut table) = read(&strings);
    let before: Vec<String> = table
        .items()
        .map(|item| item.text().unwrap().into_owned())
        .collect();

    table
        .set_text(6, "no longer a duplicate of entry 0")
        .expect("edits");
    table.intern("something new").expect("interns");

    let after: Vec<String> = table
        .items()
        .map(|item| item.text().unwrap().into_owned())
        .collect();
    assert_eq!(after.len(), before.len() + 1);
    for index in 0..before.len() {
        if index == 6 {
            continue;
        }
        assert_eq!(
            after[index], before[index],
            "entry {index} must still mean what every cell holding {index} says it means"
        );
    }
}

// -------------------------------------------------------------------------------------------
// Untrusted input
// -------------------------------------------------------------------------------------------

/// Markup a file can legally write, and nonsense it can write too, is read rather than refused.
#[test]
fn nothing_a_well_formed_file_can_say_is_refused() {
    for markup in [
        // A self-closing table.
        &br#"<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"/>"#[..],
        // An `si` with no `t` and no runs at all.
        &br#"<sst xmlns="urn:x"><si/></sst>"#[..],
        // An `rPh` missing its required attributes, and one whose values are not numbers.
        &br#"<sst xmlns="urn:x"><si><t>x</t><rPh><t>y</t></rPh><rPh sb="a" eb="b"><t>z</t></rPh></si></sst>"#[..],
        // Counts that are not `xsd:unsignedInt`.
        &br#"<sst xmlns="urn:x" count="lots" uniqueCount="-1"><si><t>x</t></si></sst>"#[..],
        // An `extLst` after the entries, which the schema allows and this crate does not model.
        &br#"<sst xmlns="urn:x"><si><t>x</t></si><extLst><ext uri="{X}"><q:k xmlns:q="urn:q"/></ext></extLst></sst>"#[..],
        // A `phoneticPr` with no `fontId`, which is `use="required"`.
        &br#"<sst xmlns="urn:x"><si><t>x</t><phoneticPr/></si></sst>"#[..],
    ] {
        let (_document, table) = read(markup);
        assert_eq!(
            String::from_utf8_lossy(&table.to_part_bytes()),
            String::from_utf8_lossy(markup),
            "every one of these must round-trip rather than be repaired"
        );
    }

    // The unreadable hints are read as absent and written back from the file's own bytes.
    let (_document, table) =
        read(br#"<sst xmlns="urn:x" count="lots" uniqueCount="-1"><si><t>x</t></si></sst>"#);
    assert_eq!(table.reference_count(), None);
    assert_eq!(table.unique_count(), None);

    let (_document, table) =
        read(br#"<sst xmlns="urn:x"><si><t>x</t><rPh><t>y</t></rPh><rPh sb="a" eb="b"><t>z</t></rPh></si></sst>"#);
    let item = table.item(0).unwrap();
    assert_eq!(item.phonetic_runs().len(), 2);
    assert_eq!(item.phonetic_runs().next().unwrap().start_base(), 0);
}

/// A table with no source bytes behind it — read from a tree somebody edited — still round-trips.
#[test]
fn a_table_read_from_a_tree_with_no_byte_ranges_is_read_and_written_from_the_model() {
    let markup = br#"<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="2" uniqueCount="2"><si><t>a</t></si><si><r><rPr><b/></rPr><t>b</t></r></si></sst>"#;
    let mut document = mjx_xml::fidelity::parse(markup).expect("parses");
    document.release_source();

    let table = SharedStringTable::read_part(&document)
        .expect("reads")
        .expect("an sst");
    assert_eq!(table.len(), 2);
    assert_eq!(table.item(0).unwrap().text().unwrap(), "a");
    assert_eq!(table.item(1).unwrap().text().unwrap(), "b");
    assert_eq!(
        table
            .item(1)
            .unwrap()
            .runs()
            .next()
            .unwrap()
            .properties()
            .unwrap()
            .unwrap()
            .bold,
        Some(true)
    );
    assert_eq!(
        String::from_utf8_lossy(&table.to_part_bytes()),
        String::from_utf8_lossy(&markup[..]),
        "the model path reaches the same answer the slow way"
    );
}

/// A document whose root is not an `sst` is a question, not an error.
#[test]
fn a_part_that_is_not_a_string_table_reads_as_none() {
    let document = mjx_xml::fidelity::parse(br#"<worksheet xmlns="urn:x"/>"#).expect("parses");
    assert!(SharedStringTable::read_part(&document)
        .expect("no error")
        .is_none());
}
