//! Structured content: content controls (`w:sdt`), custom XML, data binding, `w:altChunk` and the
//! glossary document's building blocks (MJXOFF-138).
//!
//! `structured_content.docx` is authored, not templated — hand-written `<w:sdt>`/`<w:customXml>`
//! markup spliced into a blank document's own `word/document.xml`, plus a hand-authored
//! `word/glossary/document.xml` and a `customXml/item1.xml` + `customXml/itemProps1.xml` pair, the
//! same technique `tests/tables.rs`'s `ragged_table.docx` uses (`regenerate_fixtures`, `#[ignore]`,
//! below).
//!
//! # The four-placement fixture, by construction
//!
//! A block-level content control (`w:sdt`, alias "Outer Block Control") wraps a two-column table.
//! Its first row is wrapped in a `w:customXml` region (alias-free — custom XML has no alias); that
//! row's second physical member is **not** a plain `w:tc` but a cell-level content control (alias
//! "Cell Control") wrapping one, whose own paragraph holds a run-level content control (alias "Run
//! Control", carrying a `w:dataBinding` and a `w14:checkbox` extension inside its own `w:sdtPr` to
//! prove both round-trip) wrapping the text `INNERMOST`, sandwiched between two ordinary runs. A
//! second, row-level content control (alias "Repeating Section", carrying a bare `w15:repeatingSection`
//! extension element) wraps **two** `w:tr` in its own `w:sdtContent` — a repeating section with two
//! instances:
//!
//! ```text
//! w:sdt (block, "Outer Block Control")
//!   w:tbl (2 columns)
//!     w:customXml (row 0)
//!       w:tr
//!         w:tc                                    "R0C0"
//!         w:sdt (cell, "Cell Control", w14:checkbox)
//!           w:tc
//!             w:p: "before-" + w:sdt (run, "Run Control", w:dataBinding, w:text) ["INNERMOST"] + "-after"
//!     w:sdt (row, "Repeating Section", w15:repeatingSection)
//!       w:tr  "Rep1C0" | "Rep1C1"
//!       w:tr  "Rep2C0" | "Rep2C1"
//! ```
//!
//! Three physical rows total (one reached through the `w:customXml` wrapper, two through the `w:sdt`
//! wrapper), each with exactly two grid columns — [`Table::grid_discrepancies`] reports the fixture
//! clean, so a reader is free to trust `(row, column)` addressing on it without first excusing a
//! malformed grid.

use mjx_docx::{BlockContent, BuildingBlock, Document, DocxError, ParagraphContent};
use mjx_fixtures::fixture;
use mjx_ooxml_core::FromXml;
use mjx_opc::{Package, PartName, Relationship, TargetMode};
use mjx_schema_gate::{assert_fixture_is_schema_valid, inspect_fixture, PartOutcome};

/// The `storeItemID` `structured_content.docx`'s own Run Control binds to, and the id
/// `customXml/itemProps1.xml` states.
const STORE_ITEM_ID: &str = "{11111111-1111-1111-1111-111111111111}";
/// The xpath `structured_content.docx`'s own Run Control binds to — `customXml/item1.xml`'s
/// `ns0:customer/ns0:name`.
const XPATH: &str = "/ns0:customer[1]/ns0:name[1]";

