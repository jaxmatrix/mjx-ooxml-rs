//! One document driven through **every subject of the `Document` surface**, importing only
//! `mjx_ooxml::…` — the Word sibling of `public_paths.rs`.
//!
//! `Document` is split into subject modules — text, effective properties, styles, numbering,
//! sections, headers/footers, tables, fields, hyperlinks, comments, notes (footnotes/endnotes/
//! revisions), drawings. The split is only correct if none of it is visible from outside: every
//! method must be an inherent method on the one re-exported `Document`, reachable by the path a
//! caller writes, and every type its signatures name must be nameable through `mjx_ooxml::` alone.
//!
//! Deleting a Word method from `mjx_ooxml`'s re-exports, or a supporting type its signature needs,
//! stops this file compiling.

use mjx_ooxml::{
    Document, EffectiveCharacterProperties, EffectiveParagraphProperties, ErrorCode, Field,
    GridDiscrepancy, HeaderFooterType, HyperlinkTarget, MergedCellType, PageMargins,
    PageOrientation, PageSize, RevisionInfo, SectionLocation,
};

fn fixture(name: &str) -> Vec<u8> {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()))
}

/// Every subject the surface is split into, exercised on one authored document through
/// `mjx_ooxml::`.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one document driven through every subject; splitting it would stop proving they share one Document"
)]
fn every_subject_of_the_facade_is_reachable_on_the_re_exported_document() {
    // --- lifecycle --------------------------------------------------------------------------------
    let mut document = Document::blank(PageSize::a4()).expect("a blank document");
    assert_eq!(document.format(), mjx_ooxml::Format::Document);
    assert!(document
        .conformance()
        .expect("a conformance read")
        .is_none());

    // --- text: paragraphs and runs ------------------------------------------------------------------
    assert_eq!(document.paragraph_count().expect("a count"), 1);
    document.append_paragraph().expect("a second paragraph");
    document.append_run(1, "Hello, ").expect("a run");
    document.append_run(1, "document.").expect("a second run");
    assert_eq!(document.run_count(1).expect("a run count"), 2);
    document
        .insert_run(1, 2, "!")
        .expect("inserting a third run");
    assert_eq!(
        document.paragraph_text(1).expect("the paragraph text"),
        "Hello, document.!"
    );
    document.set_run_text(1, 0, "Hi, ").expect("editing a run");
    assert_eq!(document.run_text(1, 0).expect("the edited run"), "Hi, ");
    document.remove_run(1, 2).expect("removing the third run");
    document.insert_paragraph(0).expect("inserting a paragraph");
    document.remove_paragraph(0).expect("removing it again");

    // --- effective properties ------------------------------------------------------------------------
    let run_props: EffectiveCharacterProperties = document
        .effective_run_properties(1, 0)
        .expect("effective run properties");
    assert!(run_props.bold.is_none(), "a blank document sets no bold");
    let paragraph_props: EffectiveParagraphProperties = document
        .effective_paragraph_properties(1)
        .expect("effective paragraph properties");
    let _ = paragraph_props.keep_with_next;

    // --- styles (read-only) ---------------------------------------------------------------------------
    assert!(
        document.style_ids().expect("style ids").is_empty(),
        "a blank document has no word/styles.xml"
    );
    assert_eq!(document.style_name("Normal").expect("a style lookup"), None);

    // --- numbering --------------------------------------------------------------------------------
    // `w:numPr` is written into the paragraph's own `w:pPr` directly; resolving it against
    // `word/numbering.xml` is a separate step, so attaching one needs no numbering definitions part
    // to exist yet.
    document
        .attach_paragraph_to_list(1, 1, 0)
        .expect("attaching a numbering reference");
    document
        .detach_paragraph_from_list(1)
        .expect("detaching is always a safe no-op");

    // --- sections and headers/footers ----------------------------------------------------------------
    let sections = document.sections().expect("the sections");
    assert_eq!(sections.len(), 1);
    assert_eq!(document.section_count().expect("a section count"), 1);
    let size: Option<PageSize> = sections[0].page_size;
    assert!(size.is_some(), "Document::blank writes a body-level w:pgSz");
    let margins: Option<PageMargins> = sections[0].page_margins;
    assert!(margins.is_some());

    document
        .set_section_page_size(SectionLocation::Body, Some(PageSize::us_letter()))
        .expect("resizing the section");
    let resized = document.sections().expect("the resized sections");
    assert_eq!(
        resized[0].page_size.expect("a page size").orientation,
        PageOrientation::Portrait
    );

    assert!(!document.even_and_odd_headers().expect("the flag"));
    assert_eq!(
        document
            .header_text(0, HeaderFooterType::Default)
            .expect("no header yet"),
        None
    );
    document
        .set_header_text(
            SectionLocation::Body,
            HeaderFooterType::Default,
            "Header text",
        )
        .expect("creating a header");
    assert_eq!(
        document
            .header_text(0, HeaderFooterType::Default)
            .expect("reading the header"),
        Some("Header text".to_owned())
    );
    document
        .set_footer_text(
            SectionLocation::Body,
            HeaderFooterType::Default,
            "Footer text",
        )
        .expect("creating a footer");
    assert_eq!(
        document
            .footer_text(0, HeaderFooterType::Default)
            .expect("reading the footer"),
        Some("Footer text".to_owned())
    );
    document
        .remove_header(SectionLocation::Body, HeaderFooterType::Default)
        .expect("removing the header");
    document
        .remove_footer(SectionLocation::Body, HeaderFooterType::Default)
        .expect("removing the footer");

    // --- tables -------------------------------------------------------------------------------------
    let table = document.append_table(2, 2).expect("a table");
    assert_eq!(document.table_count().expect("a table count"), 1);
    let (rows, columns) = document
        .table_dimensions(table)
        .expect("the table dimensions");
    assert_eq!((rows, columns), (2, 2));
    document
        .set_cell_text(table, 0, 0, "top-left")
        .expect("setting a cell");
    assert_eq!(
        document.cell_text(table, 0, 0).expect("the cell text"),
        "top-left"
    );
    let (row_span, column_span) = document
        .cell_span(table, 0, 0)
        .expect("the cell's own span");
    assert_eq!((row_span, column_span), (1, 1));
    document
        .set_cell_span(table, 0, 0, Some(2))
        .expect("merging horizontally");
    let (anchor_row, anchor_column) = document
        .merged_cell_anchor(table, 0, 1)
        .expect("the merge anchor");
    assert_eq!((anchor_row, anchor_column), (0, 0));
    document
        .set_cell_span(table, 0, 0, None)
        .expect("undoing the merge");
    document
        .set_cell_vertical_merge(table, 0, 0, Some(MergedCellType::Restart))
        .expect("starting a vertical merge");
    document
        .set_cell_vertical_merge(table, 1, 0, Some(MergedCellType::Continue))
        .expect("continuing it");
    let discrepancies: Vec<GridDiscrepancy> = document
        .table_grid_discrepancies(table)
        .expect("no discrepancies on a well-formed table");
    assert!(discrepancies.is_empty());
    document.insert_row(table, 2).expect("appending a row");
    document
        .insert_column(table, 2)
        .expect("appending a column");
    document.remove_column(table, 2).expect("removing it again");
    document.remove_row(table, 2).expect("removing the row");
    document.remove_table(table).expect("removing the table");

    // --- fields ---------------------------------------------------------------------------------------
    let fields: Vec<Field> = document.fields(1).expect("no fields yet");
    assert!(fields.is_empty());

    // --- hyperlinks -------------------------------------------------------------------------------------
    // Paragraph 1 holds two runs at this point ("Hi, " and "document."), so `2` is the append slot.
    let hyperlink_url = "https://example.com/mjx-ooxml-rs";
    document
        .insert_hyperlink(
            1,
            2,
            "example",
            &HyperlinkTarget::Url(hyperlink_url.to_owned()),
        )
        .expect("inserting a hyperlink");
    let target = document
        .hyperlink_target(1, 2)
        .expect("reading it")
        .expect("a stated target");
    assert_eq!(target, HyperlinkTarget::Url(hyperlink_url.to_owned()));
    document.remove_hyperlink(1, 2).expect("removing it");

    // --- comments -------------------------------------------------------------------------------------
    let comment_id = document
        .add_comment(1, "Reviewer", Some("R"), "a remark")
        .expect("adding a comment");
    let comments = document.comments().expect("the comments");
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0].author, "Reviewer");
    assert_eq!(
        document
            .comment_range_text(comment_id)
            .expect("the resolved range")
            .as_deref(),
        Some("Hi, document.")
    );
    document.remove_comment(comment_id).expect("removing it");

    // --- footnotes, endnotes and revisions ------------------------------------------------------------
    let footnote_id = document
        .add_footnote(1, "a note")
        .expect("adding a footnote");
    assert_eq!(document.footnotes().expect("the footnotes").len(), 1);
    document.remove_footnote(footnote_id).expect("removing it");
    let endnote_id = document
        .add_endnote(1, "an endnote")
        .expect("adding an endnote");
    assert_eq!(document.endnotes().expect("the endnotes").len(), 1);
    document.remove_endnote(endnote_id).expect("removing it");
    let revisions: Vec<RevisionInfo> = document.revisions().expect("no tracked changes yet");
    assert!(revisions.is_empty());
    assert_eq!(
        document
            .text_with_revisions_accepted()
            .expect("accepted text"),
        document
            .text_with_revisions_rejected()
            .expect("rejected text"),
        "with no tracked changes both readings agree"
    );

    // --- drawings ---------------------------------------------------------------------------------------
    let doc_pr_id = document
        .add_inline_picture(
            1,
            vec![0x89, b'P', b'N', b'G'],
            "image/png",
            "png",
            100,
            100,
            "pic",
        )
        .expect("adding a picture");
    assert!(document
        .remove_drawing(doc_pr_id)
        .expect("removing the picture"));
    assert!(!document
        .remove_drawing(doc_pr_id)
        .expect("removing it again is a no-op"));

    // --- save ------------------------------------------------------------------------------------------
    document.validate().expect("the document is still valid");
    let bytes = document.save().expect("saving");
    assert!(!bytes.is_empty());

    // --- reading a real fixture reaches every path detect_format opened it under ------------------------
    let mut from_disk = Document::open(&fixture("sample.docx")).expect("opening the fixture");
    assert!(from_disk.paragraph_count().expect("a count") > 0);
    let refused = mjx_ooxml::detect_format(&fixture("sample.pptx"))
        .map(|format| format.family())
        .expect("detecting a presentation");
    assert_eq!(refused, mjx_ooxml::FormatFamily::Presentation);
    let _: ErrorCode = Document::open(&fixture("sample.pptx"))
        .expect_err("a presentation is not a Word document")
        .code();
}
