//! The body as a **block tree**, tables resolved to plain numbers, and every `w:drawing`'s anchoring
//! (MJXOFF-176).
//!
//! # What this asserts that `residency.rs` cannot
//!
//! `residency.rs` proves the two orchestrations of the effective-property ladder agree paragraph by
//! paragraph. It says nothing about **order**, because a paragraph list has none to say: a document
//! whose tables were silently dropped has the same paragraphs as one that never had any. So this
//! suite asserts the shape — that a table sits between the two paragraphs the file put it between,
//! that its cells' paragraphs are in the same flat list and resolved through the same ladder, and
//! that a nested table is a `BlockFormatting::Table` inside a cell rather than a special case.

use mjx_docx::{BlockFormatting, Document, PageSize, PartName};

/// A document built from `word/document.xml` bytes, the way `mjx-layout-docx`'s own suites do.
fn document(body: &str) -> Document {
    let markup = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
<w:body>{body}<w:sectPr/></w:body>
</w:document>"#
    );
    let blank = Document::blank(PageSize::us_letter()).expect("a blank document");
    let bytes = blank.save_unchecked().expect("the blank saves");
    let mut package = mjx_docx::Package::open(&bytes).expect("the blank package opens");
    package
        .replace_part_bytes(
            &PartName::new("/word/document.xml").expect("a valid part name"),
            markup.into_bytes(),
        )
        .expect("the main document part is replaceable");
    Document::from_package(package).expect("the authored document opens")
}