/// `word/document.xml`'s own body content — see this module's own doc comment for the shape.
fn document_body_xml() -> &'static str {
    r#"<w:sdt>
  <w:sdtPr><w:alias w:val="Outer Block Control"/><w:tag w:val="outer"/><w:id w:val="1"/></w:sdtPr>
  <w:sdtContent>
    <w:tbl>
      <w:tblPr/>
      <w:tblGrid><w:gridCol w:w="2000"/><w:gridCol w:w="2000"/></w:tblGrid>
      <w:customXml w:element="introRow">
        <w:tr>
          <w:tc><w:p><w:r><w:t>R0C0</w:t></w:r></w:p></w:tc>
          <w:sdt>
            <w:sdtPr>
              <w:alias w:val="Cell Control"/>
              <w:id w:val="2"/>
              <w14:checkbox><w14:checked w14:val="0"/></w14:checkbox>
            </w:sdtPr>
            <w:sdtContent>
              <w:tc>
                <w:p>
                  <w:r><w:t>before-</w:t></w:r>
                  <w:sdt>
                    <w:sdtPr>
                      <w:alias w:val="Run Control"/>
                      <w:id w:val="3"/>
                      <w:dataBinding w:prefixMappings="xmlns:ns0='http://schemas.example.com/customer'" w:xpath="/ns0:customer[1]/ns0:name[1]" w:storeItemID="{11111111-1111-1111-1111-111111111111}"/>
                      <w:text/>
                    </w:sdtPr>
                    <w:sdtContent><w:r><w:t>INNERMOST</w:t></w:r></w:sdtContent>
                  </w:sdt>
                  <w:r><w:t>-after</w:t></w:r>
                </w:p>
              </w:tc>
            </w:sdtContent>
          </w:sdt>
        </w:tr>
      </w:customXml>
      <w:sdt>
        <w:sdtPr>
          <w:alias w:val="Repeating Section"/>
          <w:id w:val="4"/>
          <w15:repeatingSection/>
        </w:sdtPr>
        <w:sdtContent>
          <w:tr><w:tc><w:p><w:r><w:t>Rep1C0</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>Rep1C1</w:t></w:r></w:p></w:tc></w:tr>
          <w:tr><w:tc><w:p><w:r><w:t>Rep2C0</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>Rep2C1</w:t></w:r></w:p></w:tc></w:tr>
        </w:sdtContent>
      </w:sdt>
    </w:tbl>
  </w:sdtContent>
</w:sdt>
<w:sectPr>
  <w:pgSz w:w="11906" w:h="16838"/>
  <w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="720" w:footer="720" w:gutter="0"/>
</w:sectPr>"#
}

fn document_xml() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml" xmlns:w15="http://schemas.microsoft.com/office/word/2012/wordml" mc:Ignorable="w14 w15"><w:body>{body}</w:body></w:document>"#,
        body = document_body_xml(),
    )
}

fn glossary_document_xml() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:glossaryDocument xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:docParts><w:docPart><w:docPartPr><w:name w:val="Cover Page 1"/><w:category><w:name w:val="Cover Pages"/><w:gallery w:val="coverPg"/></w:category><w:behaviors><w:behavior w:val="p"/></w:behaviors><w:types><w:type w:val="bbPlcHdr"/></w:types></w:docPartPr><w:docPartBody><w:p><w:r><w:t>Cover page building block text</w:t></w:r></w:p></w:docPartBody></w:docPart></w:docParts></w:glossaryDocument>"#
}

fn custom_xml_item_xml() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<ns0:customer xmlns:ns0="http://schemas.example.com/customer"><ns0:name>Jane Doe</ns0:name></ns0:customer>"#
}

fn custom_xml_item_props_xml() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<ds:datastoreItem xmlns:ds="http://schemas.openxmlformats.org/officeDocument/2006/customXml" ds:itemID="{STORE_ITEM_ID}"/>"#
    )
}

/// Builds `structured_content.docx`: a blank A4 document whose `word/document.xml` is replaced with
/// [`document_xml`], plus a glossary document part and a Custom XML Data Storage part pair.
fn build_fixture() -> Vec<u8> {
    let document = Document::blank(mjx_docx::PageSize::a4()).expect("blank");
    let blank_bytes = document.save().expect("save blank document");
    let mut package = Package::open(&blank_bytes).expect("open blank document");

    let document_part = PartName::new("/word/document.xml").expect("valid part name");
    package
        .replace_part_bytes(&document_part, document_xml().into_bytes())
        .expect("replace word/document.xml");

    // The glossary document part.
    let glossary_part = PartName::new("/word/glossary/document.xml").expect("valid part name");
    package
        .insert_part(
            &glossary_part,
            mjx_docx::constants::CONTENT_TYPE_GLOSSARY_DOCUMENT,
            glossary_document_xml().as_bytes().to_vec(),
        )
        .expect("insert glossary document part");
    package
        .add_relationship(
            Some(&document_part),
            Relationship {
                id: "rIdGlossary".to_owned(),
                rel_type: mjx_docx::constants::REL_GLOSSARY_DOCUMENT.to_owned(),
                target: "glossary/document.xml".to_owned(),
                mode: TargetMode::Internal,
            },
        )
        .expect("relate glossary document part");

    // The Custom XML Data Storage part and its own properties part.
    let item_part = PartName::new("/customXml/item1.xml").expect("valid part name");
    package
        .insert_part(
            &item_part,
            mjx_docx::constants::CONTENT_TYPE_CUSTOM_XML_DATA,
            custom_xml_item_xml().as_bytes().to_vec(),
        )
        .expect("insert customXml/item1.xml");
    package
        .add_relationship(
            Some(&document_part),
            Relationship {
                id: "rIdCustomXmlData1".to_owned(),
                rel_type: mjx_docx::constants::REL_CUSTOM_XML_DATA.to_owned(),
                target: "../customXml/item1.xml".to_owned(),
                mode: TargetMode::Internal,
            },
        )
        .expect("relate customXml/item1.xml");

    let item_props_part = PartName::new("/customXml/itemProps1.xml").expect("valid part name");
    package
        .insert_part(
            &item_props_part,
            mjx_docx::constants::CONTENT_TYPE_CUSTOM_XML_PROPS,
            custom_xml_item_props_xml().into_bytes(),
        )
        .expect("insert customXml/itemProps1.xml");
    package
        .add_relationship(
            Some(&item_part),
            Relationship {
                id: "rIdCustomXmlProps1".to_owned(),
                rel_type: mjx_docx::constants::REL_CUSTOM_XML_PROPS.to_owned(),
                target: "itemProps1.xml".to_owned(),
                mode: TargetMode::Internal,
            },
        )
        .expect("relate customXml/itemProps1.xml");

    package.save().expect("save structured_content.docx")
}

#[test]
#[ignore = "one-shot generator for the committed fixture; run manually with --ignored"]
fn regenerate_fixtures() {
    std::fs::write(
        mjx_fixtures::fixtures_dir().join("structured_content.docx"),
        build_fixture(),
    )
    .expect("write structured_content.docx");
}

fn open_fixture() -> Document {
    Document::open(&fixture("structured_content.docx")).expect("open structured_content.docx")
}

// =================================================================================================
// The four-placement nested fixture — MJXOFF-92's paragraph/run APIs and MJXOFF-116's row/cell
// addressing reach the innermost text through all four wrapper levels.
// =================================================================================================

/// Walks `structured_content.docx`'s own `word/document.xml` down to the block-level control's
/// wrapped table — the shared entry point every test below starts from.
fn open_table() -> mjx_docx::Table {
    let bytes = fixture("structured_content.docx");
    let mut package = Package::open(&bytes).expect("open structured_content.docx");
    let part = PartName::new("/word/document.xml").expect("valid part name");
    let doc = package.part_tree(&part).expect("read word/document.xml");
    let main =
        mjx_docx::MainDocument::from_xml(&doc.root, &doc.interner).expect("parse main document");
    let body = main.body().expect("body");
    let outer = body
        .content()
        .iter()
        .find_map(|item| match item {
            BlockContent::StructuredDocumentTag(control) => Some(control),
            _ => None,
        })
        .expect("outer block-level content control");
    outer
        .content_block()
        .expect("outer control has content")
        .content()
        .iter()
        .find_map(|item| match item {
            BlockContent::Table(table) => Some(table.clone()),
            _ => None,
        })
        .expect("table inside the outer block-level control")
}

#[test]
fn the_wrapped_table_has_no_grid_discrepancies() {
    let bytes = fixture("structured_content.docx");
    let mut package = Package::open(&bytes).expect("open");
    let part = PartName::new("/word/document.xml").expect("valid part name");
    let doc = package.part_tree(&part).expect("read");
    let table = open_table();
    assert_eq!(
        table.grid_discrepancies(&doc.interner),
        Vec::new(),
        "the fixture's own grid must be clean so (row, column) tests are not excusing a malformed one"
    );
}

#[test]
fn table_rows_sees_through_both_the_custom_xml_and_the_content_control_row_wrapper() {
    let table = open_table();
    // Row 0 reached only through the w:customXml wrapper; rows 1-2 reached only through the
    // repeating-section w:sdt wrapper, which wraps *two* w:tr in one w:sdtContent — the fixture's own
    // "repeating-section control with two instances."
    assert_eq!(
        table.row_count(),
        3,
        "Table::rows must see through both wrapper kinds"
    );
}