/// One paragraph of `text`.
fn paragraph(text: &str) -> String {
    format!(r#"<w:p><w:r><w:t xml:space="preserve">{text}</w:t></w:r></w:p>"#)
}

#[test]
fn a_table_sits_between_the_paragraphs_the_file_put_it_between() {
    let body = format!(
        "{}<w:tbl><w:tblPr/><w:tblGrid><w:gridCol w:w=\"4000\"/></w:tblGrid>\
         <w:tr><w:tc>{}</w:tc></w:tr></w:tbl>{}",
        paragraph("before"),
        paragraph("inside"),
        paragraph("after"),
    );
    let mut document = document(&body);
    let formatting = document.formatting().expect("the document resolves");

    assert_eq!(
        formatting.top_level_paragraph_count(),
        2,
        "the two body paragraphs are the top level; the cell's is not"
    );
    assert_eq!(
        formatting.paragraphs().len(),
        3,
        "and all three live in one flat list, resolved in one pass"
    );

    let blocks = formatting.blocks();
    assert_eq!(blocks.len(), 3, "paragraph, table, paragraph");
    assert!(matches!(blocks[0], BlockFormatting::Paragraph(0)));
    assert!(matches!(blocks[2], BlockFormatting::Paragraph(1)));
    let BlockFormatting::Table(table) = &blocks[1] else {
        panic!("the middle block is the table: {blocks:?}");
    };
    assert_eq!(table.rows.len(), 1);
    assert_eq!(table.grid_twips, vec![4000]);

    let cell = &table.rows[0].cells[0];
    let BlockFormatting::Paragraph(index) = cell.content[0] else {
        panic!("the cell holds a paragraph");
    };
    assert_eq!(
        formatting.paragraphs()[index].text(),
        "inside",
        "and it indexes the same flat list the body's paragraphs are in"
    );
    assert!(
        index >= formatting.top_level_paragraph_count(),
        "a cell's paragraph is appended after the top-level ones, so a `w:sectPr` span still means \
         what it meant"
    );
}

#[test]
fn a_nested_table_is_a_block_inside_a_cell() {
    let inner = "<w:tbl><w:tblPr/><w:tblGrid><w:gridCol w:w=\"1000\"/></w:tblGrid>\
                 <w:tr><w:tc><w:p/></w:tc></w:tr></w:tbl>";
    let body = format!(
        "<w:tbl><w:tblPr/><w:tblGrid><w:gridCol w:w=\"4000\"/></w:tblGrid>\
         <w:tr><w:tc>{inner}</w:tc></w:tr></w:tbl>"
    );
    let mut document = document(&body);
    let formatting = document.formatting().expect("the document resolves");
    let BlockFormatting::Table(outer) = &formatting.blocks()[0] else {
        panic!("the body is a table");
    };
    let BlockFormatting::Table(nested) = &outer.rows[0].cells[0].content[0] else {
        panic!("and its cell holds another one");
    };
    assert_eq!(nested.grid_twips, vec![1000]);
}

#[test]
fn the_row_and_cell_attributes_a_page_break_reads_are_resolved() {
    let body = "<w:tbl><w:tblPr><w:tblLayout w:type=\"fixed\"/></w:tblPr>\
        <w:tblGrid><w:gridCol w:w=\"2000\"/><w:gridCol w:w=\"2000\"/></w:tblGrid>\
        <w:tr><w:trPr><w:tblHeader/><w:cantSplit/><w:trHeight w:val=\"500\" w:hRule=\"exact\"/></w:trPr>\
        <w:tc><w:tcPr><w:gridSpan w:val=\"2\"/><w:vMerge w:val=\"restart\"/></w:tcPr><w:p/></w:tc></w:tr>\
        <w:tr><w:tc><w:tcPr><w:vMerge/></w:tcPr><w:p/></w:tc><w:tc><w:p/></w:tc></w:tr></w:tbl>";
    let mut document = document(body);
    let formatting = document.formatting().expect("the document resolves");
    let BlockFormatting::Table(table) = &formatting.blocks()[0] else {
        panic!("the body is a table");
    };

    assert_eq!(
        table.layout,
        mjx_ooxml_types::wordprocessingml::TableLayoutType::Fixed
    );
    let first = &table.rows[0];
    assert!(first.repeat_as_header, "w:tblHeader");
    assert!(first.cannot_split, "w:cantSplit");
    let height = first.height.expect("w:trHeight");
    assert_eq!(height.twips, 500);
    assert_eq!(
        height.rule,
        mjx_ooxml_types::wordprocessingml::HeightRule::Exact
    );
    assert_eq!(first.cells[0].grid_span, 2, "w:gridSpan");
    assert_eq!(
        first.cells[0].vertical_merge_anchor,
        Some(true),
        "w:vMerge=\"restart\" is the anchor"
    );
    assert_eq!(
        table.rows[1].cells[0].vertical_merge_anchor,
        Some(false),
        "and a bare w:vMerge is a covered continuation"
    );
    assert_eq!(
        table.rows[1].cells[1].vertical_merge_anchor, None,
        "a cell in no merge says so"
    );
}

#[test]
fn an_anchored_drawing_resolves_to_numbers() {
    let body = r#"<w:p><w:r><w:drawing xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><wp:anchor distT="10" distB="20" distL="30" distR="40" simplePos="0" relativeHeight="7" behindDoc="1" locked="0" layoutInCell="1" allowOverlap="1"><wp:simplePos x="0" y="0"/><wp:positionH relativeFrom="page"><wp:align>right</wp:align></wp:positionH><wp:positionV relativeFrom="paragraph"><wp:posOffset>12345</wp:posOffset></wp:positionV><wp:extent cx="914400" cy="457200"/><wp:effectExtent l="1" t="2" r="3" b="4"/><wp:wrapTight wrapText="left" distL="5" distR="6"><wp:wrapPolygon edited="0"><wp:start x="0" y="0"/><wp:lineTo x="21600" y="0"/><wp:lineTo x="10800" y="21600"/><wp:lineTo x="0" y="0"/></wp:wrapPolygon></wp:wrapTight><wp:docPr id="1" name="Object"/><a:graphic><a:graphicData uri="urn:test"/></a:graphic></wp:anchor></w:drawing></w:r></w:p>"#;
    let mut document = document(body);
    let formatting = document.formatting().expect("the document resolves");
    let drawings = formatting.paragraphs()[0].drawings();
    assert_eq!(drawings.len(), 1);
    let drawing = &drawings[0];
    assert_eq!(drawing.width, 914_400);
    assert_eq!(drawing.height, 457_200);

    let mjx_docx::DrawingPlacement::Anchored(anchor) = &drawing.placement else {
        panic!("it floats");
    };
    assert!(anchor.behind_text, "@behindDoc");
    assert_eq!(anchor.relative_height, 7);
    assert_eq!(anchor.distance.top, 10);
    assert_eq!(anchor.effect_extent.left, 1);
    assert_eq!(
        anchor.horizontal.relative_to,
        mjx_ooxml_types::wordprocessingdrawing::HorizontalRelativeFrom::Page
    );
    assert!(matches!(
        anchor.horizontal.placement,
        mjx_docx::AxisPlacement::Aligned(
            mjx_ooxml_types::wordprocessingdrawing::HorizontalAlignment::Right
        )
    ));
    assert!(matches!(
        anchor.vertical.placement,
        mjx_docx::AxisPlacement::Offset(12_345)
    ));

    let mjx_docx::WrapFormatting::Tight {
        side,
        polygon,
        distance_left,
        distance_right,
    } = &anchor.wrap
    else {
        panic!("it wraps tight: {:?}", anchor.wrap);
    };
    assert_eq!(
        *side,
        mjx_ooxml_types::wordprocessingdrawing::WrapText::Left
    );
    assert_eq!(*distance_left, 5);
    assert_eq!(*distance_right, 6);
    // **The coordinates travel exactly as the file wrote them.** What unit they are in is a layout
    // reading and lives in `mjx_layout_docx::wrap`, beside the rest of them.
    assert_eq!(
        polygon,
        &vec![(0, 0), (21_600, 0), (10_800, 21_600), (0, 0)]
    );
}

#[test]
fn an_inline_drawing_is_reported_as_inline() {
    let body = r#"<w:p><w:r><w:drawing xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><wp:inline distT="0" distB="0" distL="0" distR="0"><wp:extent cx="100" cy="200"/><wp:docPr id="1" name="Object"/><a:graphic><a:graphicData uri="urn:test"/></a:graphic></wp:inline></w:drawing></w:r></w:p>"#;
    let mut document = document(body);
    let formatting = document.formatting().expect("the document resolves");
    let drawing = &formatting.paragraphs()[0].drawings()[0];
    assert_eq!((drawing.width, drawing.height), (100, 200));
    assert!(matches!(
        drawing.placement,
        mjx_docx::DrawingPlacement::Inline(_)
    ));
}

#[test]
fn a_header_carries_its_own_block_tree() {
    // A header with no table in it still answers `blocks()`, and its indices are stream-local — a
    // caller indexes `HeaderFooterFormatting::paragraphs` and never the document's own list.
    let mut document = document(&paragraph("body"));
    let formatting = document.formatting().expect("the document resolves");
    for stream in formatting.header_footer_streams() {
        for block in stream.blocks() {
            if let BlockFormatting::Paragraph(index) = block {
                assert!(
                    *index < stream.paragraphs().len(),
                    "a header's block indexes its own paragraphs"
                );
            }
        }
    }
}