#[test]
fn a_run_level_content_control_nested_four_deep_reaches_its_innermost_text() {
    let bytes = fixture("structured_content.docx");
    let mut package = Package::open(&bytes).expect("open");
    let part = PartName::new("/word/document.xml").expect("valid part name");
    let doc = package.part_tree(&part).expect("read");
    let table = open_table();

    // Row 0, reached through the w:customXml wrapper.
    let row0 = table.row(0).expect("row 0");
    assert_eq!(
        row0.cell_count(),
        2,
        "Row::cells must see through the cell-level wrapper too"
    );
    assert_eq!(row0.cell(0).expect("cell 0").text(), "R0C0");

    // Cell 1 is not a plain w:tc — it is wrapped in a cell-level content control, reached only
    // through Row::cells()'s own recursion.
    let cell1 = row0
        .cell(1)
        .expect("cell 1, reached through the cell-level wrapper");
    let paragraph = cell1
        .paragraph(0)
        .expect("the wrapped cell's own paragraph");

    // The run-level control is one ParagraphContent item among ordinary runs.
    let run_control = paragraph
        .content()
        .iter()
        .find_map(|item| match item {
            ParagraphContent::StructuredDocumentTag(control) => Some(control),
            _ => None,
        })
        .expect("run-level content control");
    let inner_text = run_control
        .content_run()
        .expect("run-level control has content")
        .content()
        .iter()
        .find_map(|item| match item {
            ParagraphContent::Run(run) => Some(run.text()),
            _ => None,
        })
        .expect("run inside the run-level control");
    assert_eq!(inner_text, "INNERMOST");

    // Paragraph::text() — MJXOFF-92's own simplest reading API — must reach the same text without
    // the caller navigating the wrapper chain by hand: "before-" + "INNERMOST" + "-after".
    assert_eq!(paragraph.text(), "before-INNERMOST-after");

    // Paragraph::run(RunPath) must also descend through the run-level wrapper, exactly as it already
    // descends through a w:hyperlink — a depth-2 RunPath: slot 1 is the wrapper, index 0 inside it.
    let run_index = paragraph
        .content()
        .iter()
        .filter(|item| {
            matches!(
                item,
                ParagraphContent::Run(_) | ParagraphContent::StructuredDocumentTag(_)
            )
        })
        .position(|item| matches!(item, ParagraphContent::StructuredDocumentTag(_)))
        .expect("the run-level control occupies a top-level run-addressing slot");
    let addressed = paragraph
        .run([run_index, 0])
        .expect("RunPath must descend into the run-level content control");
    assert_eq!(addressed.text(), "INNERMOST");

    // The w14:checkbox extension inside the cell-level control's own w:sdtPr, and the w:dataBinding
    // plus w15:repeatingSection extension elsewhere, all round-trip byte-identically (proved by the
    // whole-document round-trip test below); confirm the run-level control's own dataBinding is
    // reachable here too.
    let binding = run_control
        .properties()
        .expect("run-level control has properties")
        .data_binding()
        .expect("run-level control has a data binding");
    assert_eq!(
        binding.store_item_id(&doc.interner).expect("storeItemID"),
        STORE_ITEM_ID
    );
    assert_eq!(binding.xpath(&doc.interner).expect("xpath"), XPATH);
}

#[test]
fn the_repeating_section_control_wraps_two_row_instances_with_correct_cell_text() {
    let table = open_table();
    let row1 = table.row(1).expect("row 1 (Rep1)");
    let row2 = table.row(2).expect("row 2 (Rep2)");
    assert_eq!(row1.cell(0).expect("cell").text(), "Rep1C0");
    assert_eq!(row1.cell(1).expect("cell").text(), "Rep1C1");
    assert_eq!(row2.cell(0).expect("cell").text(), "Rep2C0");
    assert_eq!(row2.cell(1).expect("cell").text(), "Rep2C1");
}

// =================================================================================================
// C11's (row, column) mapping is still correct for a table whose row is wrapped in a w:sdt/w:customXml
// =================================================================================================

#[test]
fn row_column_cell_addressing_is_correct_across_both_wrapper_kinds() {
    let bytes = fixture("structured_content.docx");
    let mut package = Package::open(&bytes).expect("open");
    let part = PartName::new("/word/document.xml").expect("valid part name");
    let doc = package.part_tree(&part).expect("read");
    let table = open_table();

    // (row, column) addresses every physical cell correctly, including the cell-level wrapper's own
    // wrapped cell at (0, 1) and every cell of the two rows the repeating-section control wraps.
    assert_eq!(
        table.cell(&doc.interner, 0, 0).expect("cell").text(),
        "R0C0"
    );
    let cell01 = table
        .cell(&doc.interner, 0, 1)
        .expect("cell (0,1), reached through the cell-level wrapper");
    assert_eq!(
        cell01.paragraph(0).expect("paragraph").text(),
        "before-INNERMOST-after"
    );
    assert_eq!(
        table.cell(&doc.interner, 1, 0).expect("cell").text(),
        "Rep1C0"
    );
    assert_eq!(
        table.cell(&doc.interner, 1, 1).expect("cell").text(),
        "Rep1C1"
    );
    assert_eq!(
        table.cell(&doc.interner, 2, 0).expect("cell").text(),
        "Rep2C0"
    );
    assert_eq!(
        table.cell(&doc.interner, 2, 1).expect("cell").text(),
        "Rep2C1"
    );
    assert_eq!(
        table.cell(&doc.interner, 3, 0),
        None,
        "the table has only three rows"
    );
}

// =================================================================================================
// Round trip — the whole fixture, untouched, byte-identical (proves w14:/w15: extensions and the
// w:dataBinding survive, alongside every content-control/custom-XML wrapper).
// =================================================================================================

#[test]
fn structured_content_docx_round_trips_byte_identically_when_untouched() {
    let bytes = fixture("structured_content.docx");
    let package = Package::open(&bytes).expect("open");
    let part = PartName::new("/word/document.xml").expect("valid part name");
    let before = package
        .part_bytes(&part)
        .expect("word/document.xml bytes")
        .to_vec();
    let saved = package.save().expect("save untouched");
    let reopened = Package::open(&saved).expect("reopen");
    let after = reopened
        .part_bytes(&part)
        .expect("word/document.xml bytes")
        .to_vec();
    assert_eq!(
        before, after,
        "word/document.xml must round-trip byte-identically"
    );
}

#[test]
fn structured_content_docx_is_schema_valid() {
    assert_fixture_is_schema_valid("structured_content.docx");
}

#[test]
fn the_custom_xml_data_storage_parts_are_classified_as_preserved_foreign_markup() {
    let rows = inspect_fixture("structured_content.docx");
    if rows.is_empty() {
        return;
    }
    for part in ["/customXml/item1.xml", "/customXml/itemProps1.xml"] {
        let row = rows
            .iter()
            .find(|row| row.name == part)
            .unwrap_or_else(|| panic!("structured_content.docx: {part} is not in the sweep"));
        assert!(
            matches!(row.outcome, PartOutcome::SkippedPreservedForeign { .. }),
            "{part} must be classified as preserved foreign markup, not validated; it reported: {}",
            row.outcome.describe()
        );
    }
}

// =================================================================================================
// A data binding is a two-part reference: it resolves to the right custom XML part and node, and a
// binding naming a missing part reports rather than panics.
// =================================================================================================

#[test]
fn a_data_binding_resolves_to_its_custom_xml_part_and_node() {
    let mut document = open_fixture();
    let text = document
        .resolve_data_binding(STORE_ITEM_ID, XPATH, |node, _interner| {
            node.children
                .iter()
                .find_map(|child| match child {
                    mjx_ooxml_core::RawNode::Text(bytes) => {
                        Some(String::from_utf8_lossy(bytes).into_owned())
                    }
                    _ => None,
                })
                .unwrap_or_default()
        })
        .expect("resolve_data_binding");
    assert_eq!(text, "Jane Doe");
}

#[test]
fn a_data_binding_naming_a_missing_part_reports_rather_than_panics() {
    let mut document = open_fixture();
    let result = document.resolve_data_binding(
        "{99999999-9999-9999-9999-999999999999}",
        XPATH,
        |_node, _interner| (),
    );
    assert!(matches!(
        result,
        Err(DocxError::DataBindingPartNotFound { store_item_id })
            if store_item_id == "{99999999-9999-9999-9999-999999999999}"
    ));
}

#[test]
fn a_data_binding_whose_xpath_does_not_resolve_reports_rather_than_panics() {
    let mut document = open_fixture();
    let result = document.resolve_data_binding(
        STORE_ITEM_ID,
        "/ns0:customer[1]/ns0:missing[1]",
        |_node, _interner| (),
    );
    assert!(matches!(
        result,
        Err(DocxError::DataBindingXPathNotFound { .. })
    ));
}

// =================================================================================================
// The glossary document's body reads through the same block-content API as the main body.
// =================================================================================================

#[test]
fn the_glossary_documents_building_block_body_reads_through_the_ordinary_body_api() {
    let mut document = open_fixture();
    let block: BuildingBlock = document
        .glossary_document(|glossary, interner| {
            glossary
                .doc_parts()
                .expect("docParts")
                .building_block(interner, "Cover Page 1")
                .expect("Cover Page 1 building block")
                .clone()
        })
        .expect("glossary_document")
        .expect("this fixture relates to a glossary document part");
    let body = block.body().expect("building block has a body");
    // The exact same Body API the main document uses — no glossary-specific accessor exists.
    assert_eq!(body.paragraph_count(), 1);
    assert_eq!(
        body.paragraph(0).expect("paragraph").text(),
        "Cover page building block text"
    );
}

#[test]
fn a_document_relating_to_no_glossary_part_reports_none_not_a_panic() {
    let mut document = Document::blank(mjx_docx::PageSize::a4()).expect("blank");
    assert_eq!(
        document
            .glossary_document(|_glossary, _interner| ())
            .expect("glossary_document"),
        None
    );
}

// =================================================================================================
// An altChunk's payload and relationship survive a save byte-identically, and its content type is
// reported.
// =================================================================================================

#[test]
fn an_alt_chunk_round_trips_its_payload_relationship_and_content_type() {
    let mut document = Document::blank(mjx_docx::PageSize::a4()).expect("blank");
    let payload = b"<html><body><p>Imported content.</p></body></html>".to_vec();
    let rid = document
        .add_alt_chunk(
            mjx_docx::constants::CONTENT_TYPE_ALT_CHUNK_HTML,
            payload.clone(),
        )
        .expect("add_alt_chunk");

    let (bytes, content_type) = document.alt_chunk_payload(&rid).expect("alt_chunk_payload");
    assert_eq!(bytes, payload.as_slice());
    assert_eq!(
        content_type,
        mjx_docx::constants::CONTENT_TYPE_ALT_CHUNK_HTML
    );

    let saved = document.save().expect("save");
    let reopened = Document::open(&saved).expect("reopen");
    let (reopened_bytes, reopened_content_type) = reopened
        .alt_chunk_payload(&rid)
        .expect("alt_chunk_payload after reopen");
    assert_eq!(
        reopened_bytes,
        payload.as_slice(),
        "the altChunk's own payload must survive a save byte-identically"
    );
    assert_eq!(
        reopened_content_type,
        mjx_docx::constants::CONTENT_TYPE_ALT_CHUNK_HTML
    );

    let parts = reopened.alt_chunk_parts().expect("alt_chunk_parts");
    assert_eq!(parts.len(), 1, "the relationship must survive the save");
    assert_eq!(parts[0].0, rid);
}

#[test]
fn an_alt_chunk_relationship_id_that_does_not_resolve_reports_rather_than_panics() {
    let document = Document::blank(mjx_docx::PageSize::a4()).expect("blank");
    let result = document.alt_chunk_payload("rIdNoSuchRelationship");
    assert!(matches!(
        result,
        Err(DocxError::AltChunkRelationshipNotFound { relationship_id })
            if relationship_id == "rIdNoSuchRelationship"
    ));
}

#[test]
fn a_document_with_an_alt_chunk_is_schema_valid() {
    let mut document = Document::blank(mjx_docx::PageSize::a4()).expect("blank");
    document
        .add_alt_chunk(
            mjx_docx::constants::CONTENT_TYPE_ALT_CHUNK_HTML,
            b"<p>hi</p>".to_vec(),
        )
        .expect("add_alt_chunk");
    let saved = document.save().expect("save");
    mjx_schema_gate::assert_authored_deck_is_schema_valid("document with an altChunk", &saved);
}
